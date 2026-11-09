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
             minute ago is listed, and nothing is read from a stored list.\n\n\
             --tree prints the mount tree instead of the list. Each pile is mounted\n\
             in its parent, the nearest pile above it, so the root pile is the top\n\
             of the tree, and each other pile sits under the pile it is mounted in.\n\
             A pile's path is then printed relative to that pile, not from the top\n\
             of the repository: @model at :/design/model, mounted in @design at\n\
             :/design, is printed as model. A pile mounted in no pile is a top of\n\
             the tree, printed with its :/ path. The names, ids, link counts and\n\
             what two piles claim are as in the list.",
        )
        .override_usage("pile list [--full] [--tree]")
        .arg(no_positional())
        .arg(switch(
            "full",
            "print whole ids rather than the first eight",
        ))
        .arg(switch(
            "tree",
            "print the mount tree, each path relative to the pile it is mounted in",
        ))
}

/// The result is `piles`, each as `name` (without its @, `null` for none),
/// `path` and `id`, whole, `mount`, its path relative to the pile it is
/// mounted in, and `mounted_in`, that pile as it is typed (both `null` for a
/// top of the mount tree), and `links`, each edge with the pile at one end,
/// as `direction` (`to` or `from`), `type` and `other`, the other end as it
/// is typed, in path order; `duplicate_ids`, each as `id` and
/// `claimants`, the paths of the piles claiming it; and `duplicate_names`,
/// each as `name` and `claimants`. With --tree, `piles` holds only the tops
/// of the mount tree, and each pile holds the piles mounted in it, as
/// `piles`, in path order.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    no_args("list", m)?;
    let full = m.get_flag("full");
    let tree = m.get_flag("tree");
    let r = open()?;
    let s = r.piles()?;
    let piles = s.ordered();
    let ids = s.duplicate_ids();
    let names = s.duplicate_names();
    let rows = graph::read(r.storage(), &State::Working)?;
    let linked: Vec<Vec<links::Link>> = piles.iter().map(|p| links::of(&rows, &s, p)).collect();
    let value = |p: &Pile| {
        let i = piles
            .iter()
            .position(|q| q.path == p.path)
            .expect("a pile of the tree is listed");
        let mount = s.mount(&p.path);
        json!({
            "name": name_of(&p.name),
            "path": shown(&p.path),
            "id": p.id,
            "mount": mount.as_ref().map(|(_, at)| at.clone()),
            "mounted_in": mount.as_ref().map(|(m, _)| m.handle()),
            "links": linked[i].iter().map(|l| l.brief()).collect::<Vec<_>>(),
        })
    };
    let listed: Vec<Value> = if tree {
        s.tops()
            .into_iter()
            .map(|p| nested(&s, p, &value))
            .collect()
    } else {
        piles.iter().map(|p| value(p)).collect()
    };
    out.result(json!({
        "piles": listed,
        "duplicate_ids": claimed(&ids, "id"),
        "duplicate_names": claimed(&names, "name"),
    }));

    if piles.is_empty() {
        let top = r.top()?;
        putln!(out, "no pile in {}", top.display());
        out.next("pile init", "declares the directory you are in a pile");
        return Ok(());
    }

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
    let name = |p: &Pile| {
        if p.name.is_empty() {
            String::new()
        } else {
            piles::typed(&p.name)
        }
    };
    // Each line as its columns: the name (in a tree, after the branches that
    // lead to it), the path, and the pile's place in piles, for its id and
    // counts.
    let lines: Vec<(String, String, usize)> = if tree {
        let mut lines = Vec::new();
        for top in s.tops() {
            branches(&s, top, "", "", &mut |p, lead| {
                let path = match s.mount(&p.path) {
                    Some((_, at)) => at,
                    None => shown(&p.path),
                };
                let i = piles
                    .iter()
                    .position(|q| q.path == p.path)
                    .expect("a pile of the tree is listed");
                lines.push((format!("{lead}{}", name(p)), path, i));
            });
        }
        lines
    } else {
        piles
            .iter()
            .enumerate()
            .map(|(i, p)| (name(p), shown(&p.path), i))
            .collect()
    };
    // Each column is as wide as its widest entry, a tree's branches
    // included, so every column after it starts at the same place.
    let first = lines
        .iter()
        .map(|(n, _, _)| n.chars().count())
        .max()
        .unwrap_or(0);
    let width = lines
        .iter()
        .map(|(_, p, _)| p.chars().count())
        .max()
        .unwrap_or(0);
    for (lead, path, i) in &lines {
        let p = piles[*i];
        let id = if full { p.id.as_str() } else { p.short_id() };
        let mut line = if first > 0 {
            format!("{lead:<first$}  {path:<width$}  {id}")
        } else {
            format!("{path:<width$}  {id}")
        };
        if any {
            line.push_str(&format!("  {}", counts[*i]));
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

/// A pile as the tree's result holds it: as the list holds it, with the
/// piles mounted in it under `piles`.
fn nested(s: &piles::Structure, p: &Pile, value: &dyn Fn(&Pile) -> Value) -> Value {
    let mut one = value(p);
    let under: Vec<Value> = s
        .children(&p.path)
        .into_iter()
        .map(|c| nested(s, c, value))
        .collect();
    one["piles"] = json!(under);
    one
}

/// Visits a pile and every pile mounted under it, depth first, in path
/// order, each with the branches that lead to it: `├── ` or `└── ` after
/// the `│   ` or blank columns of the piles above it. A top has none.
fn branches(
    s: &piles::Structure,
    p: &Pile,
    lead: &str,
    above: &str,
    visit: &mut dyn FnMut(&Pile, &str),
) {
    visit(p, lead);
    let children = s.children(&p.path);
    for (n, c) in children.iter().enumerate() {
        let last = n + 1 == children.len();
        let (here, next) = if last {
            ("└── ", "    ")
        } else {
            ("├── ", "│   ")
        };
        branches(
            s,
            c,
            &format!("{above}{here}"),
            &format!("{above}{next}"),
            visit,
        );
    }
}
