+++
role = reference
summary = the three forms a pile or a path is typed in, and how pile prints one
see = piles reading
+++
A command that takes a pile or a path reads each word as exactly one of
three forms. It never reads a word as another form because of what the
word happens to match.

```sh
pile show @lib              # a name, and nothing else
pile show lib               # a name, when a pile claims lib; else a path
pile show ./lib             # a path, relative to the directory you are in
pile show :/packages/lib    # a :/ path, from the top of the repository
```

## The three forms

- **A name** is `@lib`, or a bare word such as `lib` that a pile claims as
  its name. A bare word no pile claims is a path. With its `@`, a word is a
  name and nothing else: a name no pile claims is refused
  (`unknown-name`), and the message lists the names that are claimed.
- **A path** is a path as the shell means it: relative to the directory
  you are in, or absolute. It need not exist. The part of it that is on
  disk is resolved there, symbolic links followed, and the rest goes after
  it. A path that ends up outside the repository is refused
  (`path-outside-repository`).
- **A `:/` path** is a path from the top of the repository, as git writes
  one: `:/packages/lib`, and `:/` for the top. It means the same from
  anywhere in the repository. It is read without the disk, and one that
  climbs above the top with `..` is refused.

## `/packages/lib` is the disk's, never the repository's

A path that starts with `/` is absolute on disk. So `/packages/lib` is the
disk's `/packages/lib`, even when a pile sits at `:/packages/lib`, and no
refusal offers that pile instead. From the top of the repository, write
`:/packages/lib`.

## With no pile given, a command is about the pile you are in

The pile you are in is the nearest pile at or above the directory `pile`
runs in. `pile init` takes no path at all. It declares the directory you
are in, so `cd` to the directory first.

## Every path `pile` prints is a `:/` path

Every path in every answer, as text and as json, is a `:/` path. So
whatever `pile` printed can be pasted back, and means the same. Where a
pile has a name, `pile` prints it first, with its `@`, and suggests
commands with it.

A file that does not exist yet is asked about as readily as one that does:
`pile which :/src/new.rs` says which pile it would be in.
