use storage::{Content, FileKind};

use super::*;

const STORAGE: &str = "3823eb7be13988395e3a856f282fc7dd";
const CLI: &str = "42498648a7753fb45d2ec5d33b97376b";

fn piles() -> Vec<Named> {
    vec![
        Named {
            path: "/crates/storage".into(),
            name: "storage".into(),
            id: STORAGE.into(),
        },
        Named {
            path: "/crates/pile".into(),
            name: "cli".into(),
            id: CLI.into(),
        },
        Named {
            path: "/docs".into(),
            name: String::new(),
            id: "0d8941827f34ea629c09a3b5303fe0a3".into(),
        },
    ]
}

fn diagram(source: &str) -> Diagram {
    Diagram {
        name: "crates".into(),
        dir: "/".into(),
        path: "/.pile/diagrams/crates.diagram".into(),
        source: Some(source.into()),
    }
}

fn dependency(boxes: &str) -> String {
    format!(
        "diagram dependency version=1 {{\n  title \"Crates\"\n  desc \"What depends on what.\"\n{boxes}\n}}\n"
    )
}

fn codes(findings: &[Finding]) -> Vec<&'static str> {
    findings.iter().map(|f| f.code).collect()
}

#[test]
fn a_diagram_is_a_file_directly_in_a_piles_diagrams_directory() {
    let file = |path: &str| Content {
        path: path.into(),
        kind: FileKind::File,
        bytes: b"x".to_vec(),
    };
    let all = found(&[
        file("/a/.pile/diagrams/flow.diagram"),
        file("/.pile/diagrams/crates.diagram"),
        file("/a/.pile/diagrams/notes.md"),
        file("/a/.pile/diagrams/old/flow.diagram"),
        file("/a/.pile/flow.diagram"),
    ]);
    let names: Vec<(&str, &str)> = all
        .iter()
        .map(|d| (d.dir.as_str(), d.name.as_str()))
        .collect();
    assert_eq!(names, vec![("/", "crates"), ("/a", "flow")]);
}

#[test]
fn a_diagram_whose_piles_are_the_repositorys_has_nothing_wrong() {
    let source = dependency(
        "  box cli pile=\"@cli\"\n  box storage pile=\":/crates/storage\"\n  box docs pile=\":/docs\"\n  line cli storage",
    );
    assert_eq!(findings(&diagram(&source), &piles()), vec![]);
}

#[test]
fn each_rule_a_diagram_breaks_is_a_problem_at_its_line() {
    let source = dependency("  bxo cli\n  box storage as=stroe");
    let found = findings(&diagram(&source), &piles());
    assert_eq!(codes(&found), vec!["diagram-invalid", "diagram-invalid"]);
    assert_eq!((found[0].line, found[0].column), (Some(4), Some(3)));
    assert!(
        found[0].message.contains("node-unknown"),
        "{}",
        found[0].message
    );
    assert_eq!(found[0].fix, "did you mean box?");
    assert!(found.iter().all(|f| f.severity == Severity::Problem));
}

#[test]
fn a_pile_no_pile_answers_to_is_a_problem() {
    let source = dependency("  box gone pile=\"@gone\"\n  box nowhere pile=\":/nowhere\"");
    let found = findings(&diagram(&source), &piles());
    assert_eq!(
        codes(&found),
        vec!["diagram-no-such-pile", "diagram-no-such-pile"]
    );
    assert_eq!(found[0].line, Some(4));
    assert_eq!(found[1].line, Some(5));
}

#[test]
fn a_pile_misspelt_is_answered_with_the_pile_it_is_near() {
    let first = &piles()[0].handle();
    let misspelt = format!("{}x", first);
    let source = dependency(&format!("  box near pile=\"{misspelt}\""));
    let found = findings(&diagram(&source), &piles());
    assert_eq!(codes(&found), vec!["diagram-no-such-pile"]);
    assert_eq!(
        found[0].fix,
        format!("did you mean {first}? Write pile=\"{first}\"")
    );
}

#[test]
fn a_renamed_pile_found_by_its_id_is_named_in_the_fix() {
    let source = dependency(&format!("  box cli pile=\"@command-line\" id=\"{CLI}\""));
    let found = findings(&diagram(&source), &piles());
    assert_eq!(codes(&found), vec!["diagram-no-such-pile"]);
    assert!(
        found[0].message.contains("now @cli"),
        "{}",
        found[0].message
    );
    assert_eq!(found[0].fix, "write pile=\"@cli\"");
}

#[test]
fn a_name_and_an_id_of_two_piles_is_a_problem() {
    let source = dependency(&format!("  box cli pile=\"@cli\" id=\"{STORAGE}\""));
    assert_eq!(
        codes(&findings(&diagram(&source), &piles())),
        vec!["diagram-pile-id-differs"]
    );
    let source = dependency("  box cli pile=\"@cli\" id=\"ffffffffffffffffffffffffffffffff\"");
    assert_eq!(
        codes(&findings(&diagram(&source), &piles())),
        vec!["diagram-pile-id-differs"]
    );
    let source = dependency(&format!("  box cli pile=\"@cli\" id=\"{CLI}\""));
    assert_eq!(findings(&diagram(&source), &piles()), vec![]);
}

#[test]
fn a_pile_is_checked_in_every_type_that_can_name_one() {
    let source = "diagram tree version=1 {\n  title \"Piles\"\n  desc \"A tree.\"\n  item \"root\" {\n    item pile=\"@gone\"\n  }\n}\n";
    assert_eq!(
        codes(&findings(&diagram(source), &piles())),
        vec!["diagram-no-such-pile"]
    );
    let source = "diagram nested version=1 {\n  title \"Scopes\"\n  desc \"Nested.\"\n  level \"a\" {\n    level \"b\" {\n      level pile=\"@gone\"\n    }\n  }\n}\n";
    assert_eq!(
        codes(&findings(&diagram(source), &piles())),
        vec!["diagram-no-such-pile"]
    );
}

#[test]
fn a_diagram_that_is_not_text_is_a_problem() {
    let mut d = diagram("");
    d.source = None;
    assert_eq!(codes(&findings(&d, &piles())), vec!["diagram-invalid"]);
}

#[test]
fn a_diagram_is_drawn_as_a_page_a_picture_and_an_animation() {
    let read = diagram_format::read(&dependency("  box a\n  box b\n  line a b")).unwrap();
    let html = draw(&read.diagram, As::Html).unwrap();
    assert!(
        String::from_utf8(html.bytes)
            .unwrap()
            .starts_with("<!DOCTYPE html>")
    );
    let png = draw(&read.diagram, As::Png).unwrap();
    assert_eq!(&png.bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(png.duration_ms, None);
    let gif = draw(&read.diagram, As::Gif).unwrap();
    assert_eq!(&gif.bytes[..6], b"GIF89a");
    assert!(gif.duration_ms.is_some_and(|ms| ms > 0));
}

fn at(path: &str, source: &str) -> Diagram {
    let name = path
        .rsplit('/')
        .next()
        .unwrap()
        .trim_end_matches(".diagram");
    Diagram {
        name: name.into(),
        dir: "/".into(),
        path: path.into(),
        source: Some(source.into()),
    }
}

#[test]
fn each_diagram_says_how_it_changed_since_a_revision() {
    let kept = dependency("  box a pile=\"@storage\"\n  box b pile=\"@cli\"\n  line a b");
    let then = vec![
        at(
            "/.pile/diagrams/edited.diagram",
            &dependency("  box a\n  box b\n  line a b"),
        ),
        at("/.pile/diagrams/gone.diagram", &kept),
        at("/.pile/diagrams/piles.diagram", &kept),
        at(
            "/.pile/diagrams/quiet.diagram",
            &dependency("  box a pile=\"@cli\"\n  box b\n  line a b"),
        ),
    ];
    let now = vec![
        at("/.pile/diagrams/added.diagram", &kept),
        at(
            "/.pile/diagrams/edited.diagram",
            &dependency("  box a\n  box c\n  line a c"),
        ),
        at("/.pile/diagrams/piles.diagram", &kept),
        at(
            "/.pile/diagrams/quiet.diagram",
            &dependency("  box a pile=\"@cli\"\n  box b\n  line a b"),
        ),
    ];
    let found = changes(&then, &now, |pile| pile == "@storage");
    let words: Vec<(&str, Option<&str>)> = found
        .iter()
        .map(|(d, c)| (d.name.as_str(), c.as_ref().map(Change::word)))
        .collect();
    assert_eq!(
        words,
        vec![
            ("added", Some("added")),
            ("edited", Some("edited")),
            ("gone", Some("removed")),
            ("piles", Some("piles")),
            ("quiet", None),
        ]
    );
    let piles = found.iter().find(|(d, _)| d.name == "piles").unwrap();
    assert_eq!(piles.1, Some(Change::Piles(vec!["@storage".to_string()])));
}

#[test]
fn a_diagram_names_each_pile_once_in_the_order_it_names_them() {
    let d = diagram(&dependency(
        "  box a pile=\"@cli\"\n  box b pile=\"@storage\"\n  box c pile=\"@cli\"\n  line a b",
    ));
    assert_eq!(shown_piles(&d), vec!["@cli", "@storage"]);
    assert!(shown_piles(&diagram("not kdl {")).is_empty());
}

#[test]
fn a_change_is_drawn_as_a_diff_and_touched_piles_as_marks() {
    let before = diagram_format::read(&dependency("  box a\n  box b\n  line a b")).unwrap();
    let after = diagram_format::read(&dependency("  box a\n  box c\n  line a c")).unwrap();
    let html = draw_diff(&before.diagram, &after.diagram, As::Html)
        .unwrap()
        .unwrap();
    let page = String::from_utf8(html.bytes).unwrap();
    assert!(
        page.contains("data-added=\"2\"") && page.contains("data-removed=\"2\""),
        "{page}"
    );
    let gif = draw_diff(&before.diagram, &after.diagram, As::Gif)
        .unwrap()
        .unwrap();
    assert!(gif.duration_ms.is_some_and(|ms| ms > 0));
    let tree = diagram_format::read(
        "diagram tree version=1 {\n  title \"T\"\n  desc \"A tree.\"\n  item \"root\"\n}\n",
    )
    .unwrap();
    assert!(
        draw_diff(&before.diagram, &tree.diagram, As::Png).is_none(),
        "two types"
    );

    let shown =
        diagram_format::read(&dependency("  box a pile=\"@cli\"\n  box b\n  line a b")).unwrap();
    let marked = draw_marked(&shown.diagram, |pile| pile == "@cli", As::Html).unwrap();
    let page = String::from_utf8(marked.bytes).unwrap();
    assert!(page.contains("data-changed=\"1\""), "{page}");
    assert!(
        draw_marked(&shown.diagram, |_| true, As::Gif).is_err(),
        "a marked diagram is still"
    );
}
