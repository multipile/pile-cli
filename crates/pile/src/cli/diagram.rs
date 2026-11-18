use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;

use diagrams::{As, Change, Diagram, Listed};
use piles::{SIGIL, shown};
use storage::{ChangeKind, Revision, State};

use super::{Declared, Out, Result, name_of, open, os_says, plural, say, target};
use crate::exit::{Code, Refusal};

pub(super) fn command() -> Command {
    Command::new("diagram")
        .about("List the diagrams the piles keep, and draw one")
        .long_about(
            "List the diagrams the piles keep, and draw one. A pile keeps its diagrams\n\
             in its .pile/diagrams/, a file <name>.diagram each, which says what the\n\
             diagram shows and never where (pile guide diagrams).\n\n\
             pile diagram list says which diagrams there are, and with --since which\n\
             of them a change touches. pile diagram render draws one, and with\n\
             --since draws it with its change.",
        )
        .override_usage(
            "pile diagram list [--since <revision>]\n       \
             pile diagram render [--since <revision>] [--as html|png|gif] [-o <file>] <diagram>",
        )
        .subcommand(super::one_help(super::with_help(list_command())))
        .subcommand(super::one_help(super::with_help(render_command())))
}

/// What --since says, as list and render take it.
const SINCE: &str = "--since takes a revision, as git names one: a branch, a tag or a commit.\n\
     <a>...<b> is where b left a, their merge base, and a side left out is\n\
     HEAD, so origin/main... is where the branch left main, as a pull request\n\
     into main counts its change. The change is from there to the working copy.";

fn since_arg() -> Arg {
    Arg::new("since")
        .long("since")
        .value_name("revision")
        .help("the revision a change is counted from, to the working copy")
}

fn list_command() -> Command {
    Command::new("list")
        .about("List the diagrams the piles keep, and which a change touches")
        .long_about(format!(
            "List the diagrams the piles keep, in path order: each one's name as it is\n\
             typed, the pile that keeps it, and its file.\n\n\
             With --since, each says how a change touches it, from the revision to the\n\
             working copy:\n  \
             added     its file is new\n  \
             edited    its file changed\n  \
             piles     its file did not change, and the change touched something it\n            \
             stands for: a pile, as pile gitdiff counts a changed pile, or\n            \
             another node or a link, which is touched when it is added or\n            \
             removed, or what it declares changes\n  \
             removed   its file is gone, so it is listed and not drawn\n\
             After piles comes each thing touched, as pile query writes one: @name\n\
             for a pile, type:name for another node, and a link as its source, its\n\
             type and its target. A diagram the change does not touch says none.\n\n\
             With --since, each diagram also says what it misses: the nodes and the\n\
             links its includes say it should show and it does not, in the working\n\
             copy, whatever the change touched (pile guide diagram-includes).\n\
             pile diagram render --since draws each one with its change.\n\n\
             {SINCE}"
        ))
        .override_usage("pile diagram list [--since <revision>]")
        .after_help(
            "Examples:\n  \
             pile diagram list                        every diagram the piles keep\n  \
             pile diagram list --since origin/main... the diagrams a pull request into main touches\n\n\
             Refusals:\n  \
             unknown-revision         2  --since names no revision: git fetch, or name another\n  \
             no-shared-history        1  the two sides of ... share no revision",
        )
        .arg(super::no_positional())
        .arg(since_arg())
}

fn render_command() -> Command {
    Command::new("render")
        .about("Draw one diagram as a page, a picture or an animation")
        .long_about(format!(
            "Draw one diagram as a page, a picture or an animation, and write it to a\n\
             file. --as says which: html, a page that loads and runs nothing, with its\n\
             fonts in it; png, a picture at twice the diagram's size; or gif, the\n\
             diagram revealed element by element. Without --as, the name -o is given\n\
             decides by its ending, and without either it is html. The answer's\n\
             format is --format's, as for every command.\n\n\
             The file is <name>.html, .png or .gif in the directory pile runs in,\n\
             unless -o names another. A file already there is replaced.\n\n\
             A diagram is typed by its name, which is its file's name without\n\
             .diagram. Where two piles keep a diagram of one name, the name is typed\n\
             after its pile: @lib/flow, or :/packages/lib/flow for a pile with no name.\n\
             The diagram's file, as a path on disk or a :/ path, names it too.\n\n\
             A diagram that breaks a rule of its format, or has a box or a line that\n\
             stands for nothing the repository declares, is not drawn: pile doctor\n\
             --diagrams says what is wrong with each. One that only misses what it\n\
             should show is still drawn.\n\n\
             --since draws the diagram with how a change touched it, as pile diagram\n\
             list says. An edited diagram is drawn as its two versions in one: as a\n\
             page or a picture, the second with what was added, removed and changed\n\
             marked, and listed under it; as a gif, the first becoming the second. A\n\
             diagram that stands for something the change touched is drawn with each\n\
             such box and line marked, and is still, so it is a page or a picture. An\n\
             added diagram, or one the change does not touch, is drawn as it is. {SINCE}"
        ))
        .override_usage("pile diagram render [--since <revision>] [--as html|png|gif] [-o <file>] <diagram>")
        .after_help(
            "Examples:\n  \
             pile diagram render crates                      crates.html, a page\n  \
             pile diagram render --as png crates             crates.png, a picture\n  \
             pile diagram render -o docs/crates.gif crates   the reveal, as an animation\n  \
             pile diagram render @lib/flow                   the diagram flow that @lib keeps\n  \
             pile diagram render --since main... -o c.gif crates   crates, becoming what the branch made it\n\n\
             Refusals:\n  \
             no-such-diagram          1  no pile keeps a diagram of that name: the message lists them\n  \
             diagram-name-ambiguous   1  two piles keep one of that name: type it after its pile\n  \
             diagram-invalid          1  the diagram breaks a rule: pile doctor --diagrams says which\n  \
             unknown-output           2  --as or -o's ending is not html, png or gif\n  \
             draw-failed              1  the picture could not be drawn: the message says why\n  \
             write-failed             1  the file could not be written\n  \
             diagram-still            2  --as gif for a diagram drawn still with --since: use png\n  \
             diagram-not-comparable   1  the diagram at --since does not read, or is another type\n  \
             unknown-revision         2  --since names no revision: git fetch, or name another\n  \
             no-shared-history        1  the two sides of ... share no revision",
        )
        .arg(Arg::new("diagram").num_args(0..).action(ArgAction::Append))
        .arg(since_arg())
        .arg(
            Arg::new("as")
                .long("as")
                .value_name("html|png|gif")
                .help("what to draw the diagram as: html, png or gif"),
        )
        .arg(
            Arg::new("out")
                .short('o')
                .long("out")
                .value_name("file")
                .help("the file to write, <name>.<html|png|gif> here unless given"),
        )
}

pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    match m.subcommand() {
        Some(("list", m)) => list(m, out),
        Some(("render", m)) => render(m, out),
        _ => Err(Refusal::new(
            Code::Usage,
            "pile diagram takes what to do with the diagrams: list, or render",
        )
        .next("pile diagram list", "lists the diagrams the piles keep")
        .next("pile diagram render --help", "says what render takes")),
    }
}

/// The revision a change is counted from, as --since names it.
struct Since {
    name: String,
    revision: Revision,
}

/// Reads --since: a revision, or a...b, their merge base.
fn since_of(r: &crate::repository::Repository, m: &ArgMatches) -> Result<Option<Since>> {
    let Some(given) = m.get_one::<String>("since") else {
        return Ok(None);
    };
    let storage = r.storage();
    let side = |s: &str| if s.is_empty() { "HEAD" } else { s }.to_string();
    let revision = if let Some((a, b)) = given.split_once("...") {
        let (a, b) = (storage.resolve(&side(a))?, storage.resolve(&side(b))?);
        storage.merge_base(&a, &b)?.ok_or_else(|| {
            Refusal::new(
                Code::NoSharedHistory,
                format!("the two sides of {given} share no revision, so there is no merge base to count from"),
            )
            .next("git log --oneline --graph --all", "shows how the branches are related")
        })?
    } else if given.contains("..") {
        return Err(Refusal::new(
            Code::Usage,
            format!(
                "--since takes one revision, or a...b for where b left a, and was given {given}"
            ),
        ));
    } else {
        storage.resolve(given)?
    };
    Ok(Some(Since {
        name: given.clone(),
        revision,
    }))
}

/// The piles a diagram may name, as the working copy declares them.
fn named_piles(s: &piles::Structure) -> Vec<diagrams::Named> {
    s.ordered()
        .into_iter()
        .map(|p| diagrams::Named {
            path: p.path.clone(),
            name: p.name.clone(),
            id: p.id.clone(),
        })
        .collect()
}

/// A change since a revision: the diagrams there were at the revision, and
/// what the change from there to the working copy touched.
struct Changed {
    then: Vec<Diagram>,
    /// The ids of the piles whose files changed.
    piles: BTreeSet<String>,
    /// The other nodes, and the edges, that differ.
    touched: graph::Touched,
}

fn changed_since(
    r: &crate::repository::Repository,
    s: &piles::Structure,
    since: &Since,
    declared: &Declared,
) -> Result<Changed> {
    let storage = r.storage();
    let at = State::At(since.revision.clone());
    let found = storage.declarations(&at)?;
    let then = diagrams::found(&found);
    let mut changed = storage.changes(Some(&since.revision), &State::Working)?;
    // A pile whose diagrams alone changed did not change what it holds.
    changed.retain(|c| c.kind != ChangeKind::Untracked && piles::diagram_of(&c.path).is_none());
    let view = piles::changes::arrange(s, piles::ROOT, &changed);
    // A pile is touched by its files.
    let piles = view
        .groups
        .iter()
        .filter(|g| g.changed())
        .map(|g| g.pile.id.clone())
        .collect();
    // A node that is not a pile, and an edge, is touched by what it
    // declares. Where the revision's declarations do not read, nothing says
    // what they were, so none is.
    let touched = piles::structure(&found)
        .map(|s| graph::touched(&graph::relations(&s, &found), &declared.rows))
        .unwrap_or_default();
    Ok(Changed {
        then,
        piles,
        touched,
    })
}

/// Each diagram there is, and each that is gone since the revision, with
/// how the change from the revision to the working copy touches it, and
/// what it misses.
fn changes_since(
    r: &crate::repository::Repository,
    s: &piles::Structure,
    now: &[Diagram],
    since: &Since,
    declared: &Declared,
) -> Result<Vec<Listed>> {
    let changed = changed_since(r, s, since, declared)?;
    Ok(diagrams::changes(
        &changed.then,
        now,
        &named_piles(s),
        Some(declared.graph()),
        &changed.piles,
        &changed.touched,
    ))
}

/// How a change touches a diagram, as text says it: the word, and after
/// `piles` each thing touched.
fn change_said(change: Option<&Change>) -> String {
    match change {
        Some(Change::Piles(touched)) => format!("piles {}", touched.references.join(", ")),
        Some(change) => change.word().to_string(),
        None => "none".to_string(),
    }
}

/// The result is `diagrams`, in path order, each `name`; `handle`, how it is
/// typed to draw it; `pile`, the pile that keeps it (`name`, `path` and
/// `id`), or `null` for a removed one whose pile is gone; and `path`, its
/// file. With --since, each also has `change`, `added`, `edited`, `piles`,
/// `removed` or `null`; `piles`, the piles the change touched that it
/// shows; `touched`, everything the change touched that it stands for,
/// piles, other nodes and links, each as pile query writes one; and
/// `misses`, the nodes and links it should show and does not. The result
/// has `since`, `name` as typed and `revision`. `total` is `diagrams`, and
/// with --since `changed`.
fn list(m: &ArgMatches, out: &mut Out) -> Result {
    super::no_args("diagram list", m)?;
    let r = open()?;
    let found = r.storage().declarations(&State::Working)?;
    let s = piles::structure(&found)?;
    let now = diagrams::found(&found);
    let since = since_of(&r, m)?;
    let listed: Vec<Listed> = match &since {
        Some(since) => changes_since(&r, &s, &now, since, &Declared::of(&found, &s))?,
        None => now
            .iter()
            .map(|d| Listed {
                diagram: d.clone(),
                change: None,
                misses: Vec::new(),
            })
            .collect(),
    };
    let mut rows = Vec::new();
    for (i, one) in listed.iter().enumerate() {
        let (diagram, change) = (&one.diagram, &one.change);
        if i > 0 {
            putln!(out);
        }
        let handle = typed(&s, &now, diagram);
        let pile = s.ordered().into_iter().find(|p| p.path == diagram.dir);
        putln!(out, "{}  {}", handle, shown(&diagram.path));
        if let Some(p) = pile {
            say(out, "pile", &p.heading());
        }
        let touched = change.as_ref().and_then(Change::touched);
        if since.is_some() {
            say(out, "change", &change_said(change.as_ref()));
            if !one.misses.is_empty() {
                say(out, "misses", &one.misses.join(", "));
            }
        }
        let mut row = json!({
            "name": diagram.name,
            "handle": handle,
            "pile": pile.map(|p| json!({"name": name_of(&p.name), "path": shown(&p.path), "id": p.id})),
            "path": shown(&diagram.path),
        });
        if since.is_some() {
            row["change"] = json!(change.as_ref().map(Change::word));
            row["piles"] = json!(touched.map_or(&[][..], |t| &t.piles[..]));
            row["touched"] = json!(touched.map_or(&[][..], |t| &t.references[..]));
            row["misses"] = json!(one.misses);
        }
        rows.push(row);
    }
    let touched = listed.iter().filter(|l| l.change.is_some()).count();
    if listed.is_empty() {
        putln!(out, "no pile keeps a diagram");
        out.next(
            "pile guide diagrams",
            "says where a pile keeps its diagrams, and how one is written",
        );
    } else {
        putln!(out);
    }
    let mut total = json!({ "diagrams": listed.len() });
    match &since {
        Some(since) => {
            putln!(
                out,
                "{}, {} touched since {}",
                plural(listed.len(), "diagram"),
                touched,
                since.name
            );
            total["changed"] = json!(touched);
            out.result(json!({
                "diagrams": rows,
                "since": {"name": since.name, "revision": since.revision.as_str()},
                "total": total,
            }));
        }
        None => {
            if !listed.is_empty() {
                putln!(out, "{}", plural(listed.len(), "diagram"));
            }
            out.result(json!({ "diagrams": rows, "total": total }));
        }
    }
    Ok(())
}

/// The result is `diagram`, the diagram's name; `pile`, the pile that keeps
/// it (`name`, `path` and `id`); `path`, its file; `as`; `out`, the file
/// written, as given or chosen; `bytes`, how many were written; `width` and
/// `height`, the diagram's size in pixels of the page; and `duration_ms`,
/// how long a gif moves, or `null`. With --since it also has `change`,
/// `piles` and `touched`, as pile diagram list gives them.
fn render(m: &ArgMatches, out: &mut Out) -> Result {
    let args: Vec<&String> = m
        .get_many::<String>("diagram")
        .into_iter()
        .flatten()
        .collect();
    let arg = match args.as_slice() {
        [one] => one.as_str(),
        [] => {
            return Err(Refusal::new(
                Code::Usage,
                "pile diagram render takes the diagram to draw, and was given none",
            )
            .next(
                "pile doctor --diagrams",
                "lists every diagram the piles keep",
            ));
        }
        many => {
            return Err(Refusal::new(
                Code::Usage,
                format!(
                    "pile diagram render draws one diagram, and was given {}",
                    many.len()
                ),
            ));
        }
    };
    let given = m.get_one::<String>("out").map(PathBuf::from);
    let as_ = chosen(
        m.get_one::<String>("as").map(String::as_str),
        given.as_deref(),
    )?;

    let r = open()?;
    let found = r.storage().declarations(&State::Working)?;
    let s = piles::structure(&found)?;
    let all = diagrams::found(&found);
    let since = since_of(&r, m)?;
    let diagram = match which(&r, &s, &all, arg) {
        Ok(diagram) => diagram,
        Err(refusal) => {
            // A diagram gone since the revision is named as gone.
            if let Some(since) = &since
                && let Some(gone) = changes_since(&r, &s, &all, since, &Declared::of(&found, &s))?
                    .into_iter()
                    .filter(|l| l.change == Some(Change::Removed))
                    .map(|l| l.diagram)
                    .find(|d| d.name == arg || shown(&d.path) == arg)
            {
                return Err(Refusal::new(
                    Code::NoSuchDiagram,
                    format!(
                        "{} was removed since {}, so there is nothing to draw",
                        shown(&gone.path),
                        since.name
                    ),
                )
                .next(
                    format!("pile diagram list --since {}", since.name),
                    "says how the change touches each diagram",
                ));
            }
            return Err(refusal);
        }
    };

    let source = diagram
        .source
        .as_deref()
        .ok_or_else(|| invalid(diagram, "it is not UTF-8 text"))?;
    let read = diagram_read(diagram, source)?;
    let named = named_piles(&s);
    // The graph is built only for a diagram that says something of it, or
    // is drawn with a change.
    let declared =
        (since.is_some() || diagrams::asks_graph(&read)).then(|| Declared::of(&found, &s));
    let graph = declared.as_ref().map(Declared::graph);
    // A diagram that misses what it should show is still drawn.
    let wrong: Vec<_> = diagrams::findings(diagram, &named, graph)
        .into_iter()
        .filter(diagrams::stops_drawing)
        .collect();
    if let Some(first) = wrong.first() {
        return Err(invalid(
            diagram,
            &format!("{} ({})", first.message, plural(wrong.len(), "problem")),
        ));
    }
    let change = match (&since, &declared) {
        (Some(since), Some(declared)) => {
            let changed = changed_since(&r, &s, since, declared)?;
            diagrams::listed(
                &changed.then,
                diagram,
                &named,
                graph,
                &changed.piles,
                &changed.touched,
            )
            .change
        }
        _ => None,
    };
    let failed = |why: String| {
        Refusal::new(
            Code::DrawFailed,
            format!("{} could not be drawn: {why}", shown(&diagram.path)),
        )
    };
    let drawn = match (&change, &since) {
        (Some(Change::Edited), Some(since)) => {
            let not_comparable = |why: &str| {
                Refusal::new(
                    Code::DiagramNotComparable,
                    format!(
                        "{} cannot be drawn as a change since {}: {why}",
                        shown(&diagram.path),
                        since.name
                    ),
                )
                .next(
                    format!("pile diagram render {arg}"),
                    "draws the diagram as it is now",
                )
            };
            let then = r
                .storage()
                .declarations(&State::At(since.revision.clone()))?;
            let before = diagrams::found(&then)
                .into_iter()
                .find(|d| d.path == diagram.path)
                .and_then(|d| d.source)
                .ok_or_else(|| not_comparable("its file there is not text"))?;
            let before = diagrams::format::read(&before)
                .map_err(|_| not_comparable("its file there breaks a rule of its format"))?;
            diagrams::draw_diff(&before.diagram, &read, as_)
                .ok_or_else(|| not_comparable("it was a diagram of another type"))?
                .map_err(failed)?
        }
        (Some(Change::Piles(marked)), Some(_)) => {
            if as_ == As::Gif {
                return Err(Refusal::new(
                    Code::DiagramStill,
                    format!(
                        "{} is drawn with what a change touched marked, which is still, so it is not a gif",
                        shown(&diagram.path)
                    ),
                )
                .next(
                    format!(
                        "pile diagram render --as png --since {} {arg}",
                        since.as_ref().map_or("", |s| s.name.as_str())
                    ),
                    "draws it as a picture",
                ));
            }
            diagrams::draw_marked(&read, marked, as_).map_err(failed)?
        }
        _ => diagrams::draw(&read, as_).map_err(failed)?,
    };

    let to = given.unwrap_or_else(|| PathBuf::from(format!("{}.{}", diagram.name, as_.word())));
    if let Some(dir) = to.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| {
            Refusal::new(
                Code::WriteFailed,
                format!("could not write {}: {}", to.display(), os_says(&e)),
            )
        })?;
    }
    crate::write::replace(&to, &drawn.bytes)?;

    let pile = s.ordered().into_iter().find(|p| p.path == diagram.dir);
    putln!(out, "{}  {}", diagram.name, shown(&diagram.path));
    if let Some(p) = pile {
        say(out, "pile", &p.heading());
    }
    say(out, "as", as_.word());
    if since.is_some() {
        say(out, "change", &change_said(change.as_ref()));
    }
    say(
        out,
        "wrote",
        &format!("{} ({})", to.display(), plural(drawn.bytes.len(), "byte")),
    );
    say(out, "size", &format!("{} × {}", drawn.width, drawn.height));
    if let Some(ms) = drawn.duration_ms {
        say(out, "moves", &format!("{ms} ms"));
    }
    out.result(json!({
        "diagram": diagram.name,
        "pile": pile.map(|p| json!({"name": name_of(&p.name), "path": shown(&p.path), "id": p.id})),
        "path": shown(&diagram.path),
        "as": as_.word(),
        "out": to.display().to_string(),
        "bytes": drawn.bytes.len(),
        "width": drawn.width,
        "height": drawn.height,
        "duration_ms": drawn.duration_ms,
    }));
    if since.is_some()
        && let Some(result) = out.result.as_mut()
    {
        let touched = change.as_ref().and_then(Change::touched);
        result["change"] = json!(change.as_ref().map(Change::word));
        result["piles"] = json!(touched.map_or(&[][..], |t| &t.piles[..]));
        result["touched"] = json!(touched.map_or(&[][..], |t| &t.references[..]));
    }
    Ok(())
}

/// What to draw as: --as, or the ending of the file -o names, or html.
fn chosen(as_: Option<&str>, out: Option<&Path>) -> Result<As> {
    if let Some(word) = as_ {
        return As::from_word(word).ok_or_else(|| {
            Refusal::new(
                Code::UnknownOutput,
                format!(
                    "--as takes {}, and was given {word:?}",
                    As::WORDS.join(", ")
                ),
            )
        });
    }
    let Some(out) = out else {
        return Ok(As::Html);
    };
    let ending = out.extension().and_then(|e| e.to_str()).unwrap_or_default();
    As::from_word(&ending.to_ascii_lowercase()).ok_or_else(|| {
        Refusal::new(
            Code::UnknownOutput,
            format!(
                "{} does not end in .html, .png or .gif, so it does not say what to draw: give --as",
                out.display()
            ),
        )
    })
}

/// The one diagram an argument names.
fn which<'d>(
    r: &crate::repository::Repository,
    s: &piles::Structure,
    all: &'d [Diagram],
    arg: &str,
) -> Result<&'d Diagram> {
    let listed = || {
        if all.is_empty() {
            "no pile keeps a diagram".to_string()
        } else {
            format!(
                "the diagrams are {}",
                all.iter()
                    .map(|d| typed(s, all, d))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    };
    let none = |what: &str| {
        Refusal::new(
            Code::NoSuchDiagram,
            format!("no pile keeps a diagram {what}: {}", listed()),
        )
        .next(
            "pile guide diagrams",
            "says where a pile keeps its diagrams",
        )
    };
    // The diagram's own file.
    if arg.ends_with(piles::DIAGRAM) {
        let path = target::path_of(r, arg)?;
        return all
            .iter()
            .find(|d| d.path == path)
            .ok_or_else(|| none(&format!("at {}", shown(&path))));
    }
    // A name after its pile: @lib/flow, or :/packages/lib/flow.
    if let Some((pile, name)) = arg.rsplit_once('/') {
        let dir = match pile.strip_prefix(SIGIL) {
            Some(pile_name) => target::by_name(s, pile_name)?.path.clone(),
            None => target::path_of(r, if pile.is_empty() { ":/" } else { pile })?,
        };
        return all
            .iter()
            .find(|d| d.dir == dir && d.name == name)
            .ok_or_else(|| none(&format!("{name} in {}", shown(&dir))));
    }
    let named: Vec<&Diagram> = all.iter().filter(|d| d.name == arg).collect();
    match named.as_slice() {
        [one] => Ok(one),
        [] => Err(none(&format!("named {arg}"))),
        many => Err(Refusal::new(
            Code::DiagramNameAmbiguous,
            format!(
                "{} piles keep a diagram named {arg}: type it after its pile, as {}",
                many.len(),
                many.iter()
                    .map(|d| typed(s, all, d))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ),
        )),
    }
}

/// How a diagram is typed: by its name alone where no other pile keeps one
/// of that name, and after its pile where one does.
fn typed(s: &piles::Structure, all: &[Diagram], d: &Diagram) -> String {
    // A diagram gone from the working copy is no longer among them.
    if all.iter().filter(|other| other.name == d.name).count() <= 1 {
        return d.name.clone();
    }
    match s.ordered().into_iter().find(|p| p.path == d.dir) {
        Some(p) if !p.name.is_empty() => format!("{SIGIL}{}/{}", p.name, d.name),
        _ => format!("{}/{}", shown(&d.dir).trim_end_matches('/'), d.name),
    }
}

fn diagram_read(diagram: &Diagram, source: &str) -> Result<diagrams::format::Diagram> {
    diagrams::format::read(source)
        .map(|read| read.diagram)
        .map_err(|found| {
            let first = &found.0[0];
            invalid(
                diagram,
                &format!(
                    "line {}: {} ({})",
                    first.line,
                    first.message,
                    plural(found.0.len(), "problem")
                ),
            )
        })
}

fn invalid(diagram: &Diagram, why: &str) -> Refusal {
    Refusal::new(
        Code::DiagramInvalid,
        format!("{} is not drawn: {why}", shown(&diagram.path)),
    )
    .next(
        "pile doctor --diagrams",
        "says what is wrong with each diagram, and what fixes it",
    )
}
