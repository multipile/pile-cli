## Piles

This repository declares piles: subtrees that each have a `.pile/` directory
and say in its `intent.md` why they are one thing. Work in it with the
`pile` command. Start every session with `pile guide`, and read what it
prints whole: it says how to read the repository by pile, and lists the
workflows and pages the binary carries. `pile doctor` must exit 0 before a
commit. `pile` makes no commit: commit with git, as this repository's own
rules say.

## Working in this repository

This repository builds `pile`, the binary of multipile.

### Before a commit

`pile guide commit-a-change` says what holds before any commit. Here, also:

- Run the whole check: `mise x -- scripts/check`. It must pass. It builds
  `bin/pile`, checks the formatting, runs clippy, every test and every
  journey, builds every platform a release is built for, and runs
  `pile doctor` on this repository's piles.
- `pile doctor` must exit 0. Run it on its own while you work: it says
  what is wrong with any pile, and how to fix it.
- `pile` makes no commit. Commit with `git commit`. Do not push or tag:
  CI tags and releases what reaches `main`.

### The toolchain

Every script here runs with the tools `mise.toml` pins. Either activate
mise in the shell, or put `mise x --` in front. mise puts `bin/` first on
PATH, and `bin/pile` is what `scripts/build` last wrote. Git does not track
it, so it can be stale.

### The piles

This repository declares its own piles, and each has a `.pile/config` and an
`intent.md`, and nothing else. Read them with the binary built here:
`pile list`, `pile show`, `pile which`, `pile files`, `pile status` and
`pile log`, and check them with `pile doctor`. `pile guide reading` says
which to reach for. Declare a new pile by
running `pile init` in its directory, and name it with `pile set name`.

- Each crate is a folder under `crates/`. So far, each crate is also a pile.
- An intent takes the shape the intents here already take: it starts with
  its headline, what the pile is, then says what belongs in it and what
  does not, and ends with the closing pair "Read this pile to find out…
  Change it when…".
- Read a pile's intent before you change it, and keep the intent true in the
  same commit.
- The skill that teaches piles to an agent with no `pile` binary lives in
  the plugin repository,
  [multipile/multipile-agent-plugins](https://github.com/multipile/multipile-agent-plugins),
  in `plugins/gitrepo/skills/multipile`, and not
  here. It names no `pile` command, and nothing checks it against the
  binary. Its `reference/` pages describe the same format the binary reads.
  So a change to what a config, an intent or a declaration file is, or to a
  rule doctor holds piles to, changes the skill too, in a commit there that
  goes with the one here.

### Tests

Every command has at least one test in `crates/pile/tests/suite` that drives
the built binary against a real git repository. A command's tests come in
the same commit as the command.

A change to what a user of `pile` sees also comes with a new or changed
journey in `journeys/stories/`, in the same commit. A journey is a whole
session told as a bash script, and the check runs every one. Run them with
`mise x -- journeys/run`, watch one with `SHOW=1`, and record one with
`mise x -- journeys/record <journey>`. `journeys/README.md` says how to
write one.

The guide's pages describe the commands. A change to a command is done
only once every page that names it has been read again, and changed where
it is no longer true. The tests check that each command a page names
exists. They cannot check what a page says of it.
