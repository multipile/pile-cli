+++
role = workflow
summary = bring piles to a project that declares none yet
needs = boundaries proposal
see = leaves relies-on intent links aux refine commit-a-change agent-mode
+++
## When to use

A project in a git repository declares no pile, and somebody asked for
it to be laid out in piles. You find the boundaries it has, and the ones
a few moves of files would give it. You change no file except by a move
the person picked. Where piles are declared, use `pile guide refine`.

## What to have first

- The person who owns the project: you stop twice to wait for them.
- The repository's own rules for a commit, such as its AGENTS.md.

## Steps

### 1. Make sure no pile is declared yet

```sh
pile version
pile list
```

Look for `no pile in`. If it names piles, stop: they are declared.

### 2. Find the boundaries, and draft the links

```sh
git ls-files
git log --stat -n 50
git grep -n -F '<dir>/' -- ':!<dir>'
```

Look for the leaves (`pile guide leaves`): a build file or a task of its
own, a kind of thing, and a subject people call by a name. Hold each
candidate to the one-sentence test. With no history to read, say so. Then
run the search for each candidate, and follow `pile guide relies-on` in
its order, for one pass. Draft a title and a description for each pair
where one pile relies on another, and none that names a file or a path.

### 3. Propose the layout, and wait

```sh
git ls-files -- ':(exclude)<dir>'
```

Exclude every candidate. Look in what is left for each file another pile
relies on, and each build file candidates share: each is a move to show.
Propose as `pile guide proposal` says. Stop, and wait until they agree.

### 4. Make each accepted move

```sh
git mv <old path> <new path>
git grep -n -F '<old path>'
```

Look for the search to find nothing once each reference is rewritten.
Run the project's build and tests, and commit each move on its own,
before any pile is declared. With no move accepted, skip this step.

### 5. Declare each agreed pile, the top first, and draft its intent

```sh
pile init --set name=<name>
cd <dir> && pile init --set name=<name>
cd <dir> && pile init --set name=<name> --set kind=auxiliary-gitpile
pile doctor
```

The third declares the aux pile, whose `.pile/types.toml` you then write.
Look for `intent-unanswered` against each pile you declared. Write a
draft in each (`pile guide intent`), and name in it each part another
pile relies on. Hand the drafts back, saying which sentences you are sure
of and which you guessed. Stop, and wait until the person approves them.

### 6. Write the links and the section, and make one commit

```sh
pile guide --agents
pile doctor
pile status --all --brief
```

Write each accepted link at its source (`pile guide links`), and the
section near the start of each rules file (`pile guide agent-mode`). Look
for doctor exiting 0, and for status to show only the `.pile/`
directories and the rules files as changed. Commit them in one commit.

## Done when

- `pile doctor` exits 0, and every pile, link and intent is one the
  person agreed to.
- No link's title or description names a file or a path.
- No file was changed, except by an accepted move, in its own commit.
- One commit holds every `.pile/` directory and the rules files.

## What to hand back

The piles and their headlines, the links, each move with its commit's
id, each question about code, each sentence you guessed, the last commit.
