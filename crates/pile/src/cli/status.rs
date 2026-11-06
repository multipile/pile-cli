use clap::{ArgMatches, Command};
use serde_json::{Map, Value, json};

use piles::changes::{Entry, Group, View, arrange};
use storage::State;

use super::{Out, Result, name_of, no_args, no_positional, open, plural, switch, target};

pub(super) fn command() -> Command {
    Command::new("status")
        .about("What changed since the last commit, by pile")
        .long_about(
            "Report what differs from the last commit, under the pile each change is\n\
             in, with each path as a :/ path. It is git's answer, arranged:\n  \
             - A change reads as a change to a pile.\n  \
             - A pile with no change is named, as unchanged, and not left out.\n  \
             - A path that moved from one pile to another is reported in both.\n  \
             - A change in no pile is printed under in no pile, not dropped.\n\n\
             Each path has one letter, measured against the last commit: A added,\n\
             M modified, D deleted, R moved inside the pile with where it was, ?\n\
             untracked, and U a conflict nobody has resolved yet. (mode) beside it\n\
             means the file became executable or a symbolic link, or stopped being\n\
             one. git's index is not a column of its own: the letter says what the\n\
             path holds now. In a repository with no commit yet, every file is new.\n\n\
             A move from one pile to another is two lines, a half in each pile: an A\n\
             (arrived from ...) where it arrived, and a D (left this pile, now ...)\n\
             where it left. Each names the other end.\n\n\
             status reports the pile you are in and the piles under it. --all reports\n\
             every pile, from anywhere. --brief prints a line per pile instead of\n\
             every path, and does not widen what is reported.",
        )
        .override_usage("pile status [--all] [--brief]")
        .after_help(format!(
            "Examples:\n  \
             pile status           the pile you are in, and the piles under it\n  \
             pile status --all     every pile in the repository\n  \
             pile status --brief   a line per pile\n\n\
             Refusals:\n{}",
            target::REFUSALS
        ))
        .arg(no_positional())
        .arg(switch(
            "brief",
            "print a line per pile instead of every path",
        ))
        .arg(switch(
            "all",
            "report every pile, not just the one you are in",
        ))
}

/// The result is `piles`, every pile reported, in path order, each `name`,
/// `path` and `id`, then the counts `changed` (paths changed that are not
/// untracked), `untracked` and `crossed` (halves of a move from one pile to
/// another), and `entries`. Then `in_no_pile`, as `entries`, from the top.
/// Then `total`: `piles`, how many `changed`, the `paths` in them, the paths
/// that `crossed`, and the paths `in_no_pile`.
///
/// Each entry is `state`, the letter; `path`, as the pile names it; `was`,
/// where a moved path came from, or `null`; `now`, where a path that left
/// this pile went, or `null`; `crossed`; and `mode`.
///
/// --brief leaves out every `entries` and `in_no_pile`, and keeps the counts.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    no_args("status", m)?;
    let brief = m.get_flag("brief");
    let r = open()?;
    let s = target::declared(&r)?;
    let scope = if m.get_flag("all") {
        piles::ROOT.to_string()
    } else {
        target::standing(&r, &s)?.path.clone()
    };
    let storage = r.storage();
    let head = storage.head()?;
    let changes = storage.changes(head.as_ref(), &State::Working)?;
    let view = arrange(&s, &scope, &changes);
    report(out, &view, brief);
    out.result(result(&view, brief));
    Ok(())
}

fn result(view: &View, brief: bool) -> Value {
    let piles: Vec<Value> = view
        .groups
        .iter()
        .map(|g| {
            let mut p = heading(g);
            if !brief {
                p.insert("entries".into(), g.entries.iter().map(entry).collect());
            }
            Value::Object(p)
        })
        .collect();
    let mut out = Map::new();
    out.insert("piles".into(), piles.into());
    if !brief {
        out.insert(
            "in_no_pile".into(),
            view.in_no_pile.iter().map(entry).collect(),
        );
    }
    out.insert(
        "total".into(),
        json!({
            "piles": view.groups.len(),
            "changed": view.groups.iter().filter(|g| g.changed()).count(),
            "paths": view.groups.iter().map(|g| g.entries.len()).sum::<usize>(),
            "crossed": view.groups.iter().map(Group::crossed).sum::<usize>(),
            "in_no_pile": view.in_no_pile.len(),
        }),
    );
    Value::Object(out)
}

pub(super) fn heading(g: &Group) -> Map<String, Value> {
    let untracked = g.untracked();
    let mut p = Map::new();
    p.insert("name".into(), name_of(&g.pile.name));
    p.insert("path".into(), json!(piles::shown(&g.pile.path)));
    p.insert("id".into(), json!(g.pile.id));
    p.insert("changed".into(), json!(g.entries.len() - untracked));
    p.insert("untracked".into(), json!(untracked));
    p.insert("crossed".into(), json!(g.crossed()));
    p
}

pub(super) fn entry(e: &Entry) -> Value {
    let or_null = |s: &str| {
        if s.is_empty() {
            Value::Null
        } else {
            json!(piles::shown(s))
        }
    };
    json!({
        "state": e.letter(),
        "path": piles::shown(&e.path),
        "was": if e.to.is_empty() { or_null(&e.from) } else { Value::Null },
        "now": or_null(&e.to),
        "crossed": e.crossed,
        "mode": e.mode,
    })
}

fn report(out: &mut Out, view: &View, brief: bool) {
    let groups = &view.groups;
    let path_width = groups
        .iter()
        .map(|g| piles::shown(&g.pile.path).len())
        .max()
        .unwrap_or(0);
    let name_width = groups
        .iter()
        .filter(|g| !g.pile.name.is_empty())
        .map(|g| g.pile.name.len() + 1)
        .max()
        .unwrap_or(0);

    for (i, g) in groups.iter().enumerate() {
        if brief {
            let name = if g.pile.name.is_empty() {
                String::new()
            } else {
                piles::typed(&g.pile.name)
            };
            if name_width > 0 {
                putln!(
                    out,
                    "{name:<name_width$}  {:<path_width$}  {:<8}   {}",
                    piles::shown(&g.pile.path),
                    g.pile.short_id(),
                    summary(g)
                );
            } else {
                putln!(
                    out,
                    "{:<path_width$}  {}   {}",
                    piles::shown(&g.pile.path),
                    g.pile.short_id(),
                    summary(g)
                );
            }
            continue;
        }
        if i > 0 {
            putln!(out);
        }
        putln!(out, "{}", g.pile.heading());
        if !g.changed() {
            putln!(out, "   (unchanged)");
        }
        for e in &g.entries {
            line(out, e);
        }
    }

    if brief {
        if !view.in_no_pile.is_empty() {
            putln!(out, "{} in no pile", plural(view.in_no_pile.len(), "path"));
        }
        return;
    }
    if !view.in_no_pile.is_empty() {
        putln!(out, "\nin no pile");
        for e in &view.in_no_pile {
            line(out, e);
        }
    }
    putln!(out, "\n{}", total(view));
}

pub(super) fn line(out: &mut Out, e: &Entry) {
    put!(out, "   {}  {}", e.letter(), piles::shown(&e.path));
    if e.crossed && !e.to.is_empty() {
        put!(out, "  (left this pile, now {})", piles::shown(&e.to));
    } else if e.crossed {
        put!(out, "  (arrived from {})", piles::shown(&e.from));
    } else if !e.from.is_empty() {
        put!(out, "  (was {})", piles::shown(&e.from));
    }
    if e.mode {
        put!(out, "  (mode)");
    }
    putln!(out);
}

fn summary(g: &Group) -> String {
    if !g.changed() {
        return "unchanged".to_string();
    }
    let (untracked, crossed) = (g.untracked(), g.crossed());
    let mut out = plural(g.entries.len() - untracked, "path") + " changed";
    if untracked > 0 {
        out += &format!(", {untracked} untracked");
    }
    if crossed > 0 {
        out += &format!(", {} crossed a boundary", plural(crossed, "path"));
    }
    out
}

fn total(view: &View) -> String {
    let groups = &view.groups;
    let changed = groups.iter().filter(|g| g.changed()).count();
    let paths: usize = groups.iter().map(|g| g.entries.len()).sum();
    let crossed: usize = groups.iter().map(Group::crossed).sum();
    let nowhere = view.in_no_pile.len();
    if paths == 0 && nowhere == 0 {
        return format!("{}, none changed", plural(groups.len(), "pile"));
    }
    let mut out = format!(
        "{}, {changed} changed, {}",
        plural(groups.len(), "pile"),
        plural(paths, "path")
    );
    if crossed > 0 {
        out += &format!(", {} crossed a boundary", plural(crossed, "path"));
    }
    if nowhere > 0 {
        out += &format!(", {} in no pile", plural(nowhere, "path"));
    }
    out
}
