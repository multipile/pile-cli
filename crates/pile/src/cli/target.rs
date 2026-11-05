//! What an argument naming a pile or a path means, read the same way by
//! every command that takes one.
//!
//! There are three forms, and an argument is read as exactly one of them,
//! never as another because of what it happens to match:
//!
//! 1. A name: `@lib`, which is only ever a name, or a bare word such as `lib`
//!    when a pile claims it as its name. A bare word no pile claims is a
//!    path.
//! 2. A path from the top of the repository: `:/src/x.rs`, and `:/` for the
//!    top. It is read without the disk, so it need not exist, and one that
//!    climbs above the top with `..` is refused.
//! 3. Anything else is a path as the shell means it: relative to the
//!    directory `pile` runs in, or absolute. The deepest part of it that is
//!    on disk is resolved there, symbolic links followed, and the rest is
//!    put after it, so it need not exist either. One that ends up outside
//!    the repository is refused.
//!
//! Every path `pile` prints is in the second form, so whatever it printed
//! can be pasted back and means the same.

use std::path::{Component, Path, PathBuf};

use piles::{Pile, Structure};

use super::Result;
use crate::exit::{Code, Refusal};
use crate::repository::Repository;

/// What a command answers when it says which refusals any pile-taking
/// command gives. The help of each such command prints it.
pub(super) const REFUSALS: &str =
    "  no-piles                 1  the repository declares no pile: pile init declares one
  no-pile-here             1  no pile here or above: name one, or pile list
  path-outside-repository  1  the path is outside the repository, or a :/ path climbs above it
  not-a-pile               1  the path is in a pile, and not one: the message names it
  unknown-name             1  no pile claims the name: the message lists those that do
  name-claimed-twice       1  two piles claim the name, so it names neither
  invalid-name             2  the name could never be one
  unreadable-config        1  a pile's config is wrong: the message names the line";

/// The repository's piles, refused when it declares none.
pub(super) fn declared(r: &Repository) -> Result<Structure> {
    let s = r.piles()?;
    if s.is_empty() {
        return Err(no_piles(r));
    }
    Ok(s)
}

pub(super) fn no_piles(r: &Repository) -> Refusal {
    let top = r.top().map(|t| t.display().to_string()).unwrap_or_default();
    Refusal::new(Code::NoPiles, format!("no pile in {top}"))
        .next("pile init", "declares the directory you are in a pile")
        .next(
            "pile guide adopt",
            "brings piles to a project that declares none, step by step",
        )
}

/// The pile the command was run in: the nearest pile at or above it.
pub(super) fn standing<'a>(r: &Repository, s: &'a Structure) -> Result<&'a Pile> {
    s.owner(r.here()).ok_or_else(|| {
        Refusal::new(Code::NoPileHere, "no pile here or above")
            .next("pile list", "lists the piles there are")
            .next("pile status --all", "reports every pile, from anywhere")
            .next("pile init", "declares this directory a pile")
    })
}

/// The pile an argument names, or the pile the command was run in when it
/// names none. A path that is not a pile is refused, naming the pile it is
/// in, because quietly answering for that pile would hide whether the path
/// is declared.
pub(super) fn chosen<'a>(r: &Repository, s: &'a Structure, arg: Option<&str>) -> Result<&'a Pile> {
    let Some(arg) = arg else {
        return standing(r, s);
    };
    if let Some(name) = claimed(s, arg) {
        return by_name(s, name);
    }
    let path = path_of(r, arg)?;
    if let Some(pile) = s.find(&path) {
        return Ok(pile);
    }
    let at = piles::shown(&path);
    Err(match s.owner(&path) {
        Some(owner) => Refusal::new(
            Code::NotAPile,
            format!("{at} is not a pile. It is in {}", owner.heading()),
        )
        .next(format!("pile show {}", owner.handle()), "the pile it is in"),
        None => Refusal::new(
            Code::NotAPile,
            format!("{at} is not a pile, and is in no pile"),
        )
        .next("pile list", "lists the piles there are"),
    })
}

/// The path from the top that an argument names, whether or not anything is
/// there: a file about to be written is asked about too. A name stands for
/// its pile's own directory.
pub(super) fn where_is(r: &Repository, s: &Structure, arg: &str) -> Result<String> {
    if let Some(name) = claimed(s, arg) {
        return Ok(by_name(s, name)?.path.clone());
    }
    path_of(r, arg)
}

/// The path from the top that a path names, in either of its forms.
pub(super) fn path_of(r: &Repository, arg: &str) -> Result<String> {
    match arg.strip_prefix(piles::FROM_TOP) {
        Some(rest) => from_top(arg, rest),
        None if arg == ":" => from_top(arg, ""),
        None => on_disk(r, arg),
    }
}

/// A `:/` path, read without the disk.
fn from_top(arg: &str, rest: &str) -> Result<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(Refusal::new(
                        Code::PathOutsideRepository,
                        format!("{arg} climbs above the top of the repository"),
                    )
                    .next(
                        "pile guide paths",
                        "says the three forms a pile or a path is typed in",
                    ));
                }
            }
            p => parts.push(p),
        }
    }
    Ok(format!("/{}", parts.join("/")))
}

/// A path as the shell means it, as a path from the top of the repository.
fn on_disk(r: &Repository, arg: &str) -> Result<String> {
    let given = Path::new(arg);
    let full = if given.is_absolute() {
        given.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| {
                Refusal::new(
                    Code::UnresolvablePath,
                    format!("the directory pile runs in cannot be read: {}", os_says(&e)),
                )
            })?
            .join(given)
    };
    let at = resolved(&full);
    let top = r.top()?;
    let top = std::fs::canonicalize(top).unwrap_or_else(|_| top.to_path_buf());
    let Ok(rel) = at.strip_prefix(&top) else {
        // The place it resolved to, when that is not what was typed.
        let place = if at == Path::new(arg) {
            String::new()
        } else {
            format!(" is {}, which", at.display())
        };
        return Err(Refusal::new(
            Code::PathOutsideRepository,
            format!(
                "{arg}{place} is outside the repository at {}. A path from the top of the \
                 repository is written :/path",
                top.display()
            ),
        )
        .next(
            "pile guide paths",
            "says the three forms a pile or a path is typed in",
        ));
    };
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    Ok(format!("/{}", parts.join("/")))
}

/// An absolute path with its deepest part that is on disk resolved there,
/// symbolic links followed, and the rest put after it, with `.` and `..` in
/// the rest read off the path.
fn resolved(full: &Path) -> PathBuf {
    let parts: Vec<Component> = full.components().collect();
    for there in (1..=parts.len()).rev() {
        let prefix: PathBuf = parts[..there].iter().collect();
        let Ok(mut at) = std::fs::canonicalize(&prefix) else {
            continue;
        };
        for part in &parts[there..] {
            match part {
                Component::CurDir => {}
                Component::ParentDir => {
                    at.pop();
                }
                other => at.push(other),
            }
        }
        return at;
    }
    full.to_path_buf()
}

/// The name an argument is, when it is one: what follows `@`, or the bare
/// argument when a pile claims it.
pub(super) fn claimed<'a>(s: &Structure, arg: &'a str) -> Option<&'a str> {
    if let Some(name) = arg.strip_prefix(piles::SIGIL) {
        return Some(name);
    }
    (!s.by_name(arg).is_empty()).then_some(arg)
}

/// The one pile that claims a name.
pub(super) fn by_name<'a>(s: &'a Structure, name: &str) -> Result<&'a Pile> {
    let claimants = s.by_name(name);
    match claimants.as_slice() {
        [one] => Ok(one),
        [] => {
            if let Err(e) = piles::config::check_name(name) {
                return Err(Refusal::new(Code::InvalidName, e));
            }
            let names = s.names();
            if names.is_empty() {
                return Err(Refusal::new(
                    Code::UnknownName,
                    format!(
                        "no pile is named {}, and no pile in this repository has a name",
                        piles::typed(name)
                    ),
                )
                .next(
                    format!("pile set name {name} <pile>"),
                    "gives a pile this name",
                ));
            }
            let have: Vec<String> = names.iter().map(|n| piles::typed(n)).collect();
            Err(Refusal::new(
                Code::UnknownName,
                format!(
                    "no pile is named {}. The names in this repository are {}",
                    piles::typed(name),
                    have.join(" ")
                ),
            ))
        }
        many => {
            let paths: Vec<String> = many.iter().map(|p| piles::shown(&p.path)).collect();
            Err(Refusal::new(
                Code::NameClaimedTwice,
                format!(
                    "{} is claimed by {} piles, {}, so it names neither. A person chose \
                     the name, so the repair is to choose again",
                    piles::typed(name),
                    many.len(),
                    paths.join(" ")
                ),
            ))
        }
    }
}

/// What the operating system said, without the error number, starting in
/// lower case because it follows a colon.
pub(crate) fn os_says(e: &std::io::Error) -> String {
    let said = e.to_string();
    let said = match said.rfind(" (os error ") {
        Some(at) if said.ends_with(')') => said[..at].to_string(),
        _ => said,
    };
    let mut chars = said.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => said,
    }
}
