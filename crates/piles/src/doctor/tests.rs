//! The survey, read from repositories held in memory.

use storage::FileKind;
use storage::memory::Memory;

use super::*;
use crate::config::render;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn declare(m: &mut Memory, path: &str, id: &str, name: &str, intent: Option<&str>) {
    m.write(&join(path, &[PILE_DIR, CONFIG_FILE]), render(id, name));
    if let Some(text) = intent {
        m.write(&join(path, &[PILE_DIR, INTENT_FILE]), text);
    }
}

fn codes(s: &Survey, path: &str) -> Vec<&'static str> {
    s.find(path)
        .unwrap()
        .findings
        .iter()
        .map(|f| f.code)
        .collect()
}

/// A repository whose piles are all well, committed.
fn well() -> Memory {
    let mut m = Memory::new();
    declare(&mut m, "/", A, "top", Some("The whole of it.\n"));
    declare(&mut m, "/lib", B, "lib", Some("The parser.\n"));
    m.commit("declare");
    m
}

#[test]
fn a_repository_whose_piles_are_well_has_nothing_to_report() {
    let s = survey(&well()).unwrap();
    assert_eq!(s.rows.len(), 2);
    assert!(s.rows.iter().all(|r| r.findings.is_empty()), "{s:?}");
    assert!(!s.failed());
    let lib = s.find("/lib").unwrap();
    assert_eq!((lib.is, lib.headline.as_str()), (Is::Pile, "The parser."));
    assert_eq!(lib.parent.as_deref(), Some("/"));
    assert!(lib.committed);
}

#[test]
fn a_config_that_cannot_be_read_is_a_problem_naming_its_line_and_the_rest_is_still_read() {
    let mut m = well();
    m.write(
        "/bad/.pile/config",
        format!("kind = gitpile\nversion = 1\nid = {A}\ncolour = x\n"),
    );
    let s = survey(&m).unwrap();
    let bad = s.find("/bad").unwrap();
    assert_eq!(bad.is, Is::Refused);
    let f = &bad.findings[0];
    assert_eq!(
        (f.code, f.severity, f.line, f.column),
        ("unreadable-config", Severity::Problem, Some(4), None)
    );
    assert_eq!(f.file, "/bad/.pile/config");
    assert!(
        f.message
            .contains("a config holds only kind, version, id and name"),
        "{}",
        f.message
    );
    // A refused config claims no id, so the top's id is still its own.
    assert!(s.find("/").unwrap().findings.is_empty());
    // And it is still the pile above the piles under it.
    declare(
        &mut m,
        "/bad/inner",
        "7".repeat(32).as_str(),
        "inner",
        Some("Inner.\n"),
    );
    let s = survey(&m).unwrap();
    assert_eq!(
        s.find("/bad/inner").unwrap().parent.as_deref(),
        Some("/bad")
    );
    assert!(s.failed());
}

#[test]
fn an_id_or_a_name_claimed_twice_is_a_problem_in_each_pile() {
    let mut m = well();
    declare(&mut m, "/copy", B, "lib", Some("A copy.\n"));
    m.commit("copy");
    let s = survey(&m).unwrap();
    assert_eq!(
        codes(&s, "/copy"),
        ["id-claimed-twice", "name-claimed-twice"]
    );
    assert_eq!(
        codes(&s, "/lib"),
        ["id-claimed-twice", "name-claimed-twice"]
    );
    assert!(
        s.find("/lib").unwrap().findings[0]
            .message
            .contains(":/copy")
    );
}

#[test]
fn an_intent_missing_empty_unanswered_or_unclosed_is_a_problem() {
    let mut m = Memory::new();
    declare(&mut m, "/none", A, "none", None);
    declare(
        &mut m,
        "/empty",
        "1".repeat(32).as_str(),
        "empty",
        Some("  \n"),
    );
    declare(
        &mut m,
        "/asked",
        "2".repeat(32).as_str(),
        "asked",
        Some(intent::template()),
    );
    declare(
        &mut m,
        "/open",
        "3".repeat(32).as_str(),
        "open",
        Some("A claim.\n<!-- never\nclosed\n"),
    );
    m.write_kind(
        "/link/.pile/config",
        FileKind::File,
        render(&"4".repeat(32), "link"),
    );
    m.write_kind("/link/.pile/intent.md", FileKind::Symlink, "../../x");
    m.commit("declare");
    let s = survey(&m).unwrap();
    assert_eq!(codes(&s, "/none"), ["no-intent"]);
    assert_eq!(codes(&s, "/empty"), ["intent-empty"]);
    assert_eq!(codes(&s, "/asked"), ["intent-unanswered"]);
    assert_eq!(codes(&s, "/open"), ["intent-comment-unclosed"]);
    let open = &s.find("/open").unwrap().findings[0];
    assert_eq!((open.line, open.column), (Some(2), Some(1)));
    assert_eq!(codes(&s, "/link"), ["intent-not-a-file"]);
}

#[test]
fn a_heading_or_no_name_is_a_warning_and_does_not_fail() {
    let mut m = Memory::new();
    declare(&mut m, "/", A, "", Some("# /\n\nThe whole of it.\n"));
    m.commit("declare");
    let s = survey(&m).unwrap();
    assert_eq!(codes(&s, "/"), ["no-name", "intent-heading"]);
    let heading = &s.find("/").unwrap().findings[1];
    assert_eq!(
        (heading.severity, heading.line, heading.column),
        (Severity::Warning, Some(1), Some(1))
    );
    assert_eq!(s.find("/").unwrap().headline, "The whole of it.");
    assert!(!s.failed());
}

#[test]
fn unread_files_an_uncommitted_pile_and_a_pile_directory_with_no_config_are_notes() {
    let mut m = well();
    // A .toml file is crates/graph's to read, and to report on.
    m.write("/lib/.pile/links.toml", "");
    m.write("/lib/.pile/notes.md", "");
    m.write("/lib/.pile/guide/one.md", "");
    m.write("/stray/.pile/notes.txt", "");
    declare(
        &mut m,
        "/new",
        "5".repeat(32).as_str(),
        "new",
        Some("New.\n"),
    );
    let s = survey(&m).unwrap();
    assert_eq!(codes(&s, "/lib"), ["unread-file", "unread-file"]);
    assert_eq!(
        s.find("/lib").unwrap().findings[0].file,
        "/lib/.pile/guide/one.md"
    );
    assert_eq!(codes(&s, "/stray"), ["unread-file", "not-a-pile"]);
    assert_eq!(s.find("/stray").unwrap().is, Is::NotAPile);
    assert_eq!(codes(&s, "/new"), ["not-committed"]);
    assert!(!s.failed());
    assert_eq!((s.piles(), s.declaring_none()), (3, 1));
}

#[test]
fn a_pile_whose_config_is_gone_from_the_working_copy_is_being_unmade() {
    let mut m = well();
    m.remove("/lib/.pile/config");
    let s = survey(&m).unwrap();
    let lib = s.find("/lib").unwrap();
    assert_eq!((lib.is, codes(&s, "/lib")), (Is::Unmade, vec!["unmade"]));
    m.remove("/lib/.pile/intent.md");
    assert_eq!(survey(&m).unwrap().find("/lib").unwrap().is, Is::Unmade);
}

#[test]
fn a_config_inside_a_pile_directory_declares_nothing_and_is_a_problem() {
    let mut m = well();
    declare(
        &mut m,
        "/lib/.pile/inner",
        "6".repeat(32).as_str(),
        "inner",
        Some("Inner.\n"),
    );
    let s = survey(&m).unwrap();
    assert!(s.find("/lib/.pile/inner").is_none());
    let lib = codes(&s, "/lib");
    assert!(lib.contains(&"pile-in-pile-directory"), "{lib:?}");
    assert!(s.failed());
    let piles = crate::discover(&m, &State::Working).unwrap();
    assert!(piles.find("/lib/.pile/inner").is_none());
}

#[test]
fn a_symbolic_link_for_a_config_is_a_problem() {
    let mut m = well();
    m.write_kind(
        "/link/.pile/config",
        FileKind::Symlink,
        "../../lib/.pile/config",
    );
    let s = survey(&m).unwrap();
    assert_eq!(codes(&s, "/link"), ["config-not-a-file"]);
}

#[test]
fn another_repository_inside_one_is_a_note_and_a_pile_file_a_problem() {
    let mut m = well();
    m.write_kind("/lib/vendor/other", FileKind::Submodule, "");
    m.write("/lib/docs/.pile", "not a directory");
    m.write("/loose/.pile", "");
    let s = survey(&m).unwrap();
    let lib = codes(&s, "/lib");
    assert_eq!(lib, ["pile-directory-is-a-file", "holds-repository"]);
    // With no pile above it, the finding is kept on its own.
    let mut bare = Memory::new();
    bare.write("/x/.pile", "");
    let s = survey(&bare).unwrap();
    assert_eq!(s.in_no_pile[0].code, "pile-directory-is-a-file");
    assert!(s.failed());
}

#[test]
fn findings_added_from_elsewhere_join_the_row_of_their_pile() {
    let mut s = survey(&well()).unwrap();
    let f = |file: &str, severity| Finding {
        code: "x",
        severity,
        file: file.to_string(),
        line: None,
        column: None,
        message: String::new(),
        fix: String::new(),
    };
    s.add(vec![
        f("/lib/.pile/a.toml", Severity::Note),
        f("/lib/.pile/b.toml", Severity::Problem),
        f("/.pile/c.toml", Severity::Problem),
    ]);
    let lib = s.find("/lib").unwrap();
    let files: Vec<&str> = lib.findings.iter().map(|f| f.file.as_str()).collect();
    assert_eq!(files, ["/lib/.pile/b.toml", "/lib/.pile/a.toml"]);
    assert_eq!(s.find("/").unwrap().findings.len(), 1);
    assert!(s.failed());
}
