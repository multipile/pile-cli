//! `pile which`: the pile each path is in, and the three forms a path or a
//! pile is given in (pile --help).

use serde_json::json;

use crate::harness::*;
use crate::piles::nested;

#[test]
fn each_path_is_in_the_nearest_pile_above_it_a_block_per_pile() {
    let r = nested();
    let out = r.stdout(&[
        "which",
        "lib/parse.rs",
        "lib/sub/deep.rs",
        "lib/sub/new.rs",
        "README",
    ]);
    assert_eq!(
        out,
        "@top  :/  aaaaaaaa  gitpile/1\n   \
         paths     :/README\n   \
         intent    The whole repository.\n   \
         whole     pile show @top\n\n\
         @lib  :/lib  bbbbbbbb  gitpile/1\n   \
         paths     :/lib/parse.rs\n   \
         intent    The parser, and only the parser.\n   \
         whole     pile show @lib\n\n\
         :/lib/sub  cccccccc  gitpile/1\n   \
         paths     :/lib/sub/deep.rs\n             \
         :/lib/sub/new.rs\n   \
         whole     pile show :/lib/sub\n"
    );
}

#[test]
fn a_name_stands_for_its_own_directory() {
    let r = nested();
    let v = r.result(&["which", "@v2", "lib"]);
    let piles = v["piles"].as_array().unwrap();
    assert_eq!(piles.len(), 2);
    assert_eq!(piles[0]["path"], ":/lib");
    assert_eq!(piles[0]["paths"], json!([":/lib"]));
    assert_eq!(piles[1]["paths"], json!([":/lib-v2"]));
    assert_eq!(piles[0]["intent"], "The parser, and only the parser.");
    assert_eq!(piles[1]["intent"], serde_json::Value::Null);
}

#[test]
fn a_bare_word_a_pile_claims_is_a_name_and_any_other_is_a_path() {
    let r = nested();
    std::fs::create_dir_all(r.path().join("v2")).unwrap();
    let v = r.result(&["which", "--path", "v2", "./v2", "v3"]);
    assert_eq!(v["paths"], json!([":/lib-v2", ":/", ":/"]));
}

#[test]
fn a_path_no_pile_holds_is_in_no_pile() {
    let r = Repo::new();
    r.declare("a", "a", "");
    let v = r.result(&["which", "b/x", "a/y"]);
    assert_eq!(v["in_no_pile"], json!([":/b/x"]));
    contains!(
        r.stdout(&["which", "b/x"]),
        "in no pile\n   paths     :/b/x\n"
    );
}

#[test]
fn path_prints_each_pile_path_alone_in_the_order_given() {
    let r = Repo::new();
    r.declare("a", "a", "");
    assert_eq!(
        r.stdout(&["which", "--path", "a/x", "b", "a"]),
        ":/a\n\n:/a\n"
    );
    assert_eq!(
        r.result(&["which", "--path", "b"]),
        json!({"paths": [null]})
    );
}

#[test]
fn a_relative_path_is_from_the_directory_pile_runs_in() {
    let r = nested();
    let a = r.answered_in(
        "lib/sub",
        &[],
        &[
            "which",
            "--path",
            "deep.rs",
            "../parse.rs",
            "../../README",
            "./a/../b",
        ],
    );
    assert_eq!(a.stdout, ":/lib/sub\n:/lib\n:/\n:/lib/sub\n");
}

#[test]
fn a_path_need_not_exist_and_is_resolved_as_far_as_it_does() {
    let r = nested();
    let v = r.result(&[
        "which",
        "--path",
        "lib/sub/not/yet/x.rs",
        "lib/nothing/../sub/y",
    ]);
    assert_eq!(v["paths"], json!([":/lib/sub", ":/lib/sub"]));
}

#[test]
fn an_absolute_path_on_disk_is_a_path_on_disk() {
    let r = nested();
    let inside = format!("{}/lib/sub/new.rs", r.path().display());
    assert_eq!(r.stdout(&["which", "--path", &inside]), ":/lib/sub\n");
    let v = r.refused(&["which", "README", "/tmp/elsewhere"], 1);
    assert_eq!(v["error"]["code"], "path-outside-repository");
    contains!(
        v["error"]["message"].as_str().unwrap(),
        "/tmp/elsewhere is outside the repository at ",
        "A path from the top of the repository is written :/path"
    );
    assert!(
        v.get("result").is_none(),
        "nothing is answered for README either"
    );
    let climbs = r.answered_in("lib", &[], &["which", "../../x", "--format", "json"]);
    let climbs = climbs.json();
    assert_eq!(climbs["error"]["code"], "path-outside-repository");
    contains!(
        climbs["error"]["message"].as_str().unwrap(),
        "../../x is ",
        ", which is outside the repository at "
    );
}

#[test]
fn a_path_is_never_read_as_the_pile_or_repository_path_it_looks_like() {
    let r = nested();
    // /lib is the disk's /lib: not the pile at :/lib, and not the repository's
    // lib directory.
    let v = r.refused(&["which", "/lib/parse.rs"], 1);
    assert_eq!(v["error"]["code"], "path-outside-repository");
    let message = v["error"]["message"].as_str().unwrap();
    assert!(!message.contains("@lib"), "no pile is proposed: {message}");
    assert_eq!(
        r.refused(&["show", "/lib"], 1)["error"]["code"],
        "path-outside-repository"
    );
}

#[test]
fn a_from_top_path_is_from_the_top_of_the_repository_from_anywhere() {
    let r = nested();
    let a = r.answered_in(
        "lib/sub",
        &[],
        &[
            "which",
            "--path",
            ":/lib-v2/new.rs",
            ":/",
            ":/lib/./sub/../x",
            ":/README",
        ],
    );
    assert_eq!(a.stdout, ":/lib-v2\n:/\n:/lib\n:/\n");
    let v = r.refused(&["which", ":/../x"], 1);
    assert_eq!(v["error"]["code"], "path-outside-repository");
    assert_eq!(
        v["error"]["message"],
        ":/../x climbs above the top of the repository"
    );
}

#[test]
fn a_path_through_a_symbolic_link_is_where_the_link_leads() {
    let r = nested();
    std::os::unix::fs::symlink(r.path().join("lib/sub"), r.path().join("into-sub")).unwrap();
    assert_eq!(
        r.stdout(&["which", "--path", "into-sub/deep.rs", "into-sub/new.rs"]),
        ":/lib/sub\n:/lib/sub\n"
    );
}

#[test]
fn the_directory_pile_runs_in_may_be_reached_through_a_symbolic_link() {
    let r = nested();
    let elsewhere = outside();
    let link = elsewhere.path().join("into");
    std::os::unix::fs::symlink(r.path().join("lib"), &link).unwrap();
    let a = answered(&link, &[], &["which", "--path", "parse.rs", "sub/x"]);
    assert_eq!(a.code, 0, "{}", a.stderr);
    assert_eq!(a.stdout, ":/lib\n:/lib/sub\n");
    let show = answered(&link, &[], &["show", "--format", "json"]);
    assert_eq!(show.json()["result"]["path"], ":/lib");
}

#[test]
fn which_takes_at_least_one_path() {
    let r = nested();
    assert_eq!(r.refused(&["which"], 2)["error"]["code"], "usage");
}
