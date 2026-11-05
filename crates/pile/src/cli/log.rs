use std::collections::{BTreeSet, HashMap};

use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value, json};

use piles::{Pile, Structure, shown};
use storage::Touched;

use super::{Out, Result, name_of, one_path, open, path_arg, plural, target};
use crate::exit::{Code, Refusal};

/// How many commits log lists when -n is not given: about a screen.
const DEFAULT: &str = "20";

pub(super) fn command() -> Command {
    Command::new("log")
        .about("The commits that changed one pile, and what else each changed")
        .long_about(
            "List the commits that changed one pile, newest first. Each commit has its\n\
             short id, its author's date and its subject, then the other piles it\n\
             changed too, and whether it rewrote the pile's intent.\n\n\
             git log of a directory is the history of the directory: this pile's,\n\
             and every pile's under it. log is the pile's own history: its files,\n\
             less the piles declared inside it.\n\n\
             The piles are today's:\n  \
             - A commit is the pile's when it changed a path that is in the pile now.\n  \
             - A file moved from one pile to another changed both.\n  \
             - A merge is left out, because the commits it brought in are listed\n    \
             themselves.\n\
             So for a pile moved with git mv, the history starts at the move, as git\n\
             log of a directory does.\n\n\
             The last line but one counts what the listed commits changed together:\n\
             how many changed this pile alone, and how many also changed each other\n\
             pile. Two piles that always change together may be one thing. The count\n\
             is not a verdict. The last line is who made the listed commits, most\n\
             first. They are the authors, not the committers, as the date is the\n\
             author's. Both lines count only the listed commits, so with -n 0 they\n\
             count every commit to the pile.\n\n\
             The pile is the one named, or the one you are in. A path you name has\n\
             to be a pile.",
        )
        .override_usage("pile log [-n <count>] [pile]")
        .after_help(format!(
            "Examples:\n  \
             pile log               the last twenty commits to the pile you are in\n  \
             pile log -n 5 @lib     the last five to one named\n  \
             pile log -n 0 :/       every commit to the top pile's own files\n\n\
             Refusals:\n  \
             invalid-count            2  a number of commits, or 0 for every one\n{}",
            target::REFUSALS
        ))
        .arg(path_arg())
        .arg(
            Arg::new("max-count")
                .short('n')
                .long("max-count")
                .value_name("count")
                .help("how many commits to list, 0 for every one")
                .allow_negative_numbers(true)
                .default_value(DEFAULT),
        )
}

/// The result is the pile, `name`, `path` and `id`; `total`, how many
/// commits changed it; `commits`, the ones listed, newest first, each
/// `commit` (the whole id), `date`, `subject`, `also` (every other pile it
/// changed, `name` and `path`, in path order), `also_in_no_pile` (`true`
/// when it changed a path in no pile) and `intent_written`; `alone`, how many of
/// them changed this pile alone; `together`, every other pile they changed,
/// `name`, `path` and `count`, most first and in path order among equals;
/// `in_no_pile`, how many changed a path in no pile; and `by`, every
/// author, `author` and `count`, most first and by name among equals.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    let arg = one_path("log", m)?;
    let given = m
        .get_one::<String>("max-count")
        .map(String::as_str)
        .unwrap_or(DEFAULT);
    let count: usize = given.parse().map_err(|_| {
        Refusal::new(
            Code::InvalidCount,
            format!("-n is how many commits to list, and {given:?} is not a number of commits"),
        )
        .next("pile log -n 0", "lists every one")
    })?;
    let r = open()?;
    let s = target::declared(&r)?;
    let pile = target::chosen(&r, &s, arg.as_deref())?;
    let limit = (count > 0).then_some(count);
    let history = r.storage().history(&s.own(&pile.path), None, limit)?;

    let mut result = Map::new();
    result.insert("name".into(), name_of(&pile.name));
    result.insert("path".into(), json!(shown(&pile.path)));
    result.insert("id".into(), json!(pile.id));
    result.insert("total".into(), json!(history.total));

    if history.total == 0 {
        putln!(out, "{}   no commit has changed it yet", pile.heading());
        result.insert("commits".into(), json!([]));
        result.insert("alone".into(), json!(0));
        result.insert("together".into(), json!([]));
        result.insert("in_no_pile".into(), json!(0));
        result.insert("by".into(), json!([]));
        out.result(Value::Object(result));
        return Ok(());
    }
    let listed = history.revisions.len();
    if listed < history.total {
        putln!(
            out,
            "{}   the last {listed} of {} to it",
            pile.heading(),
            plural(history.total, "commit")
        );
    } else {
        putln!(
            out,
            "{}   {} to it",
            pile.heading(),
            plural(history.total, "commit")
        );
    }
    putln!(out);
    commits(out, &mut result, &s, pile, &history.revisions);
    out.result(Value::Object(result));
    Ok(())
}

fn commits(
    out: &mut Out,
    result: &mut Map<String, Value>,
    s: &Structure,
    pile: &Pile,
    revs: &[Touched],
) {
    let intent = pile.intent_path();
    let (mut alone, mut nowhere) = (0, 0);
    let mut together: HashMap<&str, (usize, &Pile)> = HashMap::new();
    let mut by: HashMap<&str, usize> = HashMap::new();
    let mut listed = Vec::new();

    for t in revs {
        let info = &t.info;
        putln!(out, "{:<8}  {}  {}", info.short, info.day(), info.subject);
        *by.entry(info.author.as_str()).or_default() += 1;

        let mut others: BTreeSet<&str> = BTreeSet::new();
        let mut unowned = false;
        for p in &t.paths {
            match s.owner(p) {
                None => unowned = true,
                Some(owner) if owner.path == pile.path => {}
                Some(owner) => {
                    others.insert(owner.path.as_str());
                }
            }
        }
        let wrote = t.paths.contains(&intent);
        if others.is_empty() && !unowned {
            alone += 1;
        }
        let others: Vec<&Pile> = others.iter().filter_map(|p| s.find(p)).collect();
        for o in &others {
            together.entry(o.path.as_str()).or_insert((0, o)).0 += 1;
        }
        if unowned {
            nowhere += 1;
        }
        if !others.is_empty() || unowned {
            let mut named: Vec<String> = others
                .iter()
                .map(|o| {
                    if o.name.is_empty() {
                        shown(&o.path)
                    } else {
                        format!("{} {}", piles::typed(&o.name), shown(&o.path))
                    }
                })
                .collect();
            if unowned {
                named.push("in no pile".to_string());
            }
            putln!(out, "          also   {}", named.join("  "));
        }
        if wrote {
            putln!(out, "          intent written here");
        }
        listed.push(json!({
            "commit": info.revision.as_str(),
            "date": info.day(),
            "subject": info.subject,
            "also": others
                .iter()
                .map(|o| json!({"name": name_of(&o.name), "path": shown(&o.path)}))
                .collect::<Vec<_>>(),
            "also_in_no_pile": unowned,
            "intent_written": wrote,
        }));
    }

    let mut ranked: Vec<(usize, &Pile)> = together.into_values().collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
    let mut tail: Vec<String> = ranked
        .iter()
        .map(|(n, p)| format!("{} {n}", shown(&p.path)))
        .collect();
    if nowhere > 0 {
        let at = ranked
            .iter()
            .position(|(n, _)| *n < nowhere)
            .unwrap_or(ranked.len());
        tail.insert(at, format!("in no pile {nowhere}"));
    }
    let tail = if tail.is_empty() {
        String::new()
    } else {
        format!("; also {}", tail.join(", "))
    };
    putln!(
        out,
        "\n{}, {alone} to this pile alone{tail}",
        plural(revs.len(), "commit")
    );
    let mut authors: Vec<(&str, usize)> = by.into_iter().collect();
    authors.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let line: Vec<String> = authors.iter().map(|(a, n)| format!("{a} {n}")).collect();
    putln!(out, "by {}", line.join(", "));

    result.insert("commits".into(), Value::Array(listed));
    result.insert("alone".into(), json!(alone));
    result.insert(
        "together".into(),
        ranked
            .iter()
            .map(|(n, p)| json!({"name": name_of(&p.name), "path": shown(&p.path), "count": n}))
            .collect(),
    );
    result.insert("in_no_pile".into(), json!(nowhere));
    result.insert(
        "by".into(),
        authors
            .iter()
            .map(|(a, n)| json!({"author": a, "count": n}))
            .collect(),
    );
}
