//! What every backend promises, as cases. Each case builds a repository
//! through a [`Fixture`], asks it questions, and checks the answers. main.rs
//! runs every case against every backend.

use storage::memory::{AUTHOR, FIRST_AT};
use storage::{BaseRole, Change, ChangeKind, Content, Error, File, FileKind, State, Subtree};

use crate::fixture::Fixture;

fn paths(files: &[File]) -> Vec<&str> {
    files.iter().map(|f| f.path.as_str()).collect()
}

fn texts(found: &[Content]) -> Vec<(&str, &str)> {
    found
        .iter()
        .map(|c| (c.path.as_str(), std::str::from_utf8(&c.bytes).unwrap()))
        .collect()
}

fn moved(from: &str, to: &str) -> Change {
    Change {
        from: Some(from.into()),
        ..Change::new(to, ChangeKind::Moved)
    }
}

pub fn a_repository_with_no_revision_has_no_head_and_no_history(f: &mut dyn Fixture) {
    let s = f.storage();
    assert_eq!(s.head().unwrap(), None);
    assert!(s.working_copy().is_none_or(|p| p.is_dir()));
    let h = s.history(&Subtree::whole(), None, None).unwrap();
    assert_eq!((h.total, h.revisions.len()), (0, 0));
    let b = s.bases(10).unwrap();
    assert_eq!(b.head, None);
    assert!(b.recent.is_empty() && b.named.is_empty(), "{b:?}");
    assert!(s.changes(None, &State::Working).unwrap().is_empty());
    assert!(
        s.files(&State::Working, &Subtree::whole())
            .unwrap()
            .is_empty()
    );
    assert!(s.declarations(&State::Working).unwrap().is_empty());
}

pub fn before_the_first_revision_every_file_is_added_or_untracked(f: &mut dyn Fixture) {
    f.write("/a", "a\n");
    f.write_untracked("/b/c", "c\n");
    let s = f.storage();
    assert_eq!(
        s.changes(None, &State::Working).unwrap(),
        [
            Change::new("/a", ChangeKind::Added),
            Change::new("/b/c", ChangeKind::Untracked),
        ]
    );
    assert_eq!(
        paths(&s.files(&State::Working, &Subtree::whole()).unwrap()),
        ["/a", "/b/c"]
    );
}

pub fn declarations_are_every_file_inside_a_pile_directory(f: &mut dyn Fixture) {
    f.write("/.pile/config", "root\n");
    f.write("/crates/x/.pile/config", "x\n");
    f.write("/crates/x/.pile/intent.md", "why x\n");
    f.write("/crates/x/.pile/guide/one.md", "one\n");
    f.write("/crates/x/src/lib.rs", "code\n");
    f.write("/a.pile/config", "not a pile\n");
    f.write_untracked("/new/.pile/config", "declared a minute ago\n");
    let found = f.storage().declarations(&State::Working).unwrap();
    assert_eq!(
        texts(&found),
        [
            ("/.pile/config", "root\n"),
            ("/crates/x/.pile/config", "x\n"),
            ("/crates/x/.pile/guide/one.md", "one\n"),
            ("/crates/x/.pile/intent.md", "why x\n"),
            ("/new/.pile/config", "declared a minute ago\n"),
        ]
    );
    assert!(found.iter().all(|c| c.kind == FileKind::File));
}

pub fn declarations_at_a_revision_are_what_it_recorded(f: &mut dyn Fixture) {
    f.write("/.pile/config", "first\n");
    f.write("/gone/.pile/config", "gone\n");
    let first = f.commit("declare");
    f.write("/.pile/config", "second\n");
    f.remove("/gone/.pile/config");
    f.write_untracked("/later/.pile/config", "later\n");
    let s = f.storage();
    assert_eq!(
        texts(&s.declarations(&State::At(first.clone())).unwrap()),
        [
            ("/.pile/config", "first\n"),
            ("/gone/.pile/config", "gone\n")
        ]
    );
    assert_eq!(
        texts(&s.declarations(&State::Working).unwrap()),
        [
            ("/.pile/config", "second\n"),
            ("/later/.pile/config", "later\n")
        ]
    );
}

pub fn the_files_of_a_subtree_leave_out_what_it_excepts(f: &mut dyn Fixture) {
    f.write("/top", "t");
    f.write("/crates/a/one", "1");
    f.write("/crates/b/two", "2");
    f.write("/crates/b/deep/three", "3");
    f.write("/crates-old/four", "4");
    let r = f.commit("files");
    f.write_untracked("/crates/a/new", "n");
    let s = f.storage();
    let of = Subtree::new("/crates").except("/crates/b");
    assert_eq!(
        paths(&s.files(&State::Working, &of).unwrap()),
        ["/crates/a/new", "/crates/a/one"]
    );
    assert_eq!(
        paths(&s.files(&State::At(r.clone()), &of).unwrap()),
        ["/crates/a/one"]
    );
    assert_eq!(
        paths(
            &s.files(&State::At(r), &Subtree::whole().except("/crates"))
                .unwrap()
        ),
        ["/crates-old/four", "/top"]
    );
}

pub fn a_file_is_listed_with_its_kind(f: &mut dyn Fixture) {
    f.write("/plain", "p");
    f.write("/run", "#!/bin/sh\n");
    f.make_executable("/run");
    f.symlink("/link", "plain");
    let r = f.commit("kinds");
    let s = f.storage();
    for at in [State::Working, State::At(r)] {
        let kinds: Vec<(String, FileKind)> = s
            .files(&at, &Subtree::whole())
            .unwrap()
            .into_iter()
            .map(|f| (f.path, f.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("/link".to_string(), FileKind::Symlink),
                ("/plain".to_string(), FileKind::File),
                ("/run".to_string(), FileKind::Executable),
            ],
            "{at:?}"
        );
        let read = s.read(&at, &["/link"]).unwrap();
        assert_eq!(
            read,
            [Some(b"plain".to_vec())],
            "a link holds where it points"
        );
    }
}

pub fn a_read_answers_each_path_in_order_and_none_where_no_file_is(f: &mut dyn Fixture) {
    f.write("/a", "first\n");
    f.write("/dir/b", "b\n");
    let r = f.commit("one");
    f.write("/a", "now\n");
    let s = f.storage();
    let asked = ["/dir/b", "/missing", "/a", "/dir"];
    assert_eq!(
        s.read(&State::At(r), &asked).unwrap(),
        [Some(b"b\n".to_vec()), None, Some(b"first\n".to_vec()), None]
    );
    assert_eq!(
        s.read(&State::Working, &asked).unwrap(),
        [Some(b"b\n".to_vec()), None, Some(b"now\n".to_vec()), None]
    );
    assert!(s.read(&State::Working, &[]).unwrap().is_empty());
}

pub fn the_changes_between_two_revisions_name_each_path_once(f: &mut dyn Fixture) {
    f.write("/keep", "same");
    f.write("/edit", "before");
    f.write("/drop", "dropped");
    f.write("/old/place", "a file that moves\n");
    f.write("/run", "x");
    let first = f.commit("first");
    f.write("/edit", "after");
    f.remove("/drop");
    f.rename("/old/place", "/new/place");
    f.write("/fresh", "new");
    f.make_executable("/run");
    let second = f.commit("second");
    let s = f.storage();
    assert_eq!(
        s.changes(Some(&first), &State::At(second.clone())).unwrap(),
        [
            Change::new("/drop", ChangeKind::Deleted),
            Change::new("/edit", ChangeKind::Modified),
            Change::new("/fresh", ChangeKind::Added),
            moved("/old/place", "/new/place"),
            Change {
                retyped: true,
                ..Change::new("/run", ChangeKind::Modified)
            },
        ]
    );
    assert!(
        s.changes(Some(&second), &State::At(second.clone()))
            .unwrap()
            .is_empty()
    );
}

pub fn from_the_empty_repository_every_file_is_added(f: &mut dyn Fixture) {
    f.write("/a", "a");
    f.write("/b/c", "c");
    let r = f.commit("first");
    assert_eq!(
        f.storage().changes(None, &State::At(r)).unwrap(),
        [
            Change::new("/a", ChangeKind::Added),
            Change::new("/b/c", ChangeKind::Added),
        ]
    );
}

pub fn the_working_copy_is_compared_with_any_base(f: &mut dyn Fixture) {
    f.write("/a", "one");
    let first = f.commit("first");
    f.write("/b", "two");
    let second = f.commit("second");
    f.write("/a", "edited");
    f.write_untracked("/u", "untracked");
    let s = f.storage();
    assert_eq!(
        s.changes(Some(&second), &State::Working).unwrap(),
        [
            Change::new("/a", ChangeKind::Modified),
            Change::new("/u", ChangeKind::Untracked),
        ]
    );
    assert_eq!(
        s.changes(Some(&first), &State::Working).unwrap(),
        [
            Change::new("/a", ChangeKind::Modified),
            Change::new("/b", ChangeKind::Added),
            Change::new("/u", ChangeKind::Untracked),
        ]
    );
}

pub fn a_move_in_the_working_copy_is_a_move(f: &mut dyn Fixture) {
    f.write("/pile-a/notes.md", "the same notes, word for word\n");
    f.write("/gone", "going");
    let r = f.commit("first");
    f.rename("/pile-a/notes.md", "/pile-b/notes.md");
    f.remove("/gone");
    assert_eq!(
        f.storage().changes(Some(&r), &State::Working).unwrap(),
        [
            Change::new("/gone", ChangeKind::Deleted),
            moved("/pile-a/notes.md", "/pile-b/notes.md"),
        ]
    );
}

pub fn a_history_is_newest_first_and_counts_what_it_leaves_out(f: &mut dyn Fixture) {
    f.write("/p/a", "1");
    let r1 = f.commit("one");
    f.write("/elsewhere", "x");
    f.commit("not p");
    f.write("/p/a", "2");
    f.write("/elsewhere", "y");
    let r3 = f.commit("three");
    f.write("/p/b", "3");
    let r4 = f.commit("four");
    let s = f.storage();
    let p = Subtree::new("/p");

    let all = s.history(&p, None, None).unwrap();
    assert_eq!(all.total, 3);
    let ids: Vec<_> = all
        .revisions
        .iter()
        .map(|t| t.info.revision.clone())
        .collect();
    assert_eq!(ids, [r4.clone(), r3.clone(), r1.clone()]);
    assert_eq!(
        all.revisions[1].paths,
        ["/elsewhere", "/p/a"],
        "every path it changed"
    );

    let last = s.history(&p, None, Some(1)).unwrap();
    assert_eq!(last.total, 3);
    assert_eq!(last.revisions.len(), 1);
    assert_eq!(last.revisions[0].info.revision, r4);

    let since = s.history(&p, Some(&r3), None).unwrap();
    assert_eq!(since.total, 1);
    assert_eq!(since.revisions[0].info.revision, r4);

    let none = s
        .history(&Subtree::new("/nothing-here"), None, None)
        .unwrap();
    assert_eq!((none.total, none.revisions.len()), (0, 0));
}

pub fn a_history_leaves_out_what_the_subtree_excepts(f: &mut dyn Fixture) {
    f.write("/p/own", "1");
    f.commit("own");
    f.write("/p/child/x", "2");
    f.commit("child");
    let s = f.storage();
    let h = s
        .history(&Subtree::new("/p").except("/p/child"), None, None)
        .unwrap();
    assert_eq!(h.total, 1);
    assert_eq!(h.revisions[0].info.subject, "own");
}

pub fn a_move_in_a_history_is_both_of_its_paths(f: &mut dyn Fixture) {
    f.write("/a/x", "a file that moves between piles\n");
    f.commit("put");
    f.rename("/a/x", "/b/x");
    f.commit("move");
    let s = f.storage();
    for side in ["/a", "/b"] {
        let h = s.history(&Subtree::new(side), None, Some(1)).unwrap();
        assert_eq!(h.revisions[0].info.subject, "move", "{side}");
        assert_eq!(h.revisions[0].paths, ["/a/x", "/b/x"], "{side}");
    }
}

pub fn a_revision_says_who_made_it_when_and_why(f: &mut dyn Fixture) {
    f.write("/a", "a");
    f.commit("first");
    f.write("/a", "b");
    let r = f.commit("second, with more after the comma");
    let h = f.storage().history(&Subtree::whole(), None, None).unwrap();
    let info = &h.revisions[0].info;
    assert_eq!(info.revision, r);
    assert_eq!(info.author, AUTHOR);
    assert_eq!(info.at, FIRST_AT + 60);
    assert_eq!(info.offset, 0);
    assert_eq!(info.day(), "2023-11-14");
    assert_eq!(info.subject, "second, with more after the comma");
    assert!(!info.short.is_empty() && r.as_str().starts_with(&info.short));
    assert_eq!(h.revisions[1].info.at, FIRST_AT);
}

pub fn a_revision_resolves_by_its_own_id(f: &mut dyn Fixture) {
    f.write("/a", "a");
    let r = f.commit("first");
    let s = f.storage();
    assert_eq!(s.resolve(r.as_str()).unwrap(), r);
    assert_eq!(s.head().unwrap(), Some(r.clone()));
    assert_eq!(r.to_string(), r.as_str());
    for name in ["no-such-revision", "", "--all"] {
        assert_eq!(
            s.resolve(name),
            Err(Error::UnknownRevision(name.to_string())),
            "{name:?}"
        );
    }
}

pub fn the_bases_start_at_the_head_and_go_back_along_its_branch(f: &mut dyn Fixture) {
    f.write("/a", "1");
    let r1 = f.commit("one");
    f.write("/a", "2");
    let r2 = f.commit("two");
    f.write("/a", "3");
    let r3 = f.commit("three");
    let s = f.storage();
    let b = s.bases(2).unwrap();
    assert_eq!(b.head.as_ref().map(|h| &h.revision), Some(&r3));
    let recent: Vec<_> = b.recent.iter().map(|i| i.revision.clone()).collect();
    assert_eq!(recent, [r3.clone(), r2]);
    assert!(s.bases(0).unwrap().recent.is_empty());
    assert_eq!(
        s.bases(10).unwrap().recent.last().map(|i| &i.revision),
        Some(&r1)
    );
    for base in &b.named {
        assert_eq!(
            s.resolve(&base.name).ok().as_ref(),
            Some(&base.info.revision)
        );
        if base.role != BaseRole::ForkPoint {
            assert_eq!(base.info.revision, r3);
        }
    }
}

pub fn the_merge_base_of_two_revisions_on_one_line_is_the_older(f: &mut dyn Fixture) {
    f.write("/a", "1");
    let r1 = f.commit("one");
    f.write("/a", "2");
    let r2 = f.commit("two");
    f.write("/a", "3");
    let r3 = f.commit("three");
    let s = f.storage();
    assert_eq!(s.merge_base(&r1, &r3).unwrap(), Some(r1.clone()));
    assert_eq!(s.merge_base(&r3, &r1).unwrap(), Some(r1));
    assert_eq!(s.merge_base(&r2, &r2).unwrap(), Some(r2));
}
