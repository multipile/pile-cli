//! A pile's diagrams: what `pile doctor` finds wrong with them, and how
//! `pile diagram render` draws one.

use crate::harness::*;

const DEPS: &str = r#"diagram dependency version=1 {
  title "What the app depends on"
  desc "The app depends on the library."
  box app pile="@app"
  box lib pile="@lib" focal=#true
  line app lib
}
"#;

/// Two piles, app and lib, each explained, and app keeping one diagram.
fn with_diagram() -> Repo {
    let r = Repo::new();
    r.declare("app", "app", "app");
    r.intent("app", "The application.\n");
    r.declare("lib", "lib", "lib");
    r.intent("lib", "The library.\n");
    r.write("app/.pile/diagrams/deps.diagram", DEPS);
    r.commit("declare");
    r
}

/// The codes of the problems doctor finds, in the order it lists them.
fn problems(v: &serde_json::Value) -> Vec<String> {
    v["piles"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["problems"].as_array().unwrap().clone())
        .map(|f| f["code"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_diagram_that_reads_is_no_finding_and_no_unread_file() {
    let r = with_diagram();
    let v = r.result(&["doctor"]);
    assert_eq!(v["total"]["problems"], 0, "{v}");
    assert_eq!(
        v["total"]["notes"], 0,
        "a diagram is read, so it is no unread file: {v}"
    );
}

#[test]
fn a_file_in_diagrams_that_is_no_diagram_is_still_an_unread_file() {
    let r = with_diagram();
    r.write("app/.pile/diagrams/notes.md", "notes\n");
    r.commit("notes");
    let v = r.result(&["doctor"]);
    assert_eq!(v["total"]["notes"], 1, "{v}");
}

#[test]
fn each_rule_a_diagram_breaks_is_a_problem_at_its_line_and_column() {
    let r = with_diagram();
    r.write(
        "app/.pile/diagrams/deps.diagram",
        &DEPS.replace("  line app lib\n", "  line app lbi\n"),
    );
    let e = r.refused(&["doctor"], 1);
    assert_eq!(e["error"]["code"], "problems-found");
    let v = &e["result"];
    assert_eq!(problems(v), vec!["diagram-invalid"]);
    let f = &v["piles"][0]["problems"][0];
    assert_eq!(f["file"], ":/app/.pile/diagrams/deps.diagram");
    assert_eq!(
        (f["line"].as_u64(), f["column"].as_u64()),
        (Some(6), Some(3))
    );
    assert!(
        f["message"].as_str().unwrap().contains("key-unknown"),
        "{f}"
    );
    assert_eq!(f["fix"], "did you mean lib?");
}

#[test]
fn a_pile_no_pile_answers_to_is_a_problem() {
    let r = with_diagram();
    r.write(
        "app/.pile/diagrams/deps.diagram",
        &DEPS.replace("pile=\"@lib\"", "pile=\"@library\""),
    );
    let e = r.refused(&["doctor"], 1);
    assert_eq!(problems(&e["result"]), vec!["diagram-no-such-pile"]);
}

#[test]
fn a_renamed_pile_is_found_by_the_id_the_diagram_gives() {
    let r = with_diagram();
    let id = id_of("lib");
    r.write(
        "app/.pile/diagrams/deps.diagram",
        &DEPS.replace("pile=\"@lib\"", &format!("pile=\"@library\" id=\"{id}\"")),
    );
    let e = r.refused(&["doctor"], 1);
    let f = &e["result"]["piles"][0]["problems"][0];
    assert_eq!(f["code"], "diagram-no-such-pile");
    assert!(f["message"].as_str().unwrap().contains("now @lib"), "{f}");
    assert_eq!(f["fix"], "write pile=\"@lib\"");
}

#[test]
fn a_name_and_an_id_of_two_piles_is_a_problem() {
    let r = with_diagram();
    let id = id_of("app");
    r.write(
        "app/.pile/diagrams/deps.diagram",
        &DEPS.replace("pile=\"@lib\"", &format!("pile=\"@lib\" id=\"{id}\"")),
    );
    let e = r.refused(&["doctor"], 1);
    assert_eq!(problems(&e["result"]), vec!["diagram-pile-id-differs"]);
}

#[test]
fn doctor_diagrams_reports_only_the_diagrams() {
    let r = with_diagram();
    r.write("app/.pile/intent.md", "");
    r.write(
        "lib/.pile/diagrams/broken.diagram",
        "diagram tree version=1 {\n  title \"Broken\"\n  desc \"No root.\"\n}\n",
    );
    let e = r.refused(&["doctor", "--diagrams"], 1);
    assert_eq!(e["error"]["code"], "problems-found");
    let v = &e["result"];
    let names: Vec<&str> = v["diagrams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["deps", "broken"]);
    assert_eq!(v["diagrams"][0]["pile"]["name"], "app");
    assert_eq!(v["diagrams"][1]["problems"][0]["code"], "diagram-invalid");
    assert_eq!(
        v["total"]["problems"], 1,
        "the intent left empty is not a diagram's: {v}"
    );
    let out = r
        .answered(&[], &["doctor", "--diagrams", "--format", "text"])
        .stdout;
    contains!(out, "deps  :/app/.pile/diagrams/deps.diagram");
    contains!(out, "2 diagrams, 1 needing attention: 1 problem");
}

#[test]
fn doctor_diagrams_with_no_diagram_says_so_and_exits_0() {
    let r = Repo::new();
    r.declare(".", "top", "top");
    r.intent(".", "The repository.\n");
    r.commit("declare");
    let v = r.result(&["doctor", "--diagrams"]);
    assert_eq!(v["total"]["diagrams"], 0);
    contains!(
        r.stdout(&["doctor", "--diagrams", "--format", "text"]),
        "no diagram in any pile"
    );
}

#[test]
fn render_writes_a_page_by_default_named_after_the_diagram() {
    let r = with_diagram();
    let v = r.result(&["diagram", "render", "deps"]);
    assert_eq!(v["as"], "html");
    assert_eq!(v["out"], "deps.html");
    assert_eq!(v["pile"]["name"], "app");
    assert_eq!(v["path"], ":/app/.pile/diagrams/deps.diagram");
    assert!(r.read("deps.html").starts_with("<!DOCTYPE html>"));
    assert!(v["width"].as_i64().unwrap() > 0 && v["height"].as_i64().unwrap() > 0);
    assert!(v["duration_ms"].is_null());
}

#[test]
fn render_as_a_picture_and_as_an_animation() {
    let r = with_diagram();
    let v = r.result(&["diagram", "render", "--as", "png", "deps"]);
    assert_eq!(v["out"], "deps.png");
    let png = std::fs::read(r.path().join("deps.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(v["bytes"].as_u64().unwrap() as usize, png.len());
    let v = r.result(&["diagram", "render", "--as", "gif", "deps"]);
    let gif = std::fs::read(r.path().join("deps.gif")).unwrap();
    assert_eq!(&gif[..6], b"GIF89a");
    assert!(v["duration_ms"].as_u64().unwrap() > 0);
}

#[test]
fn without_as_the_ending_of_the_file_decides_and_as_wins_over_it() {
    let r = with_diagram();
    let v = r.result(&["diagram", "render", "-o", "out/deps.gif", "deps"]);
    assert_eq!(v["as"], "gif");
    assert_eq!(
        &std::fs::read(r.path().join("out/deps.gif")).unwrap()[..6],
        b"GIF89a"
    );
    let v = r.result(&[
        "diagram",
        "render",
        "--as",
        "png",
        "-o",
        "picture.gif",
        "deps",
    ]);
    assert_eq!(v["as"], "png");
    assert_eq!(
        &std::fs::read(r.path().join("picture.gif")).unwrap()[..4],
        b"\x89PNG"
    );
    let e = r.refused(&["diagram", "render", "-o", "deps.svg", "deps"], 2);
    assert_eq!(e["error"]["code"], "unknown-output");
    let e = r.refused(&["diagram", "render", "--as", "svg", "deps"], 2);
    assert_eq!(e["error"]["code"], "unknown-output");
}

#[test]
fn render_names_a_diagram_after_its_pile_or_by_its_file() {
    let r = with_diagram();
    r.write("lib/.pile/diagrams/deps.diagram", DEPS);
    r.commit("a second deps");
    let e = r.refused(&["diagram", "render", "deps"], 1);
    assert_eq!(e["error"]["code"], "diagram-name-ambiguous");
    let message = e["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("@app/deps") && message.contains("@lib/deps"),
        "{message}"
    );
    let v = r.result(&["diagram", "render", "@lib/deps"]);
    assert_eq!(v["path"], ":/lib/.pile/diagrams/deps.diagram");
    let v = r.result(&["diagram", "render", ":/app/.pile/diagrams/deps.diagram"]);
    assert_eq!(v["pile"]["name"], "app");
}

#[test]
fn render_refuses_a_name_no_pile_keeps_listing_those_there_are() {
    let r = with_diagram();
    let e = r.refused(&["diagram", "render", "flow"], 1);
    assert_eq!(e["error"]["code"], "no-such-diagram");
    assert!(
        e["error"]["message"]
            .as_str()
            .unwrap()
            .contains("the diagrams are deps"),
        "{e}"
    );
}

#[test]
fn render_refuses_a_diagram_that_breaks_a_rule_and_points_at_doctor() {
    let r = with_diagram();
    r.write(
        "app/.pile/diagrams/deps.diagram",
        &DEPS.replace("pile=\"@lib\"", "pile=\"@library\""),
    );
    let e = r.refused(&["diagram", "render", "deps"], 1);
    assert_eq!(e["error"]["code"], "diagram-invalid");
    assert_eq!(e["next"][0]["command"], "pile doctor --diagrams");
    assert!(!r.path().join("deps.html").exists(), "nothing is written");
}

#[test]
fn diagram_alone_says_what_it_takes() {
    let r = with_diagram();
    let e = r.refused(&["diagram"], 2);
    assert_eq!(e["error"]["code"], "usage");
}
