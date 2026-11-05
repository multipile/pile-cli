//! `pile doctor`: what every pile is, and what is wrong with any of them.

use serde_json::{Value, json};

use crate::harness::*;
use crate::piles::nested;

/// One row of the result, by its path.
fn row<'a>(v: &'a Value, path: &str) -> &'a Value {
    v["piles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == path)
        .unwrap_or_else(|| panic!("no row for {path}: {v}"))
}

/// The codes of one row's findings, by severity, as the result lists them.
fn findings(v: &Value, path: &str) -> Vec<(String, String)> {
    let row = row(v, path);
    let mut out = Vec::new();
    for sev in ["problems", "warnings", "notes"] {
        for f in row[sev].as_array().unwrap() {
            out.push((sev.to_string(), f["code"].as_str().unwrap().to_string()));
        }
    }
    out
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

/// nested(), with an intent for every pile, so only what a test breaks is
/// wrong. /lib's intent keeps its `# /lib` heading, which is a warning.
fn well_nested() -> Repo {
    let r = nested();
    r.intent("lib/sub", "The sub.\n");
    r.intent("lib-v2", "The next parser.\n");
    r.commit("explain");
    r
}

/// The report, whatever doctor exits with.
fn report(r: &Repo) -> (i32, Value) {
    let a = r.answered(&[], &["doctor", "--format", "json"]);
    (a.code, a.json())
}

#[test]
fn a_repository_whose_piles_are_well_exits_0_with_nothing_wrong() {
    let r = Repo::new();
    r.declare(".", "a", "top");
    r.intent(".", "The whole repository.\n");
    r.declare("lib", "b", "lib");
    r.intent("lib", "The parser.\n");
    r.commit("declare");
    let out = r.stdout(&["doctor"]);
    assert_eq!(
        out,
        "@top  :/  aaaaaaaa  gitpile/1\n   \
         clone     has this pile\n   \
         intent    The whole repository.\n   \
         ok        nothing to report\n\n\
         @lib  :/lib  bbbbbbbb  gitpile/1\n   \
         parent    @top  :/  aaaaaaaa\n   \
         clone     has this pile\n   \
         intent    The parser.\n   \
         ok        nothing to report\n\n\
         2 piles, nothing wrong\n"
    );
    let v = r.result(&["doctor"]);
    assert_eq!(
        v["total"],
        json!({"piles": 2, "declaring_none": 0, "needing_attention": 0, "problems": 0, "warnings": 0, "notes": 0})
    );
}

#[test]
fn a_problem_fails_doctor_and_the_whole_report_is_still_the_result() {
    let r = well_nested();
    r.write(
        "lib/sub/.pile/config",
        "kind = gitpile\nversion = 1\nalias = sub\n",
    );
    let (code, v) = report(&r);
    assert_eq!(code, 1);
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "problems-found");
    assert_eq!(v["result"]["piles"].as_array().unwrap().len(), 4);
    let row = &v["result"]["piles"][3];
    assert_eq!(
        (row["path"].clone(), row["is"].clone()),
        (json!(":/lib/sub"), json!("config refused"))
    );
    let f = &row["problems"][0];
    assert_eq!(f["code"], "unreadable-config");
    assert_eq!(f["file"], ":/lib/sub/.pile/config");
    assert_eq!(
        (f["line"].clone(), f["column"].clone()),
        (json!(3), json!(null))
    );
    contains!(
        f["message"].as_str().unwrap(),
        "a config holds only kind, version, id and name"
    );
    // Every other command still refuses.
    assert_eq!(
        r.refused(&["list"], 1)["error"]["code"],
        "unreadable-config"
    );
    let text = r.answered(&[], &["doctor"]);
    contains!(
        text.stdout,
        ":/lib/sub  (config refused)\n   parent    @lib  :/lib  bbbbbbbb\n   problem   :/lib/sub/.pile/config:3: the config cannot be read: line 3 names \"alias\"",
        "4 piles, 1 needing attention: 1 problem"
    );
    assert_eq!(
        text.stderr,
        "pile: 1 problem found, so doctor exits 1. Fix what the report lists\n  \
         pile guide doctor  says what each severity means, and how to work through a report\n"
    );
}

#[test]
fn an_id_or_a_name_two_piles_claim_is_a_problem_in_each() {
    let r = nested();
    r.declare("copy", "b", "lib");
    r.intent("copy", "A copy.\n");
    r.commit("copy");
    let (code, v) = report(&r);
    assert_eq!(code, 1);
    let both = pairs(&[
        ("problems", "id-claimed-twice"),
        ("problems", "name-claimed-twice"),
    ]);
    assert_eq!(findings(&v["result"], ":/copy"), both);
    assert_eq!(findings(&v["result"], ":/lib")[..2], both[..]);
    contains!(
        v["result"]["piles"][1]["problems"][0]["message"]
            .as_str()
            .unwrap(),
        "is claimed by :/lib too"
    );
}

#[test]
fn every_way_an_intent_can_fail_is_a_problem_naming_its_file() {
    let r = Repo::new();
    for (dir, id, intent) in [
        ("none", "1", None),
        ("empty", "2", Some("\n")),
        ("asked", "3", None),
        ("open", "4", Some("A claim.\n\n<!-- never closed\n")),
    ] {
        r.declare(dir, id, dir);
        if let Some(text) = intent {
            r.intent(dir, text);
        }
    }
    // The intent init writes, never answered.
    std::fs::remove_dir_all(r.path().join("asked/.pile")).unwrap();
    std::fs::create_dir_all(r.path().join("asked")).unwrap();
    r.answered_in("asked", &[], &["init"]);
    r.declare("asked", "3", "asked");
    r.declare("binary", "5", "binary");
    std::fs::write(r.path().join("binary/.pile/intent.md"), [0xff, 0xfe, b'\n']).unwrap();
    r.commit("declare");
    let (code, v) = report(&r);
    assert_eq!(code, 1);
    let v = &v["result"];
    assert_eq!(findings(v, ":/none"), pairs(&[("problems", "no-intent")]));
    assert_eq!(
        findings(v, ":/empty"),
        pairs(&[("problems", "intent-empty")])
    );
    assert_eq!(
        findings(v, ":/asked"),
        pairs(&[("problems", "intent-unanswered")])
    );
    assert_eq!(
        findings(v, ":/open"),
        pairs(&[("problems", "intent-comment-unclosed")])
    );
    assert_eq!(
        findings(v, ":/binary"),
        pairs(&[("problems", "intent-not-text")])
    );
    let open = &v["piles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == ":/open")
        .unwrap()["problems"][0];
    assert_eq!(
        (
            open["file"].clone(),
            open["line"].clone(),
            open["column"].clone()
        ),
        (json!(":/open/.pile/intent.md"), json!(3), json!(1))
    );
    contains!(
        r.answered(&[], &["doctor"]).stdout,
        "problem   :/open/.pile/intent.md:3:1: a comment opens here"
    );
}

#[test]
fn a_heading_and_no_name_are_warnings_and_doctor_still_exits_0() {
    let r = well_nested();
    // /lib's intent opens with a `# /lib` heading, and /lib/sub has no name.
    std::fs::remove_dir_all(r.path().join("lib/sub")).unwrap();
    r.commit("drop sub");
    let (code, v) = report(&r);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        findings(&v["result"], ":/lib"),
        pairs(&[("warnings", "intent-heading")])
    );
    let heading = &v["result"]["piles"][1]["warnings"][0];
    assert_eq!(
        (heading["line"].clone(), heading["column"].clone()),
        (json!(1), json!(1))
    );
    contains!(heading["fix"].as_str().unwrap(), "remove the heading line");
    r.declare("plain", "e", "");
    r.intent("plain", "Plain.\n");
    r.commit("plain");
    let (code, v) = report(&r);
    assert_eq!(code, 0);
    assert_eq!(
        findings(&v["result"], ":/plain"),
        pairs(&[("warnings", "no-name")])
    );
    contains!(
        row(&v["result"], ":/plain")["warnings"][0]["fix"]
            .as_str()
            .unwrap(),
        "pile set name <name> :/plain"
    );
    // The text starts its sentence with the command as it is typed.
    let text = r.answered(&[], &["doctor", "--format", "text"]).stdout;
    contains!(text, ". pile set name <name> :/plain gives it one");
}

#[test]
fn unread_files_an_uncommitted_pile_and_a_pile_directory_with_no_config_are_notes() {
    let r = well_nested();
    r.write("lib/.pile/notes.md", "x");
    r.write("stray/.pile/notes.txt", "x");
    r.commit("more");
    r.declare("new", "e", "new");
    r.intent("new", "New.\n");
    let (code, v) = report(&r);
    assert_eq!(code, 0, "{v}");
    let v = &v["result"];
    assert_eq!(
        findings(v, ":/lib"),
        pairs(&[("warnings", "intent-heading"), ("notes", "unread-file")])
    );
    assert_eq!(v["piles"][1]["notes"][0]["file"], ":/lib/.pile/notes.md");
    assert_eq!(findings(v, ":/lib/sub"), pairs(&[("warnings", "no-name")]));
    assert_eq!(findings(v, ":/new"), pairs(&[("notes", "not-committed")]));
    assert_eq!(
        findings(v, ":/stray"),
        pairs(&[("notes", "unread-file"), ("notes", "not-a-pile")])
    );
    assert_eq!(v["total"]["declaring_none"], 1);
    let text = r.stdout(&["doctor"]);
    contains!(
        text,
        ":/stray  (not a pile)\n",
        "   clone     will not have this pile until its config is committed\n"
    );
}

#[test]
fn a_pile_whose_config_is_deleted_and_not_yet_committed_is_being_unmade() {
    let r = well_nested();
    std::fs::remove_file(r.path().join("lib-v2/.pile/config")).unwrap();
    let (code, v) = report(&r);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        findings(&v["result"], ":/lib-v2"),
        pairs(&[("notes", "unmade")])
    );
    contains!(r.stdout(&["doctor"]), ":/lib-v2  (unmade)\n");
}

#[test]
fn a_pile_cannot_be_declared_inside_a_pile_directory_or_by_a_pile_file() {
    let r = well_nested();
    r.declare("lib/.pile/inner", "e", "inner");
    r.write("lib-v2/docs/.pile", "a file");
    let (code, v) = report(&r);
    assert_eq!(code, 1);
    let v = &v["result"];
    let lib = findings(v, ":/lib");
    assert!(
        lib.contains(&("problems".into(), "pile-in-pile-directory".into())),
        "{lib:?}"
    );
    assert!(
        findings(v, ":/lib-v2").contains(&("problems".into(), "pile-directory-is-a-file".into()))
    );
    // With no pile above it, the finding is reported on its own.
    let bare = Repo::new();
    bare.write("loose/.pile", "a file");
    let (code, b) = report(&bare);
    assert_eq!(code, 1);
    assert_eq!(
        b["result"]["in_no_pile"][0]["code"],
        "pile-directory-is-a-file"
    );
    assert_eq!(b["result"]["in_no_pile"][0]["file"], ":/loose/.pile");
    // And nothing reads the inner config as a pile.
    assert!(!r.answered(&[], &["list"]).stdout.contains("inner"));
}

#[test]
fn a_config_that_is_a_symbolic_link_is_a_problem() {
    let r = well_nested();
    std::fs::create_dir_all(r.path().join("linked/.pile")).unwrap();
    std::os::unix::fs::symlink(
        "../../lib/.pile/config",
        r.path().join("linked/.pile/config"),
    )
    .unwrap();
    let (code, v) = report(&r);
    assert_eq!(code, 1);
    assert_eq!(
        findings(&v["result"], ":/linked"),
        pairs(&[("problems", "config-not-a-file")])
    );
}

#[test]
fn another_repository_inside_a_pile_is_a_note() {
    let r = well_nested();
    let inner = r.path().join("lib/vendor/other");
    std::fs::create_dir_all(&inner).unwrap();
    command("git", &inner)
        .args(["init", "-q"])
        .output()
        .unwrap();
    std::fs::create_dir_all(inner.join(".pile")).unwrap();
    std::fs::write(inner.join(".pile/config"), "not read\n").unwrap();
    let (code, v) = report(&r);
    assert_eq!(code, 0, "{v}");
    let lib = findings(&v["result"], ":/lib");
    assert!(
        lib.contains(&("notes".into(), "holds-repository".into())),
        "{lib:?}"
    );
    assert!(
        v["result"]["piles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["path"] != ":/lib/vendor/other"),
        "a pile the other repository declares is not this one's"
    );
}

#[test]
fn doctor_reads_the_whole_repository_from_anywhere_and_takes_no_pile() {
    let r = nested();
    let a = r.answered_in("lib/sub", &[], &["doctor", "--format", "json"]);
    assert_eq!(a.json()["result"]["total"]["piles"], 4);
    assert_eq!(r.refused(&["doctor", "@lib"], 2)["error"]["code"], "usage");
    let full = r.answered(&[], &["doctor", "--full"]).stdout;
    contains!(full, &id_of("b"));
}

#[test]
fn a_repository_with_no_pile_says_so_and_exits_0() {
    let r = Repo::new();
    let out = r.stdout(&["doctor"]);
    assert!(out.starts_with("no pile in "), "{out}");
    assert_eq!(r.result(&["doctor"])["piles"], json!([]));
}
