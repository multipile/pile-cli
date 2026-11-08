//! The command line of the `pile` binary.
//!
//! Each command has a file with a `command()`, which says what it takes, and
//! a `run()`, which does it. A command writes its answer into an [`Out`].
//! [`execute`] prints it when the command returns, in the format asked for
//! (src/answer.rs).

use std::ffi::OsString;
use std::io::Write as _;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, ArgAction, ArgMatches, Command};

use serde_json::{Value, json};

use crate::answer::{self, Mode, Out};
use crate::exit::{Code, Refusal};
use crate::repository::Repository;

/// Writes into a command's output. A String cannot refuse a write, so there
/// is nothing to report.
macro_rules! put {
    ($out:expr, $($arg:tt)*) => {{
        use std::fmt::Write as _;
        let _ = write!($out, $($arg)*);
    }};
}

/// Writes a line into a command's output, or an empty line.
macro_rules! putln {
    ($out:expr) => {{
        $out.push('\n');
    }};
    ($out:expr, $($arg:tt)*) => {{
        use std::fmt::Write as _;
        let _ = writeln!($out, $($arg)*);
    }};
}

mod complete;
mod doctor;
mod files;
mod gitdiff;
mod guide;
mod id;
mod init;
mod links;
mod list;
mod log;
mod set;
mod show;
mod status;
mod target;
mod version;
mod which;

pub use complete::VAR as COMPLETE_VAR;
pub(crate) use target::os_says;

/// What a refusal says its command's help is for.
const TAKES: &str = "says what this command takes";

/// What every command returns: nothing, or the refusal.
pub(crate) type Result<T = ()> = std::result::Result<T, Refusal>;

/// What a bare pile and pile --help print above the commands: where to find
/// multipile, and the way in, which is an agent carrying out the guide.
/// What holds across commands, such as the forms a path is typed in or the
/// refusals any command can give, is the guide's to say.
const ABOUT: &str = "pile \u{2014} https://multipile.dev

Let an agent drive pile. In a project with no piles yet, hand it the
adopt workflow and it lays the project out in piles:

  pile guide adopt | claude

In a repository that already has piles:

  pile guide --agents   the section for your rules file, such as AGENTS.md
                        or CLAUDE.md, near its start, so every agent
                        session starts from the guide
  pile guide            where a session begins, for you or for an agent";

/// The whole command line: every command, its flags and its help. It is a
/// function rather than a value, because completion builds it again on every
/// keystroke.
pub fn command() -> Command {
    let root = Command::new("pile")
        .bin_name("pile")
        .about("Structure inside a git repository")
        .help_template(format!(
            "{ABOUT}\n\nUsage: pile <command> [flags]\n\nCommands:\n{{subcommands}}\n\n\
             Flags:\n{{options}}\n\n\
             Use \"pile <command> --help\" for more about one command.\n"
        ))
        .subcommand(one_help(with_help(init::command())))
        .subcommand(one_help(with_help(set::command())))
        .subcommand(one_help(with_help(id::command())))
        .subcommand(one_help(with_help(list::command())))
        .subcommand(one_help(with_help(show::command())))
        .subcommand(one_help(with_help(which::command())))
        .subcommand(one_help(with_help(files::command())))
        .subcommand(one_help(with_help(status::command())))
        .subcommand(one_help(with_help(gitdiff::command())))
        .subcommand(one_help(with_help(log::command())))
        .subcommand(one_help(with_help(doctor::command())))
        .subcommand(one_help(with_help(guide::command())))
        .subcommand(one_help(with_help(version::command())))
        .subcommand(one_help(with_help(complete::command())))
        .arg(
            Arg::new("format")
                .long("format")
                .value_name("text|json|markdown")
                .global(true)
                .help_heading("Flags")
                .help(
                    "how the answer is printed: text, json (the default for an agent), \
                     or markdown (pile gitdiff only)",
                ),
        )
        .disable_help_subcommand(true)
        .subcommand(help_command());
    with_help(root)
}

/// Lays a command's help out as one text: what it does, how it is typed, and
/// its flags.
///
/// clap tells a short help from a long one. Two helps for one command are two
/// texts to keep true, and the short one is the one nobody reads closely. So
/// the long description goes at the top of the only help, and -h, --help and
/// `pile help <command>` all print it. The usage line already names the
/// positional arguments, so they are not listed again.
fn one_help(cmd: Command) -> Command {
    let long = cmd.get_long_about().map(|a| a.to_string());
    let about = cmd.get_about().map(|a| a.to_string()).unwrap_or_default();
    cmd.long_about(None)
        .before_help(long.unwrap_or(about))
        .help_template("{before-help}{usage-heading} {usage}\n\n{all-args}{after-help}")
        .mut_args(|a| {
            if a.is_positional() {
                a.hide(true)
            } else {
                a.help_heading("Flags")
            }
        })
}

/// One help flag. -h and --help both print the whole help, for the reason
/// [`one_help`] gives.
fn with_help(cmd: Command) -> Command {
    cmd.disable_help_flag(true).arg(
        Arg::new("help")
            .short('h')
            .long("help")
            .help("print this help")
            .action(ArgAction::HelpShort),
    )
}

/// The positional arguments of a command that takes none. They are hidden,
/// and there so that one given anyway is refused in `pile`'s own words.
fn no_positional() -> Arg {
    Arg::new("args")
        .num_args(0..)
        .action(ArgAction::Append)
        .hide(true)
}

/// Refuses a positional argument given to a command that takes none.
fn no_args(name: &str, m: &ArgMatches) -> Result {
    if let Some(first) = m.get_many::<String>("args").and_then(|mut v| v.next()) {
        return Err(Refusal::new(
            Code::Usage,
            format!("pile {name} takes no arguments, and was given {first:?}"),
        ));
    }
    Ok(())
}

/// A flag that takes no value.
fn switch(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .help(help)
        .action(ArgAction::SetTrue)
}

/// Runs the command line, prints its answer in the format asked for, and
/// returns the code the process exits with.
pub fn execute() -> i32 {
    let argv: Vec<OsString> = std::env::args_os().collect();
    let (mode, bad_format) = Mode::of(&argv, |v| std::env::var_os(v));
    let mut out = Out::new();
    let result = match bad_format {
        Some(bad) => Err(answer::unknown_format(&bad)),
        None => match command().try_get_matches_from(answer::without_format(&argv)) {
            Ok(m) => dispatch(&m, &mut out, mode),
            Err(e) => clap_error(e, &argv, &mut out),
        },
    }
    .map_err(|r| with_help_next(r, &argv));
    let printed = answer::finish(mode, &answer::command_line(&argv), out, result);
    print(&printed.stdout);
    if !printed.stderr.is_empty() {
        let _ = std::io::stderr().write_all(printed.stderr.as_bytes());
    }
    printed.code
}

/// Runs the command asked for.
fn dispatch(matches: &ArgMatches, out: &mut Out, mode: Mode) -> Result {
    if mode.format == answer::Format::Markdown
        && let Some((name, _)) = matches.subcommand()
        && name != "help"
        && !answer::MARKDOWN.contains(&name)
    {
        return Err(answer::no_markdown(&format!("pile {name}")));
    }
    match matches.subcommand() {
        Some(("init", m)) => init::run(m, out),
        Some(("set", m)) => set::run(m, out),
        Some(("id", m)) => id::run(m, out),
        Some(("list", m)) => list::run(m, out),
        Some(("show", m)) => show::run(m, out),
        Some(("which", m)) => which::run(m, out),
        Some(("files", m)) => files::run(m, out),
        Some(("status", m)) => status::run(m, out),
        Some(("gitdiff", m)) => gitdiff::run(m, out),
        Some(("log", m)) => log::run(m, out),
        Some(("doctor", m)) => doctor::run(m, out),
        Some(("guide", m)) => guide::run(m, out),
        Some(("version", m)) => version::run(m, out, mode),
        Some(("completion", m)) => complete::run(m, out),
        Some(("help", m)) => help(m.get_one::<String>("command"), out),
        // A bare `pile` prints the help. That is not a mistake to have typed.
        // An agent is asking where it is, and inside a repository that
        // declares piles it is told.
        _ => {
            if mode.agent.is_some() && mode.format != answer::Format::Markdown && orient(out, mode)?
            {
                return Ok(());
            }
            help(None, out)
        }
    }
}

/// A bare `pile` for an agent: which binary this is, and every pile once,
/// with its links and what changed in it, in one answer, so a session
/// starts oriented.
/// False where there is nothing to orient in: outside a repository, in one
/// that declares no pile, or in one whose piles cannot be read. There the
/// help is the answer.
fn orient(out: &mut Out, mode: Mode) -> Result<bool> {
    let Ok(r) = open() else {
        return Ok(false);
    };
    if !r.piles().is_ok_and(|s| !s.is_empty()) {
        return Ok(false);
    }
    let mut parts = Vec::new();
    for words in [&["version"][..], &["list"], &["status", "--all", "--brief"]] {
        let argv = std::iter::once("pile").chain(words.iter().copied());
        let m = command()
            .try_get_matches_from(argv)
            .map_err(|e| Refusal::new(Code::Failed, e.to_string()))?;
        let mut part = Out::new();
        dispatch(&m, &mut part, mode)?;
        parts.push(part);
    }
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            putln!(out);
        }
        put!(out, "{}", part.as_str());
    }
    let next = [
        (
            "pile guide",
            "where a session begins, and the workflows to carry out",
        ),
        (
            "pile show <pile>",
            "prints one pile whole, its intent with it",
        ),
        (
            "pile doctor",
            "says what is wrong with any pile, and must exit 0 before a commit",
        ),
        ("pile --help", "lists every command"),
    ];
    putln!(out);
    for (command, why) in next {
        putln!(out, "  {command:<16}  {why}");
        out.next(command, why);
    }
    let mut taken = parts.into_iter().map(|p| p.result.unwrap_or(Value::Null));
    let version = taken.next().unwrap_or(Value::Null);
    let listed = taken.next().unwrap_or(Value::Null);
    let status = taken.next().unwrap_or(Value::Null);
    out.result(json!({
        "version": version,
        "piles": oriented(&listed, &status),
        "total": status.get("total"),
    }));
    Ok(true)
}

/// Each pile once, as `pile list` gives it, with the counts
/// `pile status` gives it beside: `name`, `path`, `id`, `links`, `changed`,
/// `untracked` and `crossed`. A pile is matched by its path, which no two
/// piles share.
fn oriented(listed: &Value, status: &Value) -> Vec<Value> {
    let counted = status["piles"].as_array().map_or(&[][..], Vec::as_slice);
    let listed = listed["piles"].as_array().map_or(&[][..], Vec::as_slice);
    listed
        .iter()
        .map(|p| {
            let mut p = p.clone();
            if let Some(c) = counted.iter().find(|c| c["path"] == p["path"])
                && let (Some(p), Some(c)) = (p.as_object_mut(), c.as_object())
            {
                for key in ["changed", "untracked", "crossed"] {
                    if let Some(v) = c.get(key) {
                        p.insert(key.into(), v.clone());
                    }
                }
            }
            p
        })
        .collect()
}

/// The repository `pile` was run in.
fn open() -> Result<Repository> {
    let here = std::env::current_dir().map_err(|e| {
        Refusal::new(
            Code::UnresolvablePath,
            format!("the directory pile runs in cannot be read: {}", os_says(&e)),
        )
    })?;
    Repository::open(&here)
}

/// A name as a result spells it: without its @, and `null` for none.
fn name_of(name: &str) -> Value {
    if name.is_empty() {
        Value::Null
    } else {
        json!(name)
    }
}

/// A pile as a result names one: `name`, `path`, `id`.
fn pile_value(p: &piles::Pile) -> Value {
    json!({"name": name_of(&p.name), "path": piles::shown(&p.path), "id": p.id})
}

/// "1 file", "3 files".
fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// A labelled line under a heading: three spaces in, the label in a column
/// ten wide.
fn say(out: &mut Out, label: &str, text: &str) {
    putln!(out, "   {label:<10}{text}");
}

/// A label with a line for each item, the first beside it and the rest
/// under the first.
fn say_all(out: &mut Out, label: &str, items: &[String]) {
    for (i, item) in items.iter().enumerate() {
        let l = if i == 0 { label } else { "" };
        say(out, l, item);
    }
}

/// The positional argument of a command that takes at most one pile.
fn path_arg() -> Arg {
    Arg::new("args")
        .num_args(0..)
        .action(ArgAction::Append)
        .add(complete::piles())
}

/// The one pile a command was given, or None. More than one is refused.
fn one_path(name: &str, m: &ArgMatches) -> Result<Option<String>> {
    let args: Vec<&String> = m.get_many::<String>("args").into_iter().flatten().collect();
    match args.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some((*one).clone())),
        many => Err(Refusal::new(
            Code::Usage,
            format!(
                "pile {name} takes one pile at most, and was given {}",
                many.len()
            ),
        )),
    }
}

/// `pile help`: the help of `pile`, or of the command named. It is the
/// same text --help prints, so the two cannot drift apart.
fn help_command() -> Command {
    one_help(with_help(
        Command::new("help")
            .about("Print the help of pile, or of one command")
            .long_about(
                "Print the help of pile, or of the command named.\n\
                 pile help version prints what pile version --help prints.",
            )
            .override_usage("pile help [command]")
            .arg(Arg::new("command").num_args(0..=1)),
    ))
}

fn help(name: Option<&String>, out: &mut Out) -> Result {
    let mut root = command();
    root.build();
    let text = match name {
        None => root.render_help(),
        Some(name) => match root.find_subcommand_mut(name) {
            Some(cmd) => cmd.render_help(),
            None => {
                return Err(Refusal::new(
                    Code::UnknownCommand,
                    format!("{name:?} is not a command"),
                )
                .next("pile --help", "lists every command"));
            }
        },
    };
    put!(out, "{text}");
    out.raw = true;
    Ok(())
}

/// The values an argument of a command takes, by the name its help shows
/// it by, such as `<shell>`. None when it takes any value.
fn possible_values(cmd: &str, arg: &str) -> Vec<String> {
    let id = arg.trim_start_matches('<').trim_end_matches('>');
    let name = cmd.strip_prefix("pile ").unwrap_or_default();
    command()
        .find_subcommand(name)
        .and_then(|c| c.get_arguments().find(|a| a.get_id() == id))
        .map(|a| {
            a.get_possible_values()
                .iter()
                .map(|v| v.get_name().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// "a", "a or b", "a, b or c".
fn one_of(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// Puts an answer on stdout.
///
/// A reader that went away, as in `pile version | head -1`, is not a failure
/// of the command: it has read enough. Any other failure to write has nowhere
/// left to be reported, and the exit code is still the command's.
fn print(text: &str) {
    if text.is_empty() {
        return;
    }
    let mut stdout = std::io::stdout().lock();
    let _ = stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush());
}

/// What clap refused, as `pile` says it.
///
/// Help that was asked for is not a refusal, and exits 0. Help is printed as
/// text in every format. Everything else clap refuses is a mistake in what was
/// typed, so it exits 2. The refusal names the help to read, instead of the
/// usage clap would print under it.
fn clap_error(e: clap::Error, argv: &[OsString], out: &mut Out) -> Result {
    match e.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            put!(out, "{}", e.render());
            out.raw = true;
            return Ok(());
        }
        _ => {}
    }

    let context = |kind| match e.get(kind) {
        Some(ContextValue::String(s)) => Some(s.clone()),
        Some(ContextValue::Strings(s)) => s.first().cloned(),
        _ => None,
    };
    let help = format!("{} --help", command_path(argv));
    let refusal = match e.kind() {
        ErrorKind::InvalidSubcommand => {
            let typed = context(ContextKind::InvalidSubcommand).unwrap_or_default();
            let mut message = format!("{typed:?} is not a command");
            if let Some(ContextValue::Strings(near)) = e.get(ContextKind::SuggestedSubcommand) {
                message.push_str(&format!(". Did you mean {}?", near.join(" or ")));
            }
            Refusal::new(Code::UnknownCommand, message).next("pile --help", "lists every command")
        }
        ErrorKind::UnknownArgument => {
            let typed = context(ContextKind::InvalidArg).unwrap_or_default();
            Refusal::new(
                Code::UnknownFlag,
                format!("{} has no flag {typed}", command_path(argv)),
            )
            .next(help, TAKES)
        }
        ErrorKind::InvalidValue => {
            let arg = context(ContextKind::InvalidArg).unwrap_or_default();
            let value = context(ContextKind::InvalidValue).unwrap_or_default();
            let valid = match e.get(ContextKind::ValidValue) {
                Some(ContextValue::Strings(v)) => v.clone(),
                _ => Vec::new(),
            };
            let cmd = command_path(argv);
            let message = if valid.is_empty() {
                format!("{cmd} does not take {value:?} as {arg}")
            } else {
                format!("{cmd} takes {} as {arg}, not {value:?}", one_of(&valid))
            };
            Refusal::new(Code::Usage, message).next(help, TAKES)
        }
        ErrorKind::MissingRequiredArgument => {
            let arg = context(ContextKind::InvalidArg).unwrap_or_default();
            let cmd = command_path(argv);
            let valid = possible_values(&cmd, &arg);
            let message = if valid.is_empty() {
                format!("{cmd} needs {arg}")
            } else {
                format!("{cmd} needs {arg}: {}", one_of(&valid))
            };
            Refusal::new(Code::Usage, message).next(help, TAKES)
        }
        _ => {
            // clap's own sentence, without its prefix, and without the usage
            // and the advice under it.
            let rendered = e.render().to_string();
            let said: Vec<&str> = rendered
                .lines()
                .take_while(|l| !l.trim().is_empty() && !l.starts_with("Usage:"))
                .collect();
            let message = said
                .iter()
                .map(|l| l.trim().trim_end_matches(':'))
                .collect::<Vec<_>>()
                .join(": ")
                .trim_start_matches("error: ")
                .to_string();
            Refusal::new(Code::Usage, message).next(help, TAKES)
        }
    };
    Err(refusal)
}

/// A `usage` refusal that names no fix gets the command's help as its fix,
/// as clap's own refusals do, because the help says what the command takes.
fn with_help_next(r: Refusal, argv: &[OsString]) -> Refusal {
    if r.code != Code::Usage || !r.next.is_empty() {
        return r;
    }
    let help = format!("{} --help", command_path(argv));
    r.next(help, TAKES)
}

/// The command a refused invocation was for, as it is typed: "pile version",
/// or "pile" when none was named.
fn command_path(argv: &[OsString]) -> String {
    let root = command();
    argv.iter()
        .skip(1)
        .map(|a| a.to_string_lossy())
        .find(|a| root.get_subcommands().any(|c| c.get_name() == a))
        .map(|n| format!("pile {n}"))
        .unwrap_or_else(|| "pile".to_string())
}
