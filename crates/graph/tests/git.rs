//! The rows read from real git repositories, through the git backend.
//!
//! Every git run here has no GIT_* variable from whatever started the
//! tests, and a HOME of its own, so nobody's configuration reaches a
//! repository made here. The storage is opened with `Git::open_alone` for
//! the same reason.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use graph::{HEADLINE, NAME, Node, PATH, Value, read};
use piles::Position;
use storage::git::Git;
use storage::{State, Storage};
use tempfile::TempDir;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn home() -> &'static Path {
    static HOME: OnceLock<TempDir> = OnceLock::new();
    HOME.get_or_init(|| TempDir::with_prefix("graph-test-home-").unwrap())
        .path()
}

#[track_caller]
fn git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir).stdin(Stdio::null()).args(args);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(&key);
        }
    }
    let out = cmd
        .env("HOME", home())
        .env("XDG_CONFIG_HOME", home().join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("running git");
    assert!(
        out.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write(dir: &Path, path: &str, content: &str) {
    let at = dir.join(path);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(at, content).unwrap();
}

/// A repository with a pile at the top and one at `lib`, committed.
fn repository() -> (TempDir, Git) {
    let dir = TempDir::with_prefix("graph-test-").unwrap();
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"]);
    git(p, &["config", "user.name", "Test"]);
    git(p, &["config", "user.email", "test@example.com"]);
    git(p, &["config", "commit.gpgsign", "false"]);
    write(p, ".pile/config", &piles::config::render(A, "top"));
    write(p, ".pile/intent.md", "The whole of it.\n");
    write(p, "lib/.pile/config", &piles::config::render(B, "lib"));
    write(p, "lib/.pile/intent.md", "The parser.\n");
    write(p, "lib/parse.rs", "fn parse() {}\n");
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "lay out"]);
    let storage = Git::open_alone(p).unwrap();
    (dir, storage)
}

fn lib() -> Node {
    Node::Pile(B.to_string())
}

fn text(s: &str) -> Option<Value> {
    Some(Value::Text(s.to_string()))
}

#[test]
fn the_rows_of_the_working_copy_and_of_a_revision_are_the_same_when_nothing_changed() {
    let (_dir, s) = repository();
    let head = s.head().unwrap().unwrap();
    let working = read(&s, &State::Working).unwrap();
    assert_eq!(working, read(&s, &State::At(head)).unwrap());
    assert_eq!(working.node.len(), 2);
    assert_eq!(working.value(&lib(), PATH).cloned(), text(":/lib"));
    assert_eq!(
        working.value(&lib(), NAME).cloned(),
        Some(Value::Text("lib".into()))
    );
    let headline = working.prop_at(&lib(), HEADLINE).unwrap();
    assert_eq!(
        (headline.file.as_str(), headline.at),
        (":/lib/.pile/intent.md", Position { line: 1, column: 1 })
    );
}

#[test]
fn a_pile_moved_with_git_mv_keeps_its_node() {
    let (dir, s) = repository();
    let before = s.head().unwrap().unwrap();
    git(dir.path(), &["mv", "lib", "parser"]);
    let moved = read(&s, &State::Working).unwrap();
    let then = read(&s, &State::At(before)).unwrap();
    assert_eq!(moved.node, then.node);
    assert_eq!(then.value(&lib(), PATH).cloned(), text(":/lib"));
    assert_eq!(moved.value(&lib(), PATH).cloned(), text(":/parser"));
    assert_eq!(moved.at(&lib()).unwrap().file, ":/parser/.pile/config");
    assert_eq!(
        moved.prop_at(&lib(), HEADLINE).unwrap().file,
        ":/parser/.pile/intent.md"
    );

    git(dir.path(), &["commit", "-q", "-m", "move"]);
    let after = s.head().unwrap().unwrap();
    assert_eq!(read(&s, &State::At(after)).unwrap(), moved);
}

#[test]
fn a_config_with_a_key_it_does_not_take_reads_no_rows() {
    let (dir, s) = repository();
    write(
        dir.path(),
        "lib/.pile/config",
        &format!("kind = gitpile\nversion = 1\nid = {B}\nlabels = core\n"),
    );
    let err = read(&s, &State::Working).unwrap_err();
    assert!(err.to_string().contains("a config holds only"), "{err}");
}
