//! `pile log`: the commits that changed one pile.

use serde_json::json;

use crate::harness::*;
use crate::piles::nested;

#[test]
fn log_lists_the_pile_own_commits_and_what_else_each_changed() {
    let r = nested();
    r.write("lib/parse.rs", "two\n");
    r.write("lib-v2/x.rs", "two\n");
    r.commit("both");
    r.write("lib/sub/deep.rs", "child only\n");
    r.commit("child");
    let out = r.stdout(&["log", "@lib"]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "@lib  :/lib  bbbbbbbb   2 commits to it");
    assert!(lines[2].ends_with("  both"), "{out}");
    assert_eq!(lines[3], "          also   @v2 :/lib-v2");
    assert!(lines[4].ends_with("  lay out"), "{out}");
    contains!(
        out,
        "          also   @top :/  @v2 :/lib-v2  :/lib/sub\n          intent written here\n",
        "\n2 commits, 0 to this pile alone; also :/lib-v2 2, :/ 1, :/lib/sub 1\nby pile test 2\n"
    );
    let v = r.result(&["log", "lib"]);
    assert_eq!(v["total"], 2);
    assert_eq!(
        v["commits"][0]["also"],
        json!([{"name": "v2", "path": ":/lib-v2"}])
    );
    assert_eq!(v["commits"][1]["intent_written"], true);
    assert_eq!(
        v["together"][0],
        json!({"name": "v2", "path": ":/lib-v2", "count": 2})
    );
    assert_eq!(v["by"], json!([{"author": "pile test", "count": 2}]));
}

#[test]
fn n_limits_the_commits_listed_and_0_lists_every_one() {
    let r = nested();
    for i in 0..3 {
        r.write("lib/parse.rs", &format!("{i}\n"));
        r.commit(&format!("edit {i}"));
    }
    let v = r.result(&["log", "-n", "2", "@lib"]);
    assert_eq!(
        (v["total"].as_u64(), v["commits"].as_array().unwrap().len()),
        (Some(4), 2)
    );
    contains!(
        r.stdout(&["log", "-n", "2", "@lib"]),
        "the last 2 of 4 commits to it"
    );
    assert_eq!(
        r.result(&["log", "-n", "0", "@lib"])["commits"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let bad = r.refused(&["log", "-n", "-1", "@lib"], 2);
    assert_eq!(bad["error"]["code"], "invalid-count");
    assert_eq!(bad["next"][0]["command"], "pile log -n 0");
}

#[test]
fn a_pile_moved_with_git_mv_has_a_history_from_the_move() {
    let r = nested();
    r.git(&["mv", "lib-v2", "moved"]);
    r.commit("move it");
    let v = r.result(&["log", "@v2"]);
    assert_eq!(v["path"], ":/moved");
    assert_eq!(v["total"], 1);
    assert_eq!(v["commits"][0]["subject"], "move it");
}

#[test]
fn a_pile_no_commit_has_changed_says_so() {
    let r = Repo::empty();
    r.declare(".", "a", "");
    assert_eq!(
        r.stdout(&["log"]),
        ":/  aaaaaaaa   no commit has changed it yet\n"
    );
    assert_eq!(r.result(&["log"])["commits"], json!([]));
}
