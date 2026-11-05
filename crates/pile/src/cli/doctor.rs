use clap::{ArgMatches, Command};
use serde_json::{Map, Value, json};

use piles::doctor::{Finding, Is, Row, Severity, Survey, survey_of};
use piles::shown;
use storage::State;

use super::{Out, Result, name_of, no_args, no_positional, open, plural, say, switch};
use crate::exit::{Code, Refusal};

/// How much of a headline the text prints, without --full.
const HEADLINE: usize = 64;

pub(super) fn command() -> Command {
    Command::new("doctor")
        .about("Say what every pile is, and what is wrong with any of them")
        .long_about(
            "Say what every pile is, and what is wrong with any of them. doctor is the\n\
             one command with a verdict: it exits 1 when it finds a problem, and it\n\
             must exit 0 before you commit.\n\n\
             doctor reads the whole working copy, from anywhere in it, and takes no\n\
             pile: a problem in a pile you did not touch is still one to raise before\n\
             you commit. It is the one command that does not refuse over a config it\n\
             cannot read, because reporting a broken pile is what it is for. So when\n\
             another command refuses, run doctor to find out why. doctor writes\n\
             nothing. It reads what piles are (their configs, their intents and where\n\
             they sit) and what the .toml files in each .pile/ declare: types, nodes,\n\
             links and a pile's own values, each checked against the format\n\
             (pile guide declarations). It never reads a pile's other files.\n\n\
             The report has a block for each .pile/ directory, in path order. A block\n\
             is headed by the pile as every report heads it, then:\n  \
             - parent: the pile it sits in\n  \
             - clone: whether a clone would have the pile, which is whether its\n    \
             config is in the last commit\n  \
             - intent: the headline, cut to fit unless --full is given\n  \
             - a line for each finding, problems first, each naming its file, and\n    \
             its line and column where it has them, as :/path:line:column, then\n    \
             what fixes it\n\
             A .pile/ directory that is not a pile is headed (config refused), (not a\n\
             pile) or (unmade).\n\n\
             A problem stops the structure working, and makes doctor exit 1:\n  \
             unreadable-config        the config breaks a rule: correct the line\n  \
             config-not-a-file        the config is a symbolic link: replace it\n  \
             id-claimed-twice         a copied subtree: run pile init in the copy\n  \
             name-claimed-twice       neither pile answers to it: choose again\n  \
             no-intent                nothing says why the pile exists: write it\n  \
             intent-empty             the intent holds nothing: write it\n  \
             intent-unanswered        no headline yet: answer the question init asked\n  \
             intent-comment-unclosed  a <!-- hides the rest: close it with -->\n  \
             intent-not-a-file        the intent is a symbolic link: replace it\n  \
             intent-not-text          the intent is not UTF-8 text: write it again\n  \
             pile-in-pile-directory   a config inside a .pile/ declares nothing: move it\n  \
             pile-directory-is-a-file a .pile that is a file declares nothing: remove it\n\
             In a .toml file in a .pile/, each at its line and column where it has\n\
             them, and at the file where it does not:\n  \
             unreadable-declaration   not TOML that reads, so nothing in it is read\n  \
             no-version               does not open with version = \"gitpile/1\": add it\n  \
             unknown-key              a key or header not taken, or one letter from one\n  \
             missing-key              a key the table needs is not given\n  \
             wrong-value              a name not a word, a title not text, a bad reference\n  \
             unknown-type             a type no aux pile creates, or of the wrong sort\n  \
             no-such-name             a link's reference names no node\n  \
             wrong-end                a link's end is of a type its edge type does not allow\n  \
             self-link                a link joins a node to itself\n  \
             declared-twice           written twice, so neither is read: keep one\n  \
             reserved-name            pile as a type or a node; id, name, path, headline in [pile]\n  \
             types-outside-aux        a type is created only in an aux pile\n\
             A warning is something to put right that breaks nothing yet. doctor\n\
             still exits 0:\n  \
             no-name                  typed by its path: pile set name gives it one\n  \
             intent-heading           a heading goes stale when the pile moves: remove it\n\
             A note is something to know, and breaks nothing:\n  \
             unread-file              a file in .pile/ that pile does not read\n  \
             not-committed            a clone would not have the pile yet: commit it\n  \
             not-a-pile               a .pile/ with no config: init, or remove it\n  \
             unmade                   the config is gone, and its removal is not committed\n  \
             holds-repository         another repository inside a pile, read by none",
        )
        .override_usage("pile doctor [--full]")
        .after_help(
            "Examples:\n  \
             pile doctor          every pile, and what is wrong with any of them\n  \
             pile doctor --full   with whole ids and whole headlines\n\n\
             Refusals:\n  \
             problems-found           1  the report is printed whole: fix what it lists",
        )
        .arg(no_positional())
        .arg(switch("full", "print whole ids and whole headlines"))
}

/// The result is `piles`, a row for each .pile/ directory in path order, and
/// `in_no_pile`, the findings no pile is above, and `total`. A row is
/// `name`, `path`, `id`, `kind`, `version` (`null` where there is no config
/// that reads); `is`, which is `pile`, `config refused`, `not a pile` or
/// `unmade`; `parent`, `name`, `path` and `id`, or `null`; `clone`; `intent`,
/// the headline whole, or `null`; and `problems`, `warnings` and `notes`, each
/// a list of findings. A finding is `code`, `file`, `line` and `column`
/// (each `null` where it has none), `message` and `fix`. `total` is `piles`, `declaring_none`,
/// `needing_attention` (the rows with a problem), `problems`, `warnings` and
/// `notes`.
///
/// When there is a problem, the answer refuses as `problems-found`, and the
/// result is still the whole report.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    no_args("doctor", m)?;
    let full = m.get_flag("full");
    let r = open()?;
    let found = r.storage().declarations(&State::Working)?;
    let mut s = survey_of(r.storage(), &found)?;
    graph::check(&mut s, &found);
    if s.rows.is_empty() && s.in_no_pile.is_empty() {
        let top = r.top()?;
        putln!(out, "no pile in {}", top.display());
        out.next("pile init", "declares the directory you are in a pile");
        out.next(
            "pile guide adopt",
            "brings piles to a project that declares none, step by step",
        );
        out.result(result(&s));
        return Ok(());
    }
    report(out, &s, full);
    out.result(result(&s));
    if s.failed() {
        let problems = s.count(Severity::Problem);
        return Err(Refusal::new(
            Code::ProblemsFound,
            format!(
                "{} found, so doctor exits 1. Fix what the report lists",
                plural(problems, "problem")
            ),
        )
        .next(
            "pile guide doctor",
            "says what each severity means, and how to work through a report",
        ));
    }
    Ok(())
}

fn report(out: &mut Out, s: &Survey, full: bool) {
    let id = |id: &str| {
        if full {
            id.to_string()
        } else {
            piles::short(id).to_string()
        }
    };
    let heading_of = |path: &str| -> String {
        match s.find(path).and_then(|r| r.config.as_ref()) {
            Some(cfg) => piles::heading(&cfg.name, path, &id(&cfg.id)),
            None => shown(path),
        }
    };
    for (i, row) in s.rows.iter().enumerate() {
        if i > 0 {
            putln!(out);
        }
        match &row.config {
            Some(cfg) => putln!(
                out,
                "{}  {}/{}",
                heading_of(&row.path),
                cfg.kind,
                cfg.version
            ),
            None => putln!(out, "{}  ({})", shown(&row.path), row.is.word()),
        }
        if let Some(parent) = &row.parent {
            say(out, "parent", &heading_of(parent));
        }
        if row.is == Is::Pile {
            let clone = if row.committed {
                "has this pile"
            } else {
                "will not have this pile until its config is committed"
            };
            say(out, "clone", clone);
            if !row.headline.is_empty() {
                say(out, "intent", &clip(&row.headline, full));
            }
        }
        for f in &row.findings {
            say(out, f.severity.word(), &line(f));
        }
        if row.findings.is_empty() {
            say(out, "ok", "nothing to report");
        }
    }
    if !s.in_no_pile.is_empty() {
        if !s.rows.is_empty() {
            putln!(out);
        }
        putln!(out, "in no pile");
        for f in &s.in_no_pile {
            say(out, f.severity.word(), &line(f));
        }
    }
    putln!(out, "\n{}", total(s));
}

/// A finding as a line: where it is, what is wrong, and what fixes it.
/// Where it is reads `:/path`, `:/path:line` or `:/path:line:column`, as an
/// editor opens it.
fn line(f: &Finding) -> String {
    let at = match (f.line, f.column) {
        (Some(n), Some(c)) => format!("{}:{n}:{c}", shown(&f.file)),
        (Some(n), None) => format!("{}:{n}", shown(&f.file)),
        _ => shown(&f.file),
    };
    format!("{at}: {}. {}", f.message, capital(&f.fix))
}

/// The fix, starting a sentence. A fix that starts with a command is left as
/// it is typed, because `Pile set name` is not a command anybody can run.
fn capital(s: &str) -> String {
    if s.starts_with("pile ") {
        return s.to_string();
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn clip(text: &str, full: bool) -> String {
    if full || text.chars().count() <= HEADLINE {
        return text.to_string();
    }
    let cut: String = text.chars().take(HEADLINE).collect();
    format!("{}…", cut.trim_end())
}

fn total(s: &Survey) -> String {
    let mut out = plural(s.piles(), "pile");
    let none = s.declaring_none();
    if none > 0 {
        let dirs = if none == 1 {
            "directory"
        } else {
            "directories"
        };
        out += &format!(", {none} .pile/ {dirs} declaring none");
    }
    let (p, w, n) = (
        s.count(Severity::Problem),
        s.count(Severity::Warning),
        s.count(Severity::Note),
    );
    if p + w + n == 0 {
        return out + ", nothing wrong";
    }
    let mut parts = Vec::new();
    if p > 0 {
        parts.push(plural(p, "problem"));
    }
    if w > 0 {
        parts.push(plural(w, "warning"));
    }
    if n > 0 {
        parts.push(plural(n, "note"));
    }
    if p == 0 {
        format!("{out}, nothing wrong: {}", parts.join(", "))
    } else {
        format!(
            "{out}, {} needing attention: {}",
            s.ailing(),
            parts.join(", ")
        )
    }
}

fn result(s: &Survey) -> Value {
    let rows: Vec<Value> = s.rows.iter().map(|r| row(s, r)).collect();
    json!({
        "piles": rows,
        "in_no_pile": s.in_no_pile.iter().map(finding).collect::<Vec<_>>(),
        "total": {
            "piles": s.piles(),
            "declaring_none": s.declaring_none(),
            "needing_attention": s.ailing(),
            "problems": s.count(Severity::Problem),
            "warnings": s.count(Severity::Warning),
            "notes": s.count(Severity::Note),
        },
    })
}

fn row(s: &Survey, r: &Row) -> Value {
    let mut out = Map::new();
    let cfg = r.config.as_ref();
    out.insert("name".into(), cfg.map_or(Value::Null, |c| name_of(&c.name)));
    out.insert("path".into(), json!(shown(&r.path)));
    out.insert("id".into(), json!(cfg.map(|c| c.id.clone())));
    out.insert("kind".into(), json!(cfg.map(|c| c.kind.clone())));
    out.insert("version".into(), json!(cfg.map(|c| c.version)));
    out.insert("is".into(), json!(r.is.word()));
    let parent = r.parent.as_ref().map(|p| {
        let c = s.find(p).and_then(|row| row.config.as_ref());
        json!({
            "name": c.map_or(Value::Null, |c| name_of(&c.name)),
            "path": shown(p),
            "id": c.map(|c| c.id.clone()),
        })
    });
    out.insert("parent".into(), parent.unwrap_or(Value::Null));
    out.insert("clone".into(), json!(r.committed));
    out.insert(
        "intent".into(),
        if r.headline.is_empty() {
            Value::Null
        } else {
            json!(r.headline)
        },
    );
    for (key, severity) in [
        ("problems", Severity::Problem),
        ("warnings", Severity::Warning),
        ("notes", Severity::Note),
    ] {
        let list: Vec<Value> = r
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .map(finding)
            .collect();
        out.insert(key.into(), Value::Array(list));
    }
    Value::Object(out)
}

fn finding(f: &Finding) -> Value {
    json!({
        "code": f.code,
        "file": shown(&f.file),
        "line": f.line,
        "column": f.column,
        "message": f.message,
        "fix": f.fix,
    })
}
