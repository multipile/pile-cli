//! What the `pile` binary stands on.
//!
//! - [`buildinfo`] is what this binary is, stamped at build time.
//! - [`exit`] is the exit codes, and the refusals that lead to them.
//! - [`answer`] is how an answer is printed, in which format and for whom.
//! - [`repository`] is the repository a command reads, and the refusals for
//!   a repository that cannot be read.
//! - [`write`] is the only writes `pile` makes: the `.pile/` files `init`
//!   and `set` write.
//! - [`cli`] is the command line on top of them, which src/main.rs runs.

pub mod answer;
pub mod buildinfo;
pub mod cli;
pub mod exit;
pub mod repository;
pub mod write;
