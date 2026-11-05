//! `pile version`: what this binary is.

use crate::harness::*;

#[test]
fn version_says_what_the_binary_is_inside_a_repository_and_out() {
    let r = Repo::new();
    let out = r.stdout(&["version"]);
    contains!(
        out,
        "pile dev\n",
        "commit unknown\n",
        "built  unknown\n",
        "rustc  ",
        "agent  off\n"
    );

    let dir = outside();
    let a = answered(dir.path(), &[], &["version"]);
    assert_eq!(a.code, 0, "{}", a.stderr);
    assert_eq!(a.stdout, out);
}

#[test]
fn the_result_holds_what_the_text_prints() {
    let r = Repo::new();
    let v = r.answered(&[], &["version", "--format", "json"]).json();
    let result = &v["result"];
    assert_eq!(result["version"], "dev");
    assert_eq!(result["commit"], "unknown");
    assert_eq!(result["built"], "unknown");
    assert!(result["rustc"].as_str().unwrap().contains(" ("), "{result}");
    assert_eq!(result["agent"], serde_json::Value::Null);
    let keys: Vec<&String> = result.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["version", "commit", "built", "rustc", "agent"]);
}

#[test]
fn short_is_the_version_alone_in_every_format() {
    let r = Repo::new();
    for format in ["text", "json"] {
        let out = r.stdout(&["version", "--short", "--format", format]);
        assert_eq!(out, "dev\n", "--format {format}");
    }
}

#[test]
fn version_takes_no_arguments() {
    let r = Repo::new();
    let a = r.answered(&[], &["version", "extra"]);
    assert_eq!(a.code, 2);
    assert!(a.stdout.is_empty(), "{}", a.stdout);
    assert_eq!(
        a.stderr,
        "pile: pile version takes no arguments, and was given \"extra\"\n  \
         pile version --help  says what this command takes\n"
    );
}
