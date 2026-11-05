use clap::{ArgMatches, Command};

use super::{Out, Result, no_args, no_positional};
use crate::exit::{Code, Refusal};

pub(super) fn command() -> Command {
    Command::new("id")
        .about("Print a fresh pile id")
        .long_about(
            "Print a fresh id: sixteen random bytes, in lowercase hex.\n\n\
             id writes no file and needs no repository. It is for a config written\n\
             some other way than pile init, by hand or by a script, which needs an id\n\
             of its own. pile init mints its own.\n\n\
             The id is printed alone, as text in every format, so another command\n\
             can read it as it is.",
        )
        .override_usage("pile id")
        .arg(no_positional())
}

pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    no_args("id", m)?;
    let id = piles::config::new_id().map_err(|e| {
        Refusal::new(
            Code::Failed,
            format!("could not read random bytes for an id: {e}"),
        )
    })?;
    putln!(out, "{id}");
    out.raw = true;
    Ok(())
}
