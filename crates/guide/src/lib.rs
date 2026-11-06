//! The guide: pages of prose compiled into the `pile` binary, which prints
//! them with `pile guide`.
//!
//! A page is a file in pages/, named `<name>.md`. It opens with front matter
//! between two `+++` lines, written as a pile's config is: one `key = value`
//! a line, and a line starting with `#` for a comment.
//!
//! - `role` is `start`, `workflow` or `reference`: what the page is for in
//!   a session. There is one start page, and a bare `pile guide` prints it.
//! - `summary` is one line saying what the page answers. Every list and every
//!   reference prints it, taken from the page, so it is never written twice.
//! - `needs` is the pages a reader cannot act on this one without, by name,
//!   with spaces between. Asking for a page prints these too, and the pages
//!   they need, each once ([`Guide::with_needs`]).
//! - `see` is the pages a reader may want next. They are named, and never
//!   printed for them.
//!
//! A page is addressed by its name and nothing else. Its role says which
//! part of `pile guide --list` it is listed in.
//!
//! # A workflow
//!
//! A workflow is a page that carries out one thing from the top. Its body is
//! these five sections, as `##` headings, in this order, and nothing else:
//!
//! - `## When to use`: the moment it is for, and when it is not.
//! - `## What to have first`: what has to be true, or to hand, before step 1.
//! - `## Steps`: each step a `### <n>. <title>`, numbered from 1. A step
//!   holds one fenced `sh` block, the exact commands to run. The text above
//!   the block is optional, and says what the step is for. The text under it
//!   is required, and says what to look for in the answers.
//! - `## Done when`: a list, each item one thing that holds once the
//!   workflow is done.
//! - `## What to hand back`: what the reader gives to whoever asked.
//!
//! [`Workflow`] is those parts read out of the body, so a host can turn each
//! workflow into a skill of its own without reading prose.
//!
//! # What a page is held to
//!
//! The tests below hold every page to these, and the suite beside the
//! commands holds the rest:
//!
//! - its front matter reads, and its body is at most [`MOST_LINES`] lines,
//!   none wider than [`WIDEST`] columns, with no heading on top: the page's
//!   name is printed above it;
//! - its summary is one line of at most [`SUMMARY_WIDEST`] characters;
//! - every page it needs or sees is a page, and every page a code span in it
//!   tells somebody to print with `pile guide <name>` is one it needs or sees;
//! - every page is reached from the start page along `needs` and `see`;
//! - a workflow has its five parts, and each step its commands;
//! - every `pile` command a page names is one the binary takes, with those
//!   flags (the suite in crates/pile, which has the binary).
//!
//! Whether what a page says is true, no test can say. A change to a command is
//! not done until the pages that name it are read again.

use std::collections::BTreeSet;
use std::sync::OnceLock;

/// The most lines a page's body may have. A page answers one question, and a
/// page with what it needs has to fit in a session.
pub const MOST_LINES: usize = 100;

/// The widest a line of a page may be: a terminal.
pub const WIDEST: usize = 80;

/// The longest a summary may be, since every list prints it on one line.
pub const SUMMARY_WIDEST: usize = 72;

/// The section a repository keeps at the top of the file its agents read
/// first, AGENTS.md or whatever its harness reads: a heading, and one
/// paragraph that sends a session to `pile guide`. It is the one copy of the
/// guide kept on disk, so it says as little as can be said, and names no
/// page.
pub const SECTION: &str = include_str!("../section.md");

/// Every file in pages/, by name and contents, as build.rs found them.
const FILES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/pages.rs"));

/// What a page is for in a session, which is also which part of the guide's
/// list it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Where a session begins. There is one.
    Start,
    /// One thing carried out from the top, step by step.
    Workflow,
    /// What something is, read when it applies.
    Reference,
}

impl Role {
    pub fn word(self) -> &'static str {
        match self {
            Role::Start => "start",
            Role::Workflow => "workflow",
            Role::Reference => "reference",
        }
    }

    fn parse(word: &str) -> Option<Role> {
        [Role::Start, Role::Workflow, Role::Reference]
            .into_iter()
            .find(|k| k.word() == word)
    }
}

/// One page, as its file says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub name: String,
    pub role: Role,
    pub summary: String,
    pub needs: Vec<String>,
    pub see: Vec<String>,
    /// The page's text, after its front matter.
    pub body: String,
    /// A workflow's parts. None for any other page, and for a workflow
    /// whose parts do not read, which is then a problem of the guide.
    pub workflow: Option<Workflow>,
}

/// A workflow's parts, read out of its body (the crate's doc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workflow {
    pub when: String,
    pub first: String,
    pub steps: Vec<Step>,
    pub done_when: Vec<String>,
    pub hand_back: String,
}

/// One step of a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The title, without its number.
    pub title: String,
    /// What the step is for, or empty.
    pub about: String,
    /// The commands, a line each, as they are typed.
    pub run: Vec<String>,
    /// What to look for in the answers.
    pub look_for: String,
}

/// A file in pages/ that is not a page as it should be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub file: String,
    pub message: String,
}

/// The pages, in the order `pile guide --list` prints them: the start page,
/// then the workflows, then the reference pages, each section by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guide {
    pub pages: Vec<Page>,
    pub problems: Vec<Problem>,
}

/// The guide this binary carries, read from its files the first time it is
/// asked for.
pub fn carried() -> &'static Guide {
    static GUIDE: OnceLock<Guide> = OnceLock::new();
    GUIDE.get_or_init(|| Guide::parse(FILES))
}

impl Guide {
    /// The guide made of files, each a name and its contents. A file that is
    /// not a page is left out, and named among the problems.
    pub fn parse(files: &[(&str, &str)]) -> Guide {
        let mut guide = Guide {
            pages: Vec::new(),
            problems: Vec::new(),
        };
        for (file, text) in files {
            let problem = |message: String| Problem {
                file: file.to_string(),
                message,
            };
            let Some(name) = page_name(file) else {
                guide.problems.push(problem(
                    "a page's file is <name>.md, and a name is lowercase words joined by hyphens"
                        .into(),
                ));
                continue;
            };
            match read_page(name, text) {
                Ok((page, None)) => guide.pages.push(page),
                Ok((page, Some(why))) => {
                    guide.problems.push(problem(why));
                    guide.pages.push(page);
                }
                Err(why) => guide.problems.push(problem(why)),
            }
        }
        guide
            .pages
            .sort_by(|a, b| (a.role, &a.name).cmp(&(b.role, &b.name)));
        guide
    }

    /// The page named, if there is one.
    pub fn find(&self, name: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.name == name)
    }

    /// The start page, which a bare `pile guide` prints.
    pub fn start(&self) -> Option<&Page> {
        self.pages.iter().find(|p| p.role == Role::Start)
    }

    /// The pages asked for, each followed by the pages it needs, and those by
    /// the pages they need, breadth first. Each page is printed once, where
    /// it is first reached.
    pub fn with_needs<'a>(&'a self, asked: &[&'a Page]) -> Vec<&'a Page> {
        let mut seen = BTreeSet::new();
        let mut out: Vec<&Page> = Vec::new();
        for page in asked {
            if seen.insert(page.name.as_str()) {
                out.push(page);
            }
        }
        let mut i = 0;
        while i < out.len() {
            for need in out[i].needs.iter().filter_map(|n| self.find(n)) {
                if seen.insert(need.name.as_str()) {
                    out.push(need);
                }
            }
            i += 1;
        }
        out
    }
}

/// The name a file gives its page: `paths.md` is `paths`.
fn page_name(file: &str) -> Option<&str> {
    let name = file.strip_suffix(".md")?;
    is_name(name).then_some(name)
}

/// A page's name: lowercase letters, digits and hyphens, beginning with a
/// letter, as a pile's name is.
fn is_name(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_lowercase())
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !word.ends_with('-')
        && !word.contains("--")
}

/// A page from its file, and the problem with its workflow's parts if it is
/// a workflow whose parts do not read. Err when the page cannot be read.
fn read_page(name: &str, text: &str) -> Result<(Page, Option<String>), String> {
    let mut lines = text.split_inclusive('\n');
    if lines.next().map(str::trim_end) != Some("+++") {
        return Err("a page opens with front matter between two +++ lines".into());
    }
    let (mut role, mut summary, mut needs, mut see) = (None, None, Vec::new(), Vec::new());
    let mut closed = false;
    for (i, line) in lines.by_ref().enumerate() {
        let line = line.trim();
        if line == "+++" {
            closed = true;
            break;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = i + 2;
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {at} is not key = value"));
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "role" => {
                role = Some(Role::parse(value).ok_or_else(|| {
                    format!("line {at}: role is start, workflow or reference, not {value:?}")
                })?)
            }
            "summary" => summary = Some(value.to_string()),
            "needs" => needs = value.split_whitespace().map(String::from).collect(),
            "see" => see = value.split_whitespace().map(String::from).collect(),
            other => {
                return Err(format!(
                    "line {at}: {other} is not a key of a page: role, summary, needs and see are"
                ));
            }
        }
    }
    if !closed {
        return Err("the front matter is opened with +++ and never closed".into());
    }
    let role = role.ok_or("the front matter says no role")?;
    let summary = summary.ok_or("the front matter says no summary")?;
    let body: String = lines.collect();
    let body = body.trim_start_matches('\n').to_string();
    let (workflow, problem) = if role == Role::Workflow {
        match read_workflow(&body) {
            Ok(w) => (Some(w), None),
            Err(why) => (None, Some(why)),
        }
    } else {
        (None, None)
    };
    let page = Page {
        name: name.to_string(),
        role,
        summary,
        needs,
        see,
        body,
        workflow,
    };
    Ok((page, problem))
}

/// The headings a workflow's body is made of, in order.
pub const PARTS: [&str; 5] = [
    "When to use",
    "What to have first",
    "Steps",
    "Done when",
    "What to hand back",
];

/// A workflow's parts, read out of its body.
fn read_workflow(body: &str) -> Result<Workflow, String> {
    let sections = split(body, "## ");
    if let Some((_, before)) = sections.first().filter(|(h, _)| h.is_none())
        && !before.trim().is_empty()
    {
        return Err(format!(
            "a workflow's body opens with ## {}, and this one has text before it",
            PARTS[0]
        ));
    }
    let headed: Vec<(&str, &str)> = sections
        .iter()
        .filter_map(|(h, text)| h.map(|h| (h, *text)))
        .collect();
    let headings: Vec<&str> = headed.iter().map(|(h, _)| *h).collect();
    if headings != PARTS {
        return Err(format!(
            "a workflow's body is the sections {}, in that order, and this one has {}",
            PARTS.map(|p| format!("## {p}")).join(", "),
            headings
                .iter()
                .map(|h| format!("## {h}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let text = |i: usize| -> Result<String, String> {
        let t = headed[i].1.trim();
        if t.is_empty() {
            Err(format!("## {} says nothing", PARTS[i]))
        } else {
            Ok(t.to_string())
        }
    };
    let mut steps = Vec::new();
    for (title, text) in split(headed[2].1, "### ") {
        let Some(title) = title else {
            if text.trim().is_empty() {
                continue;
            }
            return Err("## Steps holds only steps, each under a ### heading".into());
        };
        let step = read_step(steps.len() + 1, title, text);
        steps.push(step.map_err(|why| format!("step {title:?}: {why}"))?);
    }
    if steps.is_empty() {
        return Err("## Steps has no step".into());
    }
    let done_when = items(headed[3].1)?;
    Ok(Workflow {
        when: text(0)?,
        first: text(1)?,
        steps,
        done_when,
        hand_back: text(4)?,
    })
}

/// One step: its number and title, then text, one `sh` block, and text.
fn read_step(n: usize, title: &str, text: &str) -> Result<Step, String> {
    let Some(title) = title.strip_prefix(&format!("{n}. ")) else {
        return Err(format!("it is step {n}, so its heading starts \"{n}. \""));
    };
    let Some((about, rest)) = text.split_once("```sh\n") else {
        return Err("a step holds one ```sh block, the commands to run".into());
    };
    let Some((block, look_for)) = rest.split_once("```") else {
        return Err("its ```sh block is never closed".into());
    };
    if look_for.contains("```") {
        return Err("a step holds one block of commands, and this one has more".into());
    }
    let run: Vec<String> = block
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect();
    if run.is_empty() {
        return Err("its ```sh block holds no command".into());
    }
    let look_for = look_for.trim();
    if look_for.is_empty() {
        return Err("the text under its commands says what to look for, and there is none".into());
    }
    Ok(Step {
        title: title.trim().to_string(),
        about: about.trim().to_string(),
        run,
        look_for: look_for.to_string(),
    })
}

/// The items of a list, each joined into one line.
fn items(text: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(item) = t.strip_prefix("- ") {
            out.push(item.to_string());
        } else if let Some(last) = out.last_mut().filter(|_| line.starts_with("  ")) {
            last.push(' ');
            last.push_str(t);
        } else {
            return Err("## Done when is a list, each item starting \"- \"".into());
        }
    }
    if out.is_empty() {
        return Err("## Done when has no item".into());
    }
    Ok(out)
}

/// The text under each heading that starts with marker, outside fenced code:
/// the heading without its marker, and what follows it up to the next. Text
/// before the first heading comes first, with no heading.
fn split<'a>(text: &'a str, marker: &str) -> Vec<(Option<&'a str>, &'a str)> {
    let mut out = Vec::new();
    let mut heading = None;
    let mut from = 0;
    let mut at = 0;
    let mut fenced = false;
    for line in text.split_inclusive('\n') {
        if line.starts_with("```") {
            fenced = !fenced;
        } else if !fenced && let Some(h) = line.strip_prefix(marker) {
            out.push((heading, &text[from..at]));
            heading = Some(h.trim());
            from = at + line.len();
        }
        at += line.len();
    }
    out.push((heading, &text[from..]));
    out
}

/// The `pile` commands a text tells its reader to type: each command in a
/// fenced block or an inline code span that starts with `pile`. A line is
/// split where a shell would split it, at `&&`, `;` and `|`, and each command
/// ends at a redirection or a `#` comment.
pub fn commands(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for span in code(text) {
        for part in span.split(['|', ';']).flat_map(|p| p.split("&&")) {
            let part = part.trim();
            let end = [" >", " #"]
                .iter()
                .filter_map(|m| part.find(m))
                .min()
                .unwrap_or(part.len());
            let command = part[..end].trim();
            if command == "pile" || command.starts_with("pile ") {
                out.push(command.to_string());
            }
        }
    }
    out
}

/// Every fenced block's lines, and every inline code span on one line
/// outside them.
fn code(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            out.push(line);
            continue;
        }
        let mut rest = line;
        while let Some(open) = rest.find('`') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('`') else {
                break;
            };
            out.push(&after[..close]);
            rest = &after[close + 1..];
        }
    }
    out
}

#[cfg(test)]
mod tests;
