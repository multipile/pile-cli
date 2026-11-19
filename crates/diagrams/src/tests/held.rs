//! A diagram held to a graph: each finding, with a graph that gives it and
//! one that does not; a program in each of the three ways one is written;
//! and what a change touched of what a diagram stands for.

use std::collections::BTreeSet;

use diagram_format::finding::locate;
use graph::{Edge, Node, Relations};
use query::Database;
use storage::memory::Memory;
use storage::{State, Storage};

use crate::*;

const TOP: &str = "11111111111111111111111111111111";
const WORDS: &str = "22222222222222222222222222222222";
const WEB: &str = "33333333333333333333333333333333";
const BILLING: &str = "44444444444444444444444444444444";
const ORDERS: &str = "55555555555555555555555555555555";
const DOCS: &str = "66666666666666666666666666666666";
const NOBODY: &str = "ffffffffffffffffffffffffffffffff";

const TYPES: &str = r#"version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository"

[[edge-types]]
name = "relies-on"
title = "one cannot work without the other"
source = ["pile"]
target = ["pile", "external"]

[[edge-types]]
name = "tests"
title = "one says whether the other works"
source = ["pile"]
target = ["pile"]
"#;

const NODES: &str = r#"version = "gitpile/1"

[[nodes]]
type = "external"
name = "git"
title = "the git binary"

[pile]
labels = ["service"]
"#;

fn links(title: &str) -> String {
    format!(
        "version = \"gitpile/1\"\n\n[[edges]]\ntype = \"relies-on\"\ntarget = \"@billing\"\n\
         title = \"{title}\"\n\n[[edges]]\ntype = \"relies-on\"\ntarget = \"external:git\"\n"
    )
}

const ORDERS_LINKS: &str = r#"version = "gitpile/1"

[[edges]]
type = "relies-on"
target = "@billing"

[[edges]]
type = "tests"
target = "@web"
"#;

/// `@top`, which declares `external:git` and is labelled `service`; the aux
/// pile `@words`; `@web`, which relies on `@billing` and on `external:git`;
/// `@billing`; `@orders`, which relies on `@billing` and tests `@web`; and
/// a pile with no name at `/docs`.
fn repository() -> Memory {
    let mut m = Memory::new();
    let mut declare = |dir: &str, config: String| {
        m.write(&format!("{dir}.pile/config"), config);
        m.write(&format!("{dir}.pile/intent.md"), "What it is.\n");
    };
    declare("/", piles::config::render(TOP, "top"));
    declare(
        "/words/",
        piles::config::render_kind(piles::config::AUX, WORDS, "words"),
    );
    declare("/web/", piles::config::render(WEB, "web"));
    declare("/billing/", piles::config::render(BILLING, "billing"));
    declare("/orders/", piles::config::render(ORDERS, "orders"));
    declare("/docs/", piles::config::render(DOCS, ""));
    m.write("/words/.pile/types.toml", TYPES);
    m.write("/.pile/nodes.toml", NODES);
    m.write("/web/.pile/links.toml", links("charges through"));
    m.write("/orders/.pile/links.toml", ORDERS_LINKS);
    m
}

/// The graph of a repository.
struct Built {
    rows: Relations,
    db: Database,
}

impl Built {
    fn of(m: &Memory) -> Built {
        let found = m.declarations(&State::Working).unwrap();
        let s = piles::structure(&found).unwrap();
        let rows = graph::read(m, &State::Working).unwrap();
        let db = query::database(&rows, &s);
        Built { rows, db }
    }

    fn graph(&self) -> Graph<'_> {
        Graph {
            rows: &self.rows,
            db: &self.db,
        }
    }
}

fn diagram(source: &str) -> Diagram {
    Diagram {
        name: "deps".into(),
        dir: "/".into(),
        path: "/.pile/diagrams/deps.diagram".into(),
        source: Some(source.into()),
    }
}

/// A dependency diagram of these lines.
fn dependency(body: &str) -> String {
    format!(
        "diagram dependency version=1 {{\n  title \"What relies on billing\"\n  desc \"What \
         relies on what.\"\n{body}\n}}\n"
    )
}

/// The boxes most tests draw: `@web`, `@billing` and `external:git`.
fn boxes() -> String {
    format!(
        "  box web pile:id=\"{WEB}\"\n  box billing pile:id=\"{BILLING}\"\n  box git \
         pile:id=\"{TOP}\" pile:node-type=external"
    )
}

/// What is wrong with a diagram of the repository, held to its graph.
fn held(source: &str) -> Vec<Finding> {
    let built = Built::of(&repository());
    findings(&diagram(source), Some(built.graph()))
}

fn codes(findings: &[Finding]) -> Vec<&'static str> {
    findings.iter().map(|f| f.code).collect()
}

/// The line and the column of the first place a text is written.
fn at(source: &str, needle: &str) -> (Option<usize>, Option<usize>) {
    let (line, column) = locate(source, source.find(needle).expect(needle));
    (Some(line), Some(column))
}

fn place(f: &Finding) -> (Option<usize>, Option<usize>) {
    (f.line, f.column)
}

#[test]
fn a_diagram_whose_boxes_and_lines_are_the_graphs_has_nothing_wrong() {
    let source = dependency(&format!(
        "{}\n  box orders pile:id=\"{ORDERS}\"\n  box docs \"The handbook\" pile:id=\"{DOCS}\"\n  \
         line web billing pile:edge=\"relies-on\"\n  line orders billing pile:edge=\"relies-on\"\n  \
         line web git pile:edge=\"relies-on\"\n  line orders web pile:edge=\"tests\"\n  \
         line docs web\n  meta pile {{\n    boxes {{\n      include #\"?- edge(_, Node, \
         \"relies-on\", @billing).\"#\n    }}\n    lines {{\n      include #\"?- edge(Edge, _, \
         _, _).\"#\n    }}\n  }}",
        boxes()
    ));
    assert_eq!(held(&source), vec![]);
}

#[test]
fn an_id_no_pile_has_is_a_problem_and_says_the_id_of_the_pile_of_that_name() {
    let source = dependency(&format!("  box web pile:id=\"{NOBODY}\"\n  box other"));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-no-such-pile"]);
    assert_eq!(place(&found[0]), at(&source, "box web"));
    assert!(
        found[0].fix.contains(&format!("pile:id=\"{WEB}\"")),
        "{:?}",
        found[0]
    );
    assert_eq!(found[0].severity, Severity::Problem);

    let source = dependency(&format!("  box web pile:id=\"{WEB}\"\n  box other"));
    assert_eq!(held(&source), vec![]);
}

#[test]
fn a_pile_named_otherwise_than_its_box_is_a_problem_that_says_its_name() {
    let source = dependency(&format!("  box posts pile:id=\"{WEB}\"\n  box other"));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-name-differs"]);
    assert_eq!(place(&found[0]), at(&source, "box posts"));
    assert!(found[0].message.contains("named web"), "{:?}", found[0]);

    // The name it shows, the name pile:name gives, and any name for a pile
    // that has none.
    for fine in [
        format!("  box w \"web\" pile:id=\"{WEB}\""),
        format!("  box w \"The web application\" pile:id=\"{WEB}\" pile:name=\"web\""),
        format!("  box handbook pile:id=\"{DOCS}\""),
    ] {
        assert_eq!(held(&dependency(&format!("{fine}\n  box other"))), vec![]);
    }
}

#[test]
fn a_node_that_is_not_at_home_in_the_pile_is_a_problem_that_says_its_home() {
    let source = dependency(&format!(
        "  box git pile:id=\"{WEB}\" pile:node-type=external\n  box other"
    ));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-no-such-node"]);
    assert_eq!(place(&found[0]), at(&source, "box git"));
    assert!(
        found[0].message.contains("at home in @top"),
        "{:?}",
        found[0]
    );
    assert_eq!(found[0].fix, format!("write pile:id=\"{TOP}\""));

    let source = dependency(&format!(
        "  box gti pile:id=\"{TOP}\" pile:node-type=external\n  box other"
    ));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-no-such-node"]);
    assert_eq!(found[0].fix, "did you mean external:git?");

    let source = dependency(&format!(
        "  box g \"git binary\" pile:id=\"{TOP}\" pile:node-type=external pile:name=git\n  box other"
    ));
    assert_eq!(held(&source), vec![]);
}

#[test]
fn a_type_or_a_name_with_no_id_is_a_problem() {
    for (written, at_) in [
        ("box git pile:node-type=external", "pile:node-type"),
        ("box w \"Web\" pile:name=\"web\"", "pile:name"),
    ] {
        let source = dependency(&format!("  {written}\n  box other"));
        let found = held(&source);
        assert_eq!(codes(&found), vec!["diagram-needs-id"], "{written}");
        assert_eq!(place(&found[0]), at(&source, at_));
        // The file alone shows it.
        assert_eq!(
            codes(&findings(&diagram(&source), None)),
            vec!["diagram-needs-id"]
        );
    }
    assert_eq!(held(&dependency("  box git\n  box other")), vec![]);
}

#[test]
fn a_line_whose_edge_no_link_declares_is_a_problem() {
    let source = dependency(&format!(
        "{}\n  line billing web pile:edge=\"relies-on\"",
        boxes()
    ));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-no-such-edge"]);
    assert_eq!(place(&found[0]), at(&source, "line billing web"));
    assert!(
        found[0].fix.contains("write line web billing"),
        "the link runs the other way: {:?}",
        found[0]
    );

    let source = dependency(&format!(
        "{}\n  line web billing pile:edge=\"tests\"",
        boxes()
    ));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-no-such-edge"]);
    assert!(found[0].fix.contains("relies-on"), "{:?}", found[0]);

    for fine in [
        "line web billing pile:edge=\"relies-on\"",
        "line web git pile:edge=\"relies-on\"",
        "cycle web billing pile:edge=\"relies-on\"",
        // A line that says nothing of an edge draws what it likes.
        "line billing web",
    ] {
        let source = dependency(&format!("{}\n  line billing git\n  {fine}", boxes()));
        assert_eq!(held(&source), vec![], "{fine}");
    }
}

#[test]
fn an_edge_on_a_line_with_a_box_that_stands_for_no_node_is_a_problem() {
    let source = dependency(&format!(
        "  box web pile:id=\"{WEB}\"\n  box store\n  line web store pile:edge=\"relies-on\""
    ));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-edge-needs-nodes"]);
    assert_eq!(place(&found[0]), at(&source, "line web store"));
    assert!(found[0].message.contains("store stands for no node"));

    // A box that names nothing has its own finding, and the line no second.
    let source = dependency(&format!(
        "  box web pile:id=\"{WEB}\"\n  box billing pile:id=\"{NOBODY}\"\n  line web billing \
         pile:edge=\"relies-on\""
    ));
    assert_eq!(codes(&held(&source)), vec!["diagram-no-such-pile"]);
}

#[test]
fn a_word_pile_does_not_read_is_a_problem_that_says_the_nearest() {
    let unknown = |body: &str, at_: &str, fix: &str| {
        let source = dependency(body);
        let found = held(&source);
        assert_eq!(codes(&found), vec!["diagram-meta-unknown"], "{body}");
        assert_eq!(place(&found[0]), at(&source, at_), "{body}");
        assert!(found[0].fix.contains(fix), "{body}: {:?}", found[0]);
        // The file alone shows it.
        assert_eq!(
            codes(&findings(&diagram(&source), None)),
            vec!["diagram-meta-unknown"]
        );
    };
    let web = format!("  box web pile:id=\"{WEB}\"");
    unknown(
        &format!("{web} pile:nmae=\"web\"\n  box other"),
        "pile:nmae",
        "did you mean pile:name?",
    );
    unknown(
        &format!("{web} pile:tpye=external\n  box other"),
        "pile:tpye",
        "did you mean pile:node-type?",
    );
    unknown(
        &format!("{web} pile:edge=\"relies-on\"\n  box other"),
        "pile:edge",
        "on the line",
    );
    unknown(
        &format!("{web} pile:node-type=pile\n  box other"),
        "pile:node-type",
        "leave pile:node-type out",
    );
    unknown(
        &format!("{web}\n  box other\n  line web other pile:egde=\"relies-on\""),
        "pile:egde",
        "did you mean pile:edge?",
    );
    unknown(
        &format!("{web}\n  box other\n  meta pile {{\n    boxs {{\n    }}\n  }}"),
        "boxs",
        "did you mean boxes?",
    );
    unknown(
        &format!(
            "{web}\n  box other\n  meta pile {{\n    boxes {{\n      exclude \"web\"\n    }}\n  }}"
        ),
        "exclude",
        "did you mean include?",
    );
    unknown(
        &format!("{web}\n  box other\n  meta pile {{\n    boxes {{\n      include 7\n    }}\n  }}"),
        "7",
        "raw string",
    );
    unknown(
        &format!(
            "{web}\n  box other\n  meta pile {{\n    boxes {{\n      ignore \"web\" \
             id=\"{WEB}\" reson=\"soon\" reason=\"gone soon\"\n    }}\n  }}"
        ),
        "reson",
        "did you mean reason?",
    );

    // Lines, in a type no line of which stands for an edge.
    let source = "diagram tree version=1 {\n  title \"Piles\"\n  desc \"A tree.\"\n  item \
                  \"top\"\n  meta pile {\n    lines {\n    }\n  }\n}\n";
    let found = held(source);
    assert_eq!(codes(&found), vec!["diagram-meta-unknown"]);
    assert_eq!(place(&found[0]), at(source, "lines"));

    // Another owner's metadata is not pile's to read.
    let source = dependency(&format!(
        "{web} docs:page=\"web.md\"\n  box other\n  meta docs {{\n    anything 1\n  }}"
    ));
    assert_eq!(held(&source), vec![]);
}

/// A diagram of `@web` and `@billing` whose `boxes` holds this.
fn including(written: &str) -> String {
    dependency(&format!(
        "  box web pile:id=\"{WEB}\"\n  box orders pile:id=\"{ORDERS}\"\n  box billing \
         pile:id=\"{BILLING}\"\n  meta pile {{\n    boxes {{\n{written}\n    }}\n  }}"
    ))
}

#[test]
fn a_program_is_one_line_several_lines_or_several_arguments() {
    for program in [
        // On one line.
        "      include #\"?- edge(_, Node, \"relies-on\", @billing).\"#",
        // One clause for each argument.
        "      include #\"near(N) :- edge(_, N, \"relies-on\", @billing).\"# \\\n              \
         #\"near(N) :- edge(_, @billing, \"relies-on\", N).\"# \\\n              #\"?- \
         near(Node).\"#",
        // On several lines.
        "      include #\"\"\"\n        near(N) :- edge(_, N, \"relies-on\", @billing).\n        \
         ?- near(Node), !edge(_, Node, \"relies-on\", @words).\n        \"\"\"#",
        // A plain string, where nothing in the program is quoted.
        "      include \"?- edge(_, Node, _, @billing).\"",
    ] {
        assert_eq!(held(&including(program)), vec![], "{program}");
    }
}

#[test]
fn a_refused_program_is_a_problem_at_its_place_in_the_file_in_each_form() {
    let refused = |program: &str, needle: &str, says: &str| {
        let source = including(program);
        let found = held(&source);
        assert_eq!(codes(&found), vec!["diagram-include-invalid"], "{program}");
        assert_eq!(place(&found[0]), at(&source, needle), "{program}");
        assert!(found[0].message.contains(says), "{:?}", found[0]);
    };
    // On one line: a pile nobody is, and a relation there is not.
    refused(
        "      include #\"?- edge(_, Node, \"relies-on\", @nobody).\"#",
        "@nobody",
        "no pile is named @nobody",
    );
    refused(
        "      include #\"?- egde(_, Node, \"relies-on\", @billing).\"#",
        "egde",
        "egde",
    );
    // One clause for each argument: the mistake is in the second, and then
    // in the third.
    refused(
        "      include #\"near(N) :- edge(_, N, \"relies-on\", @billing).\"# \\\n              \
         #\"near(N) :- edge(_, @nobody, \"relies-on\", N).\"# \\\n              #\"?- \
         near(Node).\"#",
        "@nobody",
        "no pile is named @nobody",
    );
    refused(
        "      include #\"near(N) :- edge(_, N, \"relies-on\", @billing).\"# \\\n              \
         #\"near(N) :- edge(_, @billing, \"relies-on\", N).\"# \\\n              #\"?- \
         naer(Node).\"#",
        "naer",
        "naer",
    );
    // On several lines: the mistake is on the second line of the string.
    refused(
        "      include #\"\"\"\n        near(N) :- edge(_, N, \"relies-on\", @billing).\n        \
         ?- near(Node), !edge(_, Node, \"relies-on\", @nobody).\n        \"\"\"#",
        "@nobody",
        "no pile is named @nobody",
    );
    // An argument of several lines after another argument.
    refused(
        "      include #\"near(N) :- edge(_, N, \"relies-on\", @billing).\"# #\"\"\"\n        \
         far(N) :- near(N).\n        ?- far(Node), edge(_, Node, _, @nobody).\n        \"\"\"#",
        "@nobody",
        "no pile is named @nobody",
    );
    // A character of several bytes before the mistake, in a comment.
    refused(
        "      include #\"\"\"\n        % naïve: every pile → billing\n        ?- \
         edge(_, Node, \"relies-on\", @nobody).\n        \"\"\"#",
        "@nobody",
        "no pile is named @nobody",
    );
}

#[test]
fn a_program_with_no_query_or_one_that_binds_no_node_is_a_problem() {
    for (program, says) in [
        (
            "      include #\"near(N) :- edge(_, N, \"relies-on\", @billing).\"#",
            "no query",
        ),
        (
            "      include #\"?- edge(_, N, \"relies-on\", @billing).\"#",
            "binds no Node",
        ),
        (
            "      include #\"?- edge(Node, _, \"relies-on\", @billing).\"#",
            "which is no node",
        ),
        ("      include", "no program"),
    ] {
        let source = including(program);
        let found = held(&source);
        assert_eq!(codes(&found)[0], "diagram-include-invalid", "{program}");
        assert!(
            codes(&found)
                .iter()
                .all(|c| *c == "diagram-include-invalid"),
            "{program}"
        );
        assert_eq!(place(&found[0]), at(&source, "include"), "{program}");
        assert!(found[0].message.contains(says), "{:?}", found[0]);
    }
    // Under lines, a query binds Edge.
    let source = dependency(&format!(
        "{}\n  line web billing pile:edge=\"relies-on\"\n  line web git pile:edge=\"relies-on\"\n  \
         meta pile {{\n    lines {{\n      include #\"?- edge(_, Node, \"relies-on\", _).\"#\n    \
         }}\n  }}",
        boxes()
    ));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-include-invalid"]);
    assert!(found[0].message.contains("binds no Edge"), "{:?}", found[0]);
}

#[test]
fn an_include_that_finds_nothing_is_a_problem() {
    let source = including("      include \"?- prop(Node, labels, servce).\"");
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-include-empty"]);
    assert_eq!(place(&found[0]), at(&source, "include"));

    let source = dependency(&format!(
        "{}\n  box top pile:id=\"{TOP}\"\n  line web git\n  meta pile {{\n    boxes {{\n      \
         include \"?- prop(Node, labels, service).\"\n    }}\n  }}",
        boxes()
    ));
    assert_eq!(held(&source), vec![]);

    let source = dependency(&format!(
        "{}\n  line web git\n  meta pile {{\n    lines {{\n      include \"?- edge(Edge, _, \
         calls, _).\"\n    }}\n  }}",
        boxes()
    ));
    assert_eq!(codes(&held(&source)), vec!["diagram-include-empty"]);
}

/// A diagram of `@web` and `@billing` that should show what relies on
/// `@billing`, with this beside the include.
fn relying(boxes_: &str, beside: &str) -> String {
    dependency(&format!(
        "  box web pile:id=\"{WEB}\"\n  box billing pile:id=\"{BILLING}\"\n{boxes_}  meta pile \
         {{\n    boxes {{\n      include #\"?- edge(_, Node, \"relies-on\", \
         @billing).\"#\n{beside}    }}\n  }}"
    ))
}

#[test]
fn a_node_the_diagram_should_show_and_no_box_draws_is_a_problem_at_the_include() {
    let source = relying("", "");
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-misses-node"]);
    assert_eq!(place(&found[0]), at(&source, "include"));
    assert!(found[0].message.contains("@orders"), "{:?}", found[0]);
    assert!(
        found[0]
            .fix
            .contains(&format!("box orders pile:id=\"{ORDERS}\""))
            && found[0]
                .fix
                .contains(&format!("ignore \"orders\" id=\"{ORDERS}\" reason=")),
        "it says both fixes: {:?}",
        found[0]
    );
    let built = Built::of(&repository());
    assert_eq!(misses(&diagram(&source), built.graph()), vec!["@orders"]);
    assert!(
        !stops_drawing(&found[0]),
        "a diagram that misses a node is still true"
    );

    // Drawn, or ignored with a reason, it is no finding.
    let drawn = relying(&format!("  box orders pile:id=\"{ORDERS}\"\n"), "");
    assert_eq!(held(&drawn), vec![]);
    let ignored = relying(
        "",
        &format!("      ignore \"orders\" id=\"{ORDERS}\" reason=\"moves out next quarter\"\n"),
    );
    assert_eq!(held(&ignored), vec![]);
    assert!(misses(&diagram(&ignored), built.graph()).is_empty());

    // A node that is not a pile is missed and ignored the same way.
    let source = dependency(&format!(
        "  box web pile:id=\"{WEB}\"\n  box billing pile:id=\"{BILLING}\"\n  meta pile {{\n    \
         boxes {{\n      include \"?- node(Node, external, _).\"\n    }}\n  }}"
    ));
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-misses-node"]);
    assert!(
        found[0].fix.contains(&format!(
            "box git pile:id=\"{TOP}\" pile:node-type=external"
        )) && found[0].fix.contains(&format!(
            "ignore \"git\" id=\"{TOP}\" node-type=external reason="
        )),
        "{:?}",
        found[0]
    );
    let ignored = source.replace(
        "    }\n  }",
        &format!(
            "      ignore \"git\" id=\"{TOP}\" node-type=external reason=\"everything uses it\"\n    \
             }}\n  }}"
        ),
    );
    assert_eq!(held(&ignored), vec![]);
}

/// A diagram of `@web`, `@billing` and `@orders` that should show every
/// `relies-on` link between them, with these lines and this beside the
/// include.
fn linking(lines: &str, beside: &str) -> String {
    dependency(&format!(
        "  box web pile:id=\"{WEB}\"\n  box billing pile:id=\"{BILLING}\"\n  box orders \
         pile:id=\"{ORDERS}\"\n{lines}  meta pile {{\n    lines {{\n      include #\"?- edge(Edge, \
         _, \"relies-on\", _).\"#\n{beside}    }}\n  }}"
    ))
}

#[test]
fn a_link_between_two_drawn_nodes_that_no_line_draws_is_a_problem_at_the_include() {
    let source = linking("  line web billing pile:edge=\"relies-on\"\n", "");
    let found = held(&source);
    assert_eq!(codes(&found), vec!["diagram-misses-link"]);
    assert_eq!(place(&found[0]), at(&source, "include"));
    assert!(
        found[0].message.contains("@orders relies-on @billing"),
        "{:?}",
        found[0]
    );
    assert!(
        found[0]
            .fix
            .contains("line orders billing pile:edge=\"relies-on\""),
        "{:?}",
        found[0]
    );
    let built = Built::of(&repository());
    assert_eq!(
        misses(&diagram(&source), built.graph()),
        vec!["@orders relies-on @billing"]
    );
    assert!(!stops_drawing(&found[0]));

    // A line that joins the two and says nothing of an edge does not draw
    // the link.
    let source = linking(
        "  line web billing pile:edge=\"relies-on\"\n  line orders billing\n",
        "",
    );
    assert_eq!(codes(&held(&source)), vec!["diagram-misses-link"]);

    let both = "  line web billing pile:edge=\"relies-on\"\n  line orders billing \
                pile:edge=\"relies-on\"\n";
    assert_eq!(held(&linking(both, "")), vec![]);
    let ignored = linking(
        "  line web billing pile:edge=\"relies-on\"\n",
        &format!(
            "      ignore \"relies-on\" reason=\"drawn in the orders diagram\" {{\n        from \
             \"orders\" id=\"{ORDERS}\"\n        to \"billing\" id=\"{BILLING}\"\n      }}\n"
        ),
    );
    assert_eq!(held(&ignored), vec![]);

    // The link to external:git has an end the diagram does not draw, so
    // the diagram does not have to show it.
    assert!(
        !codes(&held(&linking(both, "")))
            .iter()
            .any(|c| c.contains("misses"))
    );
}

#[test]
fn an_ignore_says_why_and_one_the_diagram_does_not_need_is_a_note() {
    let no_reason = relying("", &format!("      ignore \"orders\" id=\"{ORDERS}\"\n"));
    let found = held(&no_reason);
    assert_eq!(codes(&found), vec!["diagram-ignore-no-reason"]);
    assert_eq!(place(&found[0]), at(&no_reason, "ignore"));
    let empty = relying(
        "",
        &format!("      ignore \"orders\" id=\"{ORDERS}\" reason=\" \"\n"),
    );
    assert_eq!(codes(&held(&empty)), vec!["diagram-ignore-no-reason"]);

    // It draws what it ignores.
    let drawn = relying(
        &format!("  box orders pile:id=\"{ORDERS}\"\n"),
        &format!("      ignore \"orders\" id=\"{ORDERS}\" reason=\"not yet\"\n"),
    );
    let found = held(&drawn);
    assert_eq!(codes(&found), vec!["diagram-ignore-unneeded"]);
    assert_eq!(found[0].severity, Severity::Note);
    assert_eq!(place(&found[0]), at(&drawn, "ignore"));
    assert!(!stops_drawing(&found[0]));

    // No include finds what it ignores.
    let unfound = relying(
        &format!("  box orders pile:id=\"{ORDERS}\"\n"),
        &format!("      ignore \"top\" id=\"{TOP}\" reason=\"not a service\"\n"),
    );
    let found = held(&unfound);
    assert_eq!(codes(&found), vec!["diagram-ignore-unneeded"]);
    assert!(found[0].message.contains("no include finds @top"));

    // An ignore is held to the graph as a box is.
    let gone = relying(
        &format!("  box orders pile:id=\"{ORDERS}\"\n"),
        &format!("      ignore \"legacy\" id=\"{NOBODY}\" reason=\"removed soon\"\n"),
    );
    assert_eq!(codes(&held(&gone)), vec!["diagram-no-such-pile"]);
    let renamed = relying(
        "",
        &format!("      ignore \"order\" id=\"{ORDERS}\" reason=\"not yet\"\n"),
    );
    assert_eq!(
        codes(&held(&renamed)),
        vec!["diagram-misses-node", "diagram-name-differs"]
    );
    let no_id = relying("", "      ignore \"orders\" reason=\"not yet\"\n");
    assert!(codes(&held(&no_id)).contains(&"diagram-needs-id"));

    // An ignored link a line draws, and one no include finds.
    let both = "  line web billing pile:edge=\"relies-on\"\n  line orders billing \
                pile:edge=\"relies-on\"\n";
    let drawn = linking(
        both,
        &format!(
            "      ignore \"relies-on\" reason=\"elsewhere\" {{\n        from \"orders\" \
             id=\"{ORDERS}\"\n        to \"billing\" id=\"{BILLING}\"\n      }}\n"
        ),
    );
    let found = held(&drawn);
    assert_eq!(codes(&found), vec!["diagram-ignore-unneeded"]);
    assert!(found[0].message.contains("a line draws"));
    let unfound = linking(
        both,
        &format!(
            "      ignore \"tests\" reason=\"elsewhere\" {{\n        from \"orders\" \
             id=\"{ORDERS}\"\n        to \"web\" id=\"{WEB}\"\n      }}\n"
        ),
    );
    assert_eq!(codes(&held(&unfound)), vec!["diagram-ignore-unneeded"]);
    let one_end = linking(
        both,
        &format!(
            "      ignore \"tests\" reason=\"elsewhere\" {{\n        from \"orders\" \
             id=\"{ORDERS}\"\n      }}\n"
        ),
    );
    assert_eq!(codes(&held(&one_end)), vec!["diagram-meta-unknown"]);
}

#[test]
fn without_a_graph_only_what_the_file_alone_shows_is_found() {
    // An id no pile has, a link nothing declares, and a program that is
    // refused.
    let source = dependency(&format!(
        "  box web pile:id=\"{NOBODY}\"\n  box billing pile:id=\"{BILLING}\"\n  line billing web pile:edge=\"relies-on\"\n  meta pile {{\n    boxes \
         {{\n      include \"?- nothing(Node).\"\n      ignore \"orders\" id=\"{ORDERS}\"\n    \
         }}\n  }}"
    ));
    let built = Built::of(&repository());
    assert_eq!(
        codes(&findings(&diagram(&source), None)),
        vec!["diagram-ignore-no-reason"],
        "the ignore with no reason"
    );
    let with = codes(&findings(&diagram(&source), Some(built.graph())));
    assert!(
        with.contains(&"diagram-include-invalid") && with.len() > 3,
        "{with:?}"
    );
}

#[test]
fn a_box_or_a_line_that_stands_for_nothing_stops_drawing_and_a_miss_does_not() {
    let nothing = held(&dependency(&format!(
        "  box web pile:id=\"{NOBODY}\"\n  box other"
    )));
    assert!(stops_drawing(&nothing[0]));
    let no_edge = held(&dependency(&format!(
        "{}\n  line billing web pile:edge=\"relies-on\"",
        boxes()
    )));
    assert!(stops_drawing(&no_edge[0]));
}

fn edge(source: &str, target: Node) -> Edge {
    Edge {
        source: Node::Pile(source.to_string()),
        of_type: "relies-on".to_string(),
        target,
    }
}

const SHOWN: &str = "  line web billing pile:edge=\"relies-on\"\n  line web git \
                     pile:edge=\"relies-on\"\n  line billing git";

#[test]
fn a_change_touches_each_node_and_edge_a_diagram_stands_for() {
    let then = Built::of(&repository());
    let source = dependency(&format!("{}\n{SHOWN}", boxes()));
    let d = diagram(&source);
    let no_piles = BTreeSet::new();

    // A link's title changes: the edge is touched, and no node is.
    let mut m = repository();
    m.write(
        "/web/.pile/links.toml",
        links("charges every order through"),
    );
    let now = Built::of(&m);
    let changed = graph::touched(&then.rows, &now.rows);
    let touched = touched_of(&d, Some(now.graph()), &no_piles, &changed);
    assert_eq!(touched.references, vec!["@web relies-on @billing"]);
    assert!(touched.piles.is_empty() && touched.elements.is_empty());
    assert_eq!(
        touched.lines,
        vec![("web".to_string(), "billing".to_string())]
    );

    // A node's title changes: its box is touched.
    let mut m = repository();
    m.write("/.pile/nodes.toml", NODES.replace("the git binary", "git"));
    let now = Built::of(&m);
    let changed = graph::touched(&then.rows, &now.rows);
    let touched = touched_of(&d, Some(now.graph()), &no_piles, &changed);
    assert_eq!(touched.references, vec!["external:git"]);
    assert!(touched.piles.is_empty(), "a node is not a pile");
    assert_eq!(touched.elements, vec!["git"]);

    // A link is removed: the line that stood for it is touched.
    let mut m = repository();
    m.write(
        "/web/.pile/links.toml",
        "version = \"gitpile/1\"\n\n[[edges]]\ntype = \"relies-on\"\ntarget = \"@billing\"\n\
         title = \"charges through\"\n",
    );
    let now = Built::of(&m);
    let changed = graph::touched(&then.rows, &now.rows);
    assert!(changed.edges.contains(&edge(
        WEB,
        Node::Named {
            of_type: "external".into(),
            name: "git".into()
        }
    )));
    let touched = touched_of(&d, Some(now.graph()), &no_piles, &changed);
    assert_eq!(touched.references, vec!["@web relies-on external:git"]);

    // A pile is touched by its files, which its caller says.
    let by_files = [BILLING.to_string()].into();
    let touched = touched_of(
        &d,
        Some(then.graph()),
        &by_files,
        &graph::Touched::default(),
    );
    assert_eq!(touched.piles, vec!["@billing"]);
    assert_eq!(touched.references, vec!["@billing"]);
    assert_eq!(touched.elements, vec!["billing"]);
}

#[test]
fn a_diagram_that_misses_something_says_so_though_the_change_touched_nothing_it_draws() {
    let built = Built::of(&repository());
    let d = diagram(&relying("", ""));
    let listed = changes(
        std::slice::from_ref(&d),
        std::slice::from_ref(&d),
        Some(built.graph()),
        &BTreeSet::new(),
        &graph::Touched::default(),
    );
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].change, None);
    assert_eq!(listed[0].misses, vec!["@orders"]);
}

#[test]
fn a_touched_line_is_marked_by_its_two_keys() {
    let source = dependency(&format!("{}\n{SHOWN}", boxes()));
    let read = diagram_format::read(&source).unwrap();
    let touched = Touched {
        lines: vec![("web".to_string(), "billing".to_string())],
        ..Touched::default()
    };
    let page = String::from_utf8(
        draw_marked(&read.diagram, &touched, As::Html)
            .unwrap()
            .bytes,
    )
    .unwrap();
    assert!(page.contains("data-changed=\"1\""), "{page}");
    assert!(
        page.contains(&format!("data-pile-id=\"{WEB}\""))
            && page.contains("data-pile-node-type=\"external\"")
            && page.contains("data-pile-edge=\"relies-on\""),
        "the page carries what the file says of each box and line"
    );
}

#[test]
fn what_a_diagram_stands_for_is_a_node_for_each_box_and_an_edge_for_each_line() {
    let built = Built::of(&repository());
    let source = dependency(&format!("{}\n  box other\n{SHOWN}", boxes()));
    let standing = standing(&diagram(&source), Some(built.graph()));
    let keys: Vec<&str> = standing.elements.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["web", "billing", "git"]);
    assert_eq!(
        standing.lines,
        vec![
            Draws {
                from: "web".into(),
                to: "billing".into(),
                edge: edge(WEB, Node::Pile(BILLING.into())),
            },
            Draws {
                from: "web".into(),
                to: "git".into(),
                edge: edge(
                    WEB,
                    Node::Named {
                        of_type: "external".into(),
                        name: "git".into()
                    }
                ),
            },
        ]
    );
}

/// The page of a diagram's two versions, each held to its own graph.
fn compared(then: &Built, before: &str, now: &Built, after: &str) -> (Swapped, String) {
    let swapped = swapped(
        &standing(&diagram(before), Some(then.graph())),
        &standing(&diagram(after), Some(now.graph())),
    );
    let read = |source: &str| diagram_format::read(source).unwrap().diagram;
    let page = draw_diff(&read(before), &read(after), &swapped, As::Html)
        .unwrap()
        .unwrap();
    (swapped, String::from_utf8(page.bytes).unwrap())
}

#[test]
fn an_edited_diagram_that_only_writes_what_it_stands_for_differently_marks_nothing() {
    let built = Built::of(&repository());
    let before = dependency(&format!(
        "  box web\n  box billing pile:id=\"{BILLING}\"\n  box orders \
         pile:id=\"{ORDERS}\"\n  line web billing\n  line orders billing\n  line orders web"
    ));
    let after = dependency(&format!(
        "  box web pile:id=\"{WEB}\"\n  box billing pile:id=\"{BILLING}\" \
         pile:name=\"billing\"\n  box orders pile:id=\"{ORDERS}\"\n  \
         line web billing pile:edge=\"relies-on\"\n  line orders billing pile:edge=\"relies-on\""
    ));
    assert_eq!(held(&after), vec![]);
    let (swapped, page) = compared(&built, &before, &built, &after);
    assert_eq!(
        swapped,
        Swapped::default(),
        "each box stands for the pile it stood for, and a box or a line that gains what it \
         stands for is not marked"
    );
    assert!(
        page.contains("data-added=\"0\"")
            && page.contains("data-removed=\"1\"")
            && page.contains("data-changed=\"0\""),
        "only the line that is gone is marked: {page}"
    );
    assert!(page.contains("REMOVED: orders → web"), "{page}");
    // And back: a box and a line that lose what they stand for.
    let (swapped, _) = compared(&built, &after, &built, &before);
    assert_eq!(swapped, Swapped::default());
}

#[test]
fn a_box_that_stands_for_another_pile_under_its_key_is_marked_with_its_line() {
    let built = Built::of(&repository());
    let drawn = |id: &str, name: &str| {
        dependency(&format!(
            "  box buyer pile:id=\"{id}\" pile:name=\"{name}\"\n  box billing pile:id=\"{BILLING}\"\n  box git\n  \
             line buyer billing pile:edge=\"relies-on\"\n  line billing git"
        ))
    };
    let (before, after) = (drawn(WEB, "web"), drawn(ORDERS, "orders"));
    assert_eq!(held(&after), vec![]);
    let (swapped, page) = compared(&built, &before, &built, &after);
    assert_eq!(swapped.elements, vec!["buyer"]);
    assert_eq!(
        swapped.lines,
        vec![("buyer".to_string(), "billing".to_string())],
        "the line stands for the link from another pile"
    );
    assert!(
        page.contains("data-changed=\"2\"") && page.contains("data-removed=\"0\""),
        "{page}"
    );
    assert!(page.contains("CHANGED: buyer → billing · buyer"), "{page}");
    // Without what the caller found, the two versions are drawn the same.
    let read = |source: &str| diagram_format::read(source).unwrap().diagram;
    let plain = draw_diff(&read(&before), &read(&after), &Swapped::default(), As::Html)
        .unwrap()
        .unwrap();
    assert!(
        String::from_utf8(plain.bytes)
            .unwrap()
            .contains("data-changed=\"0\"")
    );
}

#[test]
fn a_box_whose_node_changes_type_under_the_same_name_is_marked() {
    let then = Built::of(&repository());
    let mut m = repository();
    m.write(
        "/words/.pile/types.toml",
        TYPES.replace(
            "[[edge-types]]\nname = \"relies-on\"",
            "[[node-types]]\nname = \"tool\"\ntitle = \"a program a pile runs\"\n\n\
             [[edge-types]]\nname = \"relies-on\"",
        ),
    );
    m.write(
        "/.pile/nodes.toml",
        format!(
            "{NODES}\n[[nodes]]\ntype = \"tool\"\nname = \"git\"\ntitle = \"git, as a tool\"\n"
        ),
    );
    let now = Built::of(&m);
    let drawn = |of_type: &str| {
        dependency(&format!(
            "  box web pile:id=\"{WEB}\"\n  box git pile:id=\"{TOP}\" pile:node-type={of_type}\n  \
             line web git"
        ))
    };
    let (before, after) = (drawn("external"), drawn("tool"));
    assert_eq!(findings(&diagram(&after), Some(now.graph())), vec![]);
    let (swapped, page) = compared(&then, &before, &now, &after);
    assert_eq!(swapped.elements, vec!["git"]);
    assert_eq!(swapped.lines, vec![]);
    assert!(page.contains("data-changed=\"1\""), "{page}");
    assert!(page.contains("CHANGED: git"), "{page}");
}

fn at_path(path: &str, source: &str) -> Diagram {
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
    let built = Built::of(&repository());
    let kept = dependency(&format!(
        "  box a \"billing\" pile:id=\"{BILLING}\"\n  box b \"web\" pile:id=\"{WEB}\"\n  line a b"
    ));
    let quiet = dependency(&format!(
        "  box a \"web\" pile:id=\"{WEB}\"\n  box b\n  line a b"
    ));
    let then = vec![
        at_path(
            "/.pile/diagrams/edited.diagram",
            &dependency("  box a\n  box b\n  line a b"),
        ),
        at_path("/.pile/diagrams/gone.diagram", &kept),
        at_path("/.pile/diagrams/piles.diagram", &kept),
        at_path("/.pile/diagrams/quiet.diagram", &quiet),
    ];
    let now = vec![
        at_path("/.pile/diagrams/added.diagram", &kept),
        at_path(
            "/.pile/diagrams/edited.diagram",
            &dependency("  box a\n  box c\n  line a c"),
        ),
        at_path("/.pile/diagrams/piles.diagram", &kept),
        at_path("/.pile/diagrams/quiet.diagram", &quiet),
    ];
    let touched = [BILLING.to_string()].into();
    let found = changes(
        &then,
        &now,
        Some(built.graph()),
        &touched,
        &graph::Touched::default(),
    );
    let words: Vec<(&str, Option<&str>)> = found
        .iter()
        .map(|l| (l.diagram.name.as_str(), l.change.as_ref().map(Change::word)))
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
    let piles = found.iter().find(|l| l.diagram.name == "piles").unwrap();
    let touched = piles.change.as_ref().and_then(Change::touched).unwrap();
    assert_eq!(touched.piles, vec!["@billing"]);
    assert_eq!(touched.references, vec!["@billing"]);
    assert_eq!(touched.elements, vec!["a"]);
}

#[test]
fn a_touched_pile_is_named_once_in_the_order_the_file_draws_it() {
    let built = Built::of(&repository());
    let d = diagram(&dependency(&format!(
        "  box a \"web\" pile:id=\"{WEB}\"\n  box b \"billing\" pile:id=\"{BILLING}\"\n  \
         box c \"web\" pile:id=\"{WEB}\"\n  line a b"
    )));
    let all = [BILLING.to_string(), WEB.to_string()].into();
    let none = graph::Touched::default();
    let touched = touched_of(&d, Some(built.graph()), &all, &none);
    assert_eq!(touched.piles, vec!["@web", "@billing"]);
    assert_eq!(touched.references, vec!["@web", "@billing"]);
    assert_eq!(touched.elements, vec!["a", "b", "c"]);
    assert!(touched_of(&diagram("not kdl {"), Some(built.graph()), &all, &none).is_empty());
    // Without a graph a diagram stands for nothing, so nothing is touched.
    assert!(touched_of(&d, None, &all, &none).is_empty());
}
