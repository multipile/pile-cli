use std::fmt::Write as _;

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value, json};

use piles::changes::{Entry, Group, View, arrange};
use storage::{ChangeKind, Revision, State};

use super::status::{entry, heading, line};
use super::{Out, Result, open, plural, say, switch, target};
use crate::exit::{Code, Refusal};

/// The first line of every Markdown answer, so a script can find a comment
/// it posted before and update it.
const MARKER: &str = "<!-- pile gitdiff -->";

pub(super) fn command() -> Command {
    Command::new("gitdiff")
        .about("What changed between two revisions, by pile, as git diff names them")
        .long_about(
            "Report what changed between two revisions, under the pile each change is\n\
             in, with what each changed pile is for: its intent's headline. The\n\
             revisions are named as git diff names them:\n  \
             pile gitdiff              HEAD and the working copy\n  \
             pile gitdiff <a>          a and the working copy\n  \
             pile gitdiff <a> <b>      a and b\n  \
             pile gitdiff <a>..<b>     a and b, the same\n  \
             pile gitdiff <a>...<b>    where b left a, their merge base, and b\n\
             A side left out of .. or ... is HEAD. So a pull request into main is\n\
             main...HEAD: what reached main after the branch left it is not part of\n\
             the change.\n\n\
             The piles are the ones declared at the second side. Each path has one\n\
             letter, as in pile status: A added, M modified, D deleted, R moved\n\
             inside the pile with where it was, and U a conflict in the working copy.\n\
             As in git diff, an untracked file is not listed: pile status lists it.\n\
             A move from one pile to another is a half in each. A change in no pile\n\
             is printed under in no pile, not dropped. A pile whose intent.md the\n\
             change wrote says intent rewritten, and one whose declaration files\n\
             (the .toml files in its .pile/, with its links) the change wrote says\n\
             declarations rewritten: pile show <pile> --at <a> shows them before.\n\n\
             Only the changed piles are listed, and the rest are counted. --all\n\
             lists every pile, in text and json.\n\n\
             --format markdown prints the answer for a pull request comment: its\n\
             first line is <!-- pile gitdiff -->, then a table of the changed piles,\n\
             then each one's paths, folded. It is the only command that answers in\n\
             markdown.",
        )
        .override_usage("pile gitdiff [--all] [<a>] [<b>]\n       pile gitdiff [--all] <a>..<b>\n       pile gitdiff [--all] <a>...<b>")
        .after_help(
            "Examples:\n  \
             pile gitdiff                                 the working copy, against HEAD\n  \
             pile gitdiff main HEAD                       what HEAD has that main does not\n  \
             pile gitdiff origin/main...HEAD              a pull request into main\n  \
             pile gitdiff main... --format markdown       the same, as a pull request comment\n\n\
             Refusals:\n  \
             usage                    2  more than two revisions, or a range and a revision\n  \
             unknown-revision         2  a side names no revision: git fetch, or name another\n  \
             no-shared-history        1  the two sides of ... share no revision\n  \
             no-piles                 1  the repository declares no pile: pile init declares one\n  \
             unreadable-config        1  a pile's config is wrong: the message names the line",
        )
        .arg(
            Arg::new("revisions")
                .value_name("revision")
                .num_args(0..)
                .action(ArgAction::Append),
        )
        .arg(switch("all", "list every pile, not just the changed ones"))
}

/// The revisions as they were typed: the first side, the second (None for
/// the working copy), and whether it was `...`.
struct Named {
    from: String,
    to: Option<String>,
    merge_base: bool,
}

/// Reads the revisions the way git diff reads them.
fn named(args: &[&String]) -> Result<Named> {
    let side = |s: &str| {
        if s.is_empty() {
            "HEAD".to_string()
        } else {
            s.to_string()
        }
    };
    let range = |s: &str| s.contains("..");
    match args {
        [] => Ok(Named {
            from: "HEAD".into(),
            to: None,
            merge_base: false,
        }),
        [one] => {
            if let Some((a, b)) = one.split_once("...") {
                Ok(Named {
                    from: side(a),
                    to: Some(side(b)),
                    merge_base: true,
                })
            } else if let Some((a, b)) = one.split_once("..") {
                Ok(Named {
                    from: side(a),
                    to: Some(side(b)),
                    merge_base: false,
                })
            } else {
                Ok(Named {
                    from: one.to_string(),
                    to: None,
                    merge_base: false,
                })
            }
        }
        [a, b] if !range(a) && !range(b) => Ok(Named {
            from: a.to_string(),
            to: Some(b.to_string()),
            merge_base: false,
        }),
        [_, _] => Err(Refusal::new(
            Code::Usage,
            "pile gitdiff takes a range, a..b or a...b, or two revisions, and not both",
        )),
        many => Err(Refusal::new(
            Code::Usage,
            format!(
                "pile gitdiff takes two revisions at most, and was given {}",
                many.len()
            ),
        )),
    }
}

/// Where the change starts and ends.
struct Ends {
    named: Named,
    /// The first side.
    from: Revision,
    /// Where the second side left the first, with `...`.
    merge_base: Option<Revision>,
    /// The second side, or None for the working copy.
    to: Option<Revision>,
}

impl Ends {
    /// The revision the change is measured from.
    fn start(&self) -> &Revision {
        self.merge_base.as_ref().unwrap_or(&self.from)
    }

    /// The second side's name, or the working copy.
    fn to_name(&self) -> &str {
        self.named.to.as_deref().unwrap_or("the working copy")
    }
}

/// The result is `from`, the first side, `name` as typed and `revision`;
/// `merge_base`, where the second side left the first with `...`, or
/// `null`; `to`, the second side, `name` and `revision`, or `null` for the
/// working copy; `piles`, the changed piles, or every pile with --all, in
/// path order, each `name`, `path`, `id`, `headline`, `intent_changed`,
/// `declarations_changed`,
/// then the counts `changed` and `crossed`, and `entries`, as pile status
/// gives them. Then `in_no_pile`, as `entries`, and `total`: `piles`, how
/// many are `changed` and `unchanged`, the `paths` in them, the paths that
/// `crossed`, and the paths `in_no_pile`.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    let args: Vec<&String> = m
        .get_many::<String>("revisions")
        .into_iter()
        .flatten()
        .collect();
    let named = named(&args)?;
    let all = m.get_flag("all");
    let r = open()?;
    let storage = r.storage();
    let from = storage.resolve(&named.from)?;
    let to = match &named.to {
        Some(name) => Some(storage.resolve(name)?),
        None => None,
    };
    let merge_base = if named.merge_base {
        let b = to.as_ref().expect("a ... range has a second side");
        let base = storage.merge_base(&from, b)?.ok_or_else(|| {
            Refusal::new(
                Code::NoSharedHistory,
                format!(
                    "{} and {} share no revision, so there is no merge base to compare from",
                    named.from,
                    named.to.as_deref().unwrap_or_default()
                ),
            )
            .next(
                "git log --oneline --graph --all",
                "shows how the branches are related",
            )
        })?;
        Some(base)
    } else {
        None
    };
    let state = match &to {
        Some(rev) => State::At(rev.clone()),
        None => State::Working,
    };
    let s = piles::discover(storage, &state)?;
    if s.is_empty() {
        return Err(target::no_piles(&r));
    }
    let ends = Ends {
        named,
        from,
        merge_base,
        to,
    };
    let mut changes = storage.changes(Some(ends.start()), &state)?;
    changes.retain(|c| c.kind != ChangeKind::Untracked);
    let view = arrange(&s, piles::ROOT, &changes);
    report(out, &ends, &view, all);
    out.markdown = Some(markdown(&ends, &view));
    out.result(result(&ends, &view, all));
    Ok(())
}

/// The groups a report lists: the changed ones, or every one with --all.
fn listed<'v, 'a>(view: &'v View<'a>, all: bool) -> Vec<&'v Group<'a>> {
    view.groups.iter().filter(|g| all || g.changed()).collect()
}

fn result(ends: &Ends, view: &View, all: bool) -> Value {
    let piles: Vec<Value> = listed(view, all)
        .into_iter()
        .map(|g| {
            let mut p = Map::new();
            let h = heading(g);
            for key in ["name", "path", "id"] {
                p.insert(key.into(), h[key].clone());
            }
            p.insert("headline".into(), json!(g.pile.headline()));
            p.insert("intent_changed".into(), json!(g.intent_written()));
            p.insert(
                "declarations_changed".into(),
                json!(g.declarations_written()),
            );
            for key in ["changed", "crossed"] {
                p.insert(key.into(), h[key].clone());
            }
            p.insert("entries".into(), g.entries.iter().map(entry).collect());
            Value::Object(p)
        })
        .collect();
    let changed = view.groups.iter().filter(|g| g.changed()).count();
    json!({
        "from": {
            "name": ends.named.from,
            "revision": ends.from.as_str(),
        },
        "merge_base": ends.merge_base.as_ref().map(Revision::as_str),
        "to": ends.to.as_ref().map(|t| json!({
            "name": ends.named.to,
            "revision": t.as_str(),
        })),
        "piles": piles,
        "in_no_pile": view.in_no_pile.iter().map(entry).collect::<Vec<_>>(),
        "total": {
            "piles": view.groups.len(),
            "changed": changed,
            "unchanged": view.groups.len() - changed,
            "paths": view.groups.iter().map(|g| g.entries.len()).sum::<usize>(),
            "crossed": view.groups.iter().map(Group::crossed).sum::<usize>(),
            "in_no_pile": view.in_no_pile.len(),
        },
    })
}

fn report(out: &mut Out, ends: &Ends, view: &View, all: bool) {
    match &ends.merge_base {
        Some(base) => putln!(
            out,
            "from  {}  where {} left {}",
            piles::short(base.as_str()),
            ends.to_name(),
            ends.named.from
        ),
        None => putln!(
            out,
            "from  {}  {}",
            piles::short(ends.from.as_str()),
            ends.named.from
        ),
    }
    match &ends.to {
        Some(to) => putln!(
            out,
            "to    {}  {}",
            piles::short(to.as_str()),
            ends.to_name()
        ),
        None => putln!(out, "to    the working copy"),
    }
    for g in listed(view, all) {
        putln!(out);
        putln!(out, "{}", g.pile.heading());
        let headline = g.pile.headline();
        let mut intent = if headline.is_empty() {
            "(no headline)".to_string()
        } else {
            headline
        };
        if g.intent_written() {
            intent.push_str("  (intent rewritten)");
        }
        if g.declarations_written() {
            intent.push_str("  (declarations rewritten)");
        }
        say(out, "intent", &intent);
        if !g.changed() {
            putln!(out, "   (unchanged)");
        }
        for e in &g.entries {
            line(out, e);
        }
    }
    if !view.in_no_pile.is_empty() {
        putln!(out, "\nin no pile");
        for e in &view.in_no_pile {
            line(out, e);
        }
    }
    putln!(out, "\n{}", total(view));
}

fn total(view: &View) -> String {
    let groups = &view.groups;
    let changed = groups.iter().filter(|g| g.changed()).count();
    let paths: usize = groups.iter().map(|g| g.entries.len()).sum();
    let crossed: usize = groups.iter().map(Group::crossed).sum();
    let nowhere = view.in_no_pile.len();
    let mut out = format!(
        "{changed} of {} changed, {}",
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

/// The answer as Markdown for GitHub: a table of the changed piles, then
/// each one's paths, folded. Unchanged piles are left out. Everything that
/// comes from the repository, a path, a headline or a revision's name, is
/// written so that nothing in it can open a block, a link, an HTML tag or
/// a table cell.
fn markdown(ends: &Ends, view: &View) -> String {
    let mut md = String::new();
    let groups: Vec<&Group> = view.groups.iter().filter(|g| g.changed()).collect();
    let paths: usize = groups.iter().map(|g| g.entries.len()).sum();
    let nowhere = view.in_no_pile.len();
    let _ = writeln!(md, "{MARKER}");
    let mut summary = format!(
        "### {} of {} changed",
        groups.len(),
        plural(view.groups.len(), "pile")
    );
    if nowhere > 0 {
        summary += &format!(", and {} in no pile", plural(nowhere, "path"));
    }
    let _ = writeln!(md, "{summary}\n");
    let to = match &ends.to {
        Some(to) => format!(
            "{} ({})",
            code(piles::short(to.as_str())),
            code(ends.to_name())
        ),
        None => "the working copy".to_string(),
    };
    let from = match &ends.merge_base {
        Some(base) => format!(
            "{}, where {} left {},",
            code(piles::short(base.as_str())),
            code(ends.to_name()),
            code(&ends.named.from)
        ),
        None => format!(
            "{} ({})",
            code(piles::short(ends.from.as_str())),
            code(&ends.named.from)
        ),
    };
    let _ = writeln!(
        md,
        "From {from} to {to}: {} changed.",
        plural(paths + nowhere, "path"),
    );
    if groups.is_empty() && nowhere == 0 {
        return md;
    }

    let _ = writeln!(md, "\n| Pile | Path | What it is for | Paths |");
    let _ = writeln!(md, "| --- | --- | --- | ---: |");
    for g in &groups {
        let headline = g.pile.headline();
        let mut purpose = if headline.is_empty() {
            "_No headline yet._".to_string()
        } else {
            prose(&headline)
        };
        if g.intent_written() {
            purpose += " **Intent rewritten.**";
        }
        if g.declarations_written() {
            purpose += " **Declarations rewritten.**";
        }
        let _ = writeln!(
            md,
            "| {} | {} | {purpose} | {} |",
            cell(&code(&g.pile.handle())),
            cell(&code(&piles::shown(&g.pile.path))),
            g.entries.len()
        );
    }
    if nowhere > 0 {
        let _ = writeln!(md, "| _In no pile_ | | | {nowhere} |");
    }

    for g in &groups {
        let _ = writeln!(md);
        files(&mut md, &html_code(&g.pile.handle()), &g.entries);
    }
    if nowhere > 0 {
        let _ = writeln!(md);
        files(&mut md, "In no pile", &view.in_no_pile);
    }
    md
}

/// The entries, folded under a line that names whose they are and counts
/// them. The line is HTML already.
fn files(md: &mut String, whose: &str, entries: &[Entry]) {
    let _ = writeln!(
        md,
        "<details>\n<summary>{whose} · {}</summary>\n",
        plural(entries.len(), "path")
    );
    for e in entries {
        let mut item = format!("- `{}` {}", e.letter(), code(&piles::shown(&e.path)));
        if e.crossed && !e.to.is_empty() {
            item += &format!(" (left this pile, now {})", code(&piles::shown(&e.to)));
        } else if e.crossed {
            item += &format!(" (arrived from {})", code(&piles::shown(&e.from)));
        } else if !e.from.is_empty() {
            item += &format!(" (was {})", code(&piles::shown(&e.from)));
        }
        if e.mode {
            item += " (mode)";
        }
        let _ = writeln!(md, "{item}");
    }
    let _ = writeln!(md, "\n</details>");
}

/// Markdown that goes in a table cell. A `|` ends a cell even inside a code
/// span, unless it is escaped.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// Text as HTML code, for where Markdown is not read, such as a
/// `<summary>`.
fn html_code(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    format!("<code>{escaped}</code>")
}

/// Text as a Markdown code span, which shows every character as it is.
///
/// The fence is one backtick longer than the longest run inside, so no run
/// inside closes it. Text that starts or ends with a backtick, or with a
/// space at both ends, is padded with a space at each end, which the span
/// takes off again. A line break would end the line the span is on, so it
/// is shown as `\n`.
pub(crate) fn code(text: &str) -> String {
    let text = text.replace('\r', "\\r").replace('\n', "\\n");
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat(longest + 1);
    let pad = text.starts_with('`')
        || text.ends_with('`')
        || (text.len() > 1 && text.starts_with(' ') && text.ends_with(' '));
    if pad {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

/// The characters that mean something to Markdown or HTML anywhere in a
/// line: a tag, an entity, emphasis, a link, a table cell or a
/// strikethrough. A link needs its `[`, so `(` and `)` after it are safe.
const ESCAPED: &str = "\\`*_[]<>&|~";

/// The characters that open a block when they start a line: a heading, a
/// bullet or a quote. `.` and `)` open a numbered item only after the digits
/// a line starts with, and are escaped only there.
const LEADING: &str = "#-+=>";

/// A line of prose from an intent, as Markdown that shows it as written.
///
/// Its code spans are kept, since a code span shows its text as it is.
/// Everything else has each character in [`ESCAPED`] escaped with a
/// backslash, and a first character in [`LEADING`] too, so nothing in it
/// opens a tag, a link, emphasis or a block. Nothing else is escaped, so
/// the Markdown reads as the intent does.
pub(crate) fn prose(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            let open = run_at(&chars, i);
            if let Some(close) = closing(&chars, i + open, open) {
                out.extend(&chars[i..close + open]);
                i = close + open;
                continue;
            }
            for _ in 0..open {
                out.push_str("\\`");
            }
            i += open;
            continue;
        }
        let c = chars[i];
        let numbered =
            matches!(c, '.' | ')') && i > 0 && chars[..i].iter().all(char::is_ascii_digit);
        if ESCAPED.contains(c) || (i == 0 && LEADING.contains(c)) || numbered {
            out.push('\\');
        }
        out.push(c);
        i += 1;
    }
    out
}

/// How many backticks run from i.
fn run_at(chars: &[char], i: usize) -> usize {
    chars[i..].iter().take_while(|c| **c == '`').count()
}

/// Where the run of exactly n backticks that closes a code span starts, at
/// or after i.
fn closing(chars: &[char], mut i: usize, n: usize) -> Option<usize> {
    while i < chars.len() {
        if chars[i] == '`' {
            let run = run_at(chars, i);
            if run == n {
                return Some(i);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_span_holds_any_text_as_it_is() {
        assert_eq!(code(":/a/b.rs"), "`:/a/b.rs`");
        assert_eq!(code("a`b"), "``a`b``");
        assert_eq!(code("a``"), "``` a`` ```");
        assert_eq!(code("x\ny"), "`x\\ny`");
        assert_eq!(code("<b>|*"), "`<b>|*`");
    }

    #[test]
    fn a_cell_and_a_summary_hold_any_text() {
        assert_eq!(cell(&code("a|b")), "`a\\|b`");
        assert_eq!(
            html_code("<a & \"b\">"),
            "<code>&lt;a &amp; &quot;b&quot;&gt;</code>"
        );
    }

    #[test]
    fn revisions_are_read_as_git_diff_reads_them() {
        let read = |args: &[&str]| {
            let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            let refs: Vec<&String> = owned.iter().collect();
            named(&refs).map(|n| (n.from, n.to, n.merge_base))
        };
        let s = |x: &str| x.to_string();
        assert_eq!(read(&[]).ok(), Some((s("HEAD"), None, false)));
        assert_eq!(read(&["main"]).ok(), Some((s("main"), None, false)));
        assert_eq!(
            read(&["main", "work"]).ok(),
            Some((s("main"), Some(s("work")), false))
        );
        assert_eq!(
            read(&["main..work"]).ok(),
            Some((s("main"), Some(s("work")), false))
        );
        assert_eq!(
            read(&["main...work"]).ok(),
            Some((s("main"), Some(s("work")), true))
        );
        assert_eq!(
            read(&["main..."]).ok(),
            Some((s("main"), Some(s("HEAD")), true))
        );
        assert_eq!(
            read(&["..work"]).ok(),
            Some((s("HEAD"), Some(s("work")), false))
        );
        assert!(read(&["a..b", "c"]).is_err());
        assert!(read(&["a", "b", "c"]).is_err());
    }

    #[test]
    fn prose_keeps_its_code_spans_and_escapes_the_rest() {
        assert_eq!(
            prose("Reads `pile` output, then."),
            "Reads `pile` output, then."
        );
        assert_eq!(prose("1. first"), "1\\. first");
        assert_eq!(prose("# not a heading, a-b"), "\\# not a heading, a-b");
        assert_eq!(prose("It's 5:00 + {x}!"), "It's 5:00 + {x}!");
        assert_eq!(
            prose("<script>a</script> [x](y) *b*"),
            "\\<script\\>a\\</script\\> \\[x\\](y) \\*b\\*"
        );
        assert_eq!(prose("one ` tick"), "one \\` tick");
        assert_eq!(prose("``a ` b``"), "``a ` b``");
    }
}
