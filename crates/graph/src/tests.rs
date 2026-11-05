//! The rows read from repositories held in memory.

use storage::memory::Memory;

use super::*;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "cccccccccccccccccccccccccccccccc";

fn at(line: usize, column: usize) -> Position {
    Position { line, column }
}

fn pile(id: &str) -> Node {
    Node::Pile(id.to_string())
}

fn text(s: &str) -> Option<Value> {
    Some(Value::Text(s.to_string()))
}

/// `/`, named `top`, with a headline after a heading; `/lib`, named `lib`,
/// with a headline under a comment; and `/lib/sub`, with no name and no
/// intent.
fn repository() -> Memory {
    let mut m = Memory::new();
    m.write("/.pile/config", piles::config::render(A, "top"));
    m.write("/.pile/intent.md", "# /\n\nThe whole of it.\n");
    m.write(
        "/lib/.pile/config",
        format!("# mine\nkind = gitpile\nversion = 1\nname = lib\nid =  {B}\n"),
    );
    m.write("/lib/.pile/intent.md", "<!-- a note -->\n  The parser.\n");
    m.write("/lib/.pile/links.toml", "");
    m.write("/lib/sub/.pile/config", piles::config::render(C, ""));
    m
}

#[test]
fn every_pile_is_a_node_of_type_pile_at_home_in_itself() {
    let r = read(&repository(), &State::Working).unwrap();
    assert_eq!(
        r.node,
        [A, B, C].map(|id| NodeRow {
            node: pile(id),
            of_type: PILE.to_string(),
            home: pile(id),
        })
    );
}

#[test]
fn a_pile_is_at_its_config_line_1_column_1() {
    let r = read(&repository(), &State::Working).unwrap();
    let files: Vec<(&str, Position)> = r.at.iter().map(|a| (a.file.as_str(), a.at)).collect();
    assert_eq!(
        files,
        [
            (":/.pile/config", at(1, 1)),
            (":/lib/.pile/config", at(1, 1)),
            (":/lib/sub/.pile/config", at(1, 1)),
        ]
    );
    assert_eq!(r.at(&pile(B)).unwrap().item, Item::Node(pile(B)));
}

#[test]
fn a_pile_holds_its_id_name_path_and_headline() {
    let r = read(&repository(), &State::Working).unwrap();
    let lib = pile(B);
    assert_eq!(r.value(&lib, ID).cloned(), text(B));
    assert_eq!(
        r.value(&lib, NAME).cloned(),
        Some(Value::Text("lib".into()))
    );
    assert_eq!(r.value(&lib, PATH).cloned(), text(":/lib"));
    assert_eq!(r.value(&lib, HEADLINE).cloned(), text("The parser."));
    assert_eq!(
        r.value(&pile(A), HEADLINE).cloned(),
        text("The whole of it.")
    );
    // Nothing else is a property: kind and version say how the config is
    // read, and labels are not built in.
    let mut properties: Vec<&str> = r.prop.iter().map(|p| p.property.as_str()).collect();
    properties.sort();
    properties.dedup();
    assert_eq!(properties, [HEADLINE, ID, NAME, PATH]);
}

#[test]
fn a_property_that_is_not_given_has_no_row() {
    let r = read(&repository(), &State::Working).unwrap();
    let sub = pile(C);
    assert_eq!(r.value(&sub, NAME), None);
    assert_eq!(r.value(&sub, HEADLINE), None);
    assert_eq!(r.prop_at(&sub, NAME), None);
    let props: Vec<&str> = r
        .prop
        .iter()
        .filter(|p| p.node == sub)
        .map(|p| p.property.as_str())
        .collect();
    assert_eq!(props, [ID, PATH]);
}

#[test]
fn each_value_is_where_the_file_writes_it() {
    let r = read(&repository(), &State::Working).unwrap();
    let lib = pile(B);
    let written =
        |node: &Node, property| r.prop_at(node, property).map(|p| (p.file.as_str(), p.at));
    assert_eq!(written(&lib, NAME), Some((":/lib/.pile/config", at(4, 8))));
    assert_eq!(written(&lib, ID), Some((":/lib/.pile/config", at(5, 7))));
    assert_eq!(
        written(&lib, HEADLINE),
        Some((":/lib/.pile/intent.md", at(2, 3)))
    );
    assert_eq!(
        written(&pile(A), HEADLINE),
        Some((":/.pile/intent.md", at(3, 1)))
    );
    // Where a pile sits is no line of any file.
    assert_eq!(written(&lib, PATH), None);
}

#[test]
fn a_moved_pile_is_the_same_node_at_its_new_path() {
    let mut m = repository();
    let before = m.commit("lay out");
    for file in ["config", "intent.md", "links.toml"] {
        m.rename(
            &format!("/lib/.pile/{file}"),
            &format!("/parser/.pile/{file}"),
        );
    }
    m.rename("/lib/sub/.pile/config", "/parser/sub/.pile/config");
    let then = read(&m, &State::At(before)).unwrap();
    let now = read(&m, &State::Working).unwrap();
    assert_eq!(now.node, then.node);
    assert_eq!(then.value(&pile(B), PATH).cloned(), text(":/lib"));
    assert_eq!(now.value(&pile(B), PATH).cloned(), text(":/parser"));
    assert_eq!(now.at(&pile(B)).unwrap().file, ":/parser/.pile/config");
    assert_eq!(now.value(&pile(B), NAME), then.value(&pile(B), NAME));
}

#[test]
fn a_config_that_cannot_be_read_reads_no_rows() {
    let mut m = repository();
    m.write(
        "/lib/.pile/config",
        format!("kind = gitpile\nversion = 1\nid = {B}\nlabels = core\n"),
    );
    let err = read(&m, &State::Working).unwrap_err();
    assert!(err.to_string().contains("line 4 names \"labels\""), "{err}");
}

#[test]
fn a_repository_with_no_pile_has_no_rows() {
    let r = read(&Memory::new(), &State::Working).unwrap();
    assert_eq!(r, Relations::default());
}

#[test]
fn a_node_is_referred_to_by_its_type_and_name_and_a_pile_by_its_name() {
    let external = Node::Named {
        of_type: "external".to_string(),
        name: "git".to_string(),
    };
    assert_eq!(external.reference("top"), Some("external:git".into()));
    assert_eq!(pile(A).reference("top"), Some("@top".into()));
    assert_eq!(pile(A).reference(""), None);
}
