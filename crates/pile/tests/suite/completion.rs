//! `pile completion`: the script a shell registers, and the answers the
//! binary gives when that script calls it back.

use crate::harness::*;

#[test]
fn each_shell_gets_a_script_that_calls_pile_back_by_name() {
    let r = Repo::new();
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let out = r.stdout(&["completion", shell, "--format", "json"]);
        contains!(out, "COMPLETE");
        assert!(!out.trim_start().starts_with('{'), "{shell}: {out}");
    }
}

#[test]
fn a_call_back_offers_the_commands() {
    let r = Repo::new();
    let mut cmd = command(bin(), r.path());
    cmd.env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "1")
        .args(["--", "pile", "ver"]);
    let out = cmd.output().expect("running pile");
    assert!(out.status.success());
    contains!(String::from_utf8_lossy(&out.stdout), "version");
}

#[test]
fn a_missing_shell_is_refused_naming_the_shells() {
    let r = Repo::new();
    let a = r.answered(&[], &["completion"]);
    assert_eq!(a.code, 2);
    assert_eq!(
        a.stderr,
        "pile: pile completion needs <shell>: bash, zsh, fish or powershell\n  \
         pile completion --help  says what this command takes\n"
    );
}

#[test]
fn a_shell_pile_does_not_complete_is_refused_naming_the_shells() {
    let r = Repo::new();
    let a = r.answered(&[], &["completion", "tcsh"]);
    assert_eq!(a.code, 2);
    contains!(
        a.stderr,
        "pile: pile completion takes bash, zsh, fish or powershell as <shell>, not \"tcsh\"\n"
    );
}

/// What the binary offers for the argument after `pile show`, given what
/// has been typed of it.
fn offered(r: &Repo, typed: &str) -> String {
    let mut cmd = command(bin(), r.path());
    cmd.env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "pile", "show", typed]);
    let out = cmd.output().expect("running pile");
    assert!(out.status.success());
    // A line each, the last one ended too, so a test can look for one whole.
    String::from_utf8_lossy(&out.stdout).into_owned() + "\n"
}

#[test]
fn a_pile_or_a_path_completes_in_each_of_its_three_forms() {
    let r = crate::piles::nested();
    let names = offered(&r, "@");
    contains!(names, "@lib\n", "@v2\n", "@top\n");
    let from_top = offered(&r, ":/li");
    contains!(from_top, ":/lib/\n", ":/lib-v2/\n");
    let deeper = offered(&r, ":/lib/s");
    contains!(deeper, ":/lib/sub/\n");
    // A path completes as the shell would, with the bare names beside it.
    let bare = offered(&r, "li");
    contains!(bare, "lib/\n", "lib-v2/\n", "lib\n");
}
