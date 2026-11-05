//! `pile guide`: the pages the binary carries, and what every page, help
//! and answer that names a page or a command promises about it.

use std::collections::BTreeSet;

use clap::error::ErrorKind;

use crate::harness::*;

fn pages() -> &'static [guide::Page] {
    &guide::carried().pages
}

#[test]
fn every_page_prints_under_its_name_with_its_text_whole() {
    let r = Repo::new();
    for p in pages() {
        let text = r.stdout(&["guide", &p.name, "--format", "text"]);
        assert!(
            text.starts_with(&format!("# {}", p.name)),
            "pile guide {} starts with its name:\n{text}",
            p.name
        );
        contains!(text, p.summary, p.body);

        let v = r.result(&["guide", &p.name]);
        let first = &v["pages"][0];
        assert_eq!(first["name"], p.name.as_str());
        assert_eq!(first["role"], p.role.word());
        assert_eq!(first["text"], p.body.as_str());
        assert_eq!(
            first["workflow"].is_object(),
            p.role == guide::Role::Workflow,
            "{} has workflow exactly when it is one",
            p.name
        );
    }
}

#[test]
fn the_guide_needs_no_repository() {
    let dir = outside();
    for p in pages() {
        let a = answered(dir.path(), &[], &["guide", &p.name]);
        assert_eq!(a.code, 0, "pile guide {} outside a repository", p.name);
    }
    for args in [&["guide"][..], &["guide", "--list"], &["guide", "--agents"]] {
        assert_eq!(answered(dir.path(), &[], args).code, 0, "{args:?}");
    }
}

#[test]
fn a_bare_guide_is_the_start_page_and_the_pages_it_needs() {
    let r = Repo::new();
    let v = r.result(&["guide"]);
    let printed: Vec<&str> = v["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    let start = guide::carried().start().unwrap();
    let mut want = vec!["start"];
    want.extend(start.needs.iter().map(String::as_str));
    assert_eq!(printed, want);
}

#[test]
fn a_page_comes_with_what_it_needs_once_and_names_what_to_see() {
    let r = Repo::new();
    let v = r
        .answered(&[], &["guide", "adopt", "intent", "--format", "json"])
        .json();
    let printed: Vec<&str> = v["result"]["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(printed, ["adopt", "intent", "boundaries"]);
    let next: Vec<&str> = v["next"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["command"].as_str().unwrap())
        .collect();
    assert!(next.contains(&"pile guide commit-a-change"), "{next:?}");
    assert!(!next.contains(&"pile guide boundaries"), "printed already");
}

#[test]
fn the_list_has_every_page_once_in_its_section() {
    let r = Repo::new();
    let text = r.stdout(&["guide", "--list", "--format", "text"]);
    for p in pages() {
        contains!(text, format!("  {}", p.name), p.summary);
    }
    contains!(text, "Workflows, each carried out from the top:\n");
    let v = r.result(&["guide", "--list"]);
    let listed = v["pages"].as_array().unwrap();
    assert_eq!(listed.len(), pages().len());
    let workflows: BTreeSet<&str> = listed
        .iter()
        .filter(|p| p["role"] == "workflow")
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert!(workflows.contains("adopt"), "{workflows:?}");
}

#[test]
fn a_page_there_is_not_is_refused_naming_the_pages_there_are() {
    let r = Repo::new();
    let v = r.refused(&["guide", "intnet"], 2);
    assert_eq!(v["error"]["code"], "unknown-page");
    contains!(v["error"]["message"].as_str().unwrap(), "adopt", "intent");
    assert_eq!(v["next"][0]["command"], "pile guide --list");
}

#[test]
fn list_and_agents_take_no_page_and_not_each_other() {
    let r = Repo::new();
    for args in [
        &["guide", "--list", "adopt"][..],
        &["guide", "--agents", "adopt"],
        &["guide", "--list", "--agents"],
    ] {
        let v = r.refused(args, 2);
        assert_eq!(v["error"]["code"], "usage", "{args:?}");
        assert_eq!(v["next"][0]["command"], "pile guide --help");
    }
}

#[test]
fn the_section_is_text_in_every_format_for_a_file() {
    let r = Repo::new();
    for env in [&[][..], &[("AGENT", "1")]] {
        let a = r.answered(env, &["guide", "--agents", "--format", "json"]);
        assert_eq!(a.code, 0);
        assert_eq!(a.stdout, guide::SECTION);
    }
}

#[test]
fn this_repository_s_agents_md_opens_with_the_section() {
    let agents = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../AGENTS.md");
    let text = std::fs::read_to_string(&agents).expect("AGENTS.md at the top");
    assert!(
        text.starts_with(guide::SECTION),
        "AGENTS.md opens with exactly what pile guide --agents prints:\n{}",
        guide::SECTION
    );
}

#[test]
fn a_page_name_completes_anywhere() {
    let dir = outside();
    let mut cmd = command(bin(), dir.path());
    cmd.env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "pile", "guide", "wr"]);
    let out = cmd.output().expect("running pile");
    assert!(out.status.success());
    contains!(String::from_utf8_lossy(&out.stdout), "write-an-intent");
}

/// The words of a command, with each `<placeholder>` given a value. A
/// command's place is given a command, and any other place a word.
fn typed(line: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for w in line.split_whitespace() {
        let word = if w == "<command>" {
            "list"
        } else if w.starts_with('<') {
            "x"
        } else {
            w
        };
        words.push(word.replace("<name>", "x").replace("<dir>", "x"));
    }
    words
}

#[test]
fn every_command_a_page_or_the_section_names_is_one_the_binary_takes() {
    let mut named: Vec<(String, String)> = Vec::new();
    for p in pages() {
        for c in guide::commands(&p.body) {
            named.push((p.name.clone(), c));
        }
    }
    for c in guide::commands(guide::SECTION) {
        named.push(("the section".into(), c));
    }
    assert!(named.len() > 50, "the pages name commands");
    let mut wrong = Vec::new();
    for (at, line) in named {
        let words = typed(&line);
        let argv: Vec<std::ffi::OsString> = words.iter().map(Into::into).collect();
        let argv = pile::answer::without_format(&argv);
        match pile::cli::command().try_get_matches_from(argv) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::DisplayHelp => {}
            Err(e) => wrong.push(format!("{at}: {line}: {}", e.kind())),
        }
        if let [_, cmd, page, ..] = words.as_slice()
            && cmd == "guide"
            && !page.starts_with('-')
            && guide::carried().find(page).is_none()
        {
            wrong.push(format!("{at}: {line}: no page {page}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The pages a text names: the word after each `pile guide` and one space
/// or one line break. More space than that is a column of examples.
fn pages_named(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("pile guide") {
        rest = &rest[at + "pile guide".len()..];
        let Some(after) = rest.strip_prefix([' ', '\n']) else {
            continue;
        };
        let word: String = after
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if word.starts_with(|c: char| c.is_ascii_lowercase()) {
            out.push(word.trim_end_matches('-').to_string());
        }
    }
    out
}

#[test]
fn every_page_a_help_or_an_answer_names_is_one_the_guide_carries() {
    let r = Repo::new();
    let mut texts = vec![("pile --help".to_string(), r.stdout(&["--help"]))];
    let mut root = pile::cli::command();
    root.build();
    for sub in root.get_subcommands() {
        let name = sub.get_name();
        texts.push((format!("pile {name} --help"), r.stdout(&[name, "--help"])));
    }
    let fresh = Repo::new();
    texts.push((
        "pile init".into(),
        fresh.stdout(&["init", "--format", "json"]),
    ));
    texts.push((
        "pile doctor".into(),
        fresh.answered(&[], &["doctor", "--format", "text"]).stdout,
    ));
    texts.push((
        "pile status".into(),
        Repo::new()
            .answered(&[], &["status", "--format", "json"])
            .stdout,
    ));
    let mut wrong = Vec::new();
    let mut seen = 0;
    for (at, text) in &texts {
        for page in pages_named(text) {
            seen += 1;
            if guide::carried().find(&page).is_none() {
                wrong.push(format!(
                    "{at} names pile guide {page}, and there is no such page"
                ));
            }
        }
    }
    assert!(seen > 10, "the helps and answers name pages");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
