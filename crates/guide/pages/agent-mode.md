+++
role = reference
summary = how pile knows an agent runs it, and the rules file that sends it
see = answers start
+++
Agent mode is `pile` knowing that its reader is an agent. It changes how
`pile` answers, and never what a command does.

## It is on when `AI_AGENT` or `AGENT` is set

Agent mode is on when `AI_AGENT` or `AGENT` is set to anything but the
empty string. `pile` does not read the value. A harness sets the variable
once, before the agent types anything, and every `pile` the session runs
inherits it. No flag turns agent mode on or off.

`AGENT` is a general word, and a program unrelated to agents may set it.
That puts a person's shell in agent mode without anybody choosing it. So
`pile version` says whether agent mode is on, and which variable turned it
on. To run one command as a person, set both variables empty:

```sh
AI_AGENT= AGENT= pile status
```

## What it changes

- **The format is json when none is asked for.** `--format text` still
  prints what a person reads.
- **A refusal goes to stdout, in text too.** It reads as it would on
  stderr, and the exit code is the same.
- **A bare `pile` says where the agent is.** Inside a repository that
  declares piles, it answers at once what `pile version`, `pile list` and
  `pile status --all --brief` would, and its `next` starts with
  `pile guide`. Anywhere else, there is nothing to say where it is in, so
  it prints the help.

## What it does not change

- What a command reads and writes, and the exit code it gives.
- The answers that are text in every format, such as `--help`
  (`pile guide answers`).

`pile` writes nothing into a harness: no skill, no hook, and nothing in
its configuration. What a repository keeps instead is the section
`pile guide --agents` prints, near the start of its rules file.

## A rules file is what a repository's agents read first

A rules file is a file in a repository that an agent reads at the start of
every session. Which file depends on the agent:

- `AGENTS.md`, which most agents read, Codex among them;
- `CLAUDE.md` for Claude Code, and `GEMINI.md` for Gemini CLI;
- `.github/copilot-instructions.md` for GitHub Copilot;
- a rule in `.cursor/rules/` for Cursor, with `alwaysApply: true`.

The section goes in each rules file the repository keeps, near its start:
at the top where the person agrees, and always in its first 100 lines, so
that an agent reads it before much else. A file that only points to
another, as a `CLAUDE.md` that holds `@AGENTS.md`, gets nothing: the file
it points to gets the section. A repository with no rules file gets the
one the agent you are reads.

The section's heading carries the date its text last changed:
`## Piles (2026-11-07)`. A rules file whose `## Piles` section has an older
date, or none, gets the section `pile guide --agents` prints in its place.
