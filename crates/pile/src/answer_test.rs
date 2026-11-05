use super::*;

fn argv(words: &[&str]) -> Vec<OsString> {
    std::iter::once("pile")
        .chain(words.iter().copied())
        .map(OsString::from)
        .collect()
}

fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
    move |key| {
        pairs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| OsString::from(v))
    }
}

const PERSON: Mode = Mode {
    format: Format::Text,
    agent: None,
};

const JSON: Mode = Mode {
    format: Format::Json,
    agent: None,
};

#[test]
fn the_format_is_read_anywhere_before_the_double_dash() {
    let (m, bad) = Mode::of(&argv(&["version", "--format", "json"]), env(&[]));
    assert_eq!((m.format, bad), (Format::Json, None));
    let (m, _) = Mode::of(&argv(&["--format=json", "version"]), env(&[]));
    assert_eq!(m.format, Format::Json);
    let (m, _) = Mode::of(&argv(&["version", "--", "--format", "json"]), env(&[]));
    assert_eq!(m.format, Format::Text);
    let (_, bad) = Mode::of(&argv(&["version", "--format", "yaml"]), env(&[]));
    assert_eq!(bad.as_deref(), Some("yaml"));
}

#[test]
fn an_agent_is_answered_in_json_unless_it_asks_for_text() {
    let (m, _) = Mode::of(&argv(&["version"]), env(&[("AGENT", "1")]));
    assert_eq!(m.format, Format::Json);
    assert_eq!(m.agent, Some("AGENT"));
    let (m, _) = Mode::of(
        &argv(&["version", "--format", "text"]),
        env(&[("AGENT", "1"), ("AI_AGENT", "x")]),
    );
    assert_eq!(m.format, Format::Text);
    assert_eq!(m.agent, Some("AI_AGENT"));
    let (m, _) = Mode::of(&argv(&[]), env(&[("AI_AGENT", ""), ("AGENT", "")]));
    assert_eq!(m.agent, None);
}

#[test]
fn the_format_is_taken_off_the_line_clap_sees() {
    let kept = without_format(&argv(&[
        "--format",
        "json",
        "help",
        "--format=text",
        "version",
    ]));
    assert_eq!(kept, argv(&["help", "version"]));
    let kept = without_format(&argv(&["x", "--", "--format", "json"]));
    assert_eq!(kept, argv(&["x", "--", "--format", "json"]));
}

#[test]
fn a_word_a_shell_would_misread_is_quoted() {
    assert_eq!(
        command_line(&argv(&["show", "@lib", "it's here"])),
        r"pile show @lib 'it'\''s here'"
    );
}

#[test]
fn text_prints_the_text_and_json_prints_the_envelope() {
    let answer = || {
        let mut out = Out::new();
        out.push_str("hello\n");
        out.result(json!({"greeting": "hello"}));
        out
    };
    let p = finish(PERSON, "pile hi", answer(), Ok(()));
    assert_eq!((p.stdout.as_str(), p.code), ("hello\n", 0));

    let p = finish(JSON, "pile hi", answer(), Ok(()));
    let v: Value = serde_json::from_str(&p.stdout).unwrap();
    assert_eq!(
        v,
        json!({"ok": true, "command": "pile hi", "result": {"greeting": "hello"}, "next": []})
    );
}

#[test]
fn a_raw_answer_is_text_in_every_format() {
    let mut out = Out::new();
    out.push_str("dev\n");
    out.raw = true;
    let p = finish(JSON, "pile version --short", out, Ok(()));
    assert_eq!(p.stdout, "dev\n");
}

#[test]
fn a_refusal_goes_to_stderr_for_a_person_and_stdout_for_an_agent() {
    let refusal = || Refusal::new(Code::Usage, "no").next("pile --help", "lists every command");
    let p = finish(PERSON, "pile x", Out::new(), Err(refusal()));
    assert_eq!(p.stdout, "");
    assert_eq!(p.stderr, "pile: no\n  pile --help  lists every command\n");
    assert_eq!(p.code, 2);

    let agent = Mode {
        format: Format::Text,
        agent: Some("AGENT"),
    };
    let p = finish(agent, "pile x", Out::new(), Err(refusal()));
    assert_eq!(p.stdout, "pile: no\n  pile --help  lists every command\n");
    assert_eq!(p.stderr, "");
}

#[test]
fn a_structured_refusal_is_an_envelope_with_its_code_and_what_fixes_it() {
    let mut out = Out::new();
    out.next("pile --help", "");
    let refusal = Refusal::new(Code::Failed, "it broke").next("pile --help", "to read");
    let p = finish(JSON, "pile x", out, Err(refusal));
    assert_eq!(p.code, 1);
    assert_eq!(p.stderr, "");
    let v: Value = serde_json::from_str(&p.stdout).unwrap();
    assert_eq!(
        v,
        json!({
            "ok": false,
            "command": "pile x",
            "error": {"code": "failed", "message": "it broke", "usage": false},
            "next": [{"command": "pile --help", "why": "to read"}],
        })
    );
    let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["ok", "command", "error", "next"]);
}
