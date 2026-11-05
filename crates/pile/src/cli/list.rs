use std::collections::BTreeMap;

use clap::{ArgMatches, Command};
use serde_json::{Value, json};

use super::{Out, Result, links, name_of, no_args, no_positional, open, switch};
use piles::{Pile, shown};
use storage::State;

pub(super) fn command() -> Command {
    Command::new("list")
        .about("List every pile the repository declares")
        .long_about(
            "List every pile the repository declares, in path order. A pile comes\n\
             before the piles under it, though not always right before them:\n\
             :/lib-v2 sorts between :/lib and :/lib/sub.\n\n\
             Each pile is printed the three ways it is written:\n  \
             - its name, with its @, where it has one\n  \
             - its path, as a :/ path from the top of the repository\n  \
             - its id, the first eight characters of it unless --full is given\n\
             Where the repository declares links, each pile then counts its own: ->\n\
             and how many edges go from it, <- and how many come to it. pile which\n\
             names the node at the other end of each, and pile show prints each link\n\
             whole.\n\n\
             After the piles, list names each id and each name that two or more piles\n\
             claim, with every pile that claims it. Copying a subtree copies its\n\
             config, which is how an id comes to be claimed twice. list judges\n\
             nothing, so it exits 0 whatever it finds, and in a repository that\n\
             declares no pile it says so and exits 0 too.\n\n\
             The piles are what the working copy declares now: a pile declared a\n\
             minute ago is listed, and nothing is read from a stored list.",
        )
        .override_usage("pile list [--full]")
        .arg(no_positional())
        .arg(switch(
            "full",
            "print whole ids rather than the first eight",
        ))
}

/// The result is `piles`, each as `name` (without its @, `null` for none),
/// `path` and `id`, whole, and `links`, each edge with the pile at one end,
/// as `direction` (`to` or `from`), `type` and `other`, the other end as it
/// is typed, in path order; `duplicate_ids`, each as `id` and
/// `claimants`, the paths of the piles claiming it; and `duplicate_names`,
/// each as `name` and `claimants`.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    no_args("list", m)?;
    let full = m.get_flag("full");
    let r = open()?;
    let s = r.piles()?;
    let piles = s.ordered();
    let ids = s.duplicate_ids();
    let names = s.duplicate_names();
    let rows = graph::read(r.storage(), &State::Working)?;
    let linked: Vec<Vec<links::Link>> = piles.iter().map(|p| links::of(&rows, &s, p)).collect();
    out.result(json!({
        "piles": piles
            .iter()
            .zip(&linked)
            .map(|(p, l)| json!({
                "name": name_of(&p.name),
                "path": shown(&p.path),
                "id": p.id,
                "links": l.iter().map(|l| l.brief()).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
        "duplicate_ids": claimed(&ids, "id"),
        "duplicate_names": claimed(&names, "name"),
    }));

    if piles.is_empty() {
        let top = r.top()?;
        putln!(out, "no pile in {}", top.display());
        out.next("pile init", "declares the directory you are in a pile");
        return Ok(());
    }

    let width = piles
        .iter()
        .map(|p| shown(&p.path).len())
        .max()
        .unwrap_or(0);
    let name_width = piles
        .iter()
        .filter(|p| !p.name.is_empty())
        .map(|p| p.name.len() + 1)
        .max()
        .unwrap_or(0);
    // The counts, `-> 2  <- 1`, only where some pile has a link. A link
    // with the pile at neither end is counted by neither.
    let count = |l: &[links::Link], d| l.iter().filter(|l| l.direction == d).count();
    let outward: Vec<usize> = linked
        .iter()
        .map(|l| count(l, links::Direction::To))
        .collect();
    let wide = outward
        .iter()
        .map(|n| n.to_string().len())
        .max()
        .unwrap_or(1);
    let counts: Vec<String> = linked
        .iter()
        .zip(&outward)
        .map(|(l, to)| format!("-> {to:<wide$}  <- {}", count(l, links::Direction::From)))
        .collect();
    let any = !rows.edge.is_empty();
    for (p, count) in piles.iter().zip(&counts) {
        let id = if full { p.id.as_str() } else { p.short_id() };
        let name = if p.name.is_empty() {
            String::new()
        } else {
            piles::typed(&p.name)
        };
        let mut line = if name_width > 0 {
            format!("{name:<name_width$}  {:<width$}  {id}", shown(&p.path))
        } else {
            format!("{:<width$}  {id}", shown(&p.path))
        };
        if any {
            line.push_str(&format!("  {count}"));
        }
        putln!(out, "{line}");
    }

    for (id, claimants) in &ids {
        putln!(out);
        putln!(
            out,
            "{} is claimed by {} piles: {}",
            piles::short(id),
            claimants.len(),
            paths(claimants)
        );
        putln!(
            out,
            "  copying a subtree copies its config: remove one copy's .pile/config, and run pile init there"
        );
    }
    for (name, claimants) in &names {
        putln!(out);
        putln!(
            out,
            "{} is claimed by {} piles: {}",
            piles::typed(name),
            claimants.len(),
            paths(claimants)
        );
        putln!(
            out,
            "  a person chose the name, so the repair is to choose again"
        );
    }
    putln!(out, "\n{}", super::plural(piles.len(), "pile"));
    Ok(())
}

fn paths(piles: &[&Pile]) -> String {
    piles
        .iter()
        .map(|p| shown(&p.path))
        .collect::<Vec<_>>()
        .join(" ")
}

/// What two or more piles claim: the thing claimed under key, and the paths
/// of the piles claiming it.
fn claimed(dupes: &BTreeMap<String, Vec<&Pile>>, key: &str) -> Value {
    Value::Array(
        dupes
            .iter()
            .map(|(what, claimants)| {
                let mut one = serde_json::Map::new();
                one.insert(key.into(), json!(what));
                let paths: Vec<String> = claimants.iter().map(|p| shown(&p.path)).collect();
                one.insert("claimants".into(), json!(paths));
                Value::Object(one)
            })
            .collect(),
    )
}
