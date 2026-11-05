//! `pile init`, `pile set` and `pile id`: the commands that declare a pile
//! and write its config.

use serde_json::json;

use crate::harness::*;
use crate::piles::nested;

#[test]
fn init_writes_a_config_and_an_intent_that_asks_why() {
    let r = Repo::new();
    std::fs::create_dir_all(r.path().join("packages/lib")).unwrap();
    let out = r.answered_in("packages/lib", &[], &["init"]).stdout;
    contains!(
        out,
        "pile      :/packages/lib\nid        ",
        ":/packages/lib/.pile is new, and should be committed",
        "it has no name yet:\n  pile set name lib :/packages/lib   gives it a name to be typed and shown by\n"
    );
    let config = r.read("packages/lib/.pile/config");
    assert!(config.starts_with("# What this subtree is."), "{config}");
    contains!(config, "kind = gitpile\nversion = 1\nid = ");
    let intent = r.read("packages/lib/.pile/intent.md");
    assert!(
        intent.starts_with("<!--\n"),
        "the intent has no heading: {intent}"
    );
    let v = r.result(&["show", ":/packages/lib"]);
    assert_eq!(v["id"].as_str().unwrap().len(), 32);
    // A question has no headline.
    assert_eq!(
        r.result(&["which", "packages/lib"])["piles"][0]["intent"],
        serde_json::Value::Null
    );
}

#[test]
fn init_declares_the_directory_it_runs_in_and_says_which_pile_it_was_in() {
    let r = nested();
    std::fs::create_dir_all(r.path().join("lib/core")).unwrap();
    let v = r.result_in("lib/core", &["init", "--set", "name=core"]);
    assert_eq!(v["path"], ":/lib/core");
    assert_eq!(v["name"], "core");
    assert_eq!(v["inside"], ":/lib");
    assert_eq!(v["to_commit"][0]["path"], ":/lib/core/.pile");
    assert_eq!(r.result(&["show", "@core"])["parent"]["path"], ":/lib");
    let again = r.refused_in("lib/core", &["init", "--set", "name=core2"], 1);
    assert_eq!(again["error"]["code"], "already-a-pile");
    assert_eq!(again["next"][0]["command"], "pile show :/lib/core");
}

#[test]
fn init_works_in_a_repository_with_no_commit_and_leaves_an_intent_alone() {
    let r = Repo::empty();
    r.intent(".", "Already said.\n");
    let v = r.result(&["init"]);
    assert_eq!(v["path"], ":/");
    assert_eq!(v["inside"], serde_json::Value::Null);
    assert_eq!(v["to_commit"][0]["path"], ":/.pile");
    assert_eq!(r.read(".pile/intent.md"), "Already said.\n");
}

#[test]
fn init_takes_no_path() {
    let r = nested();
    std::fs::create_dir_all(r.path().join("x")).unwrap();
    for arg in ["x", ".", ":/x", "@lib"] {
        let v = r.refused(&["init", arg], 2);
        assert_eq!(v["error"]["code"], "usage", "{arg}");
        contains!(
            v["error"]["message"].as_str().unwrap(),
            "pile init declares the directory you are in, and takes no path"
        );
    }
    assert!(!r.path().join("x/.pile").exists());
}

#[test]
fn init_refuses_what_it_cannot_declare() {
    let r = nested();
    std::fs::create_dir_all(r.path().join("x")).unwrap();
    for (args, code, exit) in [
        (&["init"][..], "already-a-pile", 1),
        (&["init", "--set", "name"], "malformed-set", 2),
        (
            &["init", "--set", "name=a", "--set", "name=b"],
            "key-given-twice",
            2,
        ),
        (&["init", "--set", "colour=red"], "unknown-key", 2),
        (&["init", "--set", "id=x"], "not-settable", 2),
        (&["init", "--set", "name=Bad"], "invalid-value", 2),
        (&["init", "--set", "kind=other"], "invalid-value", 2),
        (&["init", "--set", "version=2"], "not-settable", 2),
        (&["init", "--set", "name=lib"], "name-taken", 1),
    ] {
        let dir = if code == "already-a-pile" { "lib" } else { "x" };
        let v = r.refused_in(dir, args, exit);
        assert_eq!(v["error"]["code"], code, "{args:?}");
    }
    assert!(!r.path().join("x/.pile").exists());
}

#[test]
fn set_rewrites_the_name_line_and_leaves_every_other_line_alone() {
    let r = nested();
    let config = format!(
        "# mine\nkind = gitpile\nversion = 1\nid = {}\n\n# end\n",
        id_of("c")
    );
    r.write("lib/sub/.pile/config", &config);
    let out = r.stdout(&["set", "name", "sub", ":/lib/sub"]);
    contains!(
        out,
        "@sub  :/lib/sub  cccccccc\n",
        "lib/sub/.pile/config is changed, and should be committed"
    );
    assert_eq!(
        r.read("lib/sub/.pile/config"),
        config.replace("\n\n# end", "\nname = sub\n\n# end")
    );
    let again = r.result(&["set", "name", "sub", "@sub"]);
    assert_eq!(
        (again["written"].clone(), again["to_commit"].clone()),
        (json!(false), json!([]))
    );
    let renamed = r.result(&["set", "name", "deep", "@sub"]);
    assert_eq!(
        (renamed["was"].clone(), renamed["value"].clone()),
        (json!("sub"), json!("deep"))
    );
    r.result(&["set", "--forget", "name", "@deep"]);
    assert_eq!(r.read("lib/sub/.pile/config"), config);
}

#[test]
fn set_with_no_pile_named_sets_the_pile_it_runs_in() {
    let r = nested();
    let a = r.answered_in("lib/sub", &[], &["set", "name", "here"]);
    assert_eq!(a.code, 0, "{}", a.stderr);
    contains!(r.read("lib/sub/.pile/config"), "name = here\n");
}

#[test]
fn set_refuses_a_key_it_does_not_write_and_a_name_it_cannot_take() {
    let r = nested();
    for (args, code, exit) in [
        (&["set", "colour", "red"][..], "unknown-key", 2),
        (&["set", "alias", "x"], "unknown-key", 2),
        (&["set", "id", "x"], "not-settable", 2),
        (&["set", "labels", "x"], "unknown-key", 2),
        (&["set", "name", "Not-A-Name"], "invalid-value", 2),
        (&["set", "name", "v2", "@lib"], "name-taken", 1),
        (&["set"], "usage", 2),
        (&["set", "name"], "usage", 2),
    ] {
        let v = r.refused(args, exit);
        assert_eq!(v["error"]["code"], code, "{args:?}");
    }
    let labels = r.refused(&["set", "labels", "core"], 2);
    contains!(
        labels["error"]["message"].as_str().unwrap(),
        "\"labels\" is not a key set writes. The one key set writes is name"
    );
}

#[test]
fn set_leaves_a_config_it_cannot_read_as_it_is() {
    let r = nested();
    let with = format!(
        "kind = gitpile\nversion = 1\nid = {}\nname = lib\nlabels = core\n",
        id_of("b")
    );
    r.write("lib/.pile/config", &with);
    let v = r.refused(&["set", "name", "parser", "lib"], 1);
    assert_eq!(v["error"]["code"], "unreadable-config");
    contains!(
        v["error"]["message"].as_str().unwrap(),
        "line 5 names \"labels\", and a config holds only kind, version, id and name"
    );
    assert_eq!(r.read("lib/.pile/config"), with);
}

#[test]
fn id_prints_a_fresh_id_alone_in_every_format_and_needs_no_repository() {
    let dir = outside();
    let a = answered(dir.path(), &[], &["id", "--format", "json"]);
    assert_eq!(a.code, 0);
    let id = a.stdout.trim_end();
    assert_eq!(id.len(), 32);
    assert!(
        id.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    let b = answered(dir.path(), &[], &["id"]);
    assert_ne!(b.stdout, a.stdout);
}
