+++
role = reference
summary = what doctor's findings and severities mean, and when it must exit 0
see = commit-a-change answers intent piles declarations diagrams
+++
`pile doctor` says what every pile is, and what is wrong with any of them.
It is the one command with a verdict: it exits 1 when it finds a problem.
It must exit 0 before a commit.

## It reads the whole repository, and what others refuse

doctor reads the whole working copy, from anywhere in it, and takes no
pile. So a problem in a pile you did not touch is still reported. Raise it
with whoever owns that pile, and do not commit past it.

doctor is the one command that does not refuse over a config it cannot
read, because reporting a broken pile is what it is for. When another
command refuses over the repository, run `pile doctor` to find out why.

doctor writes nothing. It reads what piles are (their configs, their
intents and where they sit), the `.toml` files in each `.pile/`, whose
every table it checks against the format (`pile guide declarations`), and
the diagrams in each `.pile/diagrams/` (`pile guide diagrams`). It never
reads what a pile's other files say. `pile doctor --diagrams` checks the
diagrams and nothing else.

## A finding says what is wrong, and what fixes it

Each finding has:

- a code, a stable word such as `intent-unanswered`;
- the file, as a `:/` path, and the line and column where it has them.
  Text prints them as an editor opens them, `:/lib/.pile/intent.md:3:1`,
  and json gives `line` and `column`, each `null` where there is none;
- a sentence saying what is wrong;
- a sentence saying what fixes it.

`pile doctor --help` lists every code, a line each. In json, each pile's
row holds its findings under `problems`, `warnings` and `notes`.

## Three severities

- A **problem** stops the structure working: a config that cannot be
  read, two piles with one id or one name, a pile whose intent says
  nothing, or a declaration that is not read as it was written. Any
  problem makes doctor exit 1, and its answer refuses as
  `problems-found`, with the whole report still its result.
- A **warning** is something to put right that breaks nothing yet, such
  as a pile with no name. doctor still exits 0.
- A **note** is something to know, and breaks nothing, such as a pile
  whose config is not committed yet. doctor still exits 0.

## Work through a report

Fix the problems first, in the order the report gives them, then run
`pile doctor` again. A fix can bring a finding to light that the problem
hid: an intent that was not text could not be read for a headline.

Then read the warnings. Put each right, or say why not to whoever asked.
A note asks for nothing unless it surprises you: a `not-committed` note
for a pile you meant to commit is the reminder to commit it.

An intent that doctor passes is not an intent somebody approved. doctor
asks only whether an intent says something, and never what it says
(`pile guide intent`).
