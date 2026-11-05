use clap::{ArgMatches, Command};
use serde_json::json;

use super::{Out, Result, no_args, no_positional, switch};
use crate::answer::Mode;
use crate::buildinfo;

pub(super) fn command() -> Command {
    Command::new("version")
        .about("Print the version of this binary")
        .long_about(
            "Print what this binary is: its version, the commit it was built from,\n\
             when it was built, and the compiler and target it was built with.\n\
             version needs no repository.\n\n\
             A binary built from source, rather than taken from a release, has the\n\
             version dev. If its build did not record the commit and the date,\n\
             both say unknown.\n\n\
             The agent line says whether pile is answering an agent, and which\n\
             variable says so. When both are set, the line names AI_AGENT.\n\
             pile guide agent-mode says what turns it on, and what it changes.\n\n\
             --short prints the version alone, as text in every format, for a\n\
             script to compare.",
        )
        .override_usage("pile version [--short]")
        .arg(no_positional())
        .arg(switch("short", "print the version alone"))
}

/// What this binary is. The result holds `version`; `commit`, the short id
/// the build stamped in, or `unknown`; `built`; `rustc`, the compiler and the
/// target; and `agent`, which is `null` or the variable that turned agent
/// mode on.
pub(super) fn run(m: &ArgMatches, out: &mut Out, mode: Mode) -> Result {
    no_args("version", m)?;

    if m.get_flag("short") {
        putln!(out, "{}", buildinfo::VERSION);
        out.raw = true;
        return Ok(());
    }

    let rustc = buildinfo::rustc();
    putln!(out, "pile {}", buildinfo::VERSION);
    putln!(out, "commit {}", buildinfo::commit());
    putln!(out, "built  {}", buildinfo::date());
    putln!(out, "rustc  {rustc}");
    match mode.agent {
        Some(var) => putln!(out, "agent  on, by {var}"),
        None => putln!(out, "agent  off"),
    }
    out.result(json!({
        "version": buildinfo::VERSION,
        "commit": buildinfo::commit(),
        "built": buildinfo::date(),
        "rustc": rustc,
        "agent": mode.agent,
    }));
    Ok(())
}
