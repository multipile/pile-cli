+++
role = reference
summary = how pile answers: formats, the envelope, exit codes and refusals
see = agent-mode doctor
+++
Every command answers in one of two formats, chosen with `--format`:
`text`, which a person reads, or `json`. An agent gets `json` unless it
asks for text (`pile guide agent-mode`). The format changes how an answer
is printed, never what a command does or what it exits with.

`pile gitdiff` also answers in a third format, `markdown`, for a person who
reads the answer where Markdown is rendered, such as a comment on a pull
request. Every other command refuses `--format markdown` with
`format-not-answered`, before it does anything. A refusal in `markdown` is
printed as it is in text.

## A json answer is one envelope

A command prints one object on stdout, the envelope, and nothing else:

```json
{
  "ok": false,
  "command": "pile show @nope",
  "error": {"code": "unknown-name", "message": "…", "usage": false},
  "next": [{"command": "pile list", "why": "lists the piles there are"}]
}
```

- `ok` is `true` exactly when the exit code is 0.
- `command` is the command as it was run, as you would type it back.
- `result` is the answer, when `ok` is `true`. Its shape is the command's
  own.
- `error` is the refusal, in place of `result`, when `ok` is `false`: its
  `code`, its `message`, and `usage`, which is `true` when the exit code
  is 2.
- `next` is the commands worth running after this one, each with why. For
  a refusal, they are the commands that fix it.

Branch on `ok` and `error.code`, never on the wording of `message`. A
refusal code is stable, and a message may be reworded in any release.

A verdict that fails is still an answer. When `pile doctor` finds a
problem, `ok` is `false`, `error.code` is `problems-found`, and `result`
is the whole report.

## In text, a refusal goes to stderr

In text, a refusal is printed to stderr after `pile:`, with each command
that fixes it on a line of its own under it. An agent's refusal goes to
stdout instead, and reads the same.

## Exit codes

- `0`: the command did what was asked.
- `1`: it failed. Look at the repository.
- `2`: what was typed was wrong. Fix the command line.

They are the same in every format, and in agent mode.

## Some answers are text in every format

An envelope would stand between these and their reader, so they are
printed as text whatever the format: each `--help`, and a value meant for
another program or a file, such as `pile version --short`, `pile id`,
`pile completion bash` and `pile guide --agents`. A refusal from any of
them is still an envelope.

## Where each refusal code is listed

Each command's help ends with the refusals it can give, under
`Refusals:`, with the exit code and what fixes each. Any command can also
give these, before it does anything of its own:

```text
unknown-command      2  pile has no such command: pile --help lists them
unknown-flag         2  the command has no such flag: its --help lists them
unknown-format       2  --format is text, json or markdown
format-not-answered  2  only pile gitdiff answers in markdown: use text or json
usage                2  the command cannot take what was typed: its --help
```

And any command that reads a repository can give these:

```text
not-a-repository     1  not inside a git repository: git init makes one
no-git               1  there is no git on PATH: install it
storage-failed       1  git failed: the message is what it said
unreadable-config    1  a pile's config is wrong: the message names the line
unresolvable-path    1  the directory pile runs in is gone: cd somewhere that is
```
