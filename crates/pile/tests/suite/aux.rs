//! Aux piles, and what the `.toml` files of a `.pile/` declare: `init` makes
//! an aux pile, `show` prints what a pile gives, creates and holds, and
//! `doctor` reports each mistake at its line and column.

use serde_json::{Value, json};

use crate::harness::*;

const TYPES: &str = r#"version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository that a pile relies on"

[[edge-types]]
name = "uses"
title = "one pile calls the code of another"
source = ["pile"]
target = ["pile", "external"]
do = "say what the source calls"
dont = "list functions"
"#;

/// `/` (@top), `/lib` (@lib) and the aux pile `/aux/words` (@words), which
/// creates `external` and `uses`. /lib gives its labels, and the top
/// declares an external. Committed.
fn with_aux() -> Repo {
    let r = Repo::new();
    r.declare(".", "a", "top");
    r.intent(".", "The whole repository.\n");
    r.declare("lib", "b", "lib");
    r.intent("lib", "The parser.\n");
    std::fs::create_dir_all(r.path().join("aux/words")).unwrap();
    r.answered_in(
        "aux/words",
        &[],
        &[
            "init",
            "--set",
            "name=words",
            "--set",
            "kind=auxiliary-gitpile",
        ],
    );
    r.intent("aux/words", "The words this repository uses.\n");
    r.write("aux/words/.pile/types.toml", TYPES);
    r.write(
        "lib/.pile/pile.toml",
        "version = \"gitpile/1\"\n\n[pile]\nlabels = [\"core\", \"docs\"]\n",
    );
    r.write(
        ".pile/nodes.toml",
        "version = \"gitpile/1\"\n\n[[nodes]]\ntype = \"external\"\nname = \"git\"\n\
         title = \"the git binary every read runs\"\nsite = \"git-scm.com\"\n",
    );
    r.commit("an aux pile");
    r
}

/// Each finding of the report as its code and where it is, `:/path:line:column`.
pub fn located(v: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for row in v["result"]["piles"].as_array().unwrap() {
        for sev in ["problems", "warnings", "notes"] {
            for f in row[sev].as_array().unwrap() {
                let mut at = f["file"].as_str().unwrap().to_string();
                for part in ["line", "column"] {
                    if let Some(n) = f[part].as_u64() {
                        at.push_str(&format!(":{n}"));
                    }
                }
                out.push(format!("{} {at}", f["code"].as_str().unwrap()));
            }
        }
    }
    out
}

#[test]
fn init_with_kind_auxiliary_gitpile_declares_an_aux_pile() {
    let r = Repo::new();
    std::fs::create_dir_all(r.path().join("words")).unwrap();
    let out = r
        .answered_in(
            "words",
            &[],
            &[
                "init",
                "--set",
                "kind=auxiliary-gitpile",
                "--set",
                "name=words",
            ],
        )
        .stdout;
    contains!(
        out,
        "kind      auxiliary-gitpile, an aux pile, which may create types\n"
    );
    contains!(r.read("words/.pile/config"), "kind = auxiliary-gitpile\n");
    assert_eq!(
        r.result(&["show", "@words"])["kind"],
        json!("auxiliary-gitpile")
    );
}

#[test]
fn a_repository_with_an_aux_pile_that_checks_is_well() {
    let r = with_aux();
    let a = r.answered(&[], &["doctor", "--format", "json"]);
    assert_eq!(a.code, 0, "{}", a.stdout);
    assert_eq!(located(&a.json()), Vec::<String>::new());
    contains!(r.stdout(&["doctor"]), "3 piles, nothing wrong");
}

#[test]
fn show_prints_the_values_a_pile_gives_its_types_and_the_nodes_at_home_in_it() {
    let r = with_aux();
    contains!(r.stdout(&["show", "@lib"]), "   labels    core, docs\n");
    assert_eq!(
        r.result(&["show", "@lib"])["values"],
        json!({"labels": ["core", "docs"]})
    );
    contains!(
        r.stdout(&["show", "@top"]),
        "   nodes     external:git  the git binary every read runs\n"
    );
    assert_eq!(
        r.result(&["show", "@top"])["nodes"],
        json!([{
            "reference": "external:git",
            "type": "external",
            "name": "git",
            "title": "the git binary every read runs",
            "at": ":/.pile/nodes.toml:3:1",
        }])
    );
    // A type is not a node: show lists an aux pile's types as types.
    let words = r.stdout(&["show", "@words"]);
    contains!(
        words,
        "   types     node external  something outside the repository that a pile relies on\n",
        "             edge uses      pile -> external,pile  one pile calls the code of another\n",
        "                  do    say what the source calls\n",
        "                  dont  list functions\n"
    );
    assert!(!words.contains("nodes"), "{words}");
    assert_eq!(
        r.result(&["show", "@words"])["types"][1],
        json!({
            "name": "uses",
            "edge": true,
            "source": ["pile"],
            "target": ["external", "pile"],
            "title": "one pile calls the code of another",
            "description": null,
            "do": "say what the source calls",
            "dont": "list functions",
            "at": ":/aux/words/.pile/types.toml:7:1",
        })
    );
}

#[test]
fn doctor_reports_each_mistake_in_a_declaration_file_at_its_line_and_column() {
    let r = with_aux();
    r.write(
        ".pile/nodes.toml",
        "version = \"gitpile/1\"\n[[nodes]]\ntype = \"external\"\n\n[[nodes]]\n\
         type = \"service\"\nname = \"ui\"\n",
    );
    r.write(
        "lib/.pile/types.toml",
        "version = \"gitpile/1\"\n[[node-types]]\nname = \"thing\"\n",
    );
    r.write("lib/.pile/broken.toml", "version = \"gitpile/1\"\n[pile\n");
    r.write("lib/.pile/old.toml", "[pile]\ntitle = \"t\"\n");
    let a = r.answered(&[], &["doctor", "--format", "json"]);
    assert_eq!(a.code, 1, "{}", a.stdout);
    assert_eq!(
        located(&a.json()),
        [
            "missing-key :/.pile/nodes.toml:2:1",
            "unknown-type :/.pile/nodes.toml:6:8",
            "unreadable-declaration :/lib/.pile/broken.toml:2:6",
            "no-version :/lib/.pile/old.toml",
            "types-outside-aux :/lib/.pile/types.toml:2:1",
        ]
    );
    let text = r.answered(&[], &["doctor", "--format", "text"]).stdout;
    contains!(
        text,
        "   problem   :/.pile/nodes.toml:6:8: the type service is created by no aux pile. \
         Name a node type an aux pile creates, or create service in one (pile guide types)\n"
    );
}

#[test]
fn every_other_command_reads_on_past_a_declaration_file_that_does_not_check() {
    let r = with_aux();
    r.write("lib/.pile/pile.toml", "not toml at all [");
    for args in [&["list"][..], &["show", "@lib"], &["status"]] {
        r.stdout(args);
    }
    assert_eq!(r.result(&["show", "@lib"])["values"], json!({}));
}

#[test]
fn one_node_declared_in_two_piles_is_declared_twice_and_no_node() {
    let r = with_aux();
    r.write(
        "lib/.pile/nodes.toml",
        "version = \"gitpile/1\"\n[[nodes]]\ntype = \"external\"\nname = \"git\"\n",
    );
    let a = r.answered(&[], &["doctor", "--format", "json"]);
    assert_eq!(a.code, 1);
    assert_eq!(
        located(&a.json()),
        [
            "declared-twice :/.pile/nodes.toml:3:1",
            "declared-twice :/lib/.pile/nodes.toml:2:1",
        ]
    );
    assert_eq!(r.result(&["show", "@top"])["nodes"], json!([]));
}

#[test]
fn a_pile_gives_its_title_in_its_own_table_and_show_prints_it() {
    let r = with_aux();
    r.write(
        "lib/.pile/pile.toml",
        "version = \"gitpile/1\"\n[pile]\ntitle = \"the parser\"\nlabels = [\"core\"]\n",
    );
    contains!(
        r.stdout(&["show", "@lib"]),
        "   title     the parser\n",
        "   labels    core\n"
    );
}
