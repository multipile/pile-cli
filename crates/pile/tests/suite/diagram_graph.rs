//! A diagram held to the graph: a box stands for a node and a line for a
//! link, a diagram says what it should show, `pile doctor` reports what it
//! does not, and `pile diagram list --since` says what a change touched of
//! what it stands for (pile guide diagrams).

use serde_json::{Value, json};

use crate::harness::*;

const TYPES: &str = r#"version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository that a pile relies on"

[[edge-types]]
name = "relies-on"
title = "one cannot work without the other"
source = ["pile"]
target = ["pile", "external"]
"#;

/// A links file of `relies-on` links to these targets, each with a title.
fn relies_on(targets: &[(&str, &str)]) -> String {
    let mut text = "version = \"gitpile/1\"\n".to_string();
    for (target, title) in targets {
        text.push_str(&format!(
            "\n[[edges]]\ntype = \"relies-on\"\ntarget = \"{target}\"\ntitle = \"{title}\"\n"
        ));
    }
    text
}

/// A pile at the top, named after its directory, with an intent.
fn pile(r: &Repo, name: &str) {
    r.declare(name, name, name);
    r.intent(name, &format!("The {name}.\n"));
}

/// `/` (@top), which declares `external:git`; the aux pile `/aux/words`,
/// which creates `external` and `relies-on`; and `@posts`, `@auth`, `@ui`,
/// `@shared` and `@db`. `@posts` relies on `@db` and on `@shared`, and
/// `@auth` and `@ui` on `@shared`. The top keeps two diagrams: `services`
/// draws `@posts`, `@db` and the link between them, and `shared` what
/// relies on `@shared`, which it says it should show. Committed, on main.
fn graphed() -> Repo {
    let r = Repo::new();
    r.declare(".", "a", "top");
    r.intent(".", "The whole repository.\n");
    r.write(
        "aux/words/.pile/config",
        &format!(
            "kind = auxiliary-gitpile\nversion = 1\nid = {}\nname = words\n",
            id_of("b")
        ),
    );
    r.intent("aux/words", "The words this repository uses.\n");
    r.write("aux/words/.pile/types.toml", TYPES);
    r.write(
        ".pile/nodes.toml",
        "version = \"gitpile/1\"\n\n[[nodes]]\ntype = \"external\"\nname = \"git\"\ntitle = \"the git binary\"\n",
    );
    for name in ["posts", "auth", "ui", "shared", "db"] {
        pile(&r, name);
    }
    r.write(
        "posts/.pile/links.toml",
        &relies_on(&[
            ("@db", "keeps every post in"),
            ("@shared", "takes its types from"),
        ]),
    );
    r.write(
        "auth/.pile/links.toml",
        &relies_on(&[("@shared", "takes its types from")]),
    );
    r.write(
        "ui/.pile/links.toml",
        &relies_on(&[("@shared", "takes its types from")]),
    );
    r.write(".pile/diagrams/services.diagram", &services());
    r.write(".pile/diagrams/shared.diagram", &shared(""));
    r.commit("declare");
    r
}

fn services() -> String {
    format!(
        r#"diagram dependency version=1 {{
  title "Where the posts are kept"
  desc "The posts service keeps every post in the database."
  box posts pile:id="{}"
  box db pile:id="{}" as=store
  line posts db pile:edge="relies-on"
}}
"#,
        id_of("posts"),
        id_of("db")
    )
}

/// The diagram of what relies on `@shared`, with this beside its include.
fn shared(beside: &str) -> String {
    format!(
        r##"diagram dependency version=1 {{
  title "What uses the shared types"
  desc "The pages and both services take their types from the shared package."
  box ui pile:id="{}"
  box posts pile:id="{}"
  box auth pile:id="{}"
  box shared pile:id="{}" focal=#true
  line ui shared pile:edge="relies-on"
  line posts shared pile:edge="relies-on"
  line auth shared pile:edge="relies-on"
  meta pile {{
    boxes {{
      include #"?- edge(_, Node, "relies-on", @shared)."#
{beside}    }}
    lines {{
      include #"?- edge(Edge, _, "relies-on", _)."#
    }}
  }}
}}
"##,
        id_of("ui"),
        id_of("posts"),
        id_of("auth"),
        id_of("shared"),
    )
}

/// A branch that adds `@comments`, which relies on `@shared`.
fn with_comments(r: &Repo) {
    r.git(&["switch", "-q", "-c", "feature"]);
    pile(r, "comments");
    r.write(
        "comments/.pile/links.toml",
        &relies_on(&[("@shared", "takes its types from")]),
    );
    r.commit("comments");
}

/// The findings of one diagram, as `pile doctor --diagrams` gives them,
/// whether doctor passed or not: each one's code, line and column.
fn found(r: &Repo, name: &str) -> Vec<(String, u64, u64)> {
    let a = r.answered(&[], &["doctor", "--diagrams", "--format", "json"]);
    let v = a.json();
    let d = v["result"]["diagrams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == name)
        .unwrap_or_else(|| panic!("no diagram {name}: {v}"))
        .clone();
    ["problems", "notes"]
        .iter()
        .flat_map(|list| d[list].as_array().unwrap().clone())
        .map(|f| {
            (
                f["code"].as_str().unwrap().to_string(),
                f["line"].as_u64().unwrap(),
                f["column"].as_u64().unwrap(),
            )
        })
        .collect()
}

/// One diagram of `pile diagram list --since`.
fn listed(r: &Repo, since: &str, name: &str) -> Value {
    r.result(&["diagram", "list", "--since", since])["diagrams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == name)
        .unwrap()
        .clone()
}

#[test]
fn diagrams_that_stand_for_what_the_graph_holds_have_nothing_wrong() {
    let r = graphed();
    let v = r.result(&["doctor"]);
    assert_eq!(v["total"]["problems"], 0, "{v}");
    let v = r.result(&["doctor", "--diagrams"]);
    assert_eq!(
        v["total"],
        json!({"diagrams": 2, "needing_attention": 0, "problems": 0, "notes": 0})
    );
}

#[test]
fn a_changed_link_marks_its_line() {
    let r = graphed();
    r.git(&["switch", "-q", "-c", "feature"]);
    r.write(
        "posts/.pile/links.toml",
        &relies_on(&[
            ("@db", "keeps every post and its drafts in"),
            ("@shared", "takes its types from"),
        ]),
    );
    r.commit("the link says more");

    let d = listed(&r, "main...", "services");
    assert_eq!(d["change"], "piles", "{d}");
    assert_eq!(
        d["touched"],
        json!(["@posts", "@posts relies-on @db"]),
        "the pile whose file changed, and the link whose title did"
    );
    assert_eq!(d["piles"], json!(["@posts"]), "piles keeps the piles alone");
    assert_eq!(d["misses"], json!([]));
    contains!(
        r.stdout(&["diagram", "list", "--since", "main...", "--format", "text"]),
        "change    piles @posts, @posts relies-on @db"
    );

    let v = r.result(&["diagram", "render", "--since", "main...", "services"]);
    assert_eq!(v["change"], "piles");
    assert_eq!(v["touched"], json!(["@posts", "@posts relies-on @db"]));
    let page = r.read("services.html");
    assert!(
        page.contains("data-changed=\"2\""),
        "the box of the pile and the line of the link are marked"
    );
    assert!(page.contains("data-pile-edge=\"relies-on\""));
    let e = r.refused(
        &[
            "diagram", "render", "--since", "main...", "--as", "gif", "services",
        ],
        2,
    );
    assert_eq!(e["error"]["code"], "diagram-still");
}

#[test]
fn a_link_written_in_another_pile_marks_the_line_alone() {
    let r = graphed();
    let both_ends = |title: &str| {
        format!(
            "version = \"gitpile/1\"\n\n[[edges]]\ntype = \"relies-on\"\nsource = \"@auth\"\n\
             target = \"@db\"\ntitle = \"{title}\"\n"
        )
    };
    r.write(".pile/links.toml", &both_ends("keeps every session in"));
    r.write(
        ".pile/diagrams/sessions.diagram",
        &services()
            .replace("posts", "auth")
            .replace(&id_of("posts"), &id_of("auth")),
    );
    r.commit("sessions");
    assert_eq!(found(&r, "sessions"), vec![]);
    r.git(&["switch", "-q", "-c", "feature"]);
    r.write(
        ".pile/links.toml",
        &both_ends("keeps every session and its keys in"),
    );
    r.commit("the link says more");

    let d = listed(&r, "main...", "sessions");
    assert_eq!(
        d["change"], "piles",
        "the word stays, though no pile it draws was touched: {d}"
    );
    assert_eq!(d["piles"], json!([]));
    assert_eq!(d["touched"], json!(["@auth relies-on @db"]));
    r.result(&["diagram", "render", "--since", "main...", "sessions"]);
    assert!(r.read("sessions.html").contains("data-changed=\"1\""));

    // Where a link is written is never a change.
    r.write(
        "auth/.pile/links.toml",
        &relies_on(&[
            ("@shared", "takes its types from"),
            ("@db", "keeps every session and its keys in"),
        ]),
    );
    r.write(".pile/links.toml", "version = \"gitpile/1\"\n");
    r.commit("the link moves to the pile it starts at");
    let d = listed(&r, "HEAD~1", "sessions");
    assert_eq!(
        d["touched"],
        json!(["@auth"]),
        "the pile, by its files: {d}"
    );
}

#[test]
fn a_new_pile_the_diagram_should_show_is_missed_until_it_is_drawn() {
    let r = graphed();
    with_comments(&r);

    let e = r.refused(&["doctor"], 1);
    assert_eq!(e["error"]["code"], "problems-found");
    assert_eq!(
        found(&r, "shared"),
        vec![("diagram-misses-node".to_string(), 13, 7)],
        "at the include"
    );
    let v = &e["result"];
    let f = v["piles"][0]["problems"][0].clone();
    assert_eq!(f["code"], "diagram-misses-node");
    assert_eq!(f["file"], ":/.pile/diagrams/shared.diagram");
    assert!(f["message"].as_str().unwrap().contains("@comments"), "{f}");
    let fix = f["fix"].as_str().unwrap();
    assert!(
        fix.contains(&format!("box comments pile:id=\"{}\"", id_of("comments")))
            && fix.contains("ignore \"comments\""),
        "draw it, or ignore it with a reason: {fix}"
    );

    // The change touched nothing the diagram draws, and it is listed with
    // what it misses.
    let d = listed(&r, "main...", "shared");
    assert_eq!(d["change"], Value::Null, "{d}");
    assert_eq!(d["touched"], json!([]));
    assert_eq!(d["misses"], json!(["@comments"]));
    contains!(
        r.stdout(&["diagram", "list", "--since", "main...", "--format", "text"]),
        "misses    @comments"
    );
    // A miss does not stop drawing.
    r.result(&["diagram", "render", "shared"]);

    // The author draws it: the diagram is edited, and drawn before and
    // after.
    r.write(
        ".pile/diagrams/shared.diagram",
        &shared("").replace(
            "  line ui shared",
            &format!(
                "  box comments pile:id=\"{}\"\n  line comments shared pile:edge=\"relies-on\"\n  line ui shared",
                id_of("comments")
            ),
        ),
    );
    r.result(&["doctor"]);
    let d = listed(&r, "main...", "shared");
    assert_eq!(d["change"], "edited", "{d}");
    assert_eq!(d["misses"], json!([]));
    let v = r.result(&["diagram", "render", "--since", "main...", "shared"]);
    assert_eq!(v["change"], "edited");
    assert!(r.read("shared.html").contains("data-added=\"2\""));
}

#[test]
fn a_node_left_out_on_purpose_says_why_and_the_ignore_does_not_outlive_its_reason() {
    let r = graphed();
    with_comments(&r);
    let ignore = |rest: &str| {
        shared(&format!(
            "      ignore \"comments\" id=\"{}\"{rest}\n",
            id_of("comments")
        ))
    };
    r.write(
        ".pile/diagrams/shared.diagram",
        &ignore(" reason=\"removed in the next release\""),
    );
    let v = r.result(&["doctor"]);
    assert_eq!(v["total"]["problems"], 0, "{v}");
    assert_eq!(v["total"]["notes"], 0, "{v}");
    assert_eq!(listed(&r, "main...", "shared")["misses"], json!([]));

    // With no reason, it is a problem.
    r.write(".pile/diagrams/shared.diagram", &ignore(""));
    assert_eq!(
        found(&r, "shared"),
        vec![("diagram-ignore-no-reason".to_string(), 14, 7)]
    );

    // The pile no longer relies on @shared: the ignore is not needed, which
    // is a note, and doctor still exits 0.
    r.write(
        ".pile/diagrams/shared.diagram",
        &ignore(" reason=\"removed in the next release\""),
    );
    r.write("comments/.pile/links.toml", "version = \"gitpile/1\"\n");
    let v = r.result(&["doctor"]);
    assert_eq!(v["total"]["problems"], 0, "{v}");
    assert_eq!(v["piles"][0]["notes"][0]["code"], "diagram-ignore-unneeded");
    assert_eq!(
        found(&r, "shared"),
        vec![("diagram-ignore-unneeded".to_string(), 14, 7)]
    );
    let v = r.result(&["doctor", "--diagrams"]);
    assert_eq!(v["total"]["notes"], 1);
    contains!(
        r.stdout(&["doctor", "--diagrams", "--format", "text"]),
        "2 diagrams, nothing wrong: 1 note"
    );

    // The pile is gone: its id is no pile's, as for a box.
    std::fs::remove_dir_all(r.path().join("comments")).unwrap();
    assert_eq!(
        found(&r, "shared"),
        vec![("diagram-no-such-pile".to_string(), 14, 7)]
    );
}

#[test]
fn a_line_no_link_declares_and_a_link_no_line_draws() {
    let r = graphed();
    // A link between two drawn piles that no line draws.
    r.write(
        ".pile/diagrams/shared.diagram",
        &shared("").replace("  line auth shared pile:edge=\"relies-on\"\n", ""),
    );
    assert_eq!(
        found(&r, "shared"),
        vec![("diagram-misses-link".to_string(), 15, 7)],
        "at the include under lines"
    );
    assert_eq!(
        listed(&r, "HEAD", "shared")["misses"],
        json!(["@auth relies-on @shared"])
    );
    r.result(&["diagram", "render", "shared"]);

    // A line that says it stands for a link nothing declares.
    r.write(
        ".pile/diagrams/shared.diagram",
        &shared("").replace(
            "  line auth shared pile:edge=\"relies-on\"\n",
            "  line auth shared pile:edge=\"relies-on\"\n  line ui auth pile:edge=\"relies-on\"\n",
        ),
    );
    assert_eq!(
        found(&r, "shared"),
        vec![("diagram-no-such-edge".to_string(), 11, 3)],
        "at the line"
    );
    let e = r.refused(&["diagram", "render", "shared"], 1);
    assert_eq!(e["error"]["code"], "diagram-invalid");
    assert!(
        e["error"]["message"]
            .as_str()
            .unwrap()
            .contains("from @ui to @auth"),
        "a line that stands for nothing stops drawing: {e}"
    );
}

#[test]
fn a_renamed_pile_is_a_problem_at_its_box_that_says_the_name() {
    let r = graphed();
    r.write(
        "posts/.pile/config",
        &format!(
            "kind = gitpile\nversion = 1\nid = {}\nname = articles\n",
            id_of("posts")
        ),
    );
    assert_eq!(
        found(&r, "services"),
        vec![("diagram-name-differs".to_string(), 4, 3)]
    );
    let e = r.refused(&["doctor", "--diagrams"], 1);
    let f = e["result"]["diagrams"][0]["problems"][0].clone();
    assert!(
        f["message"].as_str().unwrap().contains("named articles"),
        "{f}"
    );
    let e = r.refused(&["diagram", "render", "services"], 1);
    assert_eq!(e["error"]["code"], "diagram-invalid");

    // The fix is the box's name, and nothing else in the file.
    r.write(
        ".pile/diagrams/services.diagram",
        &services().replace("box posts pile:id", "box posts \"articles\" pile:id"),
    );
    assert_eq!(found(&r, "services"), vec![]);
}

#[test]
fn an_include_that_finds_nothing_is_a_problem() {
    let r = graphed();
    r.write(
        ".pile/diagrams/shared.diagram",
        &shared("      include \"?- prop(Node, labels, servce).\"\n"),
    );
    assert_eq!(
        found(&r, "shared"),
        vec![("diagram-include-empty".to_string(), 14, 7)]
    );
}

#[test]
fn a_refused_program_is_a_problem_at_its_line_and_column_in_the_diagram() {
    let r = graphed();
    r.write(
        ".pile/diagrams/shared.diagram",
        &shared(
            "      include #\"\"\"\n        near(N) :- edge(_, N, \"relies-on\", @shared).\n        \
             ?- near(Node), !edge(_, Node, \"relies-on\", @legacy).\n        \"\"\"#\n",
        ),
    );
    assert_eq!(
        found(&r, "shared"),
        vec![("diagram-include-invalid".to_string(), 16, 52)],
        "at @legacy, on the second line of the string"
    );
    let e = r.refused(&["doctor", "--diagrams"], 1);
    let f = e["result"]["diagrams"][1]["problems"][0].clone();
    assert!(
        f["message"]
            .as_str()
            .unwrap()
            .contains("no pile is named @legacy"),
        "{f}"
    );
}

#[test]
fn a_node_that_is_not_a_pile_is_drawn_by_its_type_and_its_home() {
    let r = graphed();
    let with_git = |id: &str| {
        services().replace(
            "  line posts db",
            &format!(
                "  box git pile:id=\"{id}\" pile:node-type=external\n  line db git\n  line posts db"
            ),
        )
    };
    r.write(".pile/diagrams/services.diagram", &with_git(&id_of("a")));
    r.commit("draw git");
    assert_eq!(found(&r, "services"), vec![]);

    // What the node declares changes, and its box is touched.
    r.write(
        ".pile/nodes.toml",
        "version = \"gitpile/1\"\n\n[[nodes]]\ntype = \"external\"\nname = \"git\"\ntitle = \"git, 2.40 or later\"\n",
    );
    let d = listed(&r, "HEAD", "services");
    assert_eq!(d["change"], "piles", "{d}");
    assert_eq!(d["touched"], json!(["external:git"]));
    assert_eq!(d["piles"], json!([]));
    r.git(&["checkout", "-q", ".pile/nodes.toml"]);

    // The id of another pile than its home.
    r.write(
        ".pile/diagrams/services.diagram",
        &with_git(&id_of("posts")),
    );
    assert_eq!(
        found(&r, "services"),
        vec![("diagram-no-such-node".to_string(), 6, 3)]
    );
    let e = r.refused(&["doctor", "--diagrams"], 1);
    let f = e["result"]["diagrams"][0]["problems"][0].clone();
    assert_eq!(f["fix"], format!("write pile:id=\"{}\"", id_of("a")));
}

#[test]
fn a_word_pile_does_not_read_and_a_box_with_no_id_are_problems() {
    let r = graphed();
    r.write(
        ".pile/diagrams/services.diagram",
        &services()
            .replace("as=store", "as=store pile:nmae=\"db\"")
            .replace(
                "  line posts db",
                "  box git pile:node-type=external\n  line posts db",
            ),
    );
    assert_eq!(
        found(&r, "services"),
        vec![
            ("diagram-meta-unknown".to_string(), 5, 62),
            ("diagram-needs-id".to_string(), 6, 11),
        ]
    );
    let e = r.refused(&["doctor", "--diagrams"], 1);
    assert_eq!(
        e["result"]["diagrams"][0]["problems"][0]["fix"],
        "did you mean pile:name?"
    );
}

#[test]
fn while_a_config_does_not_read_a_diagram_is_held_to_its_file_alone() {
    let r = graphed();
    // The pile the box stands for is the one whose config does not read:
    // held to the graph, the box would be reported as standing for nothing.
    r.write(
        "db/.pile/config",
        "kind = gitpile\nversion = 1\nid = nonsense\n",
    );
    r.write(
        ".pile/diagrams/services.diagram",
        &services().replace("as=store", "as=store pile:nmae=\"db\""),
    );
    let e = r.refused(&["doctor"], 1);
    let codes: Vec<String> = e["result"]["piles"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["problems"].as_array().unwrap().clone())
        .map(|f| f["code"].as_str().unwrap().to_string())
        .collect();
    assert!(
        codes.contains(&"unreadable-config".to_string()),
        "{codes:?}"
    );
    assert!(
        codes.contains(&"diagram-meta-unknown".to_string()),
        "what the file alone shows is still found: {codes:?}"
    );
    assert!(
        !codes
            .iter()
            .any(|c| c == "diagram-no-such-pile" || c == "diagram-misses-node"),
        "{codes:?}"
    );

    // With the config read again, the graph is, and the diagram is held to it.
    r.git(&["checkout", "-q", "db/.pile/config"]);
    r.write(
        ".pile/diagrams/services.diagram",
        &services().replace(&id_of("db"), &id_of("nobody")),
    );
    assert_eq!(
        found(&r, "services"),
        vec![("diagram-no-such-pile".to_string(), 5, 3)]
    );
    let e = r.refused(&["doctor", "--diagrams"], 1);
    let fix = e["result"]["diagrams"][0]["problems"][0]["fix"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        fix.contains(&format!("pile:id=\"{}\"", id_of("db"))),
        "the box's name is a pile's, so the finding says that pile's id: {fix}"
    );
}

#[test]
fn a_diagram_written_with_pile_checks_lists_and_draws_as_it_did() {
    let r = graphed();
    r.write(
        ".pile/diagrams/old.diagram",
        &format!(
            "diagram dependency version=1 {{\n  title \"Old\"\n  desc \"Written before metadata.\"\n  \
             box posts pile=\"@posts\" id=\"{}\"\n  box db pile=\":/db\"\n  line posts db\n}}\n",
            id_of("posts")
        ),
    );
    r.write("db/schema.sql", "one\n");
    r.commit("an old diagram");
    assert_eq!(found(&r, "old"), vec![]);
    r.write("db/schema.sql", "two\n");
    let d = listed(&r, "HEAD", "old");
    assert_eq!(d["change"], "piles");
    assert_eq!(d["piles"], json!([":/db"]), "as the file names it");
    assert_eq!(d["touched"], json!(["@db"]), "as pile query writes it");
    assert_eq!(d["misses"], json!([]));
    r.result(&["diagram", "render", "--since", "HEAD", "old"]);
    let page = r.read("old.html");
    assert!(
        page.contains("data-changed=\"1\""),
        "the box of @db is marked"
    );
    assert!(
        page.contains("data-pile-ref=\"@posts\"") && !page.contains("data-pile=\""),
        "the page has data-pile-ref where it had data-pile"
    );
}

/// What a review's comment is built from: a diagram of two boxes, a line
/// that stands for the link between them, and an include; a change that
/// edits the link's title and adds a pile the include finds.
#[test]
fn a_review_gets_the_touched_link_and_the_missed_pile() {
    let r = Repo::new();
    r.declare(".", "a", "top");
    r.intent(".", "The whole repository.\n");
    r.write(
        "aux/words/.pile/config",
        &format!(
            "kind = auxiliary-gitpile\nversion = 1\nid = {}\nname = words\n",
            id_of("b")
        ),
    );
    r.intent("aux/words", "The words this repository uses.\n");
    r.write("aux/words/.pile/types.toml", TYPES);
    pile(&r, "web");
    pile(&r, "billing");
    r.write(
        "web/.pile/links.toml",
        &relies_on(&[("@billing", "charges through")]),
    );
    r.write(
        ".pile/diagrams/billing.diagram",
        &format!(
            r##"diagram dependency version=1 {{
  title "What relies on billing"
  desc "The web application charges through billing."
  box web pile:id="{}"
  box billing pile:id="{}"
  line web billing pile:edge="relies-on"
  meta pile {{
    boxes {{
      include #"?- edge(_, Node, "relies-on", @billing)."#
    }}
  }}
}}
"##,
            id_of("web"),
            id_of("billing")
        ),
    );
    r.commit("declare");
    r.git(&["switch", "-q", "-c", "feature"]);
    r.write(
        "web/.pile/links.toml",
        &relies_on(&[("@billing", "charges every order through")]),
    );
    pile(&r, "orders");
    r.write(
        "orders/.pile/links.toml",
        &relies_on(&[("@billing", "bills each order through")]),
    );
    r.commit("orders");

    let d = listed(&r, "main...", "billing");
    assert_eq!(d["change"], "piles", "{d}");
    assert!(
        d["touched"]
            .as_array()
            .unwrap()
            .contains(&json!("@web relies-on @billing")),
        "{d}"
    );
    assert_eq!(d["misses"], json!(["@orders"]));
}

/// What `pile diagram render --since main...` marks in an edited diagram:
/// how many elements and lines are added, removed and changed.
fn marked(r: &Repo, name: &str) -> (String, [bool; 3]) {
    let v = r.result(&["diagram", "render", "--since", "main...", name]);
    assert_eq!(v["change"], "edited", "{v}");
    let page = r.read(&format!("{name}.html"));
    let none =
        ["added", "removed", "changed"].map(|what| page.contains(&format!("data-{what}=\"0\"")));
    (page, none)
}

#[test]
fn an_edited_diagram_rewritten_from_pile_to_pile_id_marks_only_what_is_drawn_differently() {
    let r = graphed();
    r.write(
        ".pile/diagrams/users.diagram",
        "diagram dependency version=1 {\n  title \"What uses the shared types\"\n  \
         desc \"The pages and both services take their types from the shared package.\"\n  \
         box ui pile=\"@ui\"\n  box posts pile=\"@posts\"\n  box auth pile=\"@auth\"\n  \
         box shared pile=\"@shared\" focal=#true\n  line ui shared\n  line posts shared\n  \
         line auth shared\n  line ui auth\n}\n",
    );
    r.commit("a diagram written before metadata");
    r.git(&["switch", "-q", "-c", "feature"]);
    r.write(
        ".pile/diagrams/users.diagram",
        &format!(
            "diagram dependency version=1 {{\n  title \"What uses the shared types\"\n  \
             desc \"The pages and both services take their types from the shared package.\"\n  \
             box ui pile:id=\"{}\"\n  box posts pile:id=\"{}\"\n  box auth pile:id=\"{}\"\n  \
             box shared pile:id=\"{}\" focal=#true\n  line ui shared pile:edge=\"relies-on\"\n  \
             line posts shared pile:edge=\"relies-on\"\n  line auth shared pile:edge=\"relies-on\"\n}}\n",
            id_of("ui"),
            id_of("posts"),
            id_of("auth"),
            id_of("shared"),
        ),
    );
    r.commit("write it as metadata, without the line that stood for nothing");
    assert_eq!(found(&r, "users"), vec![]);

    let (page, none) = marked(&r, "users");
    assert_eq!(
        none,
        [true, false, true],
        "nothing is added or changed: each box stands for the pile it stood for"
    );
    assert!(page.contains("data-removed=\"1\""), "the line that is gone");
    assert!(page.contains("REMOVED: ui → auth"), "{page}");
    assert!(!page.contains("data-id=\"badge:changed\""));
    // The same as a picture, and moving.
    r.result(&[
        "diagram", "render", "--since", "main...", "--as", "png", "users",
    ]);
    let v = r.result(&[
        "diagram", "render", "--since", "main...", "--as", "gif", "users",
    ]);
    assert!(v["duration_ms"].as_u64().unwrap() > 0);
}

#[test]
fn an_edited_diagram_marks_a_box_that_stands_for_another_pile_under_its_key() {
    let r = graphed();
    r.git(&["switch", "-q", "-c", "feature"]);
    // The box keeps its key, and so what it shows, and stands for @shared.
    r.write(
        ".pile/diagrams/services.diagram",
        &services().replace(
            &format!("\"{}\"", id_of("db")),
            &format!("\"{}\" pile:name=\"shared\"", id_of("shared")),
        ),
    );
    r.commit("the store is the shared package");
    assert_eq!(found(&r, "services"), vec![]);
    let (page, none) = marked(&r, "services");
    assert_eq!(none, [true, true, false]);
    assert!(
        page.contains("data-changed=\"2\"") && page.contains("CHANGED: posts → db · db"),
        "the box, and the line that stands for the link to another pile: {page}"
    );
    for as_ in ["png", "gif"] {
        r.result(&[
            "diagram", "render", "--since", "main...", "--as", as_, "services",
        ]);
    }

    // A line that stands for nothing in either version is not marked.
    r.git(&["switch", "-q", "main"]);
    r.write(
        ".pile/diagrams/services.diagram",
        &services().replace(" pile:edge=\"relies-on\"", ""),
    );
    r.commit("the line stands for nothing");
    r.git(&["switch", "-q", "-c", "other"]);
    r.write(
        ".pile/diagrams/services.diagram",
        &services().replace(" pile:edge=\"relies-on\"", "").replace(
            &format!("\"{}\"", id_of("db")),
            &format!("\"{}\" pile:name=\"shared\"", id_of("shared")),
        ),
    );
    r.commit("the store is the shared package");
    let (page, _) = marked(&r, "services");
    assert!(
        page.contains("data-changed=\"1\"") && page.contains("CHANGED: db"),
        "{page}"
    );
}

#[test]
fn an_edited_diagram_marks_a_box_whose_node_changes_type_under_the_same_name() {
    let r = graphed();
    let with_git = |of_type: &str| {
        services().replace(
            "  line posts db",
            &format!(
                "  box git pile:id=\"{}\" pile:node-type={of_type}\n  line db git\n  line posts db",
                id_of("a")
            ),
        )
    };
    r.write(".pile/diagrams/services.diagram", &with_git("external"));
    r.commit("draw git");
    r.git(&["switch", "-q", "-c", "feature"]);
    r.write(
        "aux/words/.pile/types.toml",
        &format!("{TYPES}\n[[node-types]]\nname = \"tool\"\ntitle = \"a program a pile runs\"\n"),
    );
    r.write(
        ".pile/nodes.toml",
        "version = \"gitpile/1\"\n\n[[nodes]]\ntype = \"external\"\nname = \"git\"\n\
         title = \"the git binary\"\n\n[[nodes]]\ntype = \"tool\"\nname = \"git\"\n\
         title = \"git, as a tool\"\n",
    );
    r.write(".pile/diagrams/services.diagram", &with_git("tool"));
    r.commit("git is a tool");
    assert_eq!(found(&r, "services"), vec![]);
    let (page, none) = marked(&r, "services");
    assert_eq!(none, [true, true, false]);
    assert!(
        page.contains("data-changed=\"1\"") && page.contains("CHANGED: git"),
        "{page}"
    );
}

#[test]
fn an_edited_diagram_whose_earlier_declarations_do_not_read_is_compared_as_drawn() {
    let r = graphed();
    r.write(
        "db/.pile/config",
        "kind = gitpile\nversion = 1\nid = nonsense\n",
    );
    r.commit("a config that does not read");
    r.git(&["switch", "-q", "-c", "feature"]);
    r.declare("db", "db", "db");
    r.write(
        ".pile/diagrams/services.diagram",
        &services().replace(
            &format!("\"{}\"", id_of("db")),
            &format!("\"{}\" pile:name=\"shared\"", id_of("shared")),
        ),
    );
    r.commit("the config reads, and the store is the shared package");
    let (_, none) = marked(&r, "services");
    assert_eq!(
        none,
        [true, true, true],
        "nothing says what the first version stood for, and it is drawn the same"
    );
}
