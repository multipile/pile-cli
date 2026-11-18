+++
role = reference
summary = split a pile, merge two, and move files from one pile to another
see = reshaping boundaries links write-an-intent refine
+++
Each of these is made of the operations of `pile guide reshaping`:
declare, move and unmake. Each is proposed as an item first, and made in
a commit of its own, with `pile doctor` exiting 0 before it.

## Split a pile

1. A part that is a set of files, not one directory, is first moved into
   a directory of its own, as an item of its own.
2. Find the references to the parent.
3. Declare the child, with `pile init` in its directory.
4. Move into the child's `.pile/` each `[[edges]]` and `[[nodes]]` table
   about what the child is for. Before a link moves, write out the end it
   left out (`pile guide links`).
5. Read each `"@parent"` reference's title, or its description and type
   when it has no title. Where it is about what the child is for, point it
   at `"@child"` instead.
6. Rewrite the parent's intent: replace what became untrue, and add
   nothing beside it.
7. Draft the child's intent (`pile guide write-an-intent`).

Both intents are approved before the commit.

## Merge two piles

The person picks the id and name that survive.

1. Find the references to the other pile.
2. When the two are siblings, first `git mv` the other's files into the
   survivor, as a move of files.
3. Move every `[[nodes]]` and `[[edges]]` table in the other's `.pile/`
   into the survivor's. Here an end left out should come to mean the
   survivor, so it stays left out.
4. Merge its `[pile]` values into the survivor's, with each key written
   once.
5. Unmake the other pile, and point every reference to it at the
   survivor.
6. Remove any link from the survivor to itself, and any edge now written
   twice (`pile guide links`).
7. Rewrite the survivor's intent.

## Move files between piles

What to propose is in "A container's own files" of
`pile guide boundaries`. To make the move:

```sh
git mv <old path> <new path>
git grep -n -F '<old path>'
pile status --all --brief
```

Rewrite every reference the proposal listed, then search again: the
search must find nothing. Status shows both piles changed, because a
path that crossed a boundary counts in both. Run `pile doctor`, and
commit both halves together.
