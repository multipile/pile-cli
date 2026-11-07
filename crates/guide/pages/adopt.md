+++
role = workflow
summary = bring piles to a project that declares none yet
needs = boundaries intent
see = commit-a-change start agent-mode
+++
## When to use

A project that already exists, in a git repository, declares no pile, and
somebody has asked for it to be laid out in piles. You find the boundaries
it already has, write them down where `pile` can see them, and hand them
back for a person to correct. You are not designing the project.

Do not use it where piles are declared already. `pile doctor` and
`pile guide boundaries` are for those.

## What to have first

- `pile version` answers. Nothing here works without the binary.
- The person who owns the project, to agree each boundary and each intent.
  You stop twice to wait for them.
- The repository's own rules for a commit, in its AGENTS.md or wherever it
  keeps them.

## Steps

### 1. Make sure no pile is declared yet

```sh
pile version
pile list
```

Look for `no pile in` and the top of the repository. If `pile list` names
piles, stop: the project is laid out already.

### 2. Find the boundaries the project already has, and propose them

```sh
git ls-files
git log --stat -n 50
```

The evidence is in the project. Look for what moves together: a build file
such as `Cargo.toml`, a README of its own, and directories the history
changes together and apart. Hold each candidate to the one-sentence test
(`pile guide boundaries`). Then propose the piles to the person: each
directory, its name, and the sentence it would claim. Stop, and wait until
they agree.

### 3. Declare each agreed pile, the top first

```sh
pile init --set name=<name>
cd <dir> && pile init --set name=<name>
```

Look for the first line naming the directory as `:/<dir>`, and, inside
another pile, the `inside` line naming its parent. Each new intent is a
question.

### 4. Draft each intent, and hand the drafts back

```sh
pile doctor
```

Look for `intent-unanswered` against each pile you declared: each is an
intent to write (`pile guide intent`). Write a draft in each, run doctor
again, and read the headline it prints beside each pile. Hand the drafts
back, saying which sentences you are sure of and which you guessed. Stop,
and wait until the person approves them.

### 5. Send the next session to the guide

```sh
pile guide --agents
```

Write it into each of the project's rules files (`pile guide agent-mode`),
near the start: at the top if the person agrees, in the first 100 lines.

### 6. Ask doctor, and make one commit

```sh
pile doctor
pile status --all --brief
```

Look for doctor exiting 0, and for status to show only the `.pile/`
directories and the rules files as changed. Commit them with git, in one
commit, as the repository's rules say.

## Done when

- `pile doctor` exits 0, and reports no `intent-unanswered`.
- Every pile declared is one the person agreed to, and every intent is one
  they approved.
- Each rules file holds the section in its first 100 lines.
- One commit holds every `.pile/` directory and the rules files.

## What to hand back

The piles, as `pile list` prints them, each with its headline. The
sentences you guessed at, and any boundary the person left undecided. The
commit's id.
