//! A repository each case builds, behind one interface, so one case runs
//! against every backend.
//!
//! A case writes files, moves them and records revisions through
//! [`Fixture`]. Then it asks the [`Storage`] the fixture holds. For git the
//! fixture is a fresh repository in a temporary directory, changed with the
//! `git` command. For memory it is a [`Memory`].

#![allow(dead_code)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use storage::git::Git;
use storage::memory::{self, Memory};
use storage::{Revision, Storage};
use tempfile::TempDir;

/// A repository a case can change, and the storage over it.
pub trait Fixture {
    fn storage(&self) -> &dyn Storage;
    /// Writes a file into the working copy, and tracks it.
    fn write(&mut self, path: &str, content: &str);
    /// Writes a file into the working copy, and does not track it.
    fn write_untracked(&mut self, path: &str, content: &str);
    /// Makes a tracked file executable.
    fn make_executable(&mut self, path: &str);
    /// Writes a tracked symbolic link that points at target.
    fn symlink(&mut self, path: &str, target: &str);
    /// Removes a tracked file, and stops tracking it.
    fn remove(&mut self, path: &str);
    /// Moves a tracked file.
    fn rename(&mut self, from: &str, to: &str);
    /// Records every tracked file as a revision, and returns it.
    fn commit(&mut self, subject: &str) -> Revision;
}

impl Fixture for Memory {
    fn storage(&self) -> &dyn Storage {
        self
    }
    fn write(&mut self, path: &str, content: &str) {
        Memory::write(self, path, content);
    }
    fn write_untracked(&mut self, path: &str, content: &str) {
        Memory::write_untracked(self, path, content);
    }
    fn make_executable(&mut self, path: &str) {
        Memory::make_executable(self, path);
    }
    fn symlink(&mut self, path: &str, target: &str) {
        self.write_kind(path, storage::FileKind::Symlink, target);
    }
    fn remove(&mut self, path: &str) {
        Memory::remove(self, path);
    }
    fn rename(&mut self, from: &str, to: &str) {
        Memory::rename(self, from, to);
    }
    fn commit(&mut self, subject: &str) -> Revision {
        Memory::commit(self, subject)
    }
}

/// A HOME shared by every git the suite runs, with no configuration in it.
fn home() -> &'static Path {
    static HOME: OnceLock<TempDir> = OnceLock::new();
    HOME.get_or_init(|| {
        tempfile::Builder::new()
            .prefix("storage-test-home-")
            .tempdir()
            .expect("a temporary HOME")
    })
    .path()
}

/// git run in dir, with no GIT_* variable from whatever started the suite,
/// and nobody's configuration.
///
/// That is every git the fixture runs to change a repository. The storage
/// a fixture reads through is opened with `Git::open_alone`, so the git it
/// runs ignores a hook's GIT_DIR and GIT_INDEX_FILE too. Then a suite run
/// from inside a git hook reads only its own temporary repositories.
pub fn git_command(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir).stdin(Stdio::null());
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(&key);
        }
    }
    cmd.env("HOME", home())
        .env("XDG_CONFIG_HOME", home().join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

/// A fresh git repository, on a line called `main`, with no commit.
pub struct GitRepo {
    dir: TempDir,
    storage: Git,
    commits: i64,
}

impl Default for GitRepo {
    fn default() -> GitRepo {
        GitRepo::new()
    }
}

impl GitRepo {
    pub fn new() -> GitRepo {
        let dir = tempfile::Builder::new()
            .prefix("storage-test-")
            .tempdir()
            .expect("a temporary directory");
        run(dir.path(), &["init", "-q", "-b", "main"]);
        run(dir.path(), &["config", "user.name", memory::AUTHOR]);
        run(dir.path(), &["config", "user.email", "test@example.com"]);
        run(dir.path(), &["config", "commit.gpgsign", "false"]);
        let storage = Git::open_alone(dir.path()).expect("the repository opens");
        GitRepo {
            dir,
            storage,
            commits: 0,
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn git(&self, args: &[&str]) -> String {
        run(self.path(), args)
    }

    fn on_disk(&self, path: &str) -> std::path::PathBuf {
        self.path().join(path.trim_start_matches('/'))
    }

    fn make_parent(&self, path: &str) {
        let at = self.on_disk(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    }

    /// Writes a file and does not track it.
    pub fn put(&self, path: &str, content: &str) {
        self.make_parent(path);
        std::fs::write(self.on_disk(path), content).unwrap();
    }
}

/// Runs git, and fails the test if git fails.
#[track_caller]
pub fn run(dir: &Path, args: &[&str]) -> String {
    let out = git_command(dir).args(args).output().expect("running git");
    assert!(
        out.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

impl Fixture for GitRepo {
    fn storage(&self) -> &dyn Storage {
        &self.storage
    }
    fn write(&mut self, path: &str, content: &str) {
        self.put(path, content);
        self.git(&["add", "--", path.trim_start_matches('/')]);
    }
    fn write_untracked(&mut self, path: &str, content: &str) {
        self.put(path, content);
    }
    fn make_executable(&mut self, path: &str) {
        use std::os::unix::fs::PermissionsExt as _;
        let at = self.on_disk(path);
        std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755)).unwrap();
        self.git(&["add", "--", path.trim_start_matches('/')]);
    }
    fn symlink(&mut self, path: &str, target: &str) {
        self.make_parent(path);
        std::os::unix::fs::symlink(target, self.on_disk(path)).unwrap();
        self.git(&["add", "--", path.trim_start_matches('/')]);
    }
    fn remove(&mut self, path: &str) {
        self.git(&["rm", "-q", "--", path.trim_start_matches('/')]);
    }
    fn rename(&mut self, from: &str, to: &str) {
        self.make_parent(to);
        self.git(&[
            "mv",
            "--",
            from.trim_start_matches('/'),
            to.trim_start_matches('/'),
        ]);
    }
    /// Commits at the time the memory backend gives the same revision, so a
    /// case can expect one time from both.
    fn commit(&mut self, subject: &str) -> Revision {
        let at = format!("@{} +0000", memory::FIRST_AT + 60 * self.commits);
        self.commits += 1;
        let out = git_command(self.path())
            .args(["commit", "-q", "--allow-empty", "-m", subject])
            .env("GIT_AUTHOR_DATE", &at)
            .env("GIT_COMMITTER_DATE", &at)
            .output()
            .expect("running git");
        assert!(
            out.status.success(),
            "git commit: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        self.storage.head().unwrap().expect("a head after a commit")
    }
}
