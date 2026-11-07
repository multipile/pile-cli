//! Links: `show`, `which` and `list` print each pile's links, `doctor`
//! reports each mistake in one, and a link names its other end by name, so
//! it outlives a move and not a rename.

use std::path::Path;

use serde_json::json;

use crate::aux::located;
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

[[edge-types]]
name = "exercises"
title = "one pile runs another whole, and says whether it works"
source = ["pile"]
target = ["pile"]
"#;

const V: &str = "version = \"gitpile/1\"\n";

/// `/` (@top), `/cli` (@cli), `/storage` (@storage), `/journeys`
/// (@journeys) and the aux pile `/aux/words` (@words), which creates
/// `external`, `uses` and `exercises`. The top declares `external:git`.
/// @cli uses @storage and external:git, and @journeys exercises @cli.
/// Committed.
fn linked() -> Repo {
    let r = Repo::new();
    r.declare(".", "a", "top");
    r.intent(".", "The whole repository.\n");
    for (path, id, text) in [
        ("cli", "c", "The command line.\n"),
        ("storage", "d", "Every read of a repository.\n"),
        ("journeys", "e", "Sessions with the binary.\n"),
    ] {
        r.declare(path, id, path);
        r.intent(path, text);
    }
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
        &format!(
            "{V}\n[[nodes]]\ntype = \"external\"\nname = \"git\"\ntitle = \"the git binary\"\n"
        ),
    );
    r.write(
        "cli/.pile/links.toml",
        &format!(
            "{V}\n[[edges]]\ntype = \"uses\"\ntarget = \"@storage\"\n\
             title = \"reads every repository through\"\n\
             description = \"\"\"\nEvery read goes through the Storage trait.\n\"\"\"\n\n\
             [[edges]]\ntype = \"uses\"\ntarget = \"external:git\"\n\
             title = \"asks every question of\"\n"
        ),
    );
    r.write(
        "journeys/.pile/links.toml",
        &format!(
            "{V}\n[[edges]]\ntype = \"exercises\"\ntarget = \"@cli\"\nthrough = \"binary\"\n\
             title = \"holds what it prints to its help\"\n"
        ),
    );
    r.commit("piles and their links");
    r
}

#[test]
fn show_prints_each_link_both_ways_with_its_type_other_end_and_title() {
    let r = linked();
    contains!(
        r.stdout(&["show", "@cli"]),
        "   links     -> uses       @storage      reads every repository through\n\
         \x20            -> uses       external:git  asks every question of\n\
         \x20            <- exercises  @journeys     holds what it prints to its help  \
         (written in @journeys)\n"
    );
    let v = r.result(&["show", "@cli"]);
    assert_eq!(
        v["links"][0],
        json!({
            "direction": "to",
            "type": "uses",
            "other": "@storage",
            "source": "@cli",
            "target": "@storage",
            "title": "reads every repository through",
            "description": "Every read goes through the Storage trait.\n",
            "values": {},
            "at": ":/cli/.pile/links.toml:3:1",
            "written_in": "@cli",
        })
    );
    assert_eq!(v["links"][2]["direction"], "from");
    assert_eq!(v["links"][2]["values"], json!({"through": ["binary"]}));
    assert_eq!(v["links"][2]["written_in"], "@journeys");

    // next points at each pile at the other end, once.
    let a = r.answered(&[], &["show", "@cli", "--format", "json"]);
    let next: Vec<String> = a.json()["next"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["command"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        next,
        [
            "pile show @storage",
            "pile show @journeys",
            "pile show @words"
        ]
    );
    // Then the aux pile that creates the links' types, once.
    assert!(
        a.json()["next"][2]["why"]
            .as_str()
            .unwrap()
            .starts_with("the aux pile that creates uses, exercises:"),
        "{}",
        a.json()["next"]
    );

    // At the other end, the same edge, the other way.
    contains!(
        r.stdout(&["show", "@storage"]),
        "   links     <- uses  @cli  reads every repository through  (written in @cli)\n"
    );
    // A pile with no name is named by its path at the other end.
    r.declare("docs", "f", "");
    r.write(
        "docs/.pile/links.toml",
        &format!(
            "{V}[[edges]]\ntype = \"uses\"\ntarget = \"@storage\"\ntitle = \"says how to read\"\n"
        ),
    );
    contains!(
        r.stdout(&["show", "@storage"]),
        "             <- uses  :/docs  says how to read  (written in :/docs)\n"
    );
    assert_eq!(
        r.result(&["show", "@storage"])["links"][1]["other"],
        ":/docs"
    );
    // The top declares external:git, so @cli's link to it is the top's too,
    // with the top at neither end.
    contains!(
        r.stdout(&["show", "@top"]),
        "   links     -- uses  @cli -> external:git  asks every question of  (written in @cli)\n"
    );
    let between = &r.result(&["show", "@top"])["links"][0];
    assert_eq!(between["direction"], "between");
    // Its ends are source and target, and no one end is the other.
    assert_eq!(between["other"], json!(null));
    assert_eq!(
        (between["source"].clone(), between["target"].clone()),
        (json!("@cli"), json!("external:git"))
    );
    // A pile with no link has no links line, and an empty list.
    assert!(!r.stdout(&["show", "@words"]).contains("links"));
    assert_eq!(r.result(&["show", "@words"])["links"], json!([]));
}

#[test]
fn which_names_the_other_ends_and_list_counts_the_links() {
    let r = linked();
    contains!(
        r.stdout(&["which", "cli/src/main.rs"]),
        "   intent    The command line.\n\
         \x20  links     -> @storage, external:git\n\
         \x20            <- @journeys\n\
         \x20  whole     pile show @cli\n"
    );
    let v = r.result(&["which", "cli/src/main.rs"]);
    assert_eq!(
        v["piles"][0]["links"],
        json!([
            {"direction": "to", "type": "uses", "other": "@storage",
             "source": "@cli", "target": "@storage"},
            {"direction": "to", "type": "uses", "other": "external:git",
             "source": "@cli", "target": "external:git"},
            {"direction": "from", "type": "exercises", "other": "@journeys",
             "source": "@journeys", "target": "@cli"},
        ])
    );

    assert_eq!(
        r.stdout(&["list"]),
        "@top       :/           aaaaaaaa  -> 0  <- 0\n\
         @words     :/aux/words  bbbbbbbb  -> 0  <- 0\n\
         @cli       :/cli        cccccccc  -> 2  <- 1\n\
         @journeys  :/journeys   eeeeeeee  -> 1  <- 0\n\
         @storage   :/storage    dddddddd  -> 0  <- 1\n\n5 piles\n"
    );
    let v = r.result(&["list"]);
    assert_eq!(
        v["piles"][3]["links"],
        json!([{"direction": "to", "type": "exercises", "other": "@cli",
                "source": "@journeys", "target": "@cli"}])
    );
}

#[test]
fn doctor_finds_a_reference_to_nothing_a_wrong_end_and_an_edge_written_twice() {
    let r = linked();
    r.write(
        "cli/.pile/more.toml",
        &format!(
            "{V}[[edges]]\ntype = \"uses\"\ntarget = \"@nobody\"\n\
             [[edges]]\ntype = \"exercises\"\ntarget = \"external:git\"\n\
             [[edges]]\ntype = \"exercises\"\nsource = \"@journeys\"\nthrough = \"library\"\n"
        ),
    );
    let a = r.answered(&[], &["doctor", "--format", "json"]);
    assert_eq!(a.code, 1, "{}", a.stdout);
    assert_eq!(
        located(&a.json()),
        [
            "no-such-name :/cli/.pile/more.toml:4:10",
            "wrong-end :/cli/.pile/more.toml:7:10",
            "declared-twice :/cli/.pile/more.toml:8:1",
            "declared-twice :/journeys/.pile/links.toml:3:1",
        ]
    );
    let text = r.answered(&[], &["doctor"]).stdout;
    contains!(
        text,
        "problem   :/cli/.pile/more.toml:4:10: no pile is named @nobody. Name a pile that is \
         named: pile list shows each name\n",
        "problem   :/cli/.pile/more.toml:7:10: the target of a link of type exercises is pile, \
         and this one's is external.",
        "problem   :/journeys/.pile/links.toml:3:1: the link @journeys exercises @cli is written \
         here and at :/cli/.pile/more.toml:8:1",
    );
    // Neither copy of the edge written twice is read.
    assert!(!r.stdout(&["show", "@cli"]).contains("exercises"));

    // Fixed, doctor exits 0 again.
    std::fs::remove_file(r.path().join("cli/.pile/more.toml")).unwrap();
    contains!(r.stdout(&["doctor"]), "5 piles, nothing wrong");
}

#[test]
fn a_link_that_joins_a_node_to_itself_is_a_problem() {
    let r = linked();
    r.write(
        "cli/.pile/more.toml",
        &format!("{V}[[edges]]\ntype = \"uses\"\ntarget = \"@cli\"\n"),
    );
    let a = r.answered(&[], &["doctor", "--format", "json"]);
    assert_eq!(a.code, 1);
    assert_eq!(located(&a.json()), ["self-link :/cli/.pile/more.toml:2:1"]);
}

#[test]
fn a_pile_moved_with_git_mv_keeps_its_links() {
    let r = linked();
    r.git(&["mv", "storage", "store"]);
    let a = r.answered(&[], &["doctor"]);
    assert_eq!(a.code, 0, "{}", a.stdout);
    contains!(
        r.stdout(&["show", "@cli"]),
        "   links     -> uses       @storage      reads every repository through\n"
    );
    let v = r.result(&["show", ":/store"]);
    assert_eq!(v["links"][0]["other"], "@cli");
}

#[test]
fn renaming_a_pile_breaks_each_reference_to_its_old_name() {
    let r = linked();
    r.write(
        "journeys/.pile/more.toml",
        &format!("{V}[[edges]]\ntype = \"uses\"\ntarget = \"@cli\"\ntitle = \"runs it\"\n"),
    );
    r.stdout(&["set", "name", "command", "@cli"]);
    let a = r.answered(&[], &["doctor", "--format", "json"]);
    assert_eq!(a.code, 1, "{}", a.stdout);
    assert_eq!(
        located(&a.json()),
        [
            "no-such-name :/journeys/.pile/links.toml:5:10",
            "no-such-name :/journeys/.pile/more.toml:4:10",
        ]
    );
    // The links @command writes still read: an end left out is the pile
    // that writes them.
    contains!(
        r.stdout(&["show", "@command"]),
        "-> uses  @storage      reads every repository through\n",
        "-> uses  external:git  asks every question of\n"
    );
}

/// Copies a repository laid out under crates/graph/tests/fixtures into r,
/// each `_pile/` as its `.pile/`.
fn lay_out(r: &Repo, name: &str) {
    fn walk(r: &Repo, dir: &Path, at: &str) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry
                .file_name()
                .to_string_lossy()
                .replace("_pile", ".pile");
            let path = if at.is_empty() {
                name
            } else {
                format!("{at}/{name}")
            };
            if entry.file_type().unwrap().is_dir() {
                walk(r, &entry.path(), &path);
            } else {
                r.write(&path, &std::fs::read_to_string(entry.path()).unwrap());
            }
        }
    }
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../graph/tests/fixtures");
    walk(r, &fixtures.join(name), "");
}

#[test]
fn the_store_laid_out_reads_every_link_and_show_prints_each_both_ways() {
    let r = Repo::new();
    lay_out(&r, "store");
    r.commit("the store");
    let a = r.answered(&[], &["doctor", "--format", "json"]);
    assert_eq!(a.code, 0, "{}", a.stdout);
    // Its top has no name, which is the one thing doctor says.
    assert_eq!(located(&a.json()), ["no-name :/.pile/config"]);

    contains!(
        r.stdout(&["show", "@store-disk"]),
        "   links     -> implements   @api               implements the store interface\n\
         \x20            -> conforms-to  @api-tests         is held to the store tests\n\
         \x20            -> stores-in    external:database  keeps every value in\n\
         \x20            <- uses         @app               keeps values in a database when one \
         is given  (written in @app)\n"
    );
    // @api writes the link from @store-memory itself: @api is its target.
    contains!(
        r.stdout(&["show", "@api"]),
        "             <- implements  @store-memory  implements the store interface\n"
    );
    contains!(
        r.stdout(&["list"]),
        "@api           :/api           33333333  -> 0  <- 3\n"
    );
}
