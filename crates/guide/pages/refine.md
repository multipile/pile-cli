+++
role = workflow
summary = improve the piles a repository declares, so its layout is stabler
needs = boundaries proposal
see = relies-on leaves reshaping split-and-merge write-an-intent adopt doctor
+++
## When to use

The repository declares piles, and somebody asked for its layout to be
refined: one pile they name, or the whole repository. This is the only
workflow that moves a declared boundary, and, with `pile guide adopt`,
the only one that moves a file. During an ordinary change you report a
boundary that looks untrue, and do not move it.

Do not use it where no pile is declared. `pile guide adopt` is for those.

## What to have first

- The person who owns the repository, to accept or reject each item and to
  approve each intent. You stop to wait for them.
- The repository's own rules for a commit, in its AGENTS.md or wherever it
  keeps them.

## Steps

### 1. Scope the refine

```sh
pile status --all --brief
pile list --tree
pile show <pile>
```

Look for status to show nothing changed already. For the whole
repository, the scope is every pile the list names. For one pile, it is
that pile, its parent, its children, and the pile at the other end of
each link `pile show` prints.

### 2. Read the evidence

```sh
pile files <pile>
pile log -n 50 <pile>
git grep -n -F '<dir>/' -- ':!<dir>'
```

Look for what counts as evidence of a boundary: build files, kinds of
thing, names people call a subject by, what other piles read, and what
the history changes together and apart (`pile guide leaves`). The last
lines of `pile log` count what changed together. With no history to
read, say so.

### 3. Test the layout, propose, and wait

```sh
pile files :/
pile list
```

Follow `pile guide relies-on` in its order, for one pass, starting from
the piles the repository declares. Read each link's title, and draft the
title of each link that is missing. Check the signs that a boundary has
stopped being true. Hold every pile's own files, not only the top pile's,
to the one-sentence test, to find a leaf nobody declared.

Look in the list for an aux pile, and in `pile show` of it for the edge
type the links are of. If the repository has none, one that holds
`relies-on` is an item to propose. Then propose the items as
`pile guide proposal` says. Stop, and wait until the person agrees.

### 4. Make each accepted item

```sh
pile doctor
git commit
```

Make each item as `pile guide reshaping` and `pile guide split-and-merge`
say, one commit each. Each intent it needs goes through
`pile guide write-an-intent` first. Look for doctor to exit 0 before each
commit.

## Done when

- Only the items the person accepted were made, each in a commit of its
  own.
- `pile doctor` exits 0.
- Every intent written or rewritten was approved, and replaced what became
  untrue instead of adding beside it.
- No link's title names a file or a path.

## What to hand back

Each item, accepted or not, with its commit's id. Each question about code
the test raised, and each question the person left open.
