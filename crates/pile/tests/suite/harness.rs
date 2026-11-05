//! The harness: a fresh git repository for each test, and the built binary
//! run in it.
//!
//! It does not skip. The binary is claimed to need nothing but git, so a run
//! that quietly did not happen would leave that claim untested.
//!
//! Every process the suite starts, git and `pile` alike, runs with the
//! environment neutralised:
//!
//! - no GIT_* variable from whatever started the suite. A git hook sets
//!   GIT_DIR, and a suite run from one would otherwise act on the repository
//!   being checked.
//! - no PILE_* variable, and no AI_AGENT or AGENT from an agent's session. A
//!   test of agent mode sets one itself.
//! - a HOME of its own and no system configuration, so nobody's signing,
//!   hooks or default branch reaches a repository made here.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use tempfile::TempDir;

/// The binary under test, which cargo builds before the suite runs. Nobody
/// stamped it, so it says it is `dev`.
pub fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_pile")
}

/// The variables that put `pile` in agent mode.
pub const AGENT_VARS: &[&str] = &["AI_AGENT", "AGENT"];

/// A HOME shared by every process the suite starts, with no configuration.
fn home() -> &'static Path {
    static HOME: OnceLock<TempDir> = OnceLock::new();
    HOME.get_or_init(|| {
        tempfile::Builder::new()
            .prefix("pile-test-home-")
            .tempdir()
            .expect("a temporary HOME")
    })
    .path()
}

/// A command run in dir, with the environment neutralised.
pub fn command(prog: impl AsRef<OsStr>, dir: &Path) -> Command {
    let mut cmd = Command::new(prog);
    cmd.current_dir(dir).stdin(Stdio::null());
    for (key, _) in std::env::vars_os() {
        let key = key.to_string_lossy();
        if key.starts_with("GIT_") || key.starts_with("PILE_") || AGENT_VARS.contains(&key.as_ref())
        {
            cmd.env_remove(key.as_ref());
        }
    }
    let home = home();
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

/// What a finished `pile` printed on each stream, and what it exited with.
pub struct Answered {
    pub stdout: String,
    pub stderr: String,
    /// The exit code, and -1 for a process a signal ended.
    pub code: i32,
}

impl Answered {
    /// stdout, read as one JSON document.
    #[track_caller]
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not one JSON document ({e}):\n{}", self.stdout))
    }
}

/// Runs `pile` with args in dir, with the variables in env set.
pub fn answered(dir: &Path, env: &[(&str, &str)], args: &[&str]) -> Answered {
    let mut cmd = command(bin(), dir);
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("running pile {}: {e}", args.join(" ")));
    Answered {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code().unwrap_or(-1),
    }
}

/// A directory that is in no git repository.
pub fn outside() -> TempDir {
    tempfile::Builder::new()
        .prefix("pile-outside-")
        .tempdir()
        .expect("a temporary directory")
}

/// A fresh git repository, with one commit so that HEAD names something.
pub struct Repo {
    dir: TempDir,
}

impl Repo {
    pub fn new() -> Repo {
        let dir = tempfile::Builder::new()
            .prefix("pile-test-")
            .tempdir()
            .expect("a temporary directory");
        let r = Repo { dir };
        r.git(&["init", "-q", "-b", "main"]);
        r.git(&["config", "user.name", "pile test"]);
        r.git(&["config", "user.email", "test@example.com"]);
        r.git(&["config", "commit.gpgsign", "false"]);
        r.git(&["commit", "-q", "--allow-empty", "-m", "start"]);
        r
    }

    /// A fresh git repository with no commit at all.
    pub fn empty() -> Repo {
        let dir = tempfile::Builder::new()
            .prefix("pile-test-")
            .tempdir()
            .expect("a temporary directory");
        let r = Repo { dir };
        r.git(&["init", "-q", "-b", "main"]);
        r.git(&["config", "user.name", "pile test"]);
        r.git(&["config", "user.email", "test@example.com"]);
        r.git(&["config", "commit.gpgsign", "false"]);
        r
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Writes a file, from the top of the repository, and the directories
    /// above it.
    pub fn write(&self, path: &str, content: &str) {
        let at = self.path().join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, content).unwrap();
    }

    /// What a file holds, from the top of the repository.
    pub fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.path().join(path)).unwrap()
    }

    /// Declares the directory at path a pile, by writing its config, with
    /// the name when it is not empty. The id is the name or path, repeated
    /// to 32 hex characters, so a test can tell the piles apart.
    pub fn declare(&self, path: &str, id: &str, name: &str) {
        let dir = if path == "." {
            String::new()
        } else {
            format!("{path}/")
        };
        let mut config = format!("kind = gitpile\nversion = 1\nid = {}\n", id_of(id));
        if !name.is_empty() {
            config.push_str(&format!("name = {name}\n"));
        }
        self.write(&format!("{dir}.pile/config"), &config);
    }

    /// Writes a pile's intent.
    pub fn intent(&self, path: &str, text: &str) {
        let dir = if path == "." {
            String::new()
        } else {
            format!("{path}/")
        };
        self.write(&format!("{dir}.pile/intent.md"), text);
    }

    /// Commits everything in the working copy.
    pub fn commit(&self, subject: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "--allow-empty", "-m", subject]);
    }

    /// Runs `pile` in a directory under the top of the repository.
    pub fn answered_in(&self, dir: &str, env: &[(&str, &str)], args: &[&str]) -> Answered {
        answered(&self.path().join(dir), env, args)
    }

    /// Runs `pile` with --format json, and fails the test unless it exits 0.
    #[track_caller]
    pub fn result(&self, args: &[&str]) -> serde_json::Value {
        self.result_in(".", args)
    }

    /// The same, run in a directory under the top of the repository.
    #[track_caller]
    pub fn result_in(&self, dir: &str, args: &[&str]) -> serde_json::Value {
        let mut all = args.to_vec();
        all.extend(["--format", "json"]);
        let a = self.answered_in(dir, &[], &all);
        assert_eq!(
            a.code,
            0,
            "pile {}:\n{}{}",
            args.join(" "),
            a.stdout,
            a.stderr
        );
        a.json()["result"].clone()
    }

    /// Runs `pile` with --format json, fails the test unless it refuses
    /// with this exit code, and gives back the envelope.
    #[track_caller]
    pub fn refused(&self, args: &[&str], code: i32) -> serde_json::Value {
        self.refused_in(".", args, code)
    }

    /// The same, run in a directory under the top of the repository.
    #[track_caller]
    pub fn refused_in(&self, dir: &str, args: &[&str], code: i32) -> serde_json::Value {
        let mut all = args.to_vec();
        all.extend(["--format", "json"]);
        let a = self.answered_in(dir, &[], &all);
        assert_eq!(
            a.code,
            code,
            "pile {}:\n{}{}",
            args.join(" "),
            a.stdout,
            a.stderr
        );
        a.json()
    }

    /// Runs git in the repository, and fails the test if git fails.
    #[track_caller]
    pub fn git(&self, args: &[&str]) -> String {
        let out = command("git", self.path())
            .args(args)
            .output()
            .expect("running git");
        assert!(
            out.status.success(),
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Runs `pile` in the repository.
    pub fn answered(&self, env: &[(&str, &str)], args: &[&str]) -> Answered {
        answered(self.path(), env, args)
    }

    /// Runs `pile` in the repository, and fails the test unless it exits 0.
    #[track_caller]
    pub fn stdout(&self, args: &[&str]) -> String {
        let a = self.answered(&[], args);
        assert_eq!(
            a.code,
            0,
            "pile {} exited {}:\n{}{}",
            args.join(" "),
            a.code,
            a.stdout,
            a.stderr
        );
        a.stdout
    }
}

/// An id made from a word: its hex digits, or the hex of its bytes,
/// repeated to 32 characters.
pub fn id_of(word: &str) -> String {
    let hex: String = if word
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        && !word.is_empty()
    {
        word.to_string()
    } else {
        word.bytes().map(|b| format!("{b:02x}")).collect()
    };
    hex.repeat(32).chars().take(32).collect()
}

#[track_caller]
pub fn check_contains(out: &str, want: &str) {
    assert!(
        out.contains(want),
        "the output does not contain {want:?}:\n{out}"
    );
}

/// Every one of the strings is in the output.
macro_rules! contains {
    ($out:expr, $($want:expr),+ $(,)?) => {{
        let held = &$out;
        let out: &str = ::std::convert::AsRef::<str>::as_ref(held);
        $( $crate::harness::check_contains(out, ::std::convert::AsRef::<str>::as_ref(&$want)); )+
    }};
}
