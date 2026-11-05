//! How an answer is printed: text or JSON, the envelope, and the refusals
//! with their codes and exit codes.

use serde_json::json;

use crate::harness::*;

#[test]
fn a_structured_answer_is_one_envelope_on_stdout() {
    let r = Repo::new();
    let a = r.answered(&[], &["version", "--format", "json"]);
    assert_eq!(a.code, 0);
    assert!(a.stderr.is_empty(), "nothing on stderr: {}", a.stderr);
    let v = a.json();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "pile version --format json");
    assert_eq!(v["next"], json!([]));
    let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["ok", "command", "result", "next"]);
}

#[test]
fn the_format_may_come_anywhere_on_the_line() {
    let r = Repo::new();
    for line in [
        &["--format", "json", "version"][..],
        &["--format=json", "version"],
        &["version", "--format=json"],
    ] {
        let v = r.answered(&[], line).json();
        assert_eq!(v["ok"], true, "{line:?}");
    }
}

#[test]
fn a_refusal_in_json_is_an_envelope_with_its_code_and_what_fixes_it() {
    let r = Repo::new();
    let a = r.answered(&[], &["xyzzy", "--format", "json"]);
    assert_eq!(a.code, 2);
    assert!(a.stderr.is_empty(), "nothing on stderr: {}", a.stderr);
    let v = a.json();
    assert_eq!(
        v,
        json!({
            "ok": false,
            "command": "pile xyzzy --format json",
            "error": {"code": "unknown-command", "message": "\"xyzzy\" is not a command", "usage": true},
            "next": [{"command": "pile --help", "why": "lists every command"}],
        })
    );
    let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["ok", "command", "error", "next"]);
}

#[test]
fn each_mistake_in_what_was_typed_has_its_code_and_exits_2() {
    let r = Repo::new();
    for (line, code, help) in [
        (&["verson"][..], "unknown-command", "pile --help"),
        (
            &["version", "--frmat"],
            "unknown-flag",
            "pile version --help",
        ),
        (&["version", "extra"], "usage", "pile version --help"),
        (&["completion"], "usage", "pile completion --help"),
        (&["completion", "tcsh"], "usage", "pile completion --help"),
    ] {
        let mut args = line.to_vec();
        args.extend(["--format", "json"]);
        let a = r.answered(&[], &args);
        assert_eq!(a.code, 2, "{line:?}:\n{}", a.stdout);
        let v = a.json();
        assert_eq!(v["error"]["code"], code, "{line:?}");
        assert_eq!(v["error"]["usage"], true, "{line:?}");
        assert_eq!(v["next"][0]["command"], help, "{line:?}");
    }
}

#[test]
fn a_command_clap_could_guess_says_which() {
    let r = Repo::new();
    let a = r.answered(&[], &["verson"]);
    assert_eq!(a.code, 2);
    contains!(
        a.stderr,
        "pile: \"verson\" is not a command. Did you mean version?"
    );
}

#[test]
fn a_format_that_is_not_one_is_refused_listing_them() {
    let r = Repo::new();
    let a = r.answered(&[], &["version", "--format", "yaml"]);
    assert_eq!(a.code, 2);
    assert_eq!(a.stderr, "pile: --format is text or json, not \"yaml\"\n");
}

#[test]
fn text_refuses_on_stderr_and_leaves_stdout_empty() {
    let r = Repo::new();
    let a = r.answered(&[], &["xyzzy"]);
    assert_eq!(a.code, 2);
    assert!(a.stdout.is_empty(), "{}", a.stdout);
    assert_eq!(
        a.stderr,
        "pile: \"xyzzy\" is not a command\n  pile --help  lists every command\n"
    );
}

#[test]
fn help_is_text_in_every_format_and_exits_0() {
    let r = Repo::new();
    for line in [
        &[][..],
        &["--help"],
        &["help", "version"],
        &["version", "-h"],
    ] {
        let mut args = line.to_vec();
        args.extend(["--format", "json"]);
        let a = r.answered(&[], &args);
        assert_eq!(a.code, 0, "{line:?}");
        assert!(
            a.stdout.starts_with(char::is_alphabetic),
            "{line:?}:\n{}",
            a.stdout
        );
    }
    let out = r.stdout(&["version", "--help"]);
    contains!(out, "Usage: pile version [--short]", "--format <text|json>");
}

#[test]
fn help_names_a_command_and_prints_what_its_help_flag_prints() {
    let r = Repo::new();
    assert_eq!(
        r.stdout(&["help", "version"]),
        r.stdout(&["version", "--help"])
    );
    assert_eq!(r.stdout(&["help"]), r.stdout(&["--help"]));
}

#[test]
fn help_for_a_command_that_is_not_one_is_refused() {
    let r = Repo::new();
    let a = r.answered(&[], &["help", "xyzzy"]);
    assert_eq!(a.code, 2);
    assert_eq!(
        a.stderr,
        "pile: \"xyzzy\" is not a command\n  pile --help  lists every command\n"
    );
}
