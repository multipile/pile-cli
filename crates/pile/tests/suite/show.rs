//! `pile show`: one pile whole.

use serde_json::json;

use crate::harness::*;
use crate::piles::nested;

#[test]
fn show_prints_the_pile_where_it_sits_when_its_intent_was_written_and_the_intent() {
    let r = nested();
    r.write("lib/parse.rs", "edited\n");
    r.commit("edit");
    let out = r.stdout(&["show", "@lib"]);
    let head: Vec<&str> = out.lines().take(4).collect();
    assert_eq!(head[0], "@lib  :/lib  bbbbbbbb  gitpile/1");
    assert_eq!(head[1], "   parent    @top  :/  aaaaaaaa");
    assert_eq!(head[2], "   children  :/lib/sub  cccccccc");
    assert!(
        head[3].starts_with("   written   ") && head[3].ends_with(", 1 commit to this pile since"),
        "{out}"
    );
    contains!(
        out,
        "\n:/lib/.pile/intent.md\n\n# /lib\n\nThe parser, and only the parser.\n"
    );
    let v = r.result(&["show", "lib"]);
    assert_eq!(
        v["parent"],
        json!({"name": "top", "path": ":/", "id": id_of("a")})
    );
    assert_eq!(v["children"][0]["path"], ":/lib/sub");
    assert_eq!(v["written"]["since"], 1);
    assert_eq!(v["written"]["commit"].as_str().unwrap().len(), 40);
    assert_eq!(v["files"][0]["name"], ":/lib/.pile/intent.md");
    contains!(r.stdout(&["show", "--full", "@lib"]), id_of("b"));
}

#[test]
fn a_pile_with_only_a_config_says_its_intent_is_not_there() {
    let r = nested();
    let out = r.stdout(&["show", ":/lib/sub"]);
    contains!(
        out,
        "   children  none\n\n:/lib/sub/.pile/intent.md is not there: nothing says why this pile exists\n"
    );
    assert!(!out.contains("written"));
    let v = r.result(&["show", ":/lib/sub"]);
    assert_eq!(v["written"], serde_json::Value::Null);
    assert_eq!(
        v["files"],
        json!([{"name": ":/lib/sub/.pile/intent.md", "text": null}])
    );
}

#[test]
fn an_intent_no_commit_holds_is_not_committed_yet() {
    let r = Repo::empty();
    r.declare(".", "a", "");
    r.intent(".", "Why.\n");
    contains!(r.stdout(&["show"]), "   written   not committed yet\n");
}

#[test]
fn the_files_in_its_pile_directory_this_binary_does_not_read_are_named() {
    let r = nested();
    r.write("lib/.pile/notes.md", "x");
    r.write("lib/.pile/tasks.json", "y");
    // A .toml file is read, so it is not one of them, even one that is not
    // TOML: show judges nothing, and doctor says what is wrong with it.
    r.write("lib/.pile/links.toml", "x");
    contains!(
        r.stdout(&["show", "@lib"]),
        "   unread    :/lib/.pile/notes.md\n             :/lib/.pile/tasks.json\n"
    );
    assert_eq!(
        r.result(&["show", "@lib"])["unread"],
        json!([":/lib/.pile/notes.md", ":/lib/.pile/tasks.json"])
    );
}

#[test]
fn a_value_whose_key_is_wider_than_the_column_is_still_set_apart_from_it() {
    let r = nested();
    r.write(
        "lib/.pile/pile.toml",
        "version = \"gitpile/1\"\n[pile]\nvisibility = \"public\"\nmaintainers = [\"a\", \"b\"]\nurl = \"u\"\n",
    );
    contains!(
        r.stdout(&["show", "@lib"]),
        "   visibility public\n",
        "   maintainers a, b\n",
        "   url       u\n"
    );
}

#[test]
fn with_no_pile_named_show_prints_the_pile_it_runs_in() {
    let r = nested();
    std::fs::create_dir_all(r.path().join("lib/src/deep")).unwrap();
    let a = r.answered_in("lib/src/deep", &[], &["show"]);
    assert!(a.stdout.starts_with("@lib  :/lib  "), "{}", a.stdout);
}

#[test]
fn a_name_is_resolved_to_one_pile_or_refused() {
    let r = nested();
    let unknown = r.refused(&["show", "@nope"], 1);
    assert_eq!(unknown["error"]["code"], "unknown-name");
    assert_eq!(
        unknown["error"]["message"],
        "no pile is named @nope. The names in this repository are @lib @top @v2"
    );
    assert_eq!(
        r.refused(&["show", "@Bad"], 2)["error"]["code"],
        "invalid-name"
    );
    r.declare("other", "e", "lib");
    let twice = r.refused(&["show", "@lib"], 1);
    assert_eq!(twice["error"]["code"], "name-claimed-twice");
    // Both still answer to their paths.
    assert_eq!(r.result(&["show", ":/lib"])["path"], ":/lib");
}

#[test]
fn a_bare_word_is_a_name_before_it_is_a_directory() {
    let r = nested();
    std::fs::create_dir_all(r.path().join("v2")).unwrap();
    assert_eq!(r.result(&["show", "v2"])["path"], ":/lib-v2");
    assert_eq!(
        r.refused(&["show", "./v2"], 1)["error"]["code"],
        "not-a-pile"
    );
}

#[test]
fn for_a_pile_moved_with_git_mv_the_written_count_starts_again_at_the_move() {
    let r = nested();
    r.write("lib/parse.rs", "edited\n");
    r.commit("edit");
    assert_eq!(r.result(&["show", "@lib"])["written"]["since"], 1);
    r.git(&["mv", "lib", "parser"]);
    r.commit("move");
    let v = r.result(&["show", "@lib"]);
    assert_eq!(v["path"], ":/parser");
    assert_eq!(v["written"]["since"], 0);
}

#[test]
fn a_path_that_is_not_there_is_read_as_the_repository_path_it_is() {
    let r = nested();
    let v = r.refused(&["show", "lib/nothing-here"], 1);
    assert_eq!(v["error"]["code"], "not-a-pile");
    assert_eq!(
        v["error"]["message"],
        ":/lib/nothing-here is not a pile. It is in @lib  :/lib  bbbbbbbb"
    );
    assert_eq!(r.result(&["show", ":/lib/"])["path"], ":/lib");
    assert_eq!(r.result(&["show", "lib/sub/.."])["path"], ":/lib");
    assert_eq!(
        r.refused(&["show", ".."], 1)["error"]["code"],
        "path-outside-repository"
    );
}

#[test]
fn an_intent_with_a_heading_and_one_without_have_the_same_headline() {
    let r = Repo::new();
    r.declare("headed", "a", "headed");
    r.declare("bare", "b", "bare");
    let claim =
        "The parser, and\nonly the parser.\n\nRead this pile to find out how. Change it when.\n";
    r.intent("headed", &format!("# /headed\n\n{claim}"));
    r.intent("bare", claim);
    let v = r.result(&["which", "@headed", "@bare"]);
    assert_eq!(v["piles"][0]["intent"], "The parser, and only the parser.");
    assert_eq!(v["piles"][1]["intent"], v["piles"][0]["intent"]);
}

#[test]
fn at_a_revision_show_prints_the_pile_as_that_revision_declares_it() {
    let r = nested();
    r.intent("lib", "The parser, and the printer too.\n");
    r.write(
        "lib/.pile/links.toml",
        "[[edges]]\ntype = \"relies-on\"\ntarget = \"@v2\"\ntitle = \"prints what v2 parses\"\n",
    );
    r.commit("widen lib");
    let out = r.stdout(&["show", "@lib", "--at", "HEAD~1"]);
    let head: Vec<&str> = out.lines().take(3).collect();
    assert_eq!(head[0], "@lib  :/lib  bbbbbbbb  gitpile/1");
    assert!(head[1].starts_with("   at        HEAD~1 "), "{out}");
    assert!(!out.contains("written"), "{out}");
    assert!(!out.contains("printer"), "{out}");
    contains!(out, "The parser, and only the parser.\n");
    let v = r.result(&["show", "@lib", "--at", "HEAD~1"]);
    assert_eq!(v["at"]["name"], "HEAD~1");
    assert_eq!(v["at"]["revision"].as_str().unwrap().len(), 40);
    assert_eq!(v["written"], serde_json::Value::Null);
    assert_eq!(v["links"], json!([]));
    // Without --at, the same pile is the working copy's.
    let v = r.result(&["show", "@lib"]);
    assert_eq!(v["at"], serde_json::Value::Null);
    assert!(v["files"][0]["text"].as_str().unwrap().contains("printer"));
}

#[test]
fn at_a_revision_a_pile_it_does_not_declare_is_refused() {
    let r = nested();
    r.declare("tools", "e", "tools");
    r.commit("add tools");
    let v = r.refused(&["show", "@tools", "--at", "HEAD~1"], 1);
    assert_eq!(v["error"]["code"], "unknown-name");
    let v = r.refused(&["show", "@lib", "--at", "nope"], 2);
    assert_eq!(v["error"]["code"], "unknown-revision");
}
