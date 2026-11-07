//! The pile model, read from a repository held in memory.

use storage::memory::Memory;
use storage::{Change, ChangeKind, State};

use super::*;
use crate::changes::arrange;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "cccccccccccccccccccccccccccccccc";
const D: &str = "dddddddddddddddddddddddddddddddd";

fn declare(m: &mut Memory, path: &str, id: &str, name: &str) {
    let file = join(path, &[PILE_DIR, CONFIG_FILE]);
    m.write(&file, config::render(id, name));
}

/// /, /lib, /lib/sub, /lib-v2.
fn repository() -> Memory {
    let mut m = Memory::new();
    declare(&mut m, "/", A, "top");
    declare(&mut m, "/lib", B, "lib");
    declare(&mut m, "/lib/sub", C, "");
    declare(&mut m, "/lib-v2", D, "v2");
    m.write("/lib/.pile/intent.md", "# /lib\n\nThe parser.\n");
    m.write("/lib/.pile/links.toml", "");
    m.write("/lib/.pile/notes.md", "");
    m.write("/.pile/tasks.json", "");
    m
}

fn paths(piles: &[&Pile]) -> Vec<String> {
    piles.iter().map(|p| p.path.clone()).collect()
}

#[test]
fn every_config_is_a_pile_and_every_file_but_a_toml_one_is_unread() {
    let s = discover(&repository(), &State::Working).unwrap();
    assert_eq!(paths(&s.ordered()), ["/", "/lib", "/lib-v2", "/lib/sub"]);
    // A .toml file is a declaration crates/graph reads, so it is not unread.
    assert_eq!(s.unread, ["/.pile/tasks.json", "/lib/.pile/notes.md"]);
    assert_eq!(s.unread_in("/lib"), ["/lib/.pile/notes.md"]);
    assert_eq!(s.unread_in("/"), ["/.pile/tasks.json"]);
    let lib = s.find("/lib").unwrap();
    assert_eq!(lib.headline(), "The parser.");
    assert_eq!((lib.name.as_str(), lib.id.as_str()), ("lib", B));
    assert_eq!(s.find("/").unwrap().intent, None);
}

#[test]
fn a_path_is_in_the_nearest_pile_above_it() {
    let s = discover(&repository(), &State::Working).unwrap();
    let owner = |p: &str| s.owner(p).map(|p| p.path.as_str());
    assert_eq!(owner("/lib/src/a.rs"), Some("/lib"));
    assert_eq!(owner("/lib/sub/x"), Some("/lib/sub"));
    assert_eq!(owner("/lib-v2/x"), Some("/lib-v2"));
    assert_eq!(owner("/library/x"), Some("/"));
    assert_eq!(owner("/lib"), Some("/lib"));
    assert_eq!(s.parent("/lib/sub").map(|p| p.path.as_str()), Some("/lib"));
    assert_eq!(s.parent("/"), None);
    assert_eq!(paths(&s.children("/")), ["/lib", "/lib-v2"]);
    assert_eq!(paths(&s.children("/lib")), ["/lib/sub"]);
    assert_eq!(paths(&s.from("/lib")), ["/lib", "/lib/sub"]);
    assert_eq!(s.own("/lib").except, ["/lib/sub"]);
}

#[test]
fn a_repository_whose_top_is_not_a_pile_has_paths_in_no_pile() {
    let mut m = Memory::new();
    declare(&mut m, "/a", A, "");
    let s = discover(&m, &State::Working).unwrap();
    assert!(s.owner("/b/x").is_none());
    assert_eq!(s.parent("/a").map(|p| p.path.clone()), None);
}

#[test]
fn ids_and_names_two_piles_claim_are_named_with_every_claimant() {
    let mut m = Memory::new();
    declare(&mut m, "/", A, "one");
    declare(&mut m, "/copy", A, "one");
    declare(&mut m, "/other", B, "");
    let s = discover(&m, &State::Working).unwrap();
    assert_eq!(paths(&s.duplicate_ids()[A]), ["/", "/copy"]);
    assert_eq!(paths(&s.duplicate_names()["one"]), ["/", "/copy"]);
    assert_eq!(s.by_name("one").len(), 2);
    assert_eq!(s.names(), ["one"]);
}

#[test]
fn a_config_that_cannot_be_read_refuses_the_whole_discovery() {
    let mut m = repository();
    m.write("/lib/.pile/config", "kind = gitpile\n");
    let err = discover(&m, &State::Working).unwrap_err();
    assert_eq!(
        err,
        Error::Config {
            file: "/lib/.pile/config".into(),
            why: config::Refused {
                line: None,
                why: "it names no version".into()
            }
        }
    );
    assert_eq!(err.to_string(), ":/lib/.pile/config: it names no version");
}

#[test]
fn a_pile_directory_with_no_config_declares_nothing() {
    let mut m = Memory::new();
    m.write("/x/.pile/intent.md", "Why.\n");
    let s = discover(&m, &State::Working).unwrap();
    assert!(s.is_empty());
    assert_eq!(s.unread, ["/x/.pile/intent.md"]);
}

#[test]
fn piles_are_read_at_a_revision_as_it_recorded_them() {
    let mut m = repository();
    let r = m.commit("declare");
    m.remove("/lib-v2/.pile/config");
    declare(&mut m, "/new", A, "new");
    let then = discover(&m, &State::At(r)).unwrap();
    let now = discover(&m, &State::Working).unwrap();
    assert!(then.find("/lib-v2").is_some() && then.find("/new").is_none());
    assert!(now.find("/lib-v2").is_none() && now.find("/new").is_some());
}

#[test]
fn a_heading_names_the_pile_first_then_its_path_and_short_id() {
    let s = discover(&repository(), &State::Working).unwrap();
    assert_eq!(s.find("/lib").unwrap().heading(), "@lib  :/lib  bbbbbbbb");
    assert_eq!(s.find("/lib/sub").unwrap().heading(), ":/lib/sub  cccccccc");
    assert_eq!(s.find("/lib/sub").unwrap().handle(), ":/lib/sub");
    assert_eq!(s.find("/").unwrap().intent_path(), "/.pile/intent.md");
    assert_eq!(shown("/"), ":/");
    assert_eq!(shown("/lib/a"), ":/lib/a");
}

fn moved(from: &str, to: &str) -> Change {
    Change {
        from: Some(from.into()),
        ..Change::new(to, ChangeKind::Moved)
    }
}

#[test]
fn a_change_goes_to_its_pile_and_a_move_across_a_boundary_to_both() {
    let s = discover(&repository(), &State::Working).unwrap();
    let changes = [
        Change::new("/lib/sub/x", ChangeKind::Modified),
        Change::new("/README", ChangeKind::Untracked),
        moved("/lib/a", "/lib/b"),
        moved("/lib/c", "/lib-v2/c"),
    ];
    let v = arrange(&s, "/", &changes);
    type Seen<'a> = (&'a str, &'a str, &'a str, &'a str, bool);
    let by: Vec<(&str, Vec<Seen>)> = v
        .groups
        .iter()
        .map(|g| {
            let es = g
                .entries
                .iter()
                .map(|e| {
                    (
                        e.letter(),
                        e.path.as_str(),
                        e.from.as_str(),
                        e.to.as_str(),
                        e.crossed,
                    )
                })
                .collect();
            (g.pile.path.as_str(), es)
        })
        .collect();
    assert_eq!(
        by,
        [
            ("/", vec![("?", "/README", "", "", false)]),
            (
                "/lib",
                vec![
                    ("R", "/lib/b", "/lib/a", "", false),
                    ("D", "/lib/c", "", "/lib-v2/c", true),
                ]
            ),
            ("/lib-v2", vec![("A", "/lib-v2/c", "/lib/c", "", true)]),
            ("/lib/sub", vec![("M", "/lib/sub/x", "", "", false)]),
        ]
    );
}

#[test]
fn standing_in_a_pile_scopes_the_piles_and_what_is_in_no_pile() {
    let mut m = Memory::new();
    declare(&mut m, "/a", A, "");
    declare(&mut m, "/a/b", B, "");
    let s = discover(&m, &State::Working).unwrap();
    let changes = [
        Change::new("/a/b/x", ChangeKind::Added),
        Change::new("/stray", ChangeKind::Untracked),
        moved("/a/y", "/out/y"),
    ];
    let wide = arrange(&s, "/", &changes);
    assert_eq!(wide.in_no_pile.len(), 2);
    let a = &wide.in_no_pile[0];
    assert_eq!(
        (a.letter(), a.path.as_str(), a.from.as_str(), a.crossed),
        ("A", "/out/y", "/a/y", true)
    );
    let narrow = arrange(&s, "/a/b", &changes);
    assert_eq!(narrow.groups.len(), 1);
    assert_eq!(narrow.groups[0].entries.len(), 1);
    assert!(narrow.in_no_pile.is_empty());
}

#[test]
fn a_change_to_a_piles_intent_is_its_intent_written() {
    let s = discover(&repository(), &State::Working).unwrap();
    let changes = [
        Change::new("/lib/.pile/intent.md", ChangeKind::Modified),
        Change::new("/lib/sub/.pile/notes.md", ChangeKind::Added),
        Change::new("/lib-v2/intent.md", ChangeKind::Added),
    ];
    let v = arrange(&s, "/", &changes);
    let written: Vec<(&str, bool)> = v
        .groups
        .iter()
        .map(|g| (g.pile.path.as_str(), g.intent_written()))
        .collect();
    assert_eq!(
        written,
        [
            ("/", false),
            ("/lib", true),
            ("/lib-v2", false),
            ("/lib/sub", false)
        ]
    );
}

#[test]
fn a_change_to_a_piles_toml_is_its_declarations_written() {
    let s = discover(&repository(), &State::Working).unwrap();
    let changes = [
        Change::new("/lib/.pile/links.toml", ChangeKind::Added),
        Change::new("/lib/sub/.pile/notes.md", ChangeKind::Added),
        Change::new("/lib-v2/links.toml", ChangeKind::Added),
    ];
    let v = arrange(&s, "/", &changes);
    let written: Vec<(&str, bool)> = v
        .groups
        .iter()
        .map(|g| (g.pile.path.as_str(), g.declarations_written()))
        .collect();
    assert_eq!(
        written,
        [
            ("/", false),
            ("/lib", true),
            ("/lib-v2", false),
            ("/lib/sub", false)
        ]
    );
}
