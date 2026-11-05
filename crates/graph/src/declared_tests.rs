//! What the `.toml` files declare: the rows each one becomes, and the
//! finding each mistake gives.

use storage::memory::Memory;

use super::*;

const TOP: &str = "11111111111111111111111111111111";
const WORDS: &str = "22222222222222222222222222222222";
const API: &str = "33333333333333333333333333333333";
const APP: &str = "44444444444444444444444444444444";

const V: &str = "version = \"gitpile/1\"\n";

const TYPES: &str = r#"version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository"

[[edge-types]]
name = "uses"
source = ["pile"]
target = ["pile", "external"]

[[edge-types]]
name = "replicates-to"
source = ["external"]
target = ["external"]
"#;

fn at(line: usize, column: usize) -> Position {
    Position { line, column }
}

fn pile(id: &str) -> Node {
    Node::Pile(id.to_string())
}

fn named(of_type: &str, name: &str) -> Node {
    Node::Named {
        of_type: of_type.into(),
        name: name.into(),
    }
}

fn edge(source: Node, of_type: &str, target: Node) -> Edge {
    Edge {
        source,
        of_type: of_type.into(),
        target,
    }
}

/// An unnamed top, the aux pile `@words` with [`TYPES`], `@api` and `@app`,
/// and then each file given.
fn repository(files: &[(&str, &str)]) -> Memory {
    let mut m = Memory::new();
    let mut declare = |dir: &str, config: String| {
        m.write(&format!("{dir}.pile/config"), config);
        m.write(&format!("{dir}.pile/intent.md"), "What it is.\n");
    };
    declare("/", piles::config::render(TOP, ""));
    declare(
        "/words/",
        piles::config::render_kind(piles::config::AUX, WORDS, "words"),
    );
    declare("/api/", piles::config::render(API, "api"));
    declare("/app/", piles::config::render(APP, "app"));
    m.write("/words/.pile/types.toml", TYPES);
    for (path, text) in files {
        m.write(path, *text);
    }
    m
}

fn declared(m: &Memory) -> Declared {
    let found = m.declarations(&State::Working).unwrap();
    let s = piles::structure(&found).unwrap();
    declarations(&s.ordered(), &found)
}

fn found(d: &Declared) -> Vec<(&str, &str, Option<usize>, Option<usize>)> {
    d.findings
        .iter()
        .map(|f| (f.code, f.file.as_str(), f.line, f.column))
        .collect()
}

#[test]
fn the_types_an_aux_pile_creates_are_rows_of_type_type_prop_and_allows_and_not_nodes() {
    let d = declared(&repository(&[]));
    assert!(d.findings.is_empty(), "{:?}", d.findings);
    let r = &d.relations;
    assert!(r.node.is_empty());
    let seen: Vec<(&str, Sort, Vec<&str>, Vec<&str>)> = r
        .types
        .iter()
        .map(|t| {
            (
                t.name.as_str(),
                t.sort,
                r.allowed(&t.name, End::Source),
                r.allowed(&t.name, End::Target),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("external", Sort::Node, vec![], vec![]),
            (
                "replicates-to",
                Sort::Edge,
                vec!["external"],
                vec!["external"]
            ),
            ("uses", Sort::Edge, vec!["pile"], vec!["external", "pile"]),
        ]
    );
    assert_eq!(r.type_of("external").unwrap().home, pile(WORDS));
    let at = r.type_at("external").unwrap();
    assert_eq!(
        (at.file.as_str(), at.at),
        (":/words/.pile/types.toml", self::at(3, 1))
    );
    assert_eq!(
        r.type_value("external", "title"),
        Some(&Value::Text("something outside the repository".into()))
    );
    let title_at = r
        .prop_at
        .iter()
        .find(|p| p.item == Item::Type("external".into()) && p.property == "title")
        .unwrap();
    assert_eq!(title_at.at, self::at(5, 9));
}

#[test]
fn a_node_is_its_type_and_name_at_home_where_it_is_written_with_its_own_keys() {
    let d = declared(&repository(&[(
        "/.pile/nodes.toml",
        &format!(
            "{V}\n[[nodes]]\ntype = \"external\"\nname = \"git\"\ntitle = \"the tool\"\n\
             port = 443\nlabels = [\"a\", \"b\"]\nweight = 1.5\nup = true\nmore = {{ x = 1 }}\n"
        ),
    )]));
    assert!(d.findings.is_empty(), "{:?}", d.findings);
    let git = named("external", "git");
    let r = &d.relations;
    assert_eq!(
        r.node,
        [NodeRow {
            node: git.clone(),
            of_type: "external".into(),
            home: pile(TOP),
        }]
    );
    assert_eq!(r.at(&git).unwrap().at, at(3, 1));
    assert_eq!(
        r.value(&git, "title"),
        Some(&Value::Text("the tool".into()))
    );
    assert_eq!(r.value(&git, "port"), Some(&Value::Integer(443)));
    assert_eq!(r.value(&git, "weight"), Some(&Value::Float("1.5".into())));
    assert_eq!(r.value(&git, "up"), Some(&Value::Boolean(true)));
    assert_eq!(r.values(&git, "labels").len(), 2);
    assert_eq!(r.prop_at(&git, "labels").unwrap().at, at(8, 10));
    // A table holds no single value, so it gives no row, and no finding.
    assert_eq!(r.value(&git, "more"), None);
    // The name is the node's identity, not a value of it.
    assert_eq!(r.value(&git, "name"), None);
}

#[test]
fn a_pile_gives_its_own_values_in_pile_across_its_files() {
    let d = declared(&repository(&[
        (
            "/api/.pile/pile.toml",
            &format!("{V}[pile]\ntitle = \"the interface\"\n"),
        ),
        (
            "/api/.pile/more.toml",
            &format!("{V}[pile]\nlabels = [\"core\"]\n"),
        ),
    ]));
    assert!(d.findings.is_empty(), "{:?}", d.findings);
    let r = &d.relations;
    assert_eq!(
        r.value(&pile(API), "title"),
        Some(&Value::Text("the interface".into()))
    );
    assert_eq!(
        r.value(&pile(API), "labels"),
        Some(&Value::Text("core".into()))
    );
}

#[test]
fn an_end_left_out_is_the_pile_that_writes_the_link() {
    let d = declared(&repository(&[
        (
            "/app/.pile/links.toml",
            &format!(
                "{V}\n[[edges]]\ntype = \"uses\"\ntarget = \"@api\"\ntitle = \"calls\"\n\
                 through = \"http\"\n"
            ),
        ),
        (
            "/api/.pile/links.toml",
            &format!(
                "{V}\n[[edges]]\ntype = \"uses\"\nsource = \"@app\"\ntarget = \"external:git\"\n"
            ),
        ),
        (
            "/.pile/nodes.toml",
            &format!("{V}[[nodes]]\ntype = \"external\"\nname = \"git\"\n"),
        ),
    ]));
    assert!(d.findings.is_empty(), "{:?}", d.findings);
    let r = &d.relations;
    let calls = edge(pile(APP), "uses", pile(API));
    let to_git = edge(pile(APP), "uses", named("external", "git"));
    assert_eq!(r.edge, [calls.clone(), to_git.clone()]);
    assert_eq!(
        r.edge_values(&calls, "through"),
        [&Value::Text("http".into())]
    );
    assert_eq!(r.edge_at(&calls).unwrap().file, ":/app/.pile/links.toml");
    // Written in @api, with neither end @api.
    assert_eq!(r.edge_at(&to_git).unwrap().file, ":/api/.pile/links.toml");
}

#[test]
fn a_link_may_join_two_nodes_neither_of_which_is_a_pile() {
    let d = declared(&repository(&[(
        "/.pile/nodes.toml",
        &format!(
            "{V}[[nodes]]\ntype = \"external\"\nname = \"a\"\n\n[[nodes]]\ntype = \"external\"\n\
             name = \"b\"\n\n[[edges]]\ntype = \"replicates-to\"\nsource = \"external:a\"\n\
             target = \"external:b\"\n"
        ),
    )]));
    assert!(d.findings.is_empty(), "{:?}", d.findings);
    assert_eq!(
        d.relations.edge,
        [edge(
            named("external", "a"),
            "replicates-to",
            named("external", "b")
        )]
    );
}

#[test]
fn each_mistake_in_a_file_is_a_finding_where_it_is_written() {
    let f = "/app/.pile/links.toml";
    let cases: &[(&str, &str, (usize, usize))] = &[
        (
            "[[edges]]\ntype = \"calls\"\ntarget = \"@api\"\n",
            "unknown-type",
            (3, 8),
        ),
        (
            "[[edges]]\ntype = \"external\"\ntarget = \"@api\"\n",
            "unknown-type",
            (3, 8),
        ),
        (
            "[[edges]]\ntype = \"uses\"\ntarget = \"@nobody\"\n",
            "no-such-name",
            (4, 10),
        ),
        (
            "[[edges]]\ntype = \"uses\"\ntarget = \"pile:api\"\n",
            "no-such-name",
            (4, 10),
        ),
        (
            "[[edges]]\ntype = \"uses\"\ntarget = \"external:x\"\n",
            "no-such-name",
            (4, 10),
        ),
        (
            "[[edges]]\ntype = \"uses\"\ntarget = \":/api\"\n",
            "wrong-value",
            (4, 10),
        ),
        (
            "[[edges]]\ntype = \"uses\"\ntarget = \"@app\"\n",
            "self-link",
            (2, 1),
        ),
        ("[[edges]]\ntype = \"uses\"\n", "missing-key", (2, 1)),
        ("[[edges]]\ntarget = \"@api\"\n", "missing-key", (2, 1)),
        (
            "[[edges]]\ntype = \"uses\"\ntarget = \"@api\"\nTitle = 1\n",
            "unknown-key",
            (5, 1),
        ),
        (
            "[[edges]]\ntype = \"uses\"\ntarget = \"@api\"\ntitle = 1\n",
            "wrong-value",
            (5, 9),
        ),
        (
            "[[nodes]]\ntype = \"pile\"\nname = \"x\"\n",
            "reserved-name",
            (3, 8),
        ),
        (
            "[[nodes]]\ntype = \"uses\"\nname = \"x\"\n",
            "unknown-type",
            (3, 8),
        ),
        (
            "[[nodes]]\ntype = \"external\"\nname = \"X\"\n",
            "wrong-value",
            (4, 8),
        ),
        ("[[nodes]]\ntype = \"external\"\n", "missing-key", (2, 1)),
        ("[pile]\nname = \"x\"\n", "reserved-name", (3, 1)),
        (
            "[[node-types]]\nname = \"x\"\n",
            "types-outside-aux",
            (2, 1),
        ),
    ];
    for (body, code, (line, column)) in cases {
        let text = format!("{V}{body}");
        let d = declared(&repository(&[(f, &text)]));
        assert_eq!(
            found(&d),
            [(*code, f, Some(*line), Some(*column))],
            "{body}"
        );
        // A key of its own, or a title, that is wrong is not read, and the
        // link still is.
        let still = body.contains("itle =");
        assert_eq!(d.relations.edge.len(), usize::from(still), "{body}");
        assert!(d.relations.node.is_empty(), "{body}");
    }
}

#[test]
fn a_link_that_goes_the_wrong_way_is_one_finding_saying_so() {
    let d = declared(&repository(&[
        (
            "/.pile/nodes.toml",
            &format!("{V}[[nodes]]\ntype = \"external\"\nname = \"git\"\n"),
        ),
        (
            "/app/.pile/links.toml",
            &format!("{V}[[edges]]\ntype = \"uses\"\nsource = \"external:git\"\n"),
        ),
    ]));
    assert_eq!(
        found(&d),
        [("wrong-end", "/app/.pile/links.toml", Some(4), Some(10))]
    );
    assert_eq!(d.findings[0].fix, "exchange its source and its target");

    // Both ends wrong, and no way round right: a finding at each, the end
    // left out at the link's type.
    let d = declared(&repository(&[(
        "/app/.pile/links.toml",
        &format!("{V}[[edges]]\ntype = \"replicates-to\"\ntarget = \"@api\"\n"),
    )]));
    assert_eq!(
        found(&d),
        [
            ("wrong-end", "/app/.pile/links.toml", Some(3), Some(8)),
            ("wrong-end", "/app/.pile/links.toml", Some(4), Some(10)),
        ]
    );
}

#[test]
fn each_mistake_in_a_type_is_a_finding_and_the_type_does_not_exist() {
    let f = "/words/.pile/types.toml";
    let cases: &[(&str, &str, (usize, usize))] = &[
        (
            "[[node-types]]\nname = \"pile\"\n",
            "reserved-name",
            (18, 8),
        ),
        ("[[node-types]]\nname = \"Thing\"\n", "wrong-value", (18, 8)),
        (
            "[[node-types]]\ntitle = \"a thing\"\n",
            "missing-key",
            (17, 1),
        ),
        (
            "[[node-types]]\nname = \"x\"\nsource = [\"pile\"]\n",
            "unknown-key",
            (19, 1),
        ),
        (
            "[[edge-types]]\nname = \"x\"\nsource = [\"pile\"]\n",
            "missing-key",
            (17, 1),
        ),
        (
            "[[edge-types]]\nname = \"x\"\nsource = \"pile\"\ntarget = [\"pile\"]\n",
            "wrong-value",
            (19, 10),
        ),
        (
            "[[edge-types]]\nname = \"x\"\nsource = [\"pile\"]\ntarget = [\"thing\"]\n",
            "unknown-type",
            (20, 11),
        ),
    ];
    for (body, code, (line, column)) in cases {
        let text = format!("{TYPES}\n{body}");
        let d = declared(&repository(&[(f, &text)]));
        assert_eq!(
            found(&d),
            [(*code, f, Some(*line), Some(*column))],
            "{body}"
        );
        assert!(!d.relations.types.iter().any(|t| t.name == "x"), "{body}");
    }
}

#[test]
fn a_thing_written_twice_is_read_in_neither_place_and_nothing_follows_from_it() {
    let d = declared(&repository(&[
        (
            "/api/.pile/nodes.toml",
            &format!("{V}[[nodes]]\ntype = \"external\"\nname = \"git\"\n"),
        ),
        (
            "/app/.pile/nodes.toml",
            &format!(
                "{V}[[nodes]]\ntype = \"external\"\nname = \"git\"\n\n[[edges]]\ntype = \"uses\"\n\
                 target = \"external:git\"\n"
            ),
        ),
    ]));
    assert_eq!(
        found(&d),
        [
            ("declared-twice", "/api/.pile/nodes.toml", Some(2), Some(1)),
            ("declared-twice", "/app/.pile/nodes.toml", Some(2), Some(1)),
        ]
    );
    assert!(d.relations.node.is_empty() && d.relations.edge.is_empty());

    let link = format!("{V}[[edges]]\ntype = \"uses\"\nsource = \"@app\"\ntarget = \"@api\"\n");
    let d = declared(&repository(&[
        ("/api/.pile/links.toml", &link),
        ("/app/.pile/links.toml", &link),
    ]));
    assert_eq!(
        found(&d).iter().map(|f| f.0).collect::<Vec<_>>(),
        ["declared-twice", "declared-twice"]
    );
    assert!(d.relations.edge.is_empty());

    let d = declared(&repository(&[
        ("/api/.pile/a.toml", &format!("{V}[pile]\ntitle = \"a\"\n")),
        ("/api/.pile/b.toml", &format!("{V}[pile]\ntitle = \"b\"\n")),
    ]));
    assert_eq!(
        found(&d),
        [
            ("declared-twice", "/api/.pile/a.toml", Some(3), Some(1)),
            ("declared-twice", "/api/.pile/b.toml", Some(3), Some(1)),
        ]
    );
    assert_eq!(d.relations.value(&pile(API), "title"), None);

    // A type created twice does not exist, and a link of it is no further
    // finding.
    let d = declared(&repository(&[
        (
            "/words/.pile/more.toml",
            &format!("{V}[[node-types]]\nname = \"external\"\n"),
        ),
        (
            "/.pile/nodes.toml",
            &format!("{V}[[nodes]]\ntype = \"external\"\nname = \"git\"\n"),
        ),
    ]));
    assert_eq!(
        found(&d).iter().map(|f| f.0).collect::<Vec<_>>(),
        ["declared-twice", "declared-twice"]
    );
}

#[test]
fn a_file_that_does_not_open_with_its_version_declares_nothing() {
    let d = declared(&repository(&[(
        "/app/.pile/links.toml",
        "[[edges]]\ntype = \"uses\"\ntarget = \"@api\"\n",
    )]));
    assert_eq!(
        found(&d),
        [("no-version", "/app/.pile/links.toml", None, None)]
    );
    assert!(d.relations.edge.is_empty());
}

#[test]
fn a_key_one_letter_from_a_key_the_table_takes_is_a_mistake_and_not_a_key_of_its_own() {
    let d = declared(&repository(&[(
        "/app/.pile/links.toml",
        &format!("{V}[[edges]]\ntype = \"uses\"\ntarget = \"@api\"\ntitel = \"calls\"\n"),
    )]));
    assert_eq!(
        found(&d),
        [("unknown-key", "/app/.pile/links.toml", Some(5), Some(1))]
    );
    contains(&d.findings[0].message, "titel is one letter from title");
    // The link still reads, without the mistaken key.
    let calls = edge(pile(APP), "uses", pile(API));
    assert_eq!(d.relations.edge, std::slice::from_ref(&calls));
    assert!(d.relations.edge_values(&calls, "titel").is_empty());

    // A misspelled end is refused too, so the link is not quietly read as
    // ending at the pile that writes it.
    let d = declared(&repository(&[(
        "/app/.pile/links.toml",
        &format!("{V}[[edges]]\ntype = \"uses\"\nsource = \"@api\"\ntargte = \"@app\"\n"),
    )]));
    assert_eq!(
        found(&d),
        [("unknown-key", "/app/.pile/links.toml", Some(5), Some(1))]
    );
    assert!(d.relations.edge.is_empty());
}

fn contains(text: &str, part: &str) {
    assert!(text.contains(part), "{text:?} does not contain {part:?}");
}

#[test]
fn nothing_that_names_a_type_with_a_mistake_is_a_further_finding() {
    let d = declared(&repository(&[
        (
            "/words/.pile/more.toml",
            &format!(
                "{V}[[node-types]]\nname = \"Service\"\n\n[[node-types]]\nname = \"db\"\n\
                 colour = 1\n"
            ),
        ),
        (
            "/app/.pile/nodes.toml",
            &format!(
                "{V}[[nodes]]\ntype = \"Service\"\nname = \"web\"\n\n[[nodes]]\ntype = \"db\"\n\
                 name = \"main\"\n\n[[edges]]\ntype = \"uses\"\ntarget = \"db:main\"\n"
            ),
        ),
    ]));
    assert_eq!(
        found(&d),
        [
            ("wrong-value", "/words/.pile/more.toml", Some(3), Some(8)),
            ("unknown-key", "/words/.pile/more.toml", Some(7), Some(1)),
        ]
    );
    assert!(d.relations.node.is_empty() && d.relations.edge.is_empty());
}

#[test]
fn each_mistake_in_one_table_is_a_finding_and_none_hides_another() {
    // A node with no name and a type that is not text.
    let d = declared(&repository(&[(
        "/app/.pile/nodes.toml",
        &format!("{V}[[nodes]]\ntype = 5\n"),
    )]));
    let mut codes: Vec<&str> = found(&d).iter().map(|f| f.0).collect();
    codes.sort();
    assert_eq!(codes, ["missing-key", "wrong-value"]);
    // A link with no type, and a reference that is no reference.
    let d = declared(&repository(&[(
        "/app/.pile/links.toml",
        &format!("{V}[[edges]]\ntarget = \":/api\"\n"),
    )]));
    let mut codes: Vec<&str> = found(&d).iter().map(|f| f.0).collect();
    codes.sort();
    assert_eq!(codes, ["missing-key", "wrong-value"]);
    // A node written twice, one copy with a title that is not text.
    let d = declared(&repository(&[
        (
            "/api/.pile/nodes.toml",
            &format!("{V}[[nodes]]\ntype = \"external\"\nname = \"git\"\ntitle = 5\n"),
        ),
        (
            "/app/.pile/nodes.toml",
            &format!("{V}[[nodes]]\ntype = \"external\"\nname = \"git\"\n"),
        ),
    ]));
    assert_eq!(
        found(&d).iter().map(|f| f.0).collect::<Vec<_>>(),
        ["declared-twice", "wrong-value", "declared-twice"]
    );
}

#[test]
fn a_value_is_kept_as_it_is_written_and_a_list_gives_each_item_once() {
    let d = declared(&repository(&[(
        "/api/.pile/pile.toml",
        &format!(
            "{V}[pile]\nsize = 3.0\nwhen = 1979-05-27 07:32:00Z\nlabels = [\"a\", \"a\", \"b\"]\n"
        ),
    )]));
    assert!(d.findings.is_empty(), "{:?}", d.findings);
    let r = &d.relations;
    assert_eq!(
        r.value(&pile(API), "size"),
        Some(&Value::Float("3.0".into()))
    );
    assert_eq!(
        r.value(&pile(API), "when"),
        Some(&Value::Text("1979-05-27 07:32:00Z".into()))
    );
    assert_eq!(r.values(&pile(API), "labels").len(), 2);
}
