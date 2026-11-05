//! The git backend: every question is asked of the `git` command.
//!
//! Nothing here is git reimplemented. Each method runs git and reads back
//! what it prints, and git's answer is the answer. Every invocation:
//!
//! - runs `git` as PATH finds it. A spawn the system answers with "not
//!   found" is [`Error::NoGit`], whichever invocation it was.
//! - runs from the top of the working copy, so every path git prints is from
//!   the same place.
//! - asks for paths ended by NUL, so a path is never split at a space or a
//!   newline.
//! - writes nothing. `GIT_OPTIONAL_LOCKS=0` stops a read from refreshing
//!   the index as it goes, where git honours it. `git diff` does not, so
//!   the working copy is compared with `git diff-index`, which never writes
//!   ([`Git::changes`](Storage::changes)).
//! - reads git's messages in English, because two of them are told apart by
//!   what they say.
//!
//! The environment git is run in is otherwise the caller's own. A `pile` run
//! inside a git hook reads the repository and the index the hook is given.
//! [`Git::open_alone`] is the one exception, for tests.
//!
//! A revision here is a commit, by its whole id. A file's content is never
//! named by git's object id outside this module.

mod bases;
mod parse;

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::{
    Bases, Change, ChangeKind, Content, Error, File, FileKind, History, Result, Revision, State,
    Storage, Subtree, Touched, is_declaration, sort,
};
use parse::{Listed, fields};

/// How many ids go on one command line. A history can be tens of thousands
/// of revisions long, and every system limits how long a command line can
/// be. A few thousand ids fit inside every such limit.
const IDS_PER_ASK: usize = 2000;

/// What `git log` prints for each revision: a record separator, then the id,
/// the short id, the author's time, the author's date with its offset from
/// UTC, the author and the subject, each ended by a unit separator. A subject
/// never holds either separator.
const FORMAT: &str = "--format=%x1e%H%x1f%h%x1f%at%x1f%ai%x1f%an%x1f%s%x1f";

/// The variables that change what a pathspec means. Each pathspec here says
/// what it means itself, with magic such as `:(literal)`.
const PATHSPEC_VARS: &[&str] = &[
    "GIT_LITERAL_PATHSPECS",
    "GIT_GLOB_PATHSPECS",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_ICASE_PATHSPECS",
];

/// A git repository with a working copy.
#[derive(Debug, Clone)]
pub struct Git {
    /// The top of the working copy, resolved, so it is the same path however
    /// it was reached.
    top: PathBuf,
    /// git runs without the caller's repository variables.
    alone: bool,
}

impl Git {
    /// The repository that encloses dir.
    ///
    /// A directory in no repository, a bare repository, and a directory
    /// inside `.git` are all [`Error::NotARepository`], because none of them
    /// has a working copy to stand in.
    pub fn open(dir: &Path) -> Result<Git> {
        Git::open_in(dir, false)
    }

    /// The repository that encloses dir, read with none of the variables
    /// that point git at a repository, such as `GIT_DIR` and
    /// `GIT_INDEX_FILE`. `git rev-parse --local-env-vars` lists them.
    ///
    /// This is for a test that makes its own repository. A git hook sets
    /// those variables, so a test run from inside one would otherwise read
    /// the repository the hook was run for. `pile` itself opens with
    /// [`Git::open`].
    pub fn open_alone(dir: &Path) -> Result<Git> {
        Git::open_in(dir, true)
    }

    fn open_in(dir: &Path, alone: bool) -> Result<Git> {
        if !dir.is_dir() {
            return Err(Error::Failed(format!(
                "{} is not a directory",
                dir.display()
            )));
        }
        let out = match run(dir, alone, &["rev-parse", "--show-toplevel"]) {
            Ok(out) => out,
            Err(Error::Failed(why))
                if why.contains("not a git repository")
                    || why.contains("must be run in a work tree") =>
            {
                return Err(Error::NotARepository);
            }
            Err(e) => return Err(e),
        };
        let top = String::from_utf8_lossy(&out).trim().to_string();
        if top.is_empty() {
            return Err(Error::NotARepository);
        }
        let top = std::fs::canonicalize(&top).unwrap_or_else(|_| PathBuf::from(top));
        Ok(Git { top, alone })
    }

    fn git<S: AsRef<OsStr>>(&self, args: &[S]) -> Result<Vec<u8>> {
        run(&self.top, self.alone, args)
    }

    /// The commit a name peels to, or None where it names none.
    ///
    /// A name that names nothing is an answer, and not a failure. `rev-parse
    /// --verify` exits 1 for that, and still says why when the name named
    /// something other than a commit, such as a tree. Anything else it
    /// refuses exits 128. The options are ended before the name, so a name
    /// that starts with a dash is still a name.
    fn commit(&self, name: &str) -> Result<Option<Revision>> {
        let peeled = format!("{name}^{{commit}}");
        let args = [
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &peeled,
        ];
        let out = command(&self.top, self.alone)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .map_err(spawn_error)?;
        if out.status.code() == Some(1) {
            return Ok(None);
        }
        let id = String::from_utf8_lossy(&finish(&args, out)?)
            .trim()
            .to_string();
        Ok((!id.is_empty()).then(|| Revision::new(id)))
    }

    /// The id of the empty tree, in the hash this repository uses. Asked of
    /// git without -w, so nothing is written.
    fn empty_tree(&self) -> Result<String> {
        let out = run_with_stdin(
            &self.top,
            self.alone,
            &["hash-object", "-t", "tree", "--stdin"],
            b"",
        )?;
        Ok(String::from_utf8_lossy(&out).trim().to_string())
    }

    /// What one end of a comparison is to git: a commit's id, or the empty
    /// tree for the empty repository.
    fn end(&self, from: Option<&Revision>) -> Result<String> {
        match from {
            Some(r) => Ok(r.as_str().to_string()),
            None => self.empty_tree(),
        }
    }

    /// Every path in the working copy that git lists under the pathspecs:
    /// tracked, or untracked and not ignored. A tracked path deleted from
    /// disk since is not in the working copy, and is left out.
    fn working_files(&self, pathspecs: &[String]) -> Result<Vec<File>> {
        let mut args = vec!["ls-files", "-co", "--exclude-standard", "-z", "--"];
        args.extend(pathspecs.iter().map(String::as_str));
        let listed: BTreeSet<String> = fields(&self.git(&args)?)
            .into_iter()
            // An untracked repository inside this one is listed as a
            // directory, ending in a slash.
            .map(|p| p.trim_end_matches('/').to_string())
            .collect();
        let mut out = Vec::new();
        for rel in listed {
            if let Some(kind) = kind_on_disk(&self.top.join(&rel))? {
                out.push(File {
                    path: from_git(&rel),
                    kind,
                });
            }
        }
        Ok(out)
    }

    /// Every file a commit records, with git's object id for each.
    fn tree(&self, revision: &Revision, under: Option<&str>) -> Result<Vec<Listed>> {
        let mut args = vec!["ls-tree", "-r", "-z", "--full-tree", revision.as_str()];
        if let Some(path) = under {
            args.extend(["--", path]);
        }
        let mut out = parse::tree(&fields(&self.git(&args)?))?;
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    /// What objects hold, in the order asked, each named as `cat-file` takes
    /// a name. None for a name the store does not hold, and for an object
    /// that is not a file's content.
    fn objects(&self, names: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        if names.is_empty() {
            return Ok(Vec::new());
        }
        let mut stdin = Vec::new();
        for name in names {
            stdin.extend_from_slice(name.as_bytes());
            stdin.push(b'\n');
        }
        let out = run_with_stdin(&self.top, self.alone, &["cat-file", "--batch"], &stdin)?;
        parse::batch(&out, names.len())
    }

    /// The changes diff-index found, less the ones that only look changed.
    ///
    /// diff-index reads the index's stat data and does not refresh it. So a
    /// file whose size or time changed since the index last looked, and whose
    /// content did not, is reported modified, with no id on the working
    /// copy's side. Each such file is hashed here as git would record it, with
    /// `hash-object` and without `-w`, so nothing is written. It is dropped
    /// when it holds what the base holds and its mode is the same.
    fn unchanged_left_out(&self, raws: Vec<parse::Raw>) -> Result<Vec<Change>> {
        let unhashed = |r: &parse::Raw| {
            r.change.kind == ChangeKind::Modified
                && r.to_oid.bytes().all(|b| b == b'0')
                && !r.change.path.contains('\n')
        };
        let files: Vec<&str> = raws
            .iter()
            .filter(|r| unhashed(r) && r.to_mode != "120000" && r.to_mode != "160000")
            .map(|r| r.change.path.as_str())
            .collect();
        let mut ids: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
        if !files.is_empty() {
            let stdin: String = files.iter().map(|p| format!("{p}\n")).collect();
            let out = run_with_stdin(
                &self.top,
                self.alone,
                &["hash-object", "--stdin-paths"],
                stdin.as_bytes(),
            )?;
            let text = String::from_utf8_lossy(&out);
            ids.extend(files.iter().copied().zip(text.lines().map(str::to_string)));
        }
        for r in raws.iter().filter(|r| unhashed(r) && r.to_mode == "120000") {
            // A symbolic link is recorded as the path it points to.
            use std::os::unix::ffi::OsStrExt as _;
            if let Ok(target) = std::fs::read_link(self.top.join(&r.change.path)) {
                let out = run_with_stdin(
                    &self.top,
                    self.alone,
                    &["hash-object", "--stdin"],
                    target.as_os_str().as_bytes(),
                )?;
                ids.insert(
                    &r.change.path,
                    String::from_utf8_lossy(&out).trim().to_string(),
                );
            }
        }
        Ok(raws
            .iter()
            .filter(|r| {
                !(unhashed(r)
                    && r.from_mode == r.to_mode
                    && ids.get(r.change.path.as_str()) == Some(&r.from_oid))
            })
            .map(|r| r.change.clone())
            .collect())
    }

    /// The details of revisions by their ids, in the order given, each with
    /// every path it changed.
    ///
    /// --no-walk=unsorted makes this a lookup and not a history: the
    /// revisions named and none before them, in the order named. --full-diff
    /// is not needed, because no pathspec narrows the paths listed. A move is
    /// both of its paths, by --no-renames, because a move out of one pile
    /// changed the pile it left as well as the one it reached.
    fn describe(&self, ids: &[String]) -> Result<Vec<Touched>> {
        let mut out = Vec::new();
        for batch in ids.chunks(IDS_PER_ASK) {
            let mut args = vec![
                "log",
                "--no-walk=unsorted",
                "--no-renames",
                "--name-only",
                "-z",
                "--abbrev=8",
                FORMAT,
                "--end-of-options",
            ];
            args.extend(batch.iter().map(String::as_str));
            let mut got = parse::log(&self.git(&args)?)?;
            for t in &mut got {
                t.paths = t.paths.iter().map(|p| from_git(p)).collect();
                t.paths.sort();
            }
            out.extend(got);
        }
        Ok(out)
    }
}

impl Storage for Git {
    fn working_copy(&self) -> Option<&Path> {
        Some(&self.top)
    }

    fn head(&self) -> Result<Option<Revision>> {
        self.commit("HEAD")
    }

    fn resolve(&self, name: &str) -> Result<Revision> {
        if name.is_empty() {
            return Err(Error::UnknownRevision(String::new()));
        }
        self.commit(name)?
            .ok_or_else(|| Error::UnknownRevision(name.to_string()))
    }

    fn declarations(&self, at: &State) -> Result<Vec<Content>> {
        match at {
            State::Working => {
                // **/ also matches no directory at all, so the root's own
                // .pile/ is found with the rest.
                let spec = vec![":(glob)**/.pile/**".to_string()];
                let mut out = Vec::new();
                for f in self.working_files(&spec)? {
                    if f.kind == FileKind::Submodule || !is_declaration(&f.path) {
                        continue;
                    }
                    if let Some(bytes) = read_on_disk(&self.top, &f)? {
                        out.push(Content {
                            path: f.path,
                            kind: f.kind,
                            bytes,
                        });
                    }
                }
                Ok(out)
            }
            State::At(r) => {
                let listed: Vec<Listed> = self
                    .tree(r, None)?
                    .into_iter()
                    .filter(|l| l.kind != FileKind::Submodule && is_declaration(&l.path))
                    .collect();
                let oids: Vec<String> = listed.iter().map(|l| l.oid.clone()).collect();
                let blobs = self.objects(&oids)?;
                listed
                    .into_iter()
                    .zip(blobs)
                    .map(|(l, bytes)| {
                        let bytes = bytes.ok_or_else(|| {
                            Error::Failed(format!(
                                "git lists {} at {r} and holds nothing for it",
                                l.path
                            ))
                        })?;
                        Ok(Content {
                            path: l.path,
                            kind: l.kind,
                            bytes,
                        })
                    })
                    .collect()
            }
        }
    }

    fn files(&self, at: &State, of: &Subtree) -> Result<Vec<File>> {
        let files = match at {
            State::Working => self.working_files(&pathspecs(of))?,
            State::At(r) => {
                let under = (to_git(&of.path) != ".").then(|| to_git(&of.path));
                self.tree(r, under)?
                    .into_iter()
                    .map(|l| File {
                        path: l.path,
                        kind: l.kind,
                    })
                    .collect()
            }
        };
        Ok(files.into_iter().filter(|f| of.holds(&f.path)).collect())
    }

    fn read(&self, at: &State, paths: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        match at {
            State::Working => paths
                .iter()
                .map(|p| {
                    let rel = to_git(p);
                    match kind_on_disk(&self.top.join(rel))? {
                        None | Some(FileKind::Submodule) => Ok(None),
                        Some(kind) => read_on_disk(
                            &self.top,
                            &File {
                                path: p.to_string(),
                                kind,
                            },
                        ),
                    }
                })
                .collect(),
            State::At(r) => {
                // cat-file reads one name a line, so a path with a newline in
                // it cannot be asked for. No commit holds one in practice.
                let names: Vec<String> =
                    paths.iter().map(|p| format!("{r}:{}", to_git(p))).collect();
                let askable: Vec<String> = names
                    .iter()
                    .filter(|n| !n.contains('\n'))
                    .cloned()
                    .collect();
                let mut got = self.objects(&askable)?.into_iter();
                Ok(names
                    .iter()
                    .map(|n| {
                        if n.contains('\n') {
                            None
                        } else {
                            got.next().flatten()
                        }
                    })
                    .collect())
            }
        }
    }

    fn changes(&self, from: Option<&Revision>, to: &State) -> Result<Vec<Change>> {
        let base = self.end(from)?;
        // -M finds moves, whatever diff.renames says. --raw gives both ends'
        // modes, so a change of kind is seen beside a change of content.
        let mut out = match to {
            State::At(r) => parse::raw(&fields(&self.git(&[
                "diff-tree",
                "-r",
                "-z",
                "-M",
                "--raw",
                "--no-color",
                "--end-of-options",
                &base,
                r.as_str(),
            ])?))?,
            State::Working => {
                // The working copy against the base, for every path the index
                // tracks. Untracked files are asked for after. diff-index, and
                // not diff: diff refreshes a stale index as it reads, which is
                // a write, and GIT_OPTIONAL_LOCKS does not stop it.
                let raws = parse::raw_records(&fields(&self.git(&[
                    "diff-index",
                    "-z",
                    "-M",
                    "--raw",
                    "--no-abbrev",
                    "--end-of-options",
                    &base,
                    "--",
                ])?))?;
                let mut out = self.unchanged_left_out(raws)?;
                let untracked =
                    fields(&self.git(&["ls-files", "-o", "--exclude-standard", "-z", "--"])?);
                out.extend(
                    untracked
                        .into_iter()
                        .map(|p| Change::new(p.trim_end_matches('/'), ChangeKind::Untracked)),
                );
                // git diff reads a path in conflict as modified. The index
                // holds it at more than one stage, which says what it is.
                let conflicted: BTreeSet<String> =
                    parse::unmerged(&fields(&self.git(&["ls-files", "-u", "-z", "--"])?))
                        .into_iter()
                        .collect();
                out.retain(|c| !conflicted.contains(&c.path));
                out.extend(
                    conflicted
                        .into_iter()
                        .map(|p| Change::new(p, ChangeKind::Conflicted)),
                );
                out
            }
        };
        for c in &mut out {
            c.path = from_git(&c.path);
            c.from = c.from.as_deref().map(from_git);
        }
        sort(&mut out);
        Ok(out)
    }

    fn history(
        &self,
        of: &Subtree,
        after: Option<&Revision>,
        limit: Option<usize>,
    ) -> Result<History> {
        let Some(head) = self.head()? else {
            return Ok(History::default());
        };
        // --no-merges: what a merge brought in is the revisions under it,
        // and each of those is counted as itself.
        let mut args = vec![
            "rev-list".to_string(),
            "--no-merges".to_string(),
            "--end-of-options".to_string(),
            head.to_string(),
        ];
        if let Some(a) = after {
            args.push(format!("^{a}"));
        }
        args.push("--".to_string());
        args.extend(pathspecs(of));
        let ids: Vec<String> = String::from_utf8_lossy(&self.git(&args)?)
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let shown = limit.map_or(ids.len(), |n| n.min(ids.len()));
        Ok(History {
            total: ids.len(),
            revisions: self.describe(&ids[..shown])?,
        })
    }

    fn bases(&self, recent: usize) -> Result<Bases> {
        bases::read(self, recent)
    }
}

/// The pathspecs for a subtree: its path, and each path it excepts
/// excluded. Literal, because a directory may have a name git would
/// otherwise read as a pattern.
fn pathspecs(of: &Subtree) -> Vec<String> {
    let top = to_git(&of.path);
    let mut out = vec![if top == "." {
        ":/".to_string()
    } else {
        format!(":(literal){top}")
    }];
    for e in &of.except {
        out.push(format!(":(exclude,literal){}", to_git(e)));
    }
    out
}

/// A path as git spells it, from the top: without the leading `/`, and `.`
/// for the top itself.
fn to_git(path: &str) -> &str {
    let rel = path.trim_start_matches('/').trim_end_matches('/');
    if rel.is_empty() { "." } else { rel }
}

/// A path git printed, as this crate spells it.
fn from_git(rel: &str) -> String {
    format!("/{rel}")
}

/// What kind of file is on disk at a path, or None when nothing is there.
///
/// A directory is a submodule, because git lists a directory only when it
/// is another repository.
fn kind_on_disk(at: &Path) -> Result<Option<FileKind>> {
    use std::os::unix::fs::PermissionsExt as _;
    let meta = match at.symlink_metadata() {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::Failed(format!("reading {}: {e}", at.display()))),
    };
    let t = meta.file_type();
    Ok(Some(if t.is_symlink() {
        FileKind::Symlink
    } else if t.is_dir() {
        FileKind::Submodule
    } else if meta.permissions().mode() & 0o111 != 0 {
        FileKind::Executable
    } else {
        FileKind::File
    }))
}

/// What a file in the working copy holds, the way git would record it: a
/// symbolic link holds the path it points to. None when it went away since
/// it was listed.
fn read_on_disk(top: &Path, f: &File) -> Result<Option<Vec<u8>>> {
    use std::os::unix::ffi::OsStrExt as _;
    let at = top.join(to_git(&f.path));
    let got = if f.kind == FileKind::Symlink {
        std::fs::read_link(&at).map(|t| t.as_os_str().as_bytes().to_vec())
    } else {
        std::fs::read(&at)
    };
    match got {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Failed(format!("reading {}: {e}", f.path))),
    }
}

/// The variables that point git at a repository, as git lists them. Empty
/// when git cannot be run, which the invocation that follows reports.
fn local_vars() -> &'static [String] {
    static VARS: OnceLock<Vec<String>> = OnceLock::new();
    VARS.get_or_init(|| {
        Command::new("git")
            .args(["rev-parse", "--local-env-vars"])
            .stdin(Stdio::null())
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// git, run in dir with the environment every invocation here needs, and
/// none of the repository variables when alone.
fn command(dir: &Path, alone: bool) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .args(["-c", "color.ui=false", "-c", "log.showSignature=false"]);
    for var in PATHSPEC_VARS {
        cmd.env_remove(var);
    }
    if alone {
        for var in local_vars() {
            cmd.env_remove(var);
        }
    }
    cmd
}

/// Runs git and returns what it printed on stdout.
fn run<S: AsRef<OsStr>>(dir: &Path, alone: bool, args: &[S]) -> Result<Vec<u8>> {
    let out = command(dir, alone)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(spawn_error)?;
    finish(args, out)
}

/// Runs git with something on its stdin, and returns what it printed on
/// stdout.
fn run_with_stdin(dir: &Path, alone: bool, args: &[&str], stdin: &[u8]) -> Result<Vec<u8>> {
    let mut child = command(dir, alone)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;
    // A thread writes, so that git filling stdout while stdin is still
    // being written cannot leave the two processes waiting on each other.
    let mut pipe = child.stdin.take().expect("stdin is piped");
    let input = stdin.to_vec();
    let writer = std::thread::spawn(move || pipe.write_all(&input));
    let out = child
        .wait_with_output()
        .map_err(|e| Error::Failed(format!("running git {}: {e}", args[0])))?;
    let _ = writer.join();
    finish(args, out)
}

/// git's answer, or its refusal as a sentence naming the subcommand and
/// the first line git said.
fn finish<S: AsRef<OsStr>>(args: &[S], out: std::process::Output) -> Result<Vec<u8>> {
    if out.status.success() {
        return Ok(out.stdout);
    }
    let name = args
        .iter()
        .map(|a| a.as_ref().to_string_lossy())
        .find(|a| !a.starts_with('-') && !a.contains('='))
        .unwrap_or_default();
    let said = String::from_utf8_lossy(&out.stderr);
    let first = said.trim().lines().next().unwrap_or("").to_string();
    Err(Error::Failed(if first.is_empty() {
        format!("git {name}: {}", out.status)
    } else {
        format!("git {name}: {first}")
    }))
}

/// What failing to start git means. Not finding it has its own answer,
/// because what fixes it is installing git.
fn spawn_error(e: io::Error) -> Error {
    if e.kind() == io::ErrorKind::NotFound {
        Error::NoGit
    } else {
        Error::Failed(format!("running git: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_git_that_is_not_there_is_no_git() {
        let missing = io::Error::new(io::ErrorKind::NotFound, "no such file");
        assert_eq!(spawn_error(missing), Error::NoGit);
        let denied = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        assert!(matches!(spawn_error(denied), Error::Failed(_)));
    }

    #[test]
    fn alone_git_runs_without_the_variables_that_point_it_at_a_repository() {
        let vars = local_vars();
        for var in [
            "GIT_DIR",
            "GIT_INDEX_FILE",
            "GIT_WORK_TREE",
            "GIT_COMMON_DIR",
        ] {
            assert!(vars.iter().any(|v| v == var), "git lists {var}");
        }
        let removed = |alone| -> Vec<String> {
            command(Path::new("."), alone)
                .get_envs()
                .filter(|(_, value)| value.is_none())
                .map(|(key, _)| key.to_string_lossy().into_owned())
                .collect()
        };
        assert!(vars.iter().all(|v| removed(true).contains(v)));
        assert!(!removed(false).iter().any(|v| v == "GIT_DIR"));
    }

    #[test]
    fn a_subtree_is_literal_pathspecs_from_the_top() {
        assert_eq!(pathspecs(&Subtree::whole()), [":/"]);
        let s = Subtree::new("/crates/").except("/crates/pile");
        assert_eq!(
            pathspecs(&s),
            [":(literal)crates", ":(exclude,literal)crates/pile"]
        );
        assert_eq!(to_git("/"), ".");
        assert_eq!(from_git("a/b"), "/a/b");
    }
}
