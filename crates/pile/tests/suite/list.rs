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
        json!({
            "name": null,
            "path": ":/lib/sub",
            "id": id_of("c"),
            "mount": "sub",
            "mounted_in": "@lib",
            "links": []
        })
    );
    assert_eq!(v["piles"][0]["mount"], json!(null));
    assert_eq!(v["piles"][0]["mounted_in"], json!(null));
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

#[test]
fn tree_prints_each_pile_under_the_pile_it_is_mounted_in_at_its_path_relative_to_it() {
    let r = nested();
    assert_eq!(
        r.stdout(&["list", "--tree"]),
        "@top      :/      aaaaaaaa\n\
         ├── @lib  lib     bbbbbbbb\n\
         │   └──   sub     cccccccc\n\
         └── @v2   lib-v2  dddddddd\n\n4 piles\n"
    );
    let v = r.result(&["list", "--tree"]);
    let tops = v["piles"].as_array().unwrap();
    assert_eq!(tops.len(), 1, "{v}");
    assert_eq!(tops[0]["path"], ":/");
    assert_eq!(tops[0]["mount"], json!(null));
    let lib = &tops[0]["piles"][0];
    assert_eq!(
        (&lib["mount"], &lib["mounted_in"]),
        (&json!("lib"), &json!("@top"))
    );
    assert_eq!(lib["piles"][0]["path"], ":/lib/sub");
    assert_eq!(lib["piles"][0]["mount"], "sub");
    assert_eq!(lib["piles"][0]["piles"], json!([]));
    assert_eq!(tops[0]["piles"][1]["name"], "v2");
}

#[test]
fn tree_lines_up_every_column_when_a_deep_name_is_the_widest() {
    let r = nested();
    r.declare("lib/sub/inner", "e", "a-rather-long-pile-name");
    let out = r.stdout(&["list", "--tree"]);
    let lines: Vec<&str> = out.lines().take(5).collect();
    assert_eq!(
        lines[3], "│       └── @a-rather-long-pile-name  inner   eeeeeeee",
        "{out}"
    );
    // Each path, and each id, starts at the same column on every line.
    let column = |l: &str, of: &str| l.chars().count() - l[l.find(of).unwrap()..].chars().count();
    let paths: Vec<usize> = [":/", "lib", "sub", "inner", "lib-v2"]
        .iter()
        .zip(&lines)
        .map(|(p, l)| column(l, &format!("  {p}")))
        .collect();
    assert!(paths.iter().all(|c| *c == paths[0]), "{out}");
    let ids: Vec<usize> = ["aaaaaaaa", "bbbbbbbb", "cccccccc", "eeeeeeee", "dddddddd"]
        .iter()
        .zip(&lines)
        .map(|(id, l)| column(l, id))
        .collect();
    assert!(ids.iter().all(|c| *c == ids[0]), "{out}");
}

#[test]
fn tree_with_no_pile_at_the_top_has_a_top_for_each_pile_with_none_above_it() {
    let r = Repo::empty();
    r.declare("a", "a", "a");
    r.declare("b/c", "b", "c");
    r.declare("b/c/d/e", "c", "e");
    assert_eq!(
        r.stdout(&["list", "--tree"]),
        "@a      :/a    aaaaaaaa\n\
         @c      :/b/c  bbbbbbbb\n\
         └── @e  d/e    cccccccc\n\n3 piles\n"
    );
    let v = r.result(&["list", "--tree"]);
    assert_eq!(v["piles"].as_array().unwrap().len(), 2, "{v}");
    assert_eq!(v["piles"][1]["piles"][0]["mount"], "d/e");
    assert_eq!(v["piles"][1]["piles"][0]["mounted_in"], "@c");
}
