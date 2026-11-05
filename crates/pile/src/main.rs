//! The `pile` binary.

fn main() {
    // A shell asking what to complete sets COMPLETE and calls back here. That
    // is answered, and the process exits before anything else runs.
    clap_complete::CompleteEnv::with_factory(pile::cli::command)
        .var(pile::cli::COMPLETE_VAR)
        .complete();

    std::process::exit(pile::cli::execute());
}
