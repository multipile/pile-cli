use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;

use diagrams::{As, Diagram};
use piles::{SIGIL, shown};
use storage::State;

use super::{Out, Result, name_of, open, os_says, plural, say, target};
use crate::exit::{Code, Refusal};

pub(super) fn command() -> Command {
    Command::new("diagram")
        .about("Draw a diagram a pile keeps")
        .long_about(
            "Draw a diagram a pile keeps. A pile keeps its diagrams in its\n\
             .pile/diagrams/, a file <name>.diagram each, which says what the diagram\n\
             shows and never where (pile guide diagrams).",
        )
        .override_usage("pile diagram render [--as html|png|gif] [-o <file>] <diagram>")
        .subcommand(super::one_help(super::with_help(render_command())))
}

fn render_command() -> Command {
    Command::new("render")
        .about("Draw one diagram as a page, a picture or an animation")
        .long_about(
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
             A diagram that breaks a rule of its format, or names a pile the\n\
             repository does not have, is not drawn: pile doctor --diagrams says\n\
             what is wrong with each.",
        )
        .override_usage("pile diagram render [--as html|png|gif] [-o <file>] <diagram>")
        .after_help(
            "Examples:\n  \
             pile diagram render crates                      crates.html, a page\n  \
             pile diagram render --as png crates             crates.png, a picture\n  \
             pile diagram render -o docs/crates.gif crates   the reveal, as an animation\n  \
             pile diagram render @lib/flow                   the diagram flow that @lib keeps\n\n\
             Refusals:\n  \
             no-such-diagram          1  no pile keeps a diagram of that name: the message lists them\n  \
             diagram-name-ambiguous   1  two piles keep one of that name: type it after its pile\n  \
             diagram-invalid          1  the diagram breaks a rule: pile doctor --diagrams says which\n  \
             unknown-output           2  --as or -o's ending is not html, png or gif\n  \
             draw-failed              1  the picture could not be drawn: the message says why\n  \
             write-failed             1  the file could not be written",
        )
        .arg(Arg::new("diagram").num_args(0..).action(ArgAction::Append))
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
        Some(("render", m)) => render(m, out),
        _ => Err(Refusal::new(
            Code::Usage,
            "pile diagram takes what to do with the diagram: render",
        )
        .next("pile diagram render --help", "says what render takes")),
    }
}

/// The result is `diagram`, the diagram's name; `pile`, the pile that keeps
/// it (`name`, `path` and `id`); `path`, its file; `as`; `out`, the file
/// written, as given or chosen; `bytes`, how many were written; `width` and
/// `height`, the diagram's size in pixels of the page; and `duration_ms`,
/// how long a gif moves, or `null`.
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
    let s = r.piles()?;
    let all = diagrams::found(&found);
    let diagram = which(&r, &s, &all, arg)?;

    let source = diagram
        .source
        .as_deref()
        .ok_or_else(|| invalid(diagram, "it is not UTF-8 text"))?;
    let read = diagram_read(diagram, source)?;
    let named: Vec<diagrams::Named> = s
        .ordered()
        .into_iter()
        .map(|p| diagrams::Named {
            path: p.path.clone(),
            name: p.name.clone(),
            id: p.id.clone(),
        })
        .collect();
    let wrong = diagrams::findings(diagram, &named);
    if let Some(first) = wrong.first() {
        return Err(invalid(
            diagram,
            &format!("{} ({})", first.message, plural(wrong.len(), "problem")),
        ));
    }
    let drawn = diagrams::draw(&read, as_).map_err(|why| {
        Refusal::new(
            Code::DrawFailed,
            format!("{} could not be drawn: {why}", shown(&diagram.path)),
        )
    })?;

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
    if all.iter().filter(|other| other.name == d.name).count() == 1 {
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
