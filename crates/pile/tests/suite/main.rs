//! The suite that drives the built `pile` binary against real git
//! repositories.
//!
//! It is one test binary rather than one per file, so the harness is compiled
//! once and every module shares it. Each module puts the claims that one part
//! of the command line makes to the binary, from outside it.

#[macro_use]
mod harness;

mod agent;
mod answers;
mod aux;
mod completion;
mod declare;
mod doctor;
mod files;
mod gitdiff;
mod guide;
mod links;
mod list;
mod log;
mod piles;
mod show;
mod status;
mod version;
mod which;
