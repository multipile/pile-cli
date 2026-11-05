//! Reading back what git prints.
//!
//! Each reader takes one command's output, and refuses output it does not
//! understand rather than guessing at it. Paths come back as git spells
//! them, from the top and without a leading `/`, except where a reader
//! says otherwise.

use crate::{Change, ChangeKind, Error, FileKind, Result, Revision, RevisionInfo, Touched};

/// Splits output that git ended each record of with a NUL.
///
/// Ended, not separated: the last record has a NUL after it too, so the
/// empty string after it is not a record.
pub(super) fn fields(out: &[u8]) -> Vec<String> {
    out.split(|&b| b == 0)
        .filter(|f| !f.is_empty())
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect()
}

/// The kind of file a git mode says. None for a mode that is not a file,
/// such as a directory's.
pub(super) fn kind(mode: &str) -> Option<FileKind> {
    match mode {
        "100644" => Some(FileKind::File),
        "100755" => Some(FileKind::Executable),
        "120000" => Some(FileKind::Symlink),
        "160000" => Some(FileKind::Submodule),
        _ => None,
    }
}

/// One file a commit records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Listed {
    /// From the top, with a leading `/`.
    pub path: String,
    pub kind: FileKind,
    /// git's object id for what the file holds. It never leaves the module.
    pub oid: String,
}

/// Reads `ls-tree -r -z`: `<mode> <type> <oid>\t<path>` for each file.
pub(super) fn tree(recs: &[String]) -> Result<Vec<Listed>> {
    let mut out = Vec::new();
    for rec in recs {
        let Some((head, path)) = rec.split_once('\t') else {
            return Err(Error::Failed(format!(
                "git ls-tree said {rec:?}, which has no path in it"
            )));
        };
        let words: Vec<&str> = head.split(' ').collect();
        let [mode, _, oid] = words[..] else {
            return Err(Error::Failed(format!(
                "git ls-tree said {head:?}, which is not a mode, a type and an object"
            )));
        };
        let Some(kind) = kind(mode) else {
            return Err(Error::Failed(format!(
                "git ls-tree said {mode:?} of {path:?}, which is not a mode of a file"
            )));
        };
        out.push(Listed {
            path: format!("/{path}"),
            kind,
            oid: oid.to_string(),
        });
    }
    Ok(out)
}

/// Reads `cat-file --batch`: for each name asked, a line `<oid> <type>
/// <size>`, then that many bytes and a newline. A name the store does not
/// hold is one line ending in `missing`, or `ambiguous`.
///
/// Only a blob is a file's content. Any other object is read past, and
/// answers None.
pub(super) fn batch(out: &[u8], asked: usize) -> Result<Vec<Option<Vec<u8>>>> {
    let mut got = Vec::with_capacity(asked);
    let mut at = 0;
    for _ in 0..asked {
        let Some(nl) = out[at..].iter().position(|&b| b == b'\n') else {
            return Err(Error::Failed(
                "git cat-file ended before it answered every name".to_string(),
            ));
        };
        let header = String::from_utf8_lossy(&out[at..at + nl]).into_owned();
        at += nl + 1;
        if header.ends_with(" missing") || header.ends_with(" ambiguous") {
            got.push(None);
            continue;
        }
        let words: Vec<&str> = header.rsplitn(3, ' ').collect();
        let [size, kind, _] = words[..] else {
            return Err(Error::Failed(format!(
                "git cat-file said {header:?}, which is not an object"
            )));
        };
        let size: usize = size.parse().map_err(|_| {
            Error::Failed(format!(
                "git cat-file said {header:?}, whose size is not a number"
            ))
        })?;
        let Some(body) = out.get(at..at + size) else {
            return Err(Error::Failed(format!(
                "git cat-file ended inside {header:?}"
            )));
        };
        got.push((kind == "blob").then(|| body.to_vec()));
        // The newline after the content.
        at += size + 1;
    }
    Ok(got)
}

/// Reads `--raw -z` from `diff` or `diff-tree`:
///
/// ```text
/// :<mode> <mode> <oid> <oid> <status> NUL <path> NUL [<path> NUL]
/// ```
///
/// A move or a copy has a second path, where it went, so the cursor moves
/// three records past one and two past anything else. A copy is read as an
/// addition, because nothing left the path it was copied from.
pub(super) fn raw(recs: &[String]) -> Result<Vec<Change>> {
    Ok(raw_records(recs)?.into_iter().map(|r| r.change).collect())
}

/// One record of `--raw`: the change, and both ends' modes and object ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Raw {
    pub change: Change,
    pub from_mode: String,
    pub to_mode: String,
    pub from_oid: String,
    /// All zeros for a file in the working copy that git has not hashed.
    pub to_oid: String,
}

/// Reads `--raw -z` as [`raw`] does, keeping each record's modes and ids.
pub(super) fn raw_records(recs: &[String]) -> Result<Vec<Raw>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < recs.len() {
        let head = &recs[i];
        let words: Vec<&str> = head.trim_start_matches(':').split(' ').collect();
        let [from_mode, to_mode, from_oid, to_oid, status] = words[..] else {
            return Err(Error::Failed(format!(
                "git diff said {head:?}, which is not two modes, two objects and a status"
            )));
        };
        let Some(path) = recs.get(i + 1) else {
            return Err(Error::Failed(format!(
                "git diff ended at {head:?}, with no path after it"
            )));
        };
        i += 2;
        let retyped = match (kind(from_mode), kind(to_mode)) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        };
        let letter = status.chars().next().unwrap_or(' ');
        let mut push = |change: Change| {
            out.push(Raw {
                change,
                from_mode: from_mode.to_string(),
                to_mode: to_mode.to_string(),
                from_oid: from_oid.to_string(),
                to_oid: to_oid.to_string(),
            })
        };
        match letter {
            'A' => push(Change::new(path, ChangeKind::Added)),
            'D' => push(Change::new(path, ChangeKind::Deleted)),
            // T is git's change of type: a file that became a symbolic link,
            // or the reverse. It is a modification of a different kind.
            'M' | 'T' => push(Change {
                retyped: retyped || letter == 'T',
                ..Change::new(path, ChangeKind::Modified)
            }),
            'U' => push(Change::new(path, ChangeKind::Conflicted)),
            'R' | 'C' => {
                let Some(to) = recs.get(i) else {
                    return Err(Error::Failed(format!(
                        "git diff ended in the middle of the move of {path:?}"
                    )));
                };
                i += 1;
                push(if letter == 'R' {
                    Change {
                        from: Some(path.clone()),
                        retyped,
                        ..Change::new(to, ChangeKind::Moved)
                    }
                } else {
                    Change::new(to, ChangeKind::Added)
                });
            }
            _ => {
                return Err(Error::Failed(format!(
                    "git diff said {status:?} of {path:?}, which this does not understand"
                )));
            }
        }
    }
    Ok(out)
}

/// Reads `ls-files -u -z`: `<mode> <oid> <stage>\t<path>` for each stage of
/// each path in conflict. Each path comes back once, as git spells it.
pub(super) fn unmerged(recs: &[String]) -> Vec<String> {
    let mut out: Vec<String> = recs
        .iter()
        .filter_map(|r| r.split_once('\t').map(|(_, p)| p.to_string()))
        .collect();
    out.dedup();
    out
}

/// Reads `git log` printed with the module's FORMAT, `--name-only` and
/// `-z`.
///
/// Each revision starts with a record separator. Its six fields follow,
/// each ended by a unit separator. Then come a NUL, a newline, and each path
/// it changed ended by a NUL.
pub(super) fn log(out: &[u8]) -> Result<Vec<Touched>> {
    let text = String::from_utf8_lossy(out);
    let mut got = Vec::new();
    for chunk in text.split('\x1e').filter(|c| !c.is_empty()) {
        let parts: Vec<&str> = chunk.splitn(7, '\x1f').collect();
        let [id, short, at, date, author, subject, rest] = parts[..] else {
            return Err(Error::Failed(format!(
                "git log said {chunk:?}, which is not an id, a short id, a time, a date, an author and a subject"
            )));
        };
        let at: i64 = at.parse().map_err(|_| {
            Error::Failed(format!("git log said {at:?} of {id}, which is not a time"))
        })?;
        let offset = offset(date).ok_or_else(|| {
            Error::Failed(format!(
                "git log said {date:?} of {id}, which does not end in an offset from UTC"
            ))
        })?;
        let paths = rest
            .split('\0')
            .map(|p| p.trim_start_matches('\n'))
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        got.push(Touched {
            info: RevisionInfo {
                revision: Revision::new(id),
                short: short.to_string(),
                author: author.to_string(),
                at,
                offset,
                subject: subject.to_string(),
            },
            paths,
        });
    }
    Ok(got)
}

/// The offset from UTC at the end of a date git printed with `%ai`, such as
/// `2026-10-27 10:00:00 +0200`, in seconds.
fn offset(date: &str) -> Option<i32> {
    let zone = date.rsplit(' ').next()?;
    let (sign, digits) = match zone.split_at_checked(1)? {
        ("+", d) => (1, d),
        ("-", d) => (-1, d),
        _ => return None,
    };
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hours: i32 = digits[..2].parse().ok()?;
    let minutes: i32 = digits[2..].parse().ok()?;
    Some(sign * (hours * 3600 + minutes * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recs(r: &[&str]) -> Vec<String> {
        r.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn raw_records_read_as_changes_with_where_a_move_came_from() {
        let got = raw(&recs(&[
            ":100644 100644 aaaa bbbb M",
            "a/y.txt",
            ":000000 100644 0000 cccc A",
            "b/n.txt",
            ":100644 100644 dddd dddd R100",
            "a/x.txt",
            "b/x.txt",
            ":100644 100755 eeee eeee M",
            "bin/run",
            ":100644 120000 ffff 0000 T",
            "link",
            ":100644 000000 1111 0000 D",
            "gone",
            ":100644 100644 2222 2222 C100",
            "a/z",
            "b/z",
        ]))
        .unwrap();
        assert_eq!(
            got,
            vec![
                Change::new("a/y.txt", ChangeKind::Modified),
                Change::new("b/n.txt", ChangeKind::Added),
                Change {
                    from: Some("a/x.txt".into()),
                    ..Change::new("b/x.txt", ChangeKind::Moved)
                },
                Change {
                    retyped: true,
                    ..Change::new("bin/run", ChangeKind::Modified)
                },
                Change {
                    retyped: true,
                    ..Change::new("link", ChangeKind::Modified)
                },
                Change::new("gone", ChangeKind::Deleted),
                Change::new("b/z", ChangeKind::Added),
            ]
        );
    }

    #[test]
    fn raw_output_cut_short_is_refused() {
        assert!(raw(&recs(&[":100644 100644 a b M"])).is_err());
        assert!(raw(&recs(&[":100644 100644 a b R100", "a"])).is_err());
        assert!(raw(&recs(&["what"])).is_err());
    }

    #[test]
    fn a_batch_is_read_by_the_sizes_it_declares() {
        let out = b"aaaa blob 4\nkind\nbbbb blob 0\n\ncccc tree 3\nxyz\nr:p missing\n";
        let got = batch(out, 4).unwrap();
        assert_eq!(
            got,
            vec![Some(b"kind".to_vec()), Some(Vec::new()), None, None]
        );
        assert!(batch(b"aaaa blob 9\nshort\n", 1).is_err());
    }

    #[test]
    fn a_tree_listing_reads_each_mode_as_a_kind() {
        let got = tree(&recs(&[
            "100644 blob aaaa\ta b",
            "100755 blob bbbb\tbin/run",
            "160000 commit cccc\tvendor/x",
        ]))
        .unwrap();
        let kinds: Vec<(&str, FileKind)> = got.iter().map(|l| (l.path.as_str(), l.kind)).collect();
        assert_eq!(
            kinds,
            [
                ("/a b", FileKind::File),
                ("/bin/run", FileKind::Executable),
                ("/vendor/x", FileKind::Submodule)
            ]
        );
        assert!(tree(&recs(&["040000 tree dddd\tdir"])).is_err());
    }

    #[test]
    fn a_log_reads_each_revision_and_its_paths_and_keeps_an_empty_subject() {
        let out = "\x1eaaaa\x1faa\x1f100\x1f1970-01-01 02:31:40 +0230\x1fAna\x1fmove it\x1f\0\na\0z\0\
                   \x1ebbbb\x1fbb\x1f50\x1f1969-12-31 19:00:50 -0500\x1fBo\x1f\x1f\0\nx/.pile/config\0";
        let got = log(out.as_bytes()).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].info.revision.as_str(), "aaaa");
        assert_eq!(got[0].info.at, 100);
        assert_eq!(got[0].info.offset, 9000);
        assert_eq!(got[1].info.offset, -18_000);
        assert!(log(b"\x1eaaaa\x1faa\x1f100\x1f1970-01-01\x1fAna\x1fs\x1f").is_err());
        assert_eq!(got[0].paths, ["a", "z"]);
        assert_eq!(got[1].info.subject, "");
        assert_eq!(got[1].info.author, "Bo");
        assert_eq!(got[1].paths, ["x/.pile/config"]);
    }

    #[test]
    fn each_path_in_conflict_is_named_once() {
        let got = unmerged(&recs(&[
            "100644 aaaa 1\tz",
            "100644 bbbb 2\tz",
            "100644 cccc 3\tz",
            "100644 dddd 2\ty",
        ]));
        assert_eq!(got, ["z", "y"]);
    }
}
