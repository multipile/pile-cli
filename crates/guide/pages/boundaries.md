+++
role = reference
summary = whether a subtree is one pile, and when a boundary stops being true
see = leaves relies-on boundary-note reshaping refine adopt piles intent links
+++
A pile is worth declaring when its intent serves whoever arrives at it. So
whether a subtree is a pile is decided by the sentence that says what it is
for, not by its size, and not by how its directories are laid out.
`pile guide leaves` says which directories are piles, and
`pile guide relies-on` tests a boundary by what other piles rely on.

## A pile's shape outlasts its files

A pile's files change often. Its boundary, its intent and its links to
other piles should change rarely. An intent leaves out paths because they
go stale, and a link's title that names a file goes stale the same way. So
a stable layout has few dependencies between piles, and each is to a whole
pile, or to a part its intent claims, as a link's title says it
(`pile guide links`).

Stability is a goal for the layout. It is not a reason to keep a boundary
that has stopped being true. Changing a boundary to make the layout more
stable is what `pile guide refine` is for, and `pile guide reshaping`
says how.

## The test is one sentence

If one sentence cannot say what the subtree is for without turning into a
list, it is two piles, or one pile with its boundary in the wrong place. A
headline that needs "and" twice is the sign.

The test only splits. It never keeps two parts together. A sentence broad
enough to cover two kinds of thing, such as "the behavioral contract" over
a set of cases and the schema they follow, is a topic, not a claim
(`pile guide intent`). It is a reason to look closer, not to merge.

Many small piles are better than a few large ones. A small pile with a
sharp intent tells its reader more than a large one whose intent has to
stay vague to cover everything. Don't merge two piles only because each
is small.

## Piles cover the repository

Declare a pile at the top of the repository, beside `.git`, so that every
file git tracks is in a pile. The top pile holds what is about the whole
repository, and is where a container's own files land. Read the top
pile's own files with `pile files :/`. If any of them would have a
sentence of its own, it is a leaf nobody declared yet.

## A container's own files

A container is a directory that only groups others, and is not a pile
(`pile guide leaves`). Before one of its own files goes to the top pile,
hold it to the one-sentence test:

- Prose another pile relies on belongs in the pile whose intent claims
  it. Suggest moving it there, and leaving a short description of the
  folder behind.
- A build file in a container, shared by piles with separate lifecycles,
  is copied into each of them, so the container is not a pile. Shared by
  piles with one lifecycle, it is kept once, in a pile of its own or in
  the pile that holds them both.
- A build file at the top of the repository, such as a workspace lockfile
  or a base `tsconfig`, is about the whole repository, and stays in the
  top pile.

Suggest a move only for a file in a container or in the top pile that
another pile relies on, or a file whose content belongs to another pile's
intent. Show the layout before and after as two short trees, the link's
title before and after, every reference the move rewrites
(`git grep -n -F '<old path>'`), and for a copy, what it costs: two
lockfiles, two installs, and versions that can drift. A move is a change
of its own. Declaring a pile moves no file.

## Signs that a boundary has stopped being true

- The intent needs "and" twice to cover what is under it now.
- Two piles always change together in `pile log`, and never apart. They
  may be one thing. This is a sign and not a verdict: two leaves with two
  true sentences stay two piles.
- Nobody arrives with the question the closing pair names.
- A link's title has to name a part of the pile at its other end
  (`pile guide relies-on`).
- Commits that change one pile keep having to rewrite another pile's
  intent or link titles. A stable layout has few.

## Report a boundary, don't move it

During an ordinary change, a boundary you find untrue is reported in a
boundary note in what you hand back, and never moved
(`pile guide boundary-note`). Moving it is for `pile guide refine`, with
the person's agreement.
