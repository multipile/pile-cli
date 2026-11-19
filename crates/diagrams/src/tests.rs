use storage::{Content, FileKind};

use super::*;

const CLI: &str = "42498648a7753fb45d2ec5d33b97376b";

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
fn each_rule_a_diagram_breaks_is_a_problem_at_its_line() {
    let source = dependency("  bxo cli\n  box storage as=stroe");
    let found = findings(&diagram(&source), None);
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
fn a_box_stands_for_a_node_by_pile_id_and_by_no_other_word() {
    // A word the format does not take is a rule of the format broken.
    let source = dependency("  box cli pile=\"@cli\"");
    let found = findings(&diagram(&source), None);
    assert_eq!(codes(&found), vec!["diagram-invalid"]);
    assert!(
        found[0].message.contains("property-unknown"),
        "{}",
        found[0].message
    );
    // And a word under pile's name that pile does not read is unknown.
    let source = dependency("  box cli pile:ref=\"@cli\"");
    let found = findings(&diagram(&source), None);
    assert_eq!(codes(&found), vec!["diagram-meta-unknown"]);
    assert_eq!(found[0].line, Some(4));
    assert_eq!(
        found[0].fix,
        "a box takes pile:id, pile:node-type and pile:name"
    );
    let source = dependency(&format!("  box cli pile:id=\"{CLI}\""));
    assert_eq!(findings(&diagram(&source), None), vec![]);
}

#[test]
fn a_diagram_that_is_not_text_is_a_problem() {
    let mut d = diagram("");
    d.source = None;
    assert_eq!(codes(&findings(&d, None)), vec!["diagram-invalid"]);
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

#[test]
fn a_change_is_drawn_as_a_diff_and_touched_piles_as_marks() {
    let before = diagram_format::read(&dependency("  box a\n  box b\n  line a b")).unwrap();
    let after = diagram_format::read(&dependency("  box a\n  box c\n  line a c")).unwrap();
    let html = draw_diff(
        &before.diagram,
        &after.diagram,
        &Swapped::default(),
        As::Html,
    )
    .unwrap()
    .unwrap();
    let page = String::from_utf8(html.bytes).unwrap();
    assert!(
        page.contains("data-added=\"2\"") && page.contains("data-removed=\"2\""),
        "{page}"
    );
    let gif = draw_diff(
        &before.diagram,
        &after.diagram,
        &Swapped::default(),
        As::Gif,
    )
    .unwrap()
    .unwrap();
    assert!(gif.duration_ms.is_some_and(|ms| ms > 0));
    let tree = diagram_format::read(
        "diagram tree version=1 {\n  title \"T\"\n  desc \"A tree.\"\n  item \"root\"\n}\n",
    )
    .unwrap();
    assert!(
        draw_diff(&before.diagram, &tree.diagram, &Swapped::default(), As::Png).is_none(),
        "two types"
    );

    let shown = diagram_format::read(&dependency(&format!(
        "  box a pile:id=\"{CLI}\"\n  box b\n  line a b"
    )))
    .unwrap();
    let mut touched = Touched {
        elements: vec!["a".to_string()],
        ..Touched::default()
    };
    let marked = draw_marked(&shown.diagram, &touched, As::Html).unwrap();
    let page = String::from_utf8(marked.bytes).unwrap();
    assert!(page.contains("data-changed=\"1\""), "{page}");
    assert!(
        page.contains(&format!("data-pile-id=\"{CLI}\"")),
        "the page says what the file said of the box: {page}"
    );
    touched.lines.push(("a".to_string(), "b".to_string()));
    let marked = draw_marked(&shown.diagram, &touched, As::Html).unwrap();
    let page = String::from_utf8(marked.bytes).unwrap();
    assert!(
        page.contains("data-changed=\"2\""),
        "a line is marked by its two keys: {page}"
    );
    assert!(
        draw_marked(&shown.diagram, &touched, As::Gif).is_err(),
        "a marked diagram is still"
    );
}

mod cases;
mod held;
