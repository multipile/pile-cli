//! Agent mode: what changes when the environment says an agent is running
//! `pile`.

use crate::harness::*;

#[test]
fn either_variable_turns_it_on_and_version_says_which() {
    let r = Repo::new();
    let text = ["version", "--format", "text"];
    contains!(r.answered(&[], &text).stdout, "agent  off\n");
    contains!(
        r.answered(&[("AI_AGENT", "claude")], &text).stdout,
        "agent  on, by AI_AGENT\n"
    );
    contains!(
        r.answered(&[("AGENT", "1")], &text).stdout,
        "agent  on, by AGENT\n"
    );
    // AI_AGENT is read first.
    contains!(
        r.answered(&[("AGENT", "1"), ("AI_AGENT", "1")], &text)
            .stdout,
        "agent  on, by AI_AGENT\n"
    );
    // Set empty, both are off: how a person runs one command in an agent's
    // shell.
    contains!(
        r.answered(&[("AI_AGENT", ""), ("AGENT", "")], &text).stdout,
        "agent  off\n"
    );
}

#[test]
fn an_agent_is_answered_in_json_unless_it_asks_for_text() {
    let r = Repo::new();
    let a = r.answered(&[("AI_AGENT", "1")], &["version"]);
    assert_eq!(a.json()["result"]["agent"], "AI_AGENT");
    let a = r.answered(&[("AI_AGENT", "1")], &["version", "--format", "text"]);
    contains!(a.stdout, "pile dev\n");
}

#[test]
fn an_agent_is_refused_on_stdout_in_text_too() {
    let r = Repo::new();
    let a = r.answered(&[("AGENT", "1")], &["xyzzy", "--format", "text"]);
    assert_eq!(a.code, 2);
    assert!(a.stderr.is_empty(), "nothing on stderr: {}", a.stderr);
    assert_eq!(
        a.stdout,
        "pile: \"xyzzy\" is not a command\n  pile --help  lists every command\n"
    );
}

#[test]
fn agent_mode_does_not_change_the_exit_code() {
    let r = Repo::new();
    for env in [&[][..], &[("AGENT", "1")]] {
        assert_eq!(r.answered(env, &["version", "--frmat"]).code, 2);
        assert_eq!(r.answered(env, &["version"]).code, 0);
    }
}

#[test]
fn a_bare_pile_tells_an_agent_where_it_is_in_one_answer() {
    let r = crate::piles::nested();
    let a = r.answered(&[("AGENT", "1")], &[]);
    assert_eq!(a.code, 0, "{}", a.stderr);
    let v = a.json();
    assert_eq!(v["result"]["version"]["version"], "dev");
    assert_eq!(v["result"]["piles"].as_array().unwrap().len(), 4);
    assert_eq!(v["result"]["status"]["total"]["piles"], 4);
    assert_eq!(v["next"][0]["command"], "pile guide");
    assert_eq!(v["next"][1]["command"], "pile show <pile>");
    let text = r.answered(&[("AGENT", "1")], &["--format", "text"]);
    contains!(
        text.stdout,
        "pile dev\n",
        "@lib  :/lib ",
        "unchanged\n",
        "  pile show <pile>  "
    );
}

#[test]
fn where_there_is_nothing_to_orient_in_a_bare_pile_prints_the_help() {
    let dir = outside();
    let a = answered(dir.path(), &[("AGENT", "1")], &[]);
    assert_eq!(a.code, 0);
    assert!(
        a.stdout
            .starts_with("pile \u{2014} https://multipile.dev\n"),
        "{}",
        a.stdout
    );
    let r = Repo::new();
    let b = r.answered(&[("AGENT", "1")], &[]);
    assert!(
        b.stdout
            .starts_with("pile \u{2014} https://multipile.dev\n"),
        "{}",
        b.stdout
    );
    let person = crate::piles::nested().answered(&[], &[]);
    assert!(
        person
            .stdout
            .starts_with("pile \u{2014} https://multipile.dev\n"),
        "{}",
        person.stdout
    );
}
