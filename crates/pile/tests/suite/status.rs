//! `pile status`: what changed, by pile.

use serde_json::json;

use crate::harness::*;
use crate::piles::nested;

#[test]
fn a_change_is_under_its_pile_and_a_pile_with_none_says_unchanged() {
    let r = nested();
    r.write("lib/parse.rs", "fn parse() { todo!() }\n");
    r.write("lib/sub/fresh.rs", "new\n");
    assert_eq!(
        r.stdout(&["status", "--all"]),
        "@top  :/  aaaaaaaa\n   (unchanged)\n\n\
         @lib  :/lib  bbbbbbbb\n   M  :/lib/parse.rs\n\n\
         @v2  :/lib-v2  dddddddd\n   (unchanged)\n\n\
         :/lib/sub  cccccccc\n   ?  :/lib/sub/fresh.rs\n\n\
         4 piles, 2 changed, 2 paths\n"
    );
    assert_eq!(
        r.stdout(&["status", "--all", "--brief"]),
        "@top  :/         aaaaaaaa   unchanged\n\
         @lib  :/lib      bbbbbbbb   1 path changed\n\
         @v2   :/lib-v2   dddddddd   unchanged\n      \
         :/lib/sub  cccccccc   0 paths changed, 1 untracked\n"
    );
}

#[test]
fn a_move_from_one_pile_to_another_is_a_half_in_each() {
    let r = nested();
    r.git(&["mv", "lib/parse.rs", "lib-v2/parse.rs"]);
    r.git(&["mv", "lib-v2/x.rs", "lib-v2/y.rs"]);
    let out = r.stdout(&["status", "--all"]);
    contains!(
        out,
        "@lib  :/lib  bbbbbbbb\n   D  :/lib/parse.rs  (left this pile, now :/lib-v2/parse.rs)\n",
        "   A  :/lib-v2/parse.rs  (arrived from :/lib/parse.rs)\n   R  :/lib-v2/y.rs  (was :/lib-v2/x.rs)\n",
        "4 piles, 2 changed, 3 paths, 2 paths crossed a boundary\n"
    );
    let v = r.result(&["status", "--all"]);
    assert_eq!(
        v["piles"][1]["entries"][0],
        json!({"state": "D", "path": ":/lib/parse.rs", "was": null, "now": ":/lib-v2/parse.rs", "crossed": true, "mode": false})
    );
    assert_eq!(v["total"]["crossed"], 2);
}

#[test]
fn status_reports_the_pile_it_runs_in_and_the_piles_under_it() {
    let r = nested();
    r.write("README", "changed\n");
    r.write("lib/sub/deep.rs", "changed\n");
    let a = r.answered_in("lib/sub", &[], &["status", "--format", "json"]);
    let v = &a.json()["result"];
    assert_eq!(v["piles"].as_array().unwrap().len(), 1);
    assert_eq!(v["piles"][0]["path"], ":/lib/sub");
    let lib = r
        .answered_in("lib", &[], &["status", "--format", "json"])
        .json();
    assert_eq!(lib["result"]["total"]["piles"], 2);
}

#[test]
fn a_change_no_pile_holds_is_in_no_pile_and_only_in_scope() {
    let r = Repo::new();
    r.declare("a", "a", "");
    r.commit("a");
    r.write("stray", "s\n");
    contains!(
        r.stdout(&["status", "--all"]),
        "in no pile\n   ?  :/stray\n",
        "1 path in no pile"
    );
    let inside = r
        .answered_in("a", &[], &["status", "--format", "json"])
        .json();
    assert_eq!(inside["result"]["in_no_pile"], json!([]));
    let top = r.refused(&["status"], 1);
    assert_eq!(top["error"]["code"], "no-pile-here");
}

#[test]
fn in_a_repository_with_no_commit_every_file_is_new() {
    let r = Repo::empty();
    r.declare(".", "a", "");
    r.git(&["add", ".pile/config"]);
    r.write("draft", "d\n");
    let out = r.stdout(&["status"]);
    assert_eq!(
        out,
        ":/  aaaaaaaa\n   A  :/.pile/config\n   ?  :/draft\n\n1 pile, 1 changed, 2 paths\n"
    );
}

#[test]
fn a_mode_change_is_beside_the_letter() {
    let r = nested();
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(
        r.path().join("README"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    contains!(r.stdout(&["status", "--all"]), "   M  :/README  (mode)\n");
}

#[test]
fn a_repository_with_no_pile_is_refused() {
    let r = Repo::new();
    let v = r.refused(&["status"], 1);
    assert_eq!(v["error"]["code"], "no-piles");
    assert_eq!(v["next"][0]["command"], "pile init");
}
