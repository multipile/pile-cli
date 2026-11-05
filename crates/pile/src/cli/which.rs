use std::collections::{BTreeMap, BTreeSet};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Value, json};

use storage::State;

use super::{Out, Result, links, name_of, open, say, say_all, switch, target};
use crate::exit::{Code, Refusal};

pub(super) fn command() -> Command {
    Command::new("which")
        .about("Say which pile each path is in")
        .long_about(
            "Say which pile each path is in, and what that pile says it is for: the\n\
             headline of its intent, and the command that prints the rest of it.\n\n\
             A path is in the nearest pile at or above it. When piles nest, the path\n\
             alone does not show which one that is, so ask before you write in a\n\
             file, or when one turns up in a diff or a stack trace.\n\n\
             Each path is a path as the shell means it, or a :/ path from the top of\n\
             the repository (pile guide paths). Neither has to exist, which matters because\n\
             which is asked most often about a file about to be written. A pile's name\n\
             stands for the pile's own directory. A path in no pile is printed under\n\
             in no pile. A path outside the repository is refused, and then nothing\n\
             is answered for the other paths either, so a script reading the exit code\n\
             gets one answer.\n\n\
             which prints a block per pile, in path order, with every path given that\n\
             is in it, as a :/ path. It points at a pile, and pile show is what prints\n\
             the pile whole.\n\n\
             Where the pile has links, a links line names the node at the other end\n\
             of each, once: -> before the nodes its links go to, and <- before the\n\
             nodes whose links come to it. Each is named as it is typed: @name,\n\
             type:name, or the path of a pile with no name. A change in the pile may\n\
             be owed at those nodes, or they may owe it one, and pile show prints\n\
             each link with its type and title.\n\n\
             --path prints only the path of each path's pile, a line per path in the\n\
             order given, and an empty line for a path in no pile.",
        )
        .override_usage("pile which [--path] <path>...")
        .after_help(format!(
            "Examples:\n  \
             pile which src/parse.rs              the pile it is in, and what that pile is for\n  \
             pile which src/new.rs docs/x.md      several at once, a block per pile\n  \
             pile which --path src/parse.rs       the pile's path alone, for a script\n\n\
             Refusals:\n{}",
            target::REFUSALS
        ))
        .arg(
            Arg::new("paths")
                .num_args(0..)
                .action(ArgAction::Append)
                .add(super::complete::piles()),
        )
        .arg(switch(
            "path",
            "print the path of each path's pile and nothing else, a line per path",
        ))
}

/// The result is `piles`, a block per pile in path order, each `name`,
/// `path`, `id`, `kind` and `version`; `paths`, the paths given that are in
/// it as the pile names them, `.` for its own directory; `intent`, the
/// headline whole, or `null` for a pile with none; `links`, each edge with the
/// pile at one end, as `direction` (`to` or `from`), `type` and `other`, the
/// other end as it is typed; and `whole`, the command that prints the pile. Then `in_no_pile`, the paths in no pile, from
/// the top. With `--path` it is `paths`, the path of each path's pile in the
/// order given, `null` for one in no pile.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    let args: Vec<String> = m
        .get_many::<String>("paths")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    if args.is_empty() {
        return Err(Refusal::new(
            Code::Usage,
            "pile which takes the paths to ask about, and was given none",
        ));
    }
    let r = open()?;
    let s = r.piles()?;
    let asked = args
        .iter()
        .map(|a| target::where_is(&r, &s, a))
        .collect::<Result<Vec<String>>>()?;

    if m.get_flag("path") {
        let mut owners = Vec::new();
        for path in &asked {
            match s.owner(path) {
                Some(p) => {
                    putln!(out, "{}", piles::shown(&p.path));
                    owners.push(json!(piles::shown(&p.path)));
                }
                None => {
                    putln!(out);
                    owners.push(Value::Null);
                }
            }
        }
        out.result(json!({ "paths": owners }));
        return Ok(());
    }

    let mut given: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut nowhere = Vec::new();
    let mut seen = BTreeSet::new();
    for path in &asked {
        if !seen.insert(path) {
            continue;
        }
        match s.owner(path) {
            Some(p) => given
                .entry(p.path.as_str())
                .or_default()
                .push(piles::shown(path)),
            None => nowhere.push(piles::shown(path)),
        }
    }

    let rows = graph::read(r.storage(), &State::Working)?;
    let mut blocks = Vec::new();
    for pile in s.ordered() {
        let Some(paths) = given.get(pile.path.as_str()) else {
            continue;
        };
        if !blocks.is_empty() {
            putln!(out);
        }
        putln!(out, "{}  {}/{}", pile.heading(), pile.kind, pile.version);
        say_all(out, "paths", paths);
        let headline = pile.headline();
        if !headline.is_empty() {
            say(out, "intent", &headline);
        }
        let linked = links::of(&rows, &s, pile);
        say_all(out, "links", &links::ends(&linked));
        let whole = format!("pile show {}", pile.handle());
        say(out, "whole", &whole);
        blocks.push(json!({
            "name": name_of(&pile.name),
            "path": piles::shown(&pile.path),
            "id": pile.id,
            "kind": pile.kind,
            "version": pile.version,
            "paths": paths,
            "intent": (!headline.is_empty()).then_some(headline),
            "links": linked.iter().map(|l| l.brief()).collect::<Vec<_>>(),
            "whole": whole,
        }));
    }
    if !nowhere.is_empty() {
        if !blocks.is_empty() {
            putln!(out);
        }
        putln!(out, "in no pile");
        say_all(out, "paths", &nowhere);
    }
    out.result(json!({"piles": blocks, "in_no_pile": nowhere}));
    Ok(())
}
