+++
role = reference
summary = what a pile is: its .pile directory, config, id and name, and nesting
see = paths intent boundaries declarations links aux
+++
A pile is a subtree of a git repository that has a `.pile/` directory, and
says why it is one thing. Piles are the parts a repository is talked about
in. Whoever arrives next, a person or an agent, reads a pile before they
change it.

## A `.pile/` directory declares a pile

A pile's `.pile/` directory holds two files:

- `config`, the pile's config. Its being there declares the pile, and
  nothing else lists the piles.
- `intent.md`, the pile's intent: why the subtree is a pile
  (`pile guide intent`).

`pile init` writes both, in the directory you are in. A pile may add
`.toml` files, which declare nodes, types, links and the pile's own values,
such as its labels (`pile guide declarations`). Any other file in a `.pile/` is
an unread file, such as `notes.md`. `pile` does not read it, and refuses
nothing for it, because another program may keep a file there.

## The config says which pile it is

```
kind = gitpile
version = 1
id = 9f8e7d6c5b4a39281706f5e4d3c2b1a0
name = lib
```

A config is one `key = value` a line, and a line starting with `#` is a
comment. A `#` after a value is part of the value.

- `kind` and `version` say which format the pile is written in.
  `pile init` writes them.
- `id` is 32 hex characters that `pile init` mints once. It says a pile is
  the same pile after a move. Nobody types it, and no command takes one.
  `pile id` prints a fresh one, for a config written by hand.
- `name` is the word a pile is typed and shown by, chosen by a person:
  `lib`, or `@lib`. A pile without one is typed and shown by its path, and
  `pile doctor` warns of it. A link refers to a pile by its name alone, so
  it cannot refer to one without. `pile set --help` says what a name may be.

A config holds these four keys and no other. A line with any other key is
refused, and `pile doctor` names the line. Anything else a pile says about
itself, such as its labels, is given in `[pile]`
(`pile guide declarations`).

A config that says `kind = auxiliary-gitpile` declares an aux pile, the
one kind of pile that may create types (`pile guide aux`).

`pile set` changes a pile's name, and no other key. A link that refers to
the old name then names nothing, and `pile doctor` names each such link.
Commit a new or changed config: git is what carries it to a clone.

## A path is in the nearest pile at or above it

Piles nest. A path is in the nearest pile at or above it, and in no other.
A path with no pile at or above it is in no pile.

- A pile's parent is the nearest pile above it. Its children are the piles
  whose parent it is, and it holds them.
- A pile's own files are its directory, less every pile it holds.
  `pile files` lists them.
- Declaring a pile inside another moves no file. The paths under the new
  pile are in it from then on, and no longer in its parent.

A pile is mounted in its parent, at its path relative to the parent:
`:/services/billing/invoices` is mounted in the pile at `:/services/billing`
at `invoices`. So the piles form a mount tree, whose top is the root pile.
A pile with no pile above it is a top of its own. Nothing declares a
mount: the tree is what the directories already say. `pile list --tree`
prints it, and `pile show` names the pile one is mounted in.

## Git keeps everything else

`pile` makes no commit. Git keeps the versions, the content and the
history, and `pile` arranges git's answers by pile. A pile inside another
is in the same repository, with the same commits and branches. So a
boundary between piles is in what `pile` reports, never in storage.

The only files `pile` writes are the `.pile/` files that `pile init` and
`pile set` write.
