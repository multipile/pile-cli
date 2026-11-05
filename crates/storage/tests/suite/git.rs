//! What only the git backend does: it opens a directory, it follows git's
//! ignore rules, it sees a conflict in the index, and it works out the main
//! branch, the upstream and the fork point from the repository's refs.

use storage::git::Git;
use storage::{BaseRole, Change, ChangeKind, Error, State, Storage, Subtree};

use crate::fixture::{Fixture, GitRepo, run};

#[test]
fn a_directory_in_no_repository_is_refused_as_not_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    // A temporary directory is in no repository on any machine the suite
    // runs on.
    assert_eq!(
        Git::open_alone(dir.path()).err(),
        Some(Error::NotARepository)
    );
    assert!(matches!(
        Git::open_alone(&dir.path().join("missing")),
        Err(Error::Failed(_))
    ));
}

#[test]
fn a_directory_inside_the_repository_opens_the_repository_at_its_top() {
    let r = GitRepo::new();
    std::fs::create_dir_all(r.path().join("a/b")).unwrap();
    let inner = Git::open_alone(&r.path().join("a/b")).unwrap();
    let top = std::fs::canonicalize(r.path()).unwrap();
    assert_eq!(inner.working_copy(), Some(top.as_path()));
}

#[test]
fn an_ignored_file_is_not_in_the_working_copy() {
    let mut r = GitRepo::new();
    r.write("/.gitignore", "target/\n*.log\n");
    r.put("/target/.pile/config", "built\n");
    r.put("/run.log", "noise\n");
    r.put("/kept", "kept\n");
    let s = r.storage();
    let files = s.files(&State::Working, &Subtree::whole()).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["/.gitignore", "/kept"]);
    assert!(s.declarations(&State::Working).unwrap().is_empty());
    assert_eq!(
        s.changes(None, &State::Working).unwrap(),
        [
            Change::new("/.gitignore", ChangeKind::Added),
            Change::new("/kept", ChangeKind::Untracked),
        ]
    );
}

#[test]
fn a_tracked_file_deleted_from_disk_is_deleted_and_not_listed() {
    let mut r = GitRepo::new();
    r.write("/a", "a");
    let first = r.commit("one");
    std::fs::remove_file(r.path().join("a")).unwrap();
    let s = r.storage();
    assert!(
        s.files(&State::Working, &Subtree::whole())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        s.changes(Some(&first), &State::Working).unwrap(),
        [Change::new("/a", ChangeKind::Deleted)]
    );
}

#[test]
fn a_path_in_a_conflict_is_conflicted_and_nothing_else() {
    let mut r = GitRepo::new();
    r.write("/z", "base\n");
    r.commit("base");
    r.git(&["switch", "-q", "-c", "side"]);
    r.write("/z", "side\n");
    r.commit("side");
    r.git(&["switch", "-q", "main"]);
    r.write("/z", "main\n");
    let head = r.commit("main");
    let merge = crate::fixture::git_command(r.path())
        .args(["merge", "-q", "side"])
        .output()
        .unwrap();
    assert!(!merge.status.success(), "the merge conflicts");
    assert_eq!(
        r.storage().changes(Some(&head), &State::Working).unwrap(),
        [Change::new("/z", ChangeKind::Conflicted)]
    );
}

#[test]
fn reading_writes_nothing_into_the_repository() {
    let mut r = GitRepo::new();
    r.write("/a", "a");
    let first = r.commit("one");
    r.write("/a", "edited");
    let before = r.git(&["status", "--porcelain=v2", "-uall"]);
    let index = std::fs::read(r.path().join(".git/index")).unwrap();
    let s = r.storage();
    s.changes(Some(&first), &State::Working).unwrap();
    s.files(&State::Working, &Subtree::whole()).unwrap();
    s.declarations(&State::Working).unwrap();
    s.history(&Subtree::whole(), None, None).unwrap();
    s.bases(5).unwrap();
    assert_eq!(std::fs::read(r.path().join(".git/index")).unwrap(), index);
    assert_eq!(r.git(&["status", "--porcelain=v2", "-uall"]), before);
}

#[test]
fn a_stale_index_is_read_without_being_refreshed() {
    let mut r = GitRepo::new();
    r.write("/same", "unchanged\n");
    r.write("/edited", "before\n");
    r.make_executable("/edited");
    r.write("/link-target", "x");
    r.symlink("/link", "link-target");
    let first = r.commit("one");
    // The same content, written again an hour on: the index's stat data no
    // longer matches the file, so git would refresh it to read it.
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
    for (path, content) in [("same", "unchanged\n"), ("edited", "after\n")] {
        std::fs::write(r.path().join(path), content).unwrap();
        let f = std::fs::File::options()
            .write(true)
            .open(r.path().join(path))
            .unwrap();
        f.set_modified(later).unwrap();
    }
    let index = std::fs::read(r.path().join(".git/index")).unwrap();
    let got = r.storage().changes(Some(&first), &State::Working).unwrap();
    assert_eq!(got, [Change::new("/edited", ChangeKind::Modified)]);
    assert_eq!(std::fs::read(r.path().join(".git/index")).unwrap(), index);
}

#[test]
fn a_name_git_knows_resolves_to_its_commit() {
    let mut r = GitRepo::new();
    r.write("/a", "1");
    let first = r.commit("one");
    r.write("/a", "2");
    let second = r.commit("two");
    r.git(&["tag", "v1", first.as_str()]);
    let s = r.storage();
    assert_eq!(s.resolve("main").unwrap(), second);
    assert_eq!(s.resolve("HEAD~1").unwrap(), first);
    assert_eq!(s.resolve("v1").unwrap(), first);
    assert_eq!(s.resolve(&first.as_str()[..10]).unwrap(), first);
    // A tree is not a revision, though git can name one.
    let tree = r.git(&["rev-parse", "HEAD^{tree}"]);
    assert!(matches!(
        s.resolve(tree.trim()),
        Err(Error::UnknownRevision(_))
    ));
}

#[test]
fn on_main_the_main_branch_is_offered_and_there_is_no_fork_point() {
    let mut r = GitRepo::new();
    r.write("/a", "1");
    let head = r.commit("one");
    let b = r.storage().bases(5).unwrap();
    assert_eq!(b.branch.as_deref(), Some("main"));
    let named: Vec<(&str, BaseRole)> = b.named.iter().map(|n| (n.name.as_str(), n.role)).collect();
    assert_eq!(named, [("main", BaseRole::Main)]);
    assert_eq!(b.named[0].info.revision, head);
}

#[test]
fn on_another_branch_the_fork_point_is_where_it_left_main() {
    let mut r = GitRepo::new();
    r.write("/a", "1");
    let fork = r.commit("one");
    r.git(&["switch", "-q", "-c", "work"]);
    r.write("/a", "2");
    r.commit("on work");
    r.git(&["switch", "-q", "main"]);
    r.write("/b", "3");
    let main = r.commit("on main");
    r.git(&["switch", "-q", "work"]);
    let b = r.storage().bases(5).unwrap();
    assert_eq!(b.branch.as_deref(), Some("work"));
    let named: Vec<(&str, BaseRole, &storage::Revision)> = b
        .named
        .iter()
        .map(|n| (n.name.as_str(), n.role, &n.info.revision))
        .collect();
    assert_eq!(
        named,
        [
            ("main", BaseRole::Main, &main),
            ("main", BaseRole::ForkPoint, &fork)
        ]
    );
}

#[test]
fn the_upstream_and_the_remote_main_branch_come_from_the_refs() {
    let mut upstream = GitRepo::new();
    upstream.write("/a", "1");
    let published = upstream.commit("published");
    let clone = tempfile::tempdir().unwrap();
    run(
        clone.path(),
        &["clone", "-q", upstream.path().to_str().unwrap(), "here"],
    );
    let here = clone.path().join("here");
    run(
        &here,
        &["switch", "-q", "-c", "work", "--track", "origin/main"],
    );
    let s = Git::open_alone(&here).unwrap();
    let b = s.bases(1).unwrap();
    let named: Vec<(&str, BaseRole)> = b.named.iter().map(|n| (n.name.as_str(), n.role)).collect();
    assert_eq!(
        named,
        [
            ("origin/main", BaseRole::Main),
            ("origin/main", BaseRole::Upstream),
            ("origin/main", BaseRole::ForkPoint),
        ]
    );
    assert!(b.named.iter().all(|n| n.info.revision == published));
}

#[test]
fn a_repository_inside_the_working_copy_is_one_submodule_and_holds_nothing() {
    let mut r = GitRepo::new();
    r.write("/a", "a");
    let inner = r.path().join("vendor/inner");
    std::fs::create_dir_all(&inner).unwrap();
    run(&inner, &["init", "-q"]);
    std::fs::write(inner.join("x"), "x").unwrap();
    let s = r.storage();
    let files = s.files(&State::Working, &Subtree::whole()).unwrap();
    let kinds: Vec<(&str, storage::FileKind)> =
        files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
    assert_eq!(
        kinds,
        [
            ("/a", storage::FileKind::File),
            ("/vendor/inner", storage::FileKind::Submodule)
        ]
    );
    assert_eq!(s.read(&State::Working, &["/vendor/inner"]).unwrap(), [None]);
}

#[test]
fn a_revision_keeps_the_offset_its_author_made_it_at() {
    let mut r = GitRepo::new();
    r.write("/a", "a");
    let out = crate::fixture::git_command(r.path())
        .args(["commit", "-q", "-m", "late"])
        .env("GIT_AUTHOR_DATE", "@1793143800 -0230")
        .env("GIT_COMMITTER_DATE", "@1793143800 -0230")
        .output()
        .unwrap();
    assert!(out.status.success());
    let h = r.storage().history(&Subtree::whole(), None, None).unwrap();
    let info = &h.revisions[0].info;
    assert_eq!((info.at, info.offset), (1_793_143_800, -9000));
    assert_eq!(info.day(), "2026-10-27");
}
