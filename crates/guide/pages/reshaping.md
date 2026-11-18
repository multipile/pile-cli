+++
role = reference
summary = declare, move, unmake and rename a pile, one change at a time
see = split-and-merge boundaries links refine write-an-intent intent doctor aux
+++
`pile guide boundaries` says where a boundary goes. This page says how to
move one: declare, move, unmake and rename a pile. Splitting a pile,
merging two, and moving files from one pile to another are made of those
(`pile guide split-and-merge`).

## One change at a time

Every operation is proposed as a numbered item, and the person who owns
the repository accepts or rejects each one. An intent the operation needs
goes through `pile guide write-an-intent` and is approved before its
commit, because an intent that is still a question is a problem to
`pile doctor`. Make the accepted items one per commit, with `pile doctor`
exiting 0 before each. Then `git revert` undoes the last one cleanly, and
an earlier one if nothing later builds on it. With nobody to answer, make
nothing.

## Find the references to a pile

```sh
pile show <pile>
git grep -n -F '"<type>:<node>"' -- ':(top,glob)**/.pile/*.toml'
git grep -n -w -F '@<name>'
```

`pile show` prints every link the pile is at one end of, wherever it is
written. The first `git grep` finds the links that name a node declared
in the pile's `.pile/`, and the second the prose that names the pile,
such as another pile's intent. Run them before you rename a pile, or
remove a pile or a node. Afterwards `pile doctor` reports each link left
naming nothing as `no-such-name`, but never the prose.

## Declare a pile

Propose a boundary to whoever owns the repository before you declare it.

```sh
cd packages/lib && pile init --set name=lib
```

`pile init` declares the directory it runs in, and moves no file. Inside
another pile, the new pile is that pile's child, and its paths leave the
parent. Its intent is a question, so that nobody mistakes it for an
answer, and `pile guide intent` says how it is answered. Commit the
config and the intent together.

## Move a pile

Move a pile with `git mv` of its directory. Its id and name go with it,
so every reference to it still holds. Never copy a `.pile/` to make a new
pile: two piles with one id are both broken, and `pile doctor` reports
`id-claimed-twice`. Give the copy a config of its own, with the id
`pile id` prints.

## Unmake a pile

A pile is unmade by hand, and never on your call alone. The top pile is
never moved or unmade.

1. Find the references to the pile and its nodes.
2. Read what its `.pile/` holds. Another program may keep a file there.
   Every table there goes with the pile. In an aux pile its types go too,
   so every node and link of those types stops being read
   (`pile guide aux`).
3. For each table, the person chooses: remove it, or move it to another
   pile, usually the parent. Before a link moves, write out the end it
   left out (`pile guide links`).
4. For each link elsewhere that names the pile, the person chooses:
   remove it, or point it at another pile.
5. Remove the directory, with `git rm -r packages/lib/.pile`, or `rm -r`
   for one nobody committed.

The pile's paths are its parent's again, or in no pile. Its id is gone
for good: declared again, the directory is a new pile with a new id.

## Rename a pile

Find the references to the old name first, in links and in prose. Then
rename it, and rewrite every `"@old"` and every `@old` in the same
commit:

```sh
pile set name <new> <pile>
pile doctor
```

Look for no `no-such-name` finding.
