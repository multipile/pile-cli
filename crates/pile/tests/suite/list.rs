//! `pile list`: every pile, and what two piles claim.

use serde_json::json;

use crate::harness::*;
use crate::piles::nested;

#[test]
fn list_prints_every_pile_in_path_order_with_its_name_path_and_id() {
    let r = nested();
    assert_eq!(
        r.stdout(&["list"]),
        "@top  :/         aaaaaaaa\n\
         @lib  :/lib      bbbbbbbb\n\
         @v2   :/lib-v2   dddddddd\n      \
         :/lib/sub  cccccccc\n\n4 piles\n"
    );
    let v = r.result(&["list"]);
    assert_eq!(
        v["piles"][3],
        json!({"name": null, "path": ":/lib/sub", "id": id_of("c"), "links": []})
    );
    assert_eq!(v["duplicate_ids"], json!([]));
    contains!(r.stdout(&["list", "--full"]), id_of("a"));
}

#[test]
fn an_id_or_a_name_two_piles_claim_is_named_and_list_still_exits_0() {
    let r = nested();
    r.declare("copy", "b", "lib");
    let out = r.stdout(&["list"]);
    contains!(
        out,
        "bbbbbbbb is claimed by 2 piles: :/copy :/lib\n",
        "@lib is claimed by 2 piles: :/copy :/lib\n",
        "5 piles\n"
    );
    let v = r.result(&["list"]);
    assert_eq!(
        v["duplicate_ids"],
        json!([{"id": id_of("b"), "claimants": [":/copy", ":/lib"]}])
    );
    assert_eq!(
        v["duplicate_names"],
        json!([{"name": "lib", "claimants": [":/copy", ":/lib"]}])
    );
}

#[test]
fn a_repository_with_no_pile_says_so_and_exits_0() {
    let r = Repo::new();
    let out = r.stdout(&["list"]);
    assert!(out.starts_with("no pile in "), "{out}");
    assert_eq!(r.result(&["list"])["piles"], json!([]));
}

#[test]
fn a_pile_declared_and_not_committed_is_listed_and_an_ignored_one_is_not() {
    let r = Repo::empty();
    r.declare(".", "a", "");
    r.write(".gitignore", "build/\n");
    r.declare("build/out", "b", "");
    let v = r.result(&["list"]);
    assert_eq!(v["piles"].as_array().unwrap().len(), 1, "{v}");
}

#[test]
fn a_declaration_file_that_does_not_read_refuses_nothing() {
    let r = nested();
    r.write("lib/.pile/links.toml", "[[link]]\nnot = \"read\"\n");
    r.write("lib/.pile/tasks.toml", "garbage = [\n");
    r.write(".pile/.repo-config/labels.toml", "x");
    assert_eq!(r.result(&["list"])["piles"].as_array().unwrap().len(), 4);
}

#[test]
fn a_config_that_cannot_be_read_is_refused_naming_its_file_and_line() {
    let r = nested();
    r.write(
        "lib/.pile/config",
        "kind = gitpile\nversion = 1\nid = 123\n",
    );
    let v = r.refused(&["list"], 1);
    assert_eq!(v["error"]["code"], "unreadable-config");
    assert_eq!(
        v["error"]["message"],
        ":/lib/.pile/config: line 3: the id \"123\" is 3 characters, and an id is 32"
    );
    r.write(
        "lib/.pile/config",
        &format!(
            "kind = gitpile\nversion = 1\nid = {}\nalias = lib\n",
            id_of("b")
        ),
    );
    let a = r.answered(&[], &["list"]);
    assert_eq!(a.code, 1);
    contains!(
        a.stderr,
        "line 4 names \"alias\", and a config holds only kind, version, id and name"
    );
}

#[test]
fn outside_a_repository_it_is_refused_saying_how_to_make_one() {
    let dir = outside();
    let a = answered(dir.path(), &[], &["list", "--format", "json"]);
    assert_eq!(a.code, 1);
    assert_eq!(a.json()["error"]["code"], "not-a-repository");
}
