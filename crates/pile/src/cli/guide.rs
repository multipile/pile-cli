//! `pile guide`: the pages this binary carries (crates/guide).
//!
//! It reads no repository. The guide is about the binary, and the page for
//! a project that has no pile yet is read before there is any pile to read.

use clap::{Arg, ArgAction, ArgMatches, Command};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use serde_json::{Map, Value, json};

use guide::{Page, Role, Workflow};

use super::{Out, Result, switch};
use crate::exit::{Code, Refusal};

pub(super) fn command() -> Command {
    Command::new("guide")
        .about("Print the guide: where a session begins, workflows, and reference pages")
        .long_about(
            "Print pages of the guide this binary carries. The guide says how to work\n\
             in a repository of piles, and describes this binary and no other. Start a\n\
             session with pile guide.\n\n\
             Each page has a name, and says in one line what it answers. A page has\n\
             one of three roles:\n  \
             - start: where a session begins, printed by a bare pile guide\n  \
             - workflow: one thing to accomplish, carried out from the top, step by step\n  \
             - reference: what something is, read when it applies\n\n\
             With pages named, guide prints each of them, then the pages each one\n\
             needs, and the pages those need, each once. The pages a page names to see\n\
             next are listed under it, and not printed.\n\n\
             In json, the result is pages: each page's name, role, summary, needs, see\n\
             and text. A workflow also has workflow: when, first, steps, done_when\n\
             and hand_back. Each step has title, about, run, the commands to type, and\n\
             look_for. next is the pages the pages asked for name to see.\n\n\
             --list lists every page, a line each, under its role.\n\n\
             --agents prints the section a repository keeps near the start of its\n\
             rules file, in its first 100 lines. A rules file is the file its agents\n\
             read first: AGENTS.md, CLAUDE.md, or whichever its agents read\n\
             (pile guide agent-mode). It says that the repository declares piles,\n\
             and sends a session to the guide.\n\
             It is text in every format, so that it can be written into that file.\n\n\
             guide needs no repository, and writes nothing.",
        )
        .override_usage(
            "pile guide [page...]\n       pile guide --list\n       pile guide --agents",
        )
        .after_help(
            "Examples:\n  \
             pile guide                         where a session begins\n  \
             pile guide adopt                   a workflow, with the pages it needs\n  \
             pile guide intent doctor           two pages, each printed once\n  \
             pile guide --list                  every page\n  \
             pile guide --agents                the section, for a repository's rules file\n\n\
             Refusals:\n  \
             unknown-page             2  the message lists the pages there are\n  \
             usage                    2  --list and --agents take no page, and not each other",
        )
        .arg(
            Arg::new("pages")
                .num_args(0..)
                .action(ArgAction::Append)
                .add(ArgValueCompleter::new(names)),
        )
        .arg(switch("list", "list every page, a line each"))
        .arg(switch(
            "agents",
            "print the section for a rules file, as text",
        ))
}

/// The pages whose names start with what was typed, each with its summary.
/// They come from this binary, so they are offered anywhere.
fn names(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let typed = current.to_string_lossy();
    guide::carried()
        .pages
        .iter()
        .filter(|p| p.name.starts_with(typed.as_ref()))
        .map(|p| CompletionCandidate::new(&p.name).help(Some(p.summary.clone().into())))
        .collect()
}

/// The result is `pages`, the pages printed, each as [`page`] gives it.
/// With --list, it is `pages`, each page's `name`, `role`, `summary`,
/// `needs` and `see`, in the order of the list. `next` is the pages the
/// pages asked for name to see, less those printed, as the command that
/// prints each. --agents is text in every format.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    let asked: Vec<&String> = m
        .get_many::<String>("pages")
        .into_iter()
        .flatten()
        .collect();
    let list = m.get_flag("list");
    let agents = m.get_flag("agents");
    if list && agents {
        return Err(Refusal::new(
            Code::Usage,
            "pile guide prints --list or --agents, not both",
        ));
    }
    if (list || agents) && !asked.is_empty() {
        let flag = if list { "--list" } else { "--agents" };
        return Err(Refusal::new(
            Code::Usage,
            format!(
                "pile guide {flag} takes no page, and was given {:?}",
                asked[0]
            ),
        ));
    }
    let g = guide::carried();
    if agents {
        put!(out, "{}", guide::SECTION);
        out.raw = true;
        return Ok(());
    }
    if list {
        listing(out, &g.pages);
        return Ok(());
    }

    let mut pages = Vec::new();
    for name in &asked {
        pages.push(g.find(name).ok_or_else(|| unknown(name, &g.pages))?);
    }
    if pages.is_empty() {
        pages.extend(g.start());
    }
    let printed = g.with_needs(&pages);
    for (i, p) in printed.iter().enumerate() {
        if i > 0 {
            putln!(out);
        }
        print_page(out, p);
    }
    for p in &pages {
        for name in &p.see {
            let Some(see) = g.find(name) else { continue };
            if !printed.iter().any(|q| q.name == see.name) {
                out.next(format!("pile guide {}", see.name), see.summary.clone());
            }
        }
    }
    out.result(json!({"pages": printed.iter().map(|p| page(p)).collect::<Vec<_>>()}));
    Ok(())
}

/// A page as text: its name and role, its summary, its body, then a line for
/// each page it needs or sees, with that page's summary.
fn print_page(out: &mut Out, p: &Page) {
    match p.role {
        Role::Workflow => putln!(out, "# {}  (workflow)", p.name),
        _ => putln!(out, "# {}", p.name),
    }
    putln!(out, "{}\n", p.summary);
    put!(out, "{}", p.body);
    let refs: Vec<(&str, &Page)> = p
        .needs
        .iter()
        .map(|n| ("needs", n))
        .chain(p.see.iter().map(|n| ("see", n)))
        .filter_map(|(how, n)| guide::carried().find(n).map(|q| (how, q)))
        .collect();
    if refs.is_empty() {
        return;
    }
    let width = refs.iter().map(|(_, q)| q.name.len()).max().unwrap_or(0);
    putln!(out);
    for (how, q) in refs {
        putln!(out, "{how:<5}  {:<width$}  {}", q.name, q.summary);
    }
}

/// Every page, a line each: the start page, then the workflows, then the
/// reference pages, each section under a line saying what its pages are.
fn listing(out: &mut Out, pages: &[Page]) {
    let width = pages.iter().map(|p| p.name.len()).max().unwrap_or(0);
    let mut last = None;
    for p in pages {
        if last != Some(p.role) {
            if last.is_some() {
                putln!(out);
            }
            match p.role {
                Role::Start => putln!(out, "Where a session begins:"),
                Role::Workflow => putln!(out, "Workflows, each carried out from the top:"),
                Role::Reference => putln!(out, "Reference, each read when it applies:"),
            }
            last = Some(p.role);
        }
        putln!(out, "  {:<width$}  {}", p.name, p.summary);
    }
    out.result(json!({"pages": pages.iter().map(heading).collect::<Vec<_>>()}));
}

/// What a list says of a page.
fn heading(p: &Page) -> Value {
    json!({
        "name": p.name,
        "role": p.role.word(),
        "summary": p.summary,
        "needs": p.needs,
        "see": p.see,
    })
}

/// A page whole: what a list says of it, its text, and a workflow's parts.
fn page(p: &Page) -> Value {
    let Value::Object(mut v) = heading(p) else {
        unreachable!("a heading is an object")
    };
    v.insert("text".into(), json!(p.body));
    if let Some(w) = &p.workflow {
        v.insert("workflow".into(), workflow(w));
    }
    Value::Object(v)
}

fn workflow(w: &Workflow) -> Value {
    let steps: Vec<Value> = w
        .steps
        .iter()
        .map(|s| {
            let mut step = Map::new();
            step.insert("title".into(), json!(s.title));
            step.insert("about".into(), json!(s.about));
            step.insert("run".into(), json!(s.run));
            step.insert("look_for".into(), json!(s.look_for));
            Value::Object(step)
        })
        .collect();
    json!({
        "when": w.when,
        "first": w.first,
        "steps": steps,
        "done_when": w.done_when,
        "hand_back": w.hand_back,
    })
}

/// A page this binary does not carry: a mistake in what was typed.
fn unknown(name: &str, pages: &[Page]) -> Refusal {
    let names: Vec<&str> = pages.iter().map(|p| p.name.as_str()).collect();
    Refusal::new(
        Code::UnknownPage,
        format!(
            "{name:?} is not a page of the guide. The pages are {}",
            names.join(", ")
        ),
    )
    .next(
        "pile guide --list",
        "lists every page, with what each answers",
    )
}
