//! What a page is held to (the crate's doc), held of every page the binary
//! carries, and the reading of a page held to small ones written here.

use std::collections::BTreeSet;
use std::path::Path;

use super::*;

fn pages() -> &'static [Page] {
    &carried().pages
}

fn page(name: &str) -> &'static Page {
    carried()
        .find(name)
        .unwrap_or_else(|| panic!("no page {name}"))
}

#[test]
fn every_file_in_pages_is_a_page_that_reads() {
    let problems: Vec<String> = carried()
        .problems
        .iter()
        .map(|p| format!("pages/{}: {}", p.file, p.message))
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("pages");
    let files = std::fs::read_dir(dir).expect("pages/").count();
    assert_eq!(files, pages().len(), "every file is one page");
}

#[test]
fn every_page_is_short_narrow_and_says_in_one_line_what_it_answers() {
    let mut wrong = Vec::new();
    for p in pages() {
        let at = &p.name;
        let summary = p.summary.chars().count();
        if summary == 0 || summary > SUMMARY_WIDEST {
            wrong.push(format!(
                "{at}: the summary is {summary} characters, and is 1 to {SUMMARY_WIDEST}"
            ));
        }
        if p.body.trim().is_empty() {
            wrong.push(format!("{at}: the body is empty"));
        }
        if p.body.starts_with("# ") {
            wrong.push(format!(
                "{at}: the body opens with a heading, and the page's name is printed above it"
            ));
        }
        if !p.body.ends_with('\n') || p.body.ends_with("\n\n") {
            wrong.push(format!("{at}: the body ends in exactly one newline"));
        }
        let lines = p.body.lines().count();
        if lines > MOST_LINES {
            wrong.push(format!(
                "{at}: {lines} lines, and a page is at most {MOST_LINES}"
            ));
        }
        for (i, line) in p.body.lines().enumerate() {
            let width = line.chars().count();
            if width > WIDEST {
                wrong.push(format!(
                    "{at}: line {} is {width} columns, and a page is at most {WIDEST}",
                    i + 1
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn there_is_one_start_page_and_it_is_listed_first() {
    let starts: Vec<&str> = pages()
        .iter()
        .filter(|p| p.role == Role::Start)
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(starts, ["start"]);
    assert_eq!(pages()[0].name, "start");
}

#[test]
fn every_reference_is_to_a_page_and_every_page_is_reached_from_the_start() {
    let mut wrong = Vec::new();
    for p in pages() {
        let mut declared = BTreeSet::new();
        for name in p.needs.iter().chain(&p.see) {
            if carried().find(name).is_none() {
                wrong.push(format!("{} refers to {name:?}, which is no page", p.name));
            }
            if *name == p.name {
                wrong.push(format!("{} refers to itself", p.name));
            }
            if !declared.insert(name.as_str()) {
                wrong.push(format!("{} refers to {name:?} twice", p.name));
            }
        }
        for name in printed_in(&p.body) {
            if carried().find(&name).is_none() {
                wrong.push(format!(
                    "{} tells its reader to print pile guide {name}, and there is no such page",
                    p.name
                ));
            } else if name != p.name && !declared.contains(name.as_str()) {
                wrong.push(format!(
                    "{} sends its reader to pile guide {name} without naming it in needs or \
                     see, so its foot leaves it out",
                    p.name
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));

    let start = carried().start().expect("a start page");
    let mut reached = BTreeSet::from([start.name.as_str()]);
    let mut queue = vec![start];
    while let Some(p) = queue.pop() {
        for name in p.needs.iter().chain(&p.see) {
            let next = page(name);
            if reached.insert(next.name.as_str()) {
                queue.push(next);
            }
        }
    }
    let unreached: Vec<&str> = pages()
        .iter()
        .map(|p| p.name.as_str())
        .filter(|n| !reached.contains(n))
        .collect();
    assert!(
        unreached.is_empty(),
        "nothing reaches these from the start page: {unreached:?}"
    );
}

#[test]
fn a_page_with_what_it_needs_fits_in_a_session() {
    for p in pages() {
        let lines: usize = carried()
            .with_needs(&[p])
            .iter()
            .map(|q| q.body.lines().count())
            .sum();
        assert!(
            lines <= 3 * MOST_LINES,
            "pile guide {} prints {lines} lines of pages",
            p.name
        );
    }
}

#[test]
fn every_workflow_has_its_parts_and_every_step_its_commands() {
    let workflows: Vec<&Page> = pages()
        .iter()
        .filter(|p| p.role == Role::Workflow)
        .collect();
    assert!(!workflows.is_empty(), "the guide carries workflows");
    for p in workflows {
        let w = p
            .workflow
            .as_ref()
            .unwrap_or_else(|| panic!("{} is a workflow whose parts do not read", p.name));
        assert!(!w.when.is_empty() && !w.first.is_empty() && !w.hand_back.is_empty());
        assert!(
            !w.done_when.is_empty(),
            "{} says nothing under Done when",
            p.name
        );
        for s in &w.steps {
            assert!(!s.run.is_empty(), "{}: {} runs nothing", p.name, s.title);
            assert!(!s.look_for.is_empty(), "{}: {}", p.name, s.title);
        }
    }
    for p in pages().iter().filter(|p| p.role != Role::Workflow) {
        assert!(p.workflow.is_none());
    }
}

#[test]
fn needs_are_followed_breadth_first_and_each_page_comes_once() {
    let g = Guide::parse(&[
        (
            "a.md",
            "+++\nrole = start\nsummary = a\nneeds = b c\n+++\nA\n",
        ),
        (
            "b.md",
            "+++\nrole = reference\nsummary = b\nneeds = d c\n+++\nB\n",
        ),
        (
            "c.md",
            "+++\nrole = reference\nsummary = c\nneeds = a\n+++\nC\n",
        ),
        ("d.md", "+++\nrole = reference\nsummary = d\n+++\nD\n"),
    ]);
    assert!(g.problems.is_empty(), "{:?}", g.problems);
    let names = |asked: &[&str]| -> Vec<String> {
        let asked: Vec<&Page> = asked.iter().map(|n| g.find(n).unwrap()).collect();
        g.with_needs(&asked)
            .iter()
            .map(|p| p.name.clone())
            .collect()
    };
    assert_eq!(names(&["a"]), ["a", "b", "c", "d"]);
    assert_eq!(names(&["d", "c"]), ["d", "c", "a", "b"]);
    assert_eq!(names(&["d", "d"]), ["d"]);
}

#[test]
fn a_page_that_does_not_read_says_why() {
    for (file, text, why) in [
        (
            "A.md",
            "+++\nrole = start\nsummary = a\n+++\nA\n",
            "<name>.md",
        ),
        ("a.md", "role = start\n", "opens with front matter"),
        ("a.md", "+++\nrole = start\nsummary = a\n", "never closed"),
        ("a.md", "+++\nsummary = a\n+++\nA\n", "no role"),
        (
            "a.md",
            "+++\nrole = job\nsummary = a\n+++\nA\n",
            "not \"job\"",
        ),
        (
            "a.md",
            "+++\nrole = start\nwhy = a\n+++\nA\n",
            "why is not a key",
        ),
        (
            "a.md",
            "+++\nkind = start\nsummary = a\n+++\nA\n",
            "kind is not a key of a page: role, summary",
        ),
        (
            "a.md",
            "+++\nrole = workflow\nsummary = a\n+++\n## Steps\n",
            "## When to use",
        ),
    ] {
        let g = Guide::parse(&[(file, text)]);
        assert!(
            g.problems.iter().any(|p| p.message.contains(why)),
            "{text:?}: {:?}",
            g.problems
        );
    }
}

const WORKFLOW: &str = "+++
role = workflow
summary = w
+++
## When to use

Now.

## What to have first

A repository.

## Steps

### 1. Look

```sh
# a comment
pile list
git log -n 3
```

Look for the piles.

### 2. Check

Before the commit:

```sh
pile doctor
```

It exits 0.

## Done when

- `pile doctor` exits 0, and
  says nothing is wrong.
- The rest.

## What to hand back

The list.
";

#[test]
fn a_workflow_is_read_into_its_parts() {
    let g = Guide::parse(&[("w.md", WORKFLOW)]);
    assert!(g.problems.is_empty(), "{:?}", g.problems);
    let w = g.find("w").unwrap().workflow.clone().unwrap();
    assert_eq!(w.when, "Now.");
    assert_eq!(w.first, "A repository.");
    assert_eq!(w.steps.len(), 2);
    assert_eq!(w.steps[0].title, "Look");
    assert_eq!(w.steps[0].run, ["pile list", "git log -n 3"]);
    assert_eq!(w.steps[0].look_for, "Look for the piles.");
    assert_eq!(w.steps[1].about, "Before the commit:");
    assert_eq!(
        w.done_when,
        [
            "`pile doctor` exits 0, and says nothing is wrong.",
            "The rest."
        ]
    );
    assert_eq!(w.hand_back, "The list.");
}

#[test]
fn a_workflow_missing_a_part_or_a_command_is_a_problem() {
    for (from, to, why) in [
        ("### 2. Check", "### 3. Check", "starts \"2. \""),
        ("```sh\npile doctor\n```\n", "", "one ```sh block"),
        ("It exits 0.\n", "", "what to look for"),
        ("## Done when", "## Proof", "in that order"),
        ("- The rest.", "The rest.", "a list"),
    ] {
        let text = WORKFLOW.replace(from, to);
        let g = Guide::parse(&[("w.md", &text)]);
        assert!(
            g.problems.iter().any(|p| p.message.contains(why)),
            "{from:?} -> {to:?}: {:?}",
            g.problems
        );
    }
}

#[test]
fn the_commands_a_text_names_are_found_in_code_and_split_as_a_shell_would() {
    let text = "Run `pile list`, not pile show.\n\n```sh\ncd lib && pile init --set name=lib\n\
                pile guide --agents >> AGENTS.md\npile doctor   # the verdict\ngit log\n```\n";
    assert_eq!(
        commands(text),
        [
            "pile list",
            "pile init --set name=lib",
            "pile guide --agents",
            "pile doctor"
        ]
    );
}

#[test]
fn the_section_is_a_heading_and_one_block_that_sends_to_the_guide() {
    let (heading, body) = SECTION.split_once("\n\n").expect("a heading, then a block");
    assert!(heading.starts_with("## ") && !heading.contains('\n'));
    let date = heading
        .strip_prefix("## Piles (")
        .and_then(|d| d.strip_suffix(')'))
        .expect("the heading is ## Piles (YYYY-MM-DD)");
    let parts: Vec<&str> = date.split('-').collect();
    assert!(
        parts.len() == 3
            && [4, 2, 2].iter().zip(&parts).all(|(n, p)| p.len() == *n)
            && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())),
        "the heading's date is YYYY-MM-DD: {date}"
    );
    assert!(SECTION.ends_with('\n') && !SECTION.ends_with("\n\n"));
    assert!(
        !body.trim_end().contains("\n\n"),
        "the section is one block under its heading, with no blank line"
    );
    assert!(
        commands(SECTION).iter().any(|c| c == "pile guide"),
        "the section sends a session to pile guide"
    );
    assert!(
        printed_in(SECTION).is_empty(),
        "the section names no page: the guide names its pages"
    );
}

/// The pages a text tells its reader to print: the first word after each
/// `pile guide` command it names, when it is a word and not a flag.
fn printed_in(text: &str) -> BTreeSet<String> {
    commands(text)
        .iter()
        .filter_map(|c| c.strip_prefix("pile guide "))
        .filter_map(|rest| rest.split_whitespace().next())
        .filter(|w| !w.starts_with('-') && !w.starts_with('<'))
        .map(String::from)
        .collect()
}
