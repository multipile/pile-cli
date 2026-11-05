+++
role = reference
summary = which command reads what, and when to reach for each
see = paths links doctor start-a-change read-a-change
+++
These commands read the repository by pile. None of them writes, and none
judges: `pile doctor` is the one with a verdict. Each command's help says
what it prints. This page says when to reach for which.

```sh
pile list                   # every pile: its name, its path, its id, its links
pile show <pile>            # one pile whole: config, children, links, intent
pile which <path>           # the pile a path is in, its headline and links
pile files <pile>           # one pile's own files, and how long each is
pile status                 # what changed since the last commit, by pile
pile status --all --brief   # a line per pile, for the whole repository
pile log <pile>             # the commits that changed one pile
pile doctor                 # what is wrong with any pile
```

## Start wide, then narrow

`pile list` is the cheapest way to see what the repository declares. Then
`pile which` finds the pile a file is in. Piles nest, so the path alone
does not show which pile that is, and `which` answers for a file that does
not exist yet as readily as for one that does. Then `pile show` prints
that pile whole, its intent with it. Read it before you change the pile.

## Follow a pile's links

A link says what a pile is to another node, and a change at one end of it
may be owed at the other. `pile show` prints each link the pile is at one
end of, with the node at the other end. When that node is a pile, read it
too with `pile show`, before you change what the link says one end relies
on (`pile guide links`).

## See what reading costs

`pile files` says what reading a pile will cost, in lines and bytes,
before you read it. It lists the pile's own files, without its children's,
which `git ls-files` of the directory cannot give.

## `--all` is what widens `status`

`pile status` reports the pile you are in and the piles under it.
`--all` reports every pile, from anywhere. `--brief` prints a line per
pile, and does not widen what is reported. So inside a child pile,
`pile status --brief` says nothing of the rest of the repository.

A change that crossed a boundary shows in both piles, so read status with
`--all` even when you changed one pile. `git diff` is still how you read
the lines that changed.

## `log` is a pile's own history

`git log` of a directory is the history of every pile under it too.
`pile log` is the pile's own: the commits that changed its own files, and
what else each one changed. Its last lines count the piles it changed
together with, and who made the commits. Who made them is who to ask about
the pile.

## When a command refuses

Every refusal says what fixes it, in `next` or on the lines under it. A
config that cannot be read makes every command refuse but `pile doctor`.
So when a command refuses over the repository, run `pile doctor` to find
out why.
