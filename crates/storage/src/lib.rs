//! The read-only interface every read of a repository goes through.
//!
//! A command asks a [`Storage`] what it needs to know, and never asks git
//! itself. So no command knows which backend answers it. There are two
//! backends:
//!
//! - [`git`] asks the `git` command, which has to be on PATH.
//! - [`memory`] holds a repository in memory, for unit tests. Nothing in it
//!   is git, so it is also the check that the interface has no git in it.
//!
//! [`open`] picks the backend for a directory. Today that is always git.
//!
//! # What a command can ask
//!
//! Each method of [`Storage`] answers one question a command has:
//!
//! - which revision the working copy stands on, and which revision a name
//!   names
//! - the piles' declaration files, in the working copy or at a revision
//! - the files under a path, and what one of them holds
//! - what changed between two revisions, or between a revision and the
//!   working copy
//! - the revisions that changed a part of the repository
//! - the revisions a base is chosen from
//!
//! Nothing here writes. `pile` makes no commit, and the only files it writes
//! are `.pile/` files in the working copy, which it writes on disk itself,
//! at [`Storage::working_copy`].
//!
//! # Revisions
//!
//! A [`Revision`] is an id the backend chose. A git commit's id is one kind,
//! and a version number on a server would be another. A caller may print it
//! and hand it back, but it never reads anything into it. Only a backend
//! makes one, so a caller gets a revision from [`Storage::head`],
//! [`Storage::resolve`] or an answer that names one. How a backend addresses
//! a file's content is its own, and nothing here exposes it.
//!
//! # The working copy
//!
//! The working copy is the files as they are now, before anything records
//! them. It is where a developer works, so every question that takes a
//! [`State`] can be asked of it as well as of a revision. A file in the
//! working copy is either tracked, which means the next revision would carry
//! it, or untracked. A file the backend is told to ignore is not in the
//! working copy at all.
//!
//! # Paths
//!
//! Every path here starts with `/`, which is the top of the repository. It
//! is written the way a pile's path is: `/crates/storage/src/lib.rs`. A path
//! names a file, and a [`Subtree`] names a part of the repository.
//!
//! # Not here yet
//!
//! Two later features read a repository, and neither is built yet:
//!
//! - A query asked at a revision reads the declarations at that revision,
//!   and the files they name. It needs nothing new: [`Storage::declarations`]
//!   and [`Storage::read`] take a revision, and [`Storage::changes`] compares
//!   two.
//! - The browser's changes view compares two ends. It shows what one
//!   revision changed, so it needs that revision's parent, which
//!   [`RevisionInfo`] can gain as a field. It also compares snapshots of the
//!   working copy, as the next section says.
//!
//! # Snapshots of the working copy
//!
//! `pile ui` keeps a trail of snapshots of the working copy, so the changes
//! view can compare one moment with another. Snapshots are decided, and not
//! built yet. Taking one writes nothing, so `pile` writes nothing in the
//! browser either.
//!
//! The changes view shows paths and what happened to each, never lines. So
//! a snapshot needs no file's content, only a way to tell two contents
//! apart:
//!
//! - A snapshot is a map from each path in the working copy to its mode and
//!   git's blob id. `pile ui` holds it in memory, and drops it when it
//!   exits.
//! - The ids come from `git hash-object --stdin-paths`, without `-w`. That
//!   writes nothing, and it applies the filters `.gitattributes` names, so an
//!   id matches the one `git ls-tree -r <revision>` lists for the same
//!   content.
//! - A file is hashed again only when its size or its modification time
//!   changed since the snapshot before.
//! - Two snapshots are compared by comparing their maps. A snapshot is
//!   compared with a revision the same way, against the revision's
//!   `ls-tree`.
//! - Equal ids find a move whose file did not change on the way. Finding a
//!   move whose file changed a little reads the contents of the added and
//!   the deleted paths, and of no other path.
//!
//! A snapshot is not part of this interface, and neither is the file watcher
//! that says when to take one. Both work on the files on disk and on git
//! alone. A blob id names content the way git does, so it never leaves the
//! git backend and `pile ui`. A library that reads git directly, such as
//! gix or git2, is not needed for any of this. The git backend may use one
//! later to answer faster, and the interface stays the same either way.
//!
//! A backend over a server would answer a revision as a version number,
//! [`Storage::changes`] and [`Storage::read`] from the server's own
//! questions, and [`Storage::history`] by filtering versions by the paths
//! they changed. A server holds no working copy, so such a backend needs a
//! directory on disk beside it, or a refusal of its own for [`State::Working`].

use std::fmt;
use std::path::Path;

pub mod git;
pub mod memory;

/// What a backend could not answer.
///
/// Each one turns into a refusal with a stable code, in `pile`'s
/// crates/pile/src/repository.rs. A backend that can fail in a new way adds a
/// variant here, so the enum is not exhaustive.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The git backend found no `git` to run.
    NoGit,
    /// The directory is not inside a repository with a working copy.
    NotARepository,
    /// A name was asked for that names no revision. It holds the name.
    UnknownRevision(String),
    /// The backend failed to answer. It holds what went wrong, in a sentence
    /// that names what was being asked.
    Failed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoGit => f.write_str("there is no git on PATH"),
            Error::NotARepository => f.write_str("not inside a git repository"),
            Error::UnknownRevision(name) => write!(f, "{name:?} names no revision"),
            Error::Failed(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// One revision, by the id its backend gave it.
///
/// It is printed as the id, and [`Storage::resolve`] reads the id back to
/// the same revision. Two revisions are equal when their ids are.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Revision(String);

impl Revision {
    /// Only a backend makes a revision, out of an id it chose.
    pub(crate) fn new(id: impl Into<String>) -> Revision {
        Revision(id.into())
    }

    /// The id, as the backend spells it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which files a question is asked of: the working copy as it is now, or
/// what one revision recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Working,
    At(Revision),
}

/// What kind of file a path holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FileKind {
    /// A file that is not executable.
    File,
    /// A file that is executable.
    Executable,
    /// A symbolic link. What it holds is the path it points to.
    Symlink,
    /// Another repository inside this one. It is listed, and nothing is read
    /// from inside it.
    Submodule,
}

/// One file, and what kind it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub path: String,
    pub kind: FileKind,
}

/// One file, and what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub path: String,
    pub kind: FileKind,
    pub bytes: Vec<u8>,
}

/// A part of the repository: everything under one path, less everything
/// under each path in `except`.
///
/// A pile is a subtree like this: its directory, less the piles under it.
/// A path is under another when it is the same path, or when it continues it
/// past a `/`. So `/crates/storage` is under `/crates`, and `/crates-old` is
/// not. `/` is the whole repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subtree {
    pub path: String,
    pub except: Vec<String>,
}

impl Subtree {
    /// Everything under path.
    pub fn new(path: impl Into<String>) -> Subtree {
        Subtree {
            path: path.into(),
            except: Vec::new(),
        }
    }

    /// The whole repository.
    pub fn whole() -> Subtree {
        Subtree::new("/")
    }

    /// The same subtree, less everything under path.
    pub fn except(mut self, path: impl Into<String>) -> Subtree {
        self.except.push(path.into());
        self
    }

    /// Whether the subtree holds the path.
    pub fn holds(&self, path: &str) -> bool {
        under(path, &self.path) && !self.except.iter().any(|e| under(path, e))
    }
}

/// Whether path is under top: the same path, or one that continues it past
/// a `/`.
fn under(path: &str, top: &str) -> bool {
    let top = top.trim_end_matches('/');
    if top.is_empty() {
        return path.starts_with('/');
    }
    match path.strip_prefix(top) {
        Some(rest) => rest.is_empty() || rest.starts_with('/'),
        None => false,
    }
}

/// Whether a path is a declaration file: a file somewhere inside a `.pile/`
/// directory.
pub fn is_declaration(path: &str) -> bool {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir.split('/').any(|part| part == ".pile"),
        None => false,
    }
}

/// How one path differs between two ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChangeKind {
    /// It is new at the second end, and the next revision would carry it.
    Added,
    /// It is at both ends, and holds something different, or is a different
    /// kind of file.
    Modified,
    /// It is gone at the second end.
    Deleted,
    /// It left `from` and arrived at `path`. A backend calls a change a move
    /// only when it finds that the file moved. Otherwise a move is a
    /// deletion and an addition.
    Moved,
    /// It is in the working copy and the next revision would not carry it.
    Untracked,
    /// The working copy holds it in a conflict that is not resolved yet.
    Conflicted,
}

/// One path that differs between two ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub path: String,
    pub kind: ChangeKind,
    /// Where a moved path came from. None for every other kind.
    pub from: Option<String>,
    /// The file is a different [`FileKind`] at the two ends: it became
    /// executable or stopped being so, or a file became a symbolic link. A
    /// change of kind is reported beside what else changed, and never in its
    /// place.
    pub retyped: bool,
}

impl Change {
    /// A change with no origin and no change of kind.
    pub fn new(path: impl Into<String>, kind: ChangeKind) -> Change {
        Change {
            path: path.into(),
            kind,
            from: None,
            retyped: false,
        }
    }
}

/// One revision, as a person reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionInfo {
    pub revision: Revision,
    /// The id as it is printed where space is short. The backend decides how
    /// short it can be and still name one revision.
    pub short: String,
    /// Who made the change.
    pub author: String,
    /// When the change was made, in seconds since the Unix epoch.
    pub at: i64,
    /// How far the author's clock was ahead of UTC when they made it, in
    /// seconds, and behind it when negative. [`RevisionInfo::day`] reads the
    /// day by that clock.
    pub offset: i32,
    /// The first line of what the revision says it did.
    pub subject: String,
}

impl RevisionInfo {
    /// The day the change was made, as `YYYY-MM-DD`, by the author's own
    /// clock. A change made just before midnight in one place is made on
    /// that day, whatever the date was in UTC.
    pub fn day(&self) -> String {
        let local = self.at + i64::from(self.offset);
        let (y, m, d) = civil(local.div_euclid(86_400));
        format!("{y:04}-{m:02}-{d:02}")
    }
}

/// The year, month and day a count of days since 1970-01-01 falls on, in the
/// proleptic Gregorian calendar. Howard Hinnant's `civil_from_days`.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// One revision in a history, and every path it changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Touched {
    pub info: RevisionInfo,
    /// Every path the revision changed, sorted, and not only the paths in the
    /// subtree that was asked about. A move is both of its paths.
    pub paths: Vec<String>,
}

/// The revisions that changed a subtree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct History {
    /// How many revisions changed it.
    pub total: usize,
    /// The newest of them, newest first, as many as were asked for.
    pub revisions: Vec<Touched>,
}

/// Why a revision is offered as a base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseRole {
    /// The branch the repository's work goes into, such as `main`.
    Main,
    /// Where the branch the working copy is on is published.
    Upstream,
    /// Where the history of the working copy's branch left the main
    /// branch's.
    ForkPoint,
}

/// A revision offered as a base, by the name it is offered as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base {
    /// The branch it is offered by, such as `main` or `origin/main`. A fork
    /// point is offered by the name of the main branch it left, so that name
    /// resolves to the main branch, and not to the fork point.
    pub name: String,
    pub role: BaseRole,
    pub info: RevisionInfo,
}

/// The revisions a base is chosen from. A repository with no revision has
/// none of them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bases {
    /// The revision the working copy stands on.
    pub head: Option<RevisionInfo>,
    /// The branch the working copy is on. None when it is on none.
    pub branch: Option<String>,
    /// The head and the revisions before it on its branch, newest first. A
    /// revision that joined another branch in is followed back along the
    /// branch it was made on.
    pub recent: Vec<RevisionInfo>,
    /// The revisions worth offering by name, each at most once per role.
    pub named: Vec<Base>,
}

/// The read-only interface every read of a repository goes through.
///
/// Every list it answers is sorted by path, unless the method says another
/// order. A path that has no file is an answer, and not a failure.
pub trait Storage {
    /// Where the working copy is on disk, when it is on disk. `pile` writes
    /// `.pile/` files there, and nowhere else.
    fn working_copy(&self) -> Option<&Path>;

    /// The revision the working copy stands on. None in a repository with
    /// no revision yet.
    fn head(&self) -> Result<Option<Revision>>;

    /// The revision a name names: its id, or a name the backend gives it.
    /// A name that names none is [`Error::UnknownRevision`].
    fn resolve(&self, name: &str) -> Result<Revision>;

    /// Every declaration file, with what it holds: each file inside a
    /// `.pile/` directory ([`is_declaration`]). A submodule is not one.
    ///
    /// In the working copy, an untracked file counts, so a pile declared a
    /// minute ago is a pile.
    fn declarations(&self, at: &State) -> Result<Vec<Content>>;

    /// Every file the subtree holds.
    fn files(&self, at: &State, of: &Subtree) -> Result<Vec<File>>;

    /// What each path holds, in the order asked. None for a path with no
    /// file, and for a directory or a submodule.
    fn read(&self, at: &State, paths: &[&str]) -> Result<Vec<Option<Vec<u8>>>>;

    /// Every path that differs between from and to. A from of None is the
    /// empty repository, before any revision, so every file at `to` is new.
    ///
    /// [`ChangeKind::Untracked`] and [`ChangeKind::Conflicted`] are only
    /// ever answered when to is the working copy. A path can be in two
    /// changes: a file the next revision will no longer carry, that is
    /// still in the working copy, is both deleted and untracked.
    fn changes(&self, from: Option<&Revision>, to: &State) -> Result<Vec<Change>>;

    /// The revisions from the head back that changed a path in the subtree,
    /// newest first: the first `limit` of them, or all of them for None.
    /// With `after`, only the revisions after it count: those the head
    /// reached and `after` did not.
    ///
    /// A revision that only joins other lines of work changes nothing of its
    /// own, so it is never in a history.
    fn history(
        &self,
        of: &Subtree,
        after: Option<&Revision>,
        limit: Option<usize>,
    ) -> Result<History>;

    /// The revisions a base is chosen from, with at most `recent` of the
    /// head's own branch.
    fn bases(&self, recent: usize) -> Result<Bases>;
}

/// The storage for the repository that encloses dir.
///
/// Every repository today is a git repository, so it is always the git
/// backend.
pub fn open(dir: &Path) -> Result<Box<dyn Storage>> {
    Ok(Box::new(git::Git::open(dir)?))
}

/// Every change between two lists of files, by path and by what each holds,
/// as the memory backend and its tests compare them. A move is found when a
/// path that was deleted held exactly what a path that was added holds.
pub(crate) fn compare(from: &[Content], to: &[Content]) -> Vec<Change> {
    use std::collections::BTreeMap;
    let before: BTreeMap<&str, &Content> = from.iter().map(|c| (c.path.as_str(), c)).collect();
    let after: BTreeMap<&str, &Content> = to.iter().map(|c| (c.path.as_str(), c)).collect();

    let mut out = Vec::new();
    let mut added = Vec::new();
    let mut deleted = Vec::new();
    for (path, a) in &before {
        match after.get(path) {
            None => deleted.push(*a),
            Some(b) if a.bytes != b.bytes || a.kind != b.kind => out.push(Change {
                retyped: a.kind != b.kind,
                ..Change::new(*path, ChangeKind::Modified)
            }),
            Some(_) => {}
        }
    }
    for (path, b) in &after {
        if !before.contains_key(path) {
            added.push(*b);
        }
    }
    for b in added {
        match deleted.iter().position(|a| a.bytes == b.bytes) {
            Some(i) => {
                let a = deleted.remove(i);
                out.push(Change {
                    from: Some(a.path.clone()),
                    retyped: a.kind != b.kind,
                    ..Change::new(&b.path, ChangeKind::Moved)
                });
            }
            None => out.push(Change::new(&b.path, ChangeKind::Added)),
        }
    }
    for a in deleted {
        out.push(Change::new(&a.path, ChangeKind::Deleted));
    }
    sort(&mut out);
    out
}

/// Sorts changes by path, and two changes of one path by their kind.
pub(crate) fn sort(changes: &mut [Change]) {
    changes.sort_by(|a, b| (&a.path, a.kind).cmp(&(&b.path, b.kind)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subtree_holds_what_is_under_it_and_not_what_it_excepts() {
        let s = Subtree::new("/crates").except("/crates/pile");
        assert!(s.holds("/crates"));
        assert!(s.holds("/crates/storage/src/lib.rs"));
        assert!(!s.holds("/crates-old/x"));
        assert!(!s.holds("/crates/pile"));
        assert!(!s.holds("/crates/pile/src/main.rs"));
        assert!(s.holds("/crates/piles/x"));
        assert!(Subtree::whole().holds("/anything"));
        assert!(!Subtree::whole().except("/").holds("/anything"));
    }

    #[test]
    fn the_day_is_the_one_on_the_authors_clock() {
        let info = |at, offset| RevisionInfo {
            revision: Revision::new("r"),
            short: "r".into(),
            author: "a".into(),
            at,
            offset,
            subject: String::new(),
        };
        assert_eq!(info(0, 0).day(), "1970-01-01");
        // 2026-10-27T23:30:00Z
        let late = 1_793_143_800;
        assert_eq!(info(late, 0).day(), "2026-10-27");
        assert_eq!(info(late, 3600).day(), "2026-10-28");
        assert_eq!(info(late - 86_400 * 365, -7200).day(), "2025-10-27");
        assert_eq!(info(951_782_400, 0).day(), "2000-02-29");
        assert_eq!(info(-1, 0).day(), "1969-12-31");
    }

    #[test]
    fn a_declaration_is_a_file_inside_a_pile_directory() {
        assert!(is_declaration("/.pile/config"));
        assert!(is_declaration("/a/b/.pile/intent.md"));
        assert!(is_declaration("/a/.pile/guide/one.md"));
        assert!(!is_declaration("/a.pile/config"));
        assert!(!is_declaration("/a/.pile"));
        assert!(!is_declaration("/a/x.pile/config"));
    }
}
