//! `pile query`: a Datalog program asked of what the repository declares,
//! answered query by query, or refused at the line and column of its
//! mistake (pile query --help).

use serde_json::json;

use crate::harness::*;
use crate::links::linked;

#[test]
fn each_query_is_printed_as_it_was_read_with_its_columns_its_rows_and_their_count() {
    let r = linked();
    assert_eq!(
        r.stdout(&[
            "query",
            "?- edge(Edge, @cli, uses, Node).\n?- node(Node, pile, _), !edge(_, Node, _, _)."
        ]),
        "?- edge(Edge, @cli, uses, Node).\n   \
         Edge                    Node\n   \
         @cli uses @storage      @storage\n   \
         @cli uses external:git  external:git\n   \
         2 rows\n\n\
         ?- node(Node, pile, _), !edge(_, Node, _, _).\n   \
         Node\n   \
         @storage\n   \
         @top\n   \
         @words\n   \
         3 rows\n"
    );
}

#[test]
fn a_query_that_binds_nothing_is_answered_yes_or_no_and_one_with_no_rows_says_so() {
    let r = linked();
    assert_eq!(
        r.stdout(&["query", "?- edge(_, @journeys, exercises, @cli)."]),
        "?- edge(_, @journeys, exercises, @cli).\n   yes\n"
    );
    assert_eq!(
        r.stdout(&["query", "?- edge(_, @cli, exercises, @journeys)."]),
        "?- edge(_, @cli, exercises, @journeys).\n   no\n"
    );
    assert_eq!(
        r.stdout(&["query", "?- edge(_, @storage, _, Node)."]),
        "?- edge(_, @storage, _, Node).\n   no rows\n"
    );
    assert_eq!(
        r.result(&["query", "?- edge(_, @journeys, exercises, @cli)."]),
        json!({"answers": [{
            "query": "?- edge(_, @journeys, exercises, @cli).",
            "columns": [],
            "rows": [[]],
        }]})
    );
}

#[test]
fn the_result_gives_an_integer_as_a_number_and_anything_else_as_a_string() {
    let r = linked();
    let v = r.result(&[
        "query",
        "?- edge(Edge, _, exercises, _), edge_prop(Edge, Property, Value).\n\
         ?- node(Node, external, Home), at(Node, File, Line, Column).",
    ]);
    assert_eq!(
        v,
        json!({"answers": [
            {
                "query": "?- edge(Edge, _, exercises, _), edge_prop(Edge, Property, Value).",
                "columns": ["Edge", "Property", "Value"],
                "rows": [
                    ["@journeys exercises @cli", "through", "binary"],
                    ["@journeys exercises @cli", "title", "holds what it prints to its help"],
                ],
            },
            {
                "query": "?- node(Node, external, Home), at(Node, File, Line, Column).",
                "columns": ["Node", "Home", "File", "Line", "Column"],
                "rows": [["external:git", "@top", ":/.pile/nodes.toml", 3, 1]],
            },
        ]})
    );
}

#[test]
fn a_rule_of_the_program_is_read_by_its_queries_and_parent_is_the_pile_a_pile_sits_in() {
    let r = linked();
    let v = r.result(&[
        "query",
        "under(C, P) :- parent(C, P).\n\
         under(C, P) :- parent(C, M), under(M, P).\n\
         ?- under(Node, @top).",
    ]);
    assert_eq!(
        v["answers"][0]["rows"],
        json!([["@cli"], ["@journeys"], ["@storage"], ["@words"]])
    );
    // The rule was that program's alone.
    let e = r.refused(&["query", "?- under(Node, @top)."], 1);
    contains!(
        e["error"]["message"].as_str().unwrap(),
        "under is no relation"
    );
}

#[test]
fn a_program_that_does_not_parse_is_refused_at_its_line_and_column() {
    let r = linked();
    let a = r.answered(
        &[],
        &["query", "?- node(Node, pile, _).\n?- node(Node pile, _)."],
    );
    assert_eq!(a.code, 1);
    assert_eq!(a.stdout, "");
    contains!(
        a.stderr,
        "pile: the program is refused at 2:14: ",
        "pile guide query"
    );
    let e = r.refused(
        &["query", "?- node(Node, pile, _).\n?- node(Node pile, _)."],
        1,
    );
    assert_eq!(e["error"]["code"], "program-refused");
    assert_eq!(e["error"]["usage"], false);
    assert_eq!(e["result"]["refused"]["file"], json!(null));
    assert_eq!(e["result"]["refused"]["line"], 2);
    assert_eq!(e["result"]["refused"]["column"], 14);
    assert_eq!(e["next"][0]["command"], "pile guide query");
}

#[test]
fn a_program_that_cannot_be_evaluated_is_refused_the_same_way() {
    let r = linked();
    let e = r.refused(&["query", "?- node(Node, pile)."], 1);
    assert_eq!(e["error"]["code"], "program-refused");
    assert_eq!(
        e["error"]["message"],
        "the program is refused at 1:4: node has 3 columns and is written here with 2 columns"
    );
    let e = r.refused(&["query", "node(a, b, c)."], 1);
    contains!(
        e["error"]["message"].as_str().unwrap(),
        "at 1:1: node is pile's"
    );
}

#[test]
fn a_name_no_pile_claims_is_refused_where_it_is_written() {
    let r = linked();
    let e = r.refused(&["query", "?- edge(_, Node, uses, @store)."], 1);
    assert_eq!(e["error"]["code"], "program-refused");
    assert_eq!(
        e["error"]["message"],
        "the program is refused at 1:24: no pile is named @store"
    );
    assert_eq!(
        e["result"]["refused"],
        json!({"file": null, "line": 1, "column": 24, "message": "no pile is named @store"})
    );
}

#[test]
fn file_reads_the_program_from_a_file_and_a_refusal_names_the_file() {
    let r = linked();
    r.write(
        "uses.dl",
        "% what the cli uses\n?- edge(_, @cli, uses, Node).\n",
    );
    assert_eq!(
        r.result(&["query", "--file", "uses.dl"])["answers"][0]["rows"],
        json!([["@storage"], ["external:git"]])
    );
    r.write(
        "wrong.dl",
        "?- edge(_, @cli, uses, Node).\n\n?- edges(E).\n",
    );
    let e = r.refused(&["query", "--file", "wrong.dl"], 1);
    contains!(
        e["error"]["message"].as_str().unwrap(),
        "the program is refused at wrong.dl:3:4: edges is no relation"
    );
    assert_eq!(e["result"]["refused"]["file"], "wrong.dl");

    let e = r.refused(&["query", "--file", "none.dl"], 2);
    assert_eq!(e["error"]["code"], "unreadable-program");
    contains!(
        e["error"]["message"].as_str().unwrap(),
        "the program in none.dl cannot be read"
    );
}

#[test]
fn a_program_with_no_query_is_answered_with_nothing_and_says_so() {
    let r = linked();
    contains!(
        r.stdout(&["query", "service(P) :- prop(P, labels, service)."]),
        "the program asks nothing"
    );
    assert_eq!(
        r.result(&["query", "service(P) :- prop(P, labels, service)."]),
        json!({"answers": []})
    );
}

#[test]
fn query_takes_one_program_or_a_file_and_never_both_or_neither() {
    let r = linked();
    for args in [
        &["query"][..],
        &["query", "?- node(N, _, _).", "--file", "uses.dl"],
        &["query", "?-", "node(N, _, _)."],
    ] {
        let e = r.refused(args, 2);
        assert_eq!(e["error"]["code"], "usage", "{args:?}");
        assert_eq!(e["next"][0]["command"], "pile query --help");
    }
}

#[test]
fn query_is_refused_where_there_is_no_pile_and_where_a_config_does_not_read() {
    let r = Repo::new();
    let e = r.refused(&["query", "?- node(N, _, _)."], 1);
    assert_eq!(e["error"]["code"], "no-piles");

    let r = linked();
    r.write("cli/.pile/config", "kind = gitpile\nversion = 1\n");
    let e = r.refused(&["query", "?- node(N, _, _)."], 1);
    assert_eq!(e["error"]["code"], "unreadable-config");
}

#[test]
fn query_reads_the_working_copy_from_anywhere_in_it() {
    let r = linked();
    r.declare("cli/sub", "f", "");
    let v = r.result_in("storage", &["query", "?- parent(Node, @cli)."]);
    // A pile with no name is written by its path.
    assert_eq!(v["answers"][0]["rows"], json!([[":/cli/sub"]]));
    let v = r.result(&["query", "?- parent(\":/cli/sub\", Parent)."]);
    assert_eq!(v["answers"][0]["rows"], json!([["@cli"]]));
}

#[test]
fn help_lists_query_and_its_help_names_every_relation_a_program_reads() {
    let r = Repo::new();
    contains!(
        r.stdout(&["--help"]),
        "query       Ask a Datalog program of what the repository declares"
    );
    let help = r.stdout(&["query", "--help"]);
    contains!(
        help,
        "node(Node, Type, Home)",
        "prop(Node, Property, Value)",
        "edge(Edge, Source, Type, Target)",
        "edge_prop(Edge, Property, Value)",
        "type(Type, Sort, Home)",
        "type_prop(Type, Property, Value)",
        "allows(EdgeType, End, NodeType)",
        "at(Item, File, Line, Column)",
        "prop_at(Item, Property, File, Line, Column)",
        "parent(Child, Parent)",
        "program-refused",
        "unreadable-program",
    );
    assert_eq!(help, r.stdout(&["help", "query"]));
}
