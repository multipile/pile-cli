//! How an answer is printed: in which format, and for whom.
//!
//! A command writes its text into an [`Out`], and beside it says what its
//! answer is as data: `result`, and the commands worth running `next`. After
//! the command returns, [`finish`] decides which of the two reaches stdout.
//! So no command has to know which format it is answering in.
//!
//! A structured answer is one object on stdout, the envelope:
//!
//! - `ok` is `true` exactly when the exit code is 0.
//! - `command` is the command as run, the way a person would type it back.
//! - `result` is the answer, when `ok` is `true`.
//! - `error` is the refusal, in place of `result`, when `ok` is `false`. It
//!   holds `code`, `message`, and `usage`, which is `true` when the exit code
//!   is 2.
//! - `next` is the commands worth running after this one, possibly none. For
//!   a refusal, they are the commands that fix it.
//!
//! In text, a refusal goes to stderr after `pile:`. In a structured format it
//! is the envelope on stdout, and stderr stays empty.
//!
//! Some answers are text in every format, because an envelope would stand
//! between them and their reader: help, and a value meant to be typed into
//! another program, such as `version --short` or a completion script. A
//! command sets `raw` for those. A refusal is still an envelope.
//!
//! [`Mode::of`] is the one place agent mode is decided, and [`finish`] the
//! one place it changes what is printed.

use std::ffi::OsString;
use std::ops::{Deref, DerefMut};

use serde_json::{Map, Value, json};

use crate::exit::{self, Code, Next, Refusal};

/// The formats an answer is printed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
}

impl Format {
    pub const NAMES: &[&str] = &["text", "json"];

    fn parse(s: &str) -> Option<Format> {
        match s {
            "text" => Some(Format::Text),
            "json" => Some(Format::Json),
            _ => None,
        }
    }
}

/// The variables that say an agent is running `pile`, in the order they are
/// read. The first one set to anything but the empty string turns agent mode
/// on. Its value is not read.
pub const AGENT_VARS: &[&str] = &["AI_AGENT", "AGENT"];

/// Who is being answered, and in which format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    pub format: Format,
    /// The variable that turned agent mode on, or None for a person.
    pub agent: Option<&'static str>,
}

impl Mode {
    /// The mode a command line asks for, with the variables read through
    /// `var`.
    ///
    /// The format is `--format` where it is given. Otherwise it is JSON for
    /// an agent and text for anybody else. `--format` may come anywhere
    /// before `--`, and the last one given wins. A value that is not a
    /// format comes back as the second item. It is refused before clap sees
    /// the line, so the refusal is printed in the mode the reader gets
    /// everything else in.
    pub fn of(argv: &[OsString], var: impl Fn(&str) -> Option<OsString>) -> (Mode, Option<String>) {
        let agent = AGENT_VARS
            .iter()
            .copied()
            .find(|v| var(v).is_some_and(|s| !s.is_empty()));
        let mut format = None;
        let mut bad = None;
        let mut args = argv.iter().skip(1).map(|a| a.to_string_lossy());
        while let Some(a) = args.next() {
            if a == "--" {
                break;
            }
            let value = if a == "--format" {
                args.next().map(|v| v.into_owned())
            } else if let Some(v) = a.strip_prefix("--format=") {
                Some(v.to_string())
            } else {
                continue;
            };
            match value.as_deref().and_then(Format::parse) {
                Some(f) => format = Some(f),
                None => bad = Some(value.unwrap_or_default()),
            }
        }
        let default = if agent.is_some() {
            Format::Json
        } else {
            Format::Text
        };
        let mode = Mode {
            format: format.unwrap_or(default),
            agent,
        };
        (mode, bad)
    }
}

/// The command line without `--format` and its value, read the way
/// [`Mode::of`] reads them.
///
/// clap is handed this line. `--format` has been read by then, and clap's
/// own `help` command would take its value for the name of a command.
pub fn without_format(argv: &[OsString]) -> Vec<OsString> {
    let mut kept = Vec::with_capacity(argv.len());
    let mut args = argv.iter();
    if let Some(prog) = args.next() {
        kept.push(prog.clone());
    }
    while let Some(a) = args.next() {
        let word = a.to_string_lossy();
        if word == "--" {
            kept.push(a.clone());
            kept.extend(args.cloned());
            break;
        }
        if word == "--format" {
            args.next();
            continue;
        }
        if word.starts_with("--format=") {
            continue;
        }
        kept.push(a.clone());
    }
    kept
}

/// What a command answers with: the text a person reads, and the same answer
/// as data.
#[derive(Debug, Default)]
pub struct Out {
    text: String,
    /// The answer as data, which a structured format prints as `result`.
    pub result: Option<Value>,
    /// The commands worth running next.
    pub next: Vec<Next>,
    /// The answer is printed as text in every format.
    pub raw: bool,
}

impl Out {
    pub fn new() -> Out {
        Out::default()
    }

    /// Sets the result.
    pub fn result(&mut self, value: Value) {
        self.result = Some(value);
    }

    /// Adds a command worth running next.
    pub fn next(&mut self, command: impl Into<String>, why: impl Into<String>) {
        self.next.push(Next::new(command, why));
    }
}

impl Deref for Out {
    type Target = String;
    fn deref(&self) -> &String {
        &self.text
    }
}

impl DerefMut for Out {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.text
    }
}

/// The command as run, the way a person would type it back.
pub fn command_line(argv: &[OsString]) -> String {
    let mut words = vec!["pile".to_string()];
    words.extend(
        argv.iter()
            .skip(1)
            .map(|a| shell_word(&a.to_string_lossy())),
    );
    words.join(" ")
}

/// A word as a shell needs it typed: bare when nothing in it means anything
/// to a shell, and in single quotes otherwise.
fn shell_word(s: &str) -> String {
    let plain = !s.is_empty()
        && s.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '-' | '_' | '.' | '/' | '@' | ':' | '=' | ',' | '+')
        });
    if plain {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

/// The commands once each, in the order first named. Where one naming gave
/// no reason and another did, the reason is kept.
fn once_each(next: Vec<Next>) -> Vec<Next> {
    let mut kept: Vec<Next> = Vec::new();
    for n in next {
        match kept.iter_mut().find(|k| k.command == n.command) {
            Some(k) if k.why.is_empty() => k.why = n.why,
            Some(_) => {}
            None => kept.push(n),
        }
    }
    kept
}

/// The envelope, with its keys in the order they are listed above.
fn envelope(command: &str, result: Option<Value>, error: Option<Value>, next: &[Next]) -> Value {
    let mut env = Map::new();
    env.insert("ok".into(), Value::Bool(error.is_none()));
    env.insert("command".into(), Value::String(command.into()));
    if let Some(r) = result {
        env.insert("result".into(), r);
    }
    if let Some(e) = error {
        env.insert("error".into(), e);
    }
    let next = next
        .iter()
        .map(|n| json!({"command": n.command, "why": n.why}))
        .collect();
    env.insert("next".into(), Value::Array(next));
    Value::Object(env)
}

fn render(value: &Value) -> String {
    let mut s = serde_json::to_string_pretty(value).expect("JSON from a Value");
    s.push('\n');
    s
}

/// What a finished command prints, and where.
#[derive(Debug, PartialEq, Eq)]
pub struct Printed {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// Decides what a command's answer is printed as, and where. This is the
/// whole of the difference between the formats, and between a person and an
/// agent.
pub fn finish(mode: Mode, command: &str, mut out: Out, result: Result<(), Refusal>) -> Printed {
    let structured = mode.format != Format::Text;
    let text = std::mem::take(&mut out.text);
    let refusal = match result {
        Ok(()) => {
            let stdout = if structured && !out.raw {
                let result = out.result.unwrap_or(Value::Null);
                render(&envelope(command, Some(result), None, &out.next))
            } else {
                text
            };
            return Printed {
                stdout,
                stderr: String::new(),
                code: exit::OK,
            };
        }
        Err(r) => r,
    };
    let code = refusal.exit();
    if !structured {
        // An agent reads stdout, so a refusal goes there too, and reads the
        // same as it would on stderr.
        return if mode.agent.is_some() {
            Printed {
                stdout: text + &refusal.text(),
                stderr: String::new(),
                code,
            }
        } else {
            Printed {
                stdout: text,
                stderr: refusal.text(),
                code,
            }
        };
    }
    let mut next = refusal.next.clone();
    next.append(&mut out.next);
    let error = json!({
        "code": refusal.code.as_str(),
        "message": refusal.message,
        "usage": refusal.usage(),
    });
    // A command that refuses may still have answered, as a verdict that
    // failed does. Then the result is kept beside the error.
    let env = envelope(command, out.result.take(), Some(error), &once_each(next));
    Printed {
        stdout: render(&env),
        stderr: String::new(),
        code,
    }
}

/// The refusal for a `--format` that is not one.
pub fn unknown_format(bad: &str) -> Refusal {
    Refusal::new(
        Code::UnknownFormat,
        format!("--format is {}, not {bad:?}", Format::NAMES.join(" or ")),
    )
}

#[cfg(test)]
#[path = "answer_test.rs"]
mod tests;
