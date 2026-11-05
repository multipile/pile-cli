use clap::{ArgMatches, Command};
use serde_json::{Value, json};

use piles::measure::{Length, measure};
use storage::{FileKind, State};

use super::{Out, Result, name_of, one_path, open, path_arg, plural, target};

pub(super) fn command() -> Command {
    Command::new("files")
        .about("List one pile's own files, and how long each is")
        .long_about(
            "List one pile's own files, each as a :/ path, with the length of each\n\
             in lines and in bytes, and the totals under them.\n\n\
             A pile's own files are its directory, less every pile it holds. So git\n\
             ls-files of the directory also lists files that are another pile's, and\n\
             files does not. It names the piles under this one at the end, each as\n\
             the command that lists its files.\n\n\
             A file is one git can see: tracked, or untracked and not ignored. Its\n\
             length is what is on disk now, so you can say what reading the pile\n\
             will cost before you read it.\n  \
             - Lines are what wc -l counts, and a last line with no newline counts.\n  \
             - A file with a NUL in its first 8000 bytes is binary, as git says, and\n    \
             has no lines.\n  \
             - A symbolic link is as long as the path it points to.\n  \
             - A submodule is another repository, and is not measured.\n\
             Files are in path order, byte by byte.\n\n\
             The pile is the one named, or the one you are in. A path you name has\n\
             to be a pile.",
        )
        .override_usage("pile files [pile]")
        .after_help(format!(
            "Examples:\n  \
             pile files            the pile you are in\n  \
             pile files @lib       one typed by its name\n  \
             pile files :/         the top pile's own files, and none of its children's\n\n\
             Refusals:\n{}",
            target::REFUSALS
        ))
        .arg(path_arg())
}

/// The result is the pile, `name`, `path` and `id`; `files`, each as `path`
/// as the pile names it, `lines` and `bytes`; `total`, the `files`, `lines`
/// and `bytes` the last line counts; and `not_counted`, the piles under it,
/// each `name` and `path`, each of which is also a `next`.
///
/// `lines` is a number, or the word the text prints in its place: `binary`,
/// or `link`. It is `null` for a submodule and for a file that could not be
/// read. `bytes` is `null` for both of those.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    let arg = one_path("files", m)?;
    let r = open()?;
    let s = target::declared(&r)?;
    let pile = target::chosen(&r, &s, arg.as_deref())?;
    let storage = r.storage();

    let mut files = Vec::new();
    for f in storage.files(&State::Working, &s.own(&pile.path))? {
        let length = match f.kind {
            FileKind::Submodule => Length::Submodule,
            kind => match storage.read(&State::Working, &[f.path.as_str()]) {
                Ok(mut got) => match got.pop().flatten() {
                    Some(bytes) => measure(kind, &bytes),
                    // Gone since it was listed: there is nothing to read.
                    None => continue,
                },
                Err(_) => Length::Unreadable,
            },
        };
        files.push((piles::shown(&f.path), length));
    }

    putln!(out, "{}\n", pile.heading());
    let children = s.children(&pile.path);
    let lines: usize = files.iter().filter_map(|(_, l)| l.lines()).sum();
    let bytes: u64 = files.iter().filter_map(|(_, l)| l.bytes()).sum();
    if files.is_empty() {
        putln!(out, "nothing in it that git can see");
    } else {
        let cols: Vec<(String, String)> = files.iter().map(|(_, l)| columns(l)).collect();
        let lw = 8.max(cols.iter().map(|(l, _)| l.len()).max().unwrap_or(0) + 2);
        let bw = 8.max(cols.iter().map(|(_, b)| b.len()).max().unwrap_or(0) + 2);
        putln!(out, "{:>lw$}{:>bw$}  path", "lines", "bytes");
        for ((path, _), (l, b)) in files.iter().zip(&cols) {
            putln!(out, "{l:>lw$}{b:>bw$}  {path}");
        }
        putln!(
            out,
            "\n{}, {}, {}",
            plural(files.len(), "file"),
            plural(lines, "line"),
            plural(bytes as usize, "byte")
        );
    }
    if !children.is_empty() {
        let them: Vec<String> = children
            .iter()
            .map(|c| format!("pile files {}", c.handle()))
            .collect();
        putln!(
            out,
            "not counted, being piles of their own: {}",
            them.join(", ")
        );
    }
    for c in &children {
        out.next(format!("pile files {}", c.handle()), "a pile of its own");
    }

    out.result(json!({
        "name": name_of(&pile.name),
        "path": piles::shown(&pile.path),
        "id": pile.id,
        "files": files
            .iter()
            .map(|(path, l)| json!({"path": path, "lines": lines_value(l), "bytes": l.bytes()}))
            .collect::<Vec<_>>(),
        "total": {"files": files.len(), "lines": lines, "bytes": bytes},
        "not_counted": children
            .iter()
            .map(|c| json!({"name": name_of(&c.name), "path": piles::shown(&c.path)}))
            .collect::<Vec<_>>(),
    }));
    Ok(())
}

/// A file's two columns: what is known of its lines, and its bytes.
fn columns(l: &Length) -> (String, String) {
    let bytes = l.bytes().map_or("-".to_string(), |b| b.to_string());
    let lines = match l {
        Length::Text { lines, .. } => lines.to_string(),
        Length::Binary { .. } => "binary".to_string(),
        Length::Link { .. } => "link".to_string(),
        Length::Submodule | Length::Unreadable => "-".to_string(),
    };
    (lines, bytes)
}

fn lines_value(l: &Length) -> Value {
    match l {
        Length::Text { lines, .. } => json!(lines),
        Length::Binary { .. } => json!("binary"),
        Length::Link { .. } => json!("link"),
        Length::Submodule | Length::Unreadable => Value::Null,
    }
}
