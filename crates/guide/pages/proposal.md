+++
role = reference
summary = what a proposed layout lists, and how the person decides each item
see = adopt refine boundaries relies-on reshaping aux types links
+++
A layout is proposed before any of it is made. The person who owns the
repository accepts or rejects each item, and nothing they did not accept
is made. With nobody to answer, stop, and commit nothing.

## One layout, or two

Some moves of files change a boundary: a pile that only exists after the
move, or a link that only the move makes name a whole pile
(`pile guide boundaries`, "A container's own files").

When no move changes a boundary, propose one layout. Otherwise propose
two, side by side: **as it stands**, the piles the files hold today, and
**after the moves**, the piles once the moves are made, with each move and
its cost. The person picks one. A move that changes no boundary is listed
under either, for the person to accept or reject.

## A project with no pile yet

Propose one numbered list, in this order:

1. each move picked, with its two trees, the references it rewrites and
   its cost;
2. the top pile, with the sentence it would claim;
3. each pile: its directory, its name, and the sentence it would claim;
4. an aux pile that holds the edge type `relies-on`, in a directory that
   does not exist yet: `:/piles` named `piles`, unless that is taken. The
   person may change the directory and the name;
5. each link, as `@source relies on @target: <title>`, with its
   description;
6. each question about code the test raised, such as coupling between
   piles meant to be independent (`pile guide relies-on`).

If the person rejects the aux pile, no link is written: hand the drafted
titles and descriptions back as text.

## A repository that declares piles

Propose one numbered list. Each item names its operation (split, merge,
move a pile, move files, rename, unmake or declare), the piles and paths,
the sign and the evidence that call for it, and what it costs: ids minted
or retired, files moved, references rewritten. A change to code that the
test raised is a question for the person, not an item.

## When an item is rejected

If the person rejects an item that other items assume, draft those items
again against what they accepted, and propose them again.

## The edge type the links are of

Every link is of an edge type an aux pile creates (`pile guide types`).
A repository that has none yet gets this one, in the `.pile/types.toml`
of its aux pile (`pile guide aux`):

```toml
version = "gitpile/1"

[[edge-types]]
name = "relies-on"
title = "one pile relies on what another is for"
source = ["pile"]
target = ["pile"]
do = "say what the source relies on, in words of the target's intent"
dont = "name a file or a path"
```

The aux pile's intent says what words it gives the repository: how its
piles say what they rely on of each other.
