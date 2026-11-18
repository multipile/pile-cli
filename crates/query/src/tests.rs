//! The database of repositories held in memory, and programs asked of it.

use storage::memory::Memory;
use storage::{State, Storage};

use super::*;

const TOP: &str = "11111111111111111111111111111111";
const WORDS: &str = "22222222222222222222222222222222";
const API: &str = "33333333333333333333333333333333";
const APP: &str = "44444444444444444444444444444444";
const SUB: &str = "55555555555555555555555555555555";

const TYPES: &str = r#"version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository"

[[edge-types]]
name = "relies-on"
title = "one cannot work without the other"
source = ["pile"]
target = ["pile", "external"]
"#;

const NODES: &str = r#"version = "gitpile/1"

[[nodes]]
type = "external"
name = "git"
title = "the git binary"
since = 2005
ratio = 0.5
vendored = false
tags = ["vcs", "cli", "vcs", ["nested"]]
more = { a = 1 }

[pile]
labels = ["service"]
"#;

const LINKS: &str = r#"version = "gitpile/1"

[[edges]]
type = "relies-on"
target = "@api"
title = "calls"
port = 8080

[[edges]]
type = "relies-on"
target = "external:git"
"#;

/// An unnamed top; the aux pile `@words` with [`TYPES`]; `@api`; `@app`,
/// with [`NODES`] and [`LINKS`]; and an unnamed pile at `/app/sub`.
fn repository() -> Memory {
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
    declare("/app/sub/", piles::config::render(SUB, ""));
    m.write("/words/.pile/types.toml", TYPES);
    m.write("/app/.pile/nodes.toml", NODES);
    m.write("/app/.pile/links.toml", LINKS);
    m
}

fn built(m: &Memory) -> (Database, Relations) {
    let found = m.declarations(&State::Working).unwrap();
    let s = piles::structure(&found).unwrap();
    let rows = graph::read(m, &State::Working).unwrap();
    (database(&rows, &s), rows)
}

fn db() -> Database {
    built(&repository()).0
}

/// A row as a test writes one: an integer for an integer, text for a symbol.
macro_rules! row {
    ($($v:expr),* $(,)?) => { vec![$(Value::from_test($v)),*] };
}

trait FromTest<T> {
    fn from_test(v: T) -> Value;
}

impl FromTest<&str> for Value {
    fn from_test(v: &str) -> Value {
        Value::Symbol(v.to_string())
    }
}

impl FromTest<i64> for Value {
    fn from_test(v: i64) -> Value {
        Value::Integer(v)
    }
}

fn asked(db: &Database, program: &str) -> Vec<Answer> {
    ask(db, program, |name| db.pile_named(name)).unwrap()
}

fn refused(db: &Database, program: &str) -> Refused {
    ask(db, program, |name| db.pile_named(name)).unwrap_err()
}

const GIT: &str = "external:git";
const TO_API: &str = "@app relies-on @api";
const TO_GIT: &str = "@app relies-on external:git";

#[test]
fn node_has_a_row_per_node_with_its_type_and_its_home() {
    assert_eq!(
        db().rows("node"),
        [
            row![":/", "pile", ":/"],
            row![":/app/sub", "pile", ":/app/sub"],
            row!["@api", "pile", "@api"],
            row!["@app", "pile", "@app"],
            row!["@words", "pile", "@words"],
            row![GIT, "external", "@app"],
        ]
    );
}

#[test]
fn prop_has_a_row_per_value_an_integer_as_an_integer_and_the_rest_as_symbols() {
    let all = db().rows("prop");
    let of = |node: &str| -> Vec<Vec<Value>> {
        all.iter()
            .filter(|r| r[0] == Value::Symbol(node.into()))
            .cloned()
            .collect()
    };
    // A list gives a row per item, each once. An item that is a list, and a
    // value that is a table, give none.
    assert_eq!(
        of(GIT),
        [
            row![GIT, "ratio", "0.5"],
            row![GIT, "since", 2005],
            row![GIT, "tags", "cli"],
            row![GIT, "tags", "vcs"],
            row![GIT, "title", "the git binary"],
            row![GIT, "vendored", "false"],
        ]
    );
    assert_eq!(
        of("@app"),
        [
            row!["@app", "headline", "What it is."],
            row!["@app", "id", APP],
            row!["@app", "labels", "service"],
            row!["@app", "name", "app"],
            row!["@app", "path", ":/app"],
        ]
    );
    // A property that is not given has no row: the pile has no name.
    assert_eq!(
        of(":/app/sub"),
        [
            row![":/app/sub", "headline", "What it is."],
            row![":/app/sub", "id", SUB],
            row![":/app/sub", "path", ":/app/sub"],
        ]
    );
}

#[test]
fn edge_has_a_row_per_edge_and_edge_prop_a_row_per_value() {
    let db = db();
    assert_eq!(
        db.rows("edge"),
        [
            row![TO_API, "@app", "relies-on", "@api"],
            row![TO_GIT, "@app", "relies-on", GIT],
        ]
    );
    assert_eq!(
        db.rows("edge_prop"),
        [row![TO_API, "port", 8080], row![TO_API, "title", "calls"]]
    );
}

#[test]
fn type_type_prop_and_allows_hold_what_the_aux_pile_creates() {
    let db = db();
    // pile, which the format creates, has no row of type.
    assert_eq!(
        db.rows("type"),
        [
            row!["external", "node", "@words"],
            row!["relies-on", "edge", "@words"],
        ]
    );
    assert_eq!(
        db.rows("type_prop"),
        [
            row!["external", "title", "something outside the repository"],
            row!["relies-on", "title", "one cannot work without the other"],
        ]
    );
    assert_eq!(
        db.rows("allows"),
        [
            row!["relies-on", "source", "pile"],
            row!["relies-on", "target", "external"],
            row!["relies-on", "target", "pile"],
        ]
    );
}

#[test]
fn at_and_prop_at_say_where_each_is_written_as_integers() {
    let db = db();
    let at = db.rows("at");
    for row in [
        // A pile is at its config, line 1, column 1.
        row!["@app", ":/app/.pile/config", 1, 1],
        row![":/", ":/.pile/config", 1, 1],
        // A node and an edge are where their table starts, and a type too,
        // by its name.
        row![GIT, ":/app/.pile/nodes.toml", 3, 1],
        row![TO_API, ":/app/.pile/links.toml", 3, 1],
        row![TO_GIT, ":/app/.pile/links.toml", 9, 1],
        row!["external", ":/words/.pile/types.toml", 3, 1],
        row!["relies-on", ":/words/.pile/types.toml", 7, 1],
    ] {
        assert!(at.contains(&row), "{row:?} in {at:?}");
    }
    assert_eq!(at.len(), 10);

    let prop_at = db.rows("prop_at");
    for row in [
        row![GIT, "since", ":/app/.pile/nodes.toml", 7, 9],
        // A list has one row, at its [.
        row![GIT, "tags", ":/app/.pile/nodes.toml", 10, 8],
        row![TO_API, "port", ":/app/.pile/links.toml", 7, 8],
        row!["@app", "labels", ":/app/.pile/nodes.toml", 14, 10],
        row!["@app", "name", ":/app/.pile/config", 6, 8],
    ] {
        assert!(prop_at.contains(&row), "{row:?} in {prop_at:?}");
    }
    // Where a pile sits is no line of any file.
    assert!(!prop_at.iter().any(|r| r[1] == Value::Symbol("path".into())));
}

#[test]
fn parent_is_each_pile_and_the_pile_it_sits_in() {
    assert_eq!(
        db().rows("parent"),
        [
            row![":/app/sub", "@app"],
            row!["@api", ":/"],
            row!["@app", ":/"],
            row!["@words", ":/"],
        ]
    );
}

#[test]
fn the_relations_are_the_ten_a_program_reads() {
    let names: Vec<&str> = RELATIONS.iter().map(|(name, _, _)| *name).collect();
    assert_eq!(
        names,
        [
            "node",
            "prop",
            "edge",
            "edge_prop",
            "type",
            "type_prop",
            "allows",
            "at",
            "prop_at",
            "parent"
        ]
    );
    let db = db();
    for (name, columns, _) in RELATIONS {
        for row in db.rows(name) {
            assert_eq!(row.len(), columns.len(), "{name}");
        }
    }
}

#[test]
fn a_query_is_answered_with_its_columns_and_its_rows() {
    let db = db();
    let answers = asked(
        &db,
        "?- edge(Edge, Node, \"relies-on\", Target), prop(Edge, port, Port).",
    );
    assert_eq!(answers.len(), 1);
    let a = &answers[0];
    assert_eq!(
        a.query,
        "?- edge(Edge, Node, \"relies-on\", Target), prop(Edge, port, Port)."
    );
    assert_eq!(a.columns, ["Edge", "Node", "Target", "Port"]);
    // prop is of nodes, and an edge's values are in edge_prop.
    assert!(a.rows.is_empty());

    let a = &asked(&db, "?- edge(Edge, _, _, _), edge_prop(Edge, port, Port).")[0];
    assert_eq!(a.columns, ["Edge", "Port"]);
    assert_eq!(a.rows, [row![TO_API, 8080]]);
    // An integer is an integer, so it is compared as one.
    assert_eq!(
        asked(&db, "?- edge_prop(E, port, P), P > 8000, P < 9000.")[0]
            .rows
            .len(),
        1
    );
    assert!(
        asked(&db, "?- edge_prop(E, port, \"8080\").")[0]
            .rows
            .is_empty()
    );
}

#[test]
fn an_at_word_is_the_pile_of_that_name() {
    let db = db();
    let a = &asked(&db, "?- edge(_, @app, \"relies-on\", Node).")[0];
    assert_eq!(a.columns, ["Node"]);
    assert_eq!(a.rows, [row!["@api"], row![GIT]]);
    // A quoted string that starts with @ is the same value.
    let quoted = &asked(&db, "?- edge(_, \"@app\", \"relies-on\", Node).")[0];
    assert_eq!(quoted.rows, a.rows);
    // A name's own value is text, and not the pile.
    assert_eq!(
        asked(&db, "?- prop(@app, name, Name).")[0].rows,
        [row!["app"]]
    );
}

#[test]
fn an_at_word_that_names_no_pile_is_refused_where_it_is_written() {
    let db = db();
    let r = refused(&db, "near(N) :- parent(N, _).\n?- edge(_, N, _, @nope).");
    assert_eq!(
        r,
        Refused {
            line: 2,
            column: 18,
            message: "no pile is named @nope".into()
        }
    );
}

#[test]
fn a_node_is_one_symbol_however_it_is_referred_to() {
    let (db, _) = built(&repository());
    let app = Node::Pile(APP.into());
    assert_eq!(db.node_symbol(&app), Some("@app"));
    assert_eq!(db.pile_named("app"), Some(app.clone()));
    assert_eq!(db.node(&Value::Symbol("@app".into())), Some(&app));

    // A pile with no name is its path, and is still one node.
    let sub = Node::Pile(SUB.into());
    assert_eq!(db.node_symbol(&sub), Some(":/app/sub"));

    let git = Node::Named {
        of_type: "external".into(),
        name: "git".into(),
    };
    assert_eq!(db.node_symbol(&git), Some(GIT));

    // Whichever way a caller names it, a program is given the one symbol.
    let by_id = ask(&db, "?- edge(Edge, @x, _, _).", |_| Some(app.clone())).unwrap();
    assert_eq!(by_id[0].rows, [row![TO_API], row![TO_GIT]]);
}

#[test]
fn an_answer_s_node_and_edge_lead_back_to_the_graph() {
    let (db, rows) = built(&repository());
    let a = &asked(&db, "?- edge(Edge, _, _, Node).")[0];
    let nodes: Vec<&Node> = column(a, "Node")
        .unwrap()
        .into_iter()
        .map(|v| db.node(v).unwrap())
        .collect();
    let git = Node::Named {
        of_type: "external".into(),
        name: "git".into(),
    };
    assert_eq!(nodes, [&Node::Pile(API.into()), &git]);
    let edges: Vec<&Edge> = column(a, "Edge")
        .unwrap()
        .into_iter()
        .map(|v| db.edge(v).unwrap())
        .collect();
    assert_eq!(edges, rows.edge.iter().collect::<Vec<_>>());
    for e in &rows.edge {
        assert_eq!(db.edge(&Value::Symbol(db.written(e))), Some(e));
    }
    assert_eq!(column(a, "Other"), None);

    // A value that is no node's, or no edge's, leads nowhere.
    assert_eq!(db.node(&Value::Symbol("relies-on".into())), None);
    assert_eq!(db.node(&Value::Symbol(TO_API.into())), None);
    assert_eq!(db.edge(&Value::Symbol("@app".into())), None);
    assert_eq!(db.node(&Value::Integer(1)), None);
    assert_eq!(db.edge(&Value::Integer(1)), None);
    let gone = Edge {
        source: git.clone(),
        of_type: "relies-on".into(),
        target: Node::Pile(API.into()),
    };
    assert_eq!(db.edge(&Value::Symbol(db.written(&gone))), None);
    assert_eq!(db.node_symbol(&Node::Pile("0".repeat(32))), None);
}

#[test]
fn a_program_the_parser_refuses_gives_its_line_and_column() {
    let r = refused(&db(), "?- node(N, pile, _).\n?- node(N, pile _).");
    assert_eq!((r.line, r.column), (2, 17));
    assert!(!r.message.is_empty());
}

#[test]
fn a_program_the_evaluator_refuses_gives_its_line_and_column() {
    let db = db();
    let r = refused(&db, "?- node(N, pile, _),\n   nodes(N).");
    assert_eq!((r.line, r.column), (2, 4));
    assert!(r.message.contains("nodes is no relation"), "{r:?}");

    let r = refused(&db, "?- node(N, pile).");
    assert_eq!((r.line, r.column), (1, 4));
    assert!(r.message.contains("node has 3 columns"), "{r:?}");

    // A relation the database holds is not defined again.
    let r = refused(&db, "\nparent(a, b).");
    assert_eq!((r.line, r.column), (2, 1));
    assert!(r.message.contains("parent is pile's"), "{r:?}");
}

#[test]
fn what_a_program_defines_does_not_stay_for_the_next() {
    let db = db();
    let program = "under(C, P) :- parent(C, P).\n\
                   under(C, P) :- parent(C, M), under(M, P).\n\
                   ?- under(Node, \":/\").";
    let first = asked(&db, program);
    assert_eq!(
        first[0].rows,
        [
            row![":/app/sub"],
            row!["@api"],
            row!["@app"],
            row!["@words"]
        ]
    );
    // The next program does not see it, and may define it again.
    let r = refused(&db, "?- under(Node, _).");
    assert!(r.message.contains("under is no relation"), "{r:?}");
    assert_eq!(asked(&db, program), first);
}

#[test]
fn each_query_has_an_answer_in_the_order_written_and_a_program_with_none_has_none() {
    let db = db();
    let answers = asked(
        &db,
        "?- type(Type, node, _).\nservice(P) :- prop(P, labels, service).\n?- service(Node).",
    );
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0].rows, [row!["external"]]);
    assert_eq!(answers[1].query, "?- service(Node).");
    assert_eq!(answers[1].rows, [row!["@app"]]);
    assert!(asked(&db, "service(P) :- prop(P, labels, service).").is_empty());
    assert!(asked(&db, "").is_empty());
}

#[test]
fn a_name_two_piles_claim_names_neither_and_each_is_written_by_its_path() {
    let mut m = repository();
    m.write(
        "/other/.pile/config",
        piles::config::render("66666666666666666666666666666666", "app"),
    );
    let (db, _) = built(&m);
    assert_eq!(db.pile_named("app"), None);
    assert_eq!(db.node_symbol(&Node::Pile(APP.into())), Some(":/app"));
    assert_eq!(
        refused(&db, "?- parent(@app, P).").message,
        "no pile is named @app"
    );
    assert_eq!(
        asked(&db, "?- parent(\":/other\", P).")[0].rows,
        [row![":/"]]
    );
}
