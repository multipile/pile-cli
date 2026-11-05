//! Completion: the script a shell registers.
//!
//! The script is clap_complete's dynamic one. On every keystroke it hands the
//! line being typed back to this binary, which answers from the command line
//! [`super::command`] builds. src/main.rs answers that call before anything
//! else runs.
//!
//! Where a command takes a pile or a path, it offers what the repository
//! holds at that keystroke, in each of the forms a pile or a path is typed.

use clap::{Arg, ArgMatches, Command};
use std::collections::BTreeSet;
use std::ffi::OsStr;

use clap_complete::engine::{
    ArgValueCompleter, CompletionCandidate, PathCompleter, ValueCompleter,
};
use piles::Structure;
use storage::{State, Subtree};

use crate::repository::Repository;
use clap_complete::env::{Bash, EnvCompleter, Fish, Powershell, Zsh};

use super::{Out, Result};
use crate::exit::{Code, Refusal};

/// The environment variable that turns an invocation into a completion. The
/// registration script sets it when it calls back.
pub const VAR: &str = "COMPLETE";

pub(super) fn command() -> Command {
    Command::new("completion")
        .about("Print the completion script for a shell")
        .long_about(
            "Print the script that registers completion for pile with a shell.\n\n\
             On every keystroke, the script asks the pile on PATH what to offer.\n\
             So the commands and flags offered are always those of that binary.\n\n\
             Load the script once per shell:\n\n  \
             pile completion bash > /usr/local/etc/bash_completion.d/pile\n  \
             pile completion zsh > \"${fpath[1]}/_pile\"\n  \
             pile completion fish > ~/.config/fish/completions/pile.fish\n  \
             pile completion powershell | Out-String | Invoke-Expression\n\n\
             The script is text in every format, because a shell reads it.",
        )
        .override_usage("pile completion <shell>")
        .arg(
            Arg::new("shell")
                .required(true)
                .value_parser(["bash", "zsh", "fish", "powershell"])
                .help("bash, zsh, fish or powershell"),
        )
}

pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    out.raw = true;
    let shell: &dyn EnvCompleter = match m.get_one::<String>("shell").map(String::as_str) {
        Some("bash") => &Bash,
        Some("zsh") => &Zsh,
        Some("fish") => &Fish,
        _ => &Powershell,
    };
    let mut buf = Vec::new();
    // The script calls back pile on PATH, by name. A script pinned to the
    // path of the binary that printed it would go on calling that binary
    // after it was replaced.
    shell
        .write_registration(VAR, "pile", "pile", "pile", &mut buf)
        .map_err(|e| {
            Refusal::new(
                Code::Failed,
                format!("could not write the completion script: {e}"),
            )
        })?;
    put!(out, "{}", String::from_utf8_lossy(&buf));
    Ok(())
}

/// What an argument that takes a pile or a path offers, in each of the three
/// forms (pile guide paths): after `@`, the names piles claim; after `:`, the
/// repository's own paths from the top, a segment at a time; and otherwise
/// paths as the shell completes them, with the names a bare word can be.
/// Outside a repository, or where the piles cannot be read, it offers only
/// what the shell would.
pub(super) fn piles() -> ArgValueCompleter {
    ArgValueCompleter::new(|current: &OsStr| {
        let typed = current.to_string_lossy();
        let read = super::open()
            .ok()
            .and_then(|r| r.piles().ok().map(|s| (r, s)));
        if let Some(name) = typed.strip_prefix(piles::SIGIL) {
            let Some((_, s)) = read else {
                return Vec::new();
            };
            return named(&s, name, true);
        }
        if typed.starts_with(':') {
            let Some((r, _)) = read else {
                return Vec::new();
            };
            return from_top(&r, &typed);
        }
        let mut out = PathCompleter::any().complete(current);
        if let Some((_, s)) = read {
            out.extend(named(&s, &typed, false));
        }
        out
    })
}

/// The names that start with what was typed, each with its pile's headline.
fn named(s: &Structure, typed: &str, sigil: bool) -> Vec<CompletionCandidate> {
    s.ordered()
        .into_iter()
        .filter(|p| !p.name.is_empty() && p.name.starts_with(typed))
        .map(|p| {
            let word = if sigil {
                piles::typed(&p.name)
            } else {
                p.name.clone()
            };
            let help = p.headline();
            CompletionCandidate::new(word).help((!help.is_empty()).then(|| help.into()))
        })
        .collect()
}

/// The `:/` paths that continue what was typed, to the end of the next
/// segment: a directory ends in `/`, so the next keystroke goes inside it.
fn from_top(r: &Repository, typed: &str) -> Vec<CompletionCandidate> {
    let Ok(files) = r.storage().files(&State::Working, &Subtree::whole()) else {
        return Vec::new();
    };
    let mut seen: BTreeSet<String> = BTreeSet::new();
    // Every pile's directory holds its config, so the files reach every pile.
    for path in files.iter().map(|f| f.path.as_str()) {
        let shown = piles::shown(path);
        let Some(rest) = shown.strip_prefix(typed) else {
            continue;
        };
        let word = match rest.find('/') {
            Some(i) => &shown[..typed.len() + i + 1],
            None => shown.as_str(),
        };
        seen.insert(word.to_string());
    }
    seen.into_iter().map(CompletionCandidate::new).collect()
}
