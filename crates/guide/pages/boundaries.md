+++
role = reference
summary = whether a subtree is one pile, and declaring or unmaking one
see = piles intent adopt reading
+++
A pile is worth declaring when its intent serves whoever arrives at it. So
whether a subtree is a pile is decided by the sentence that says what it is
for, not by its size, and not by how its directories are laid out.

## A pile has a lifecycle of its own

A pile is for something with its own lifecycle: something designed, built,
tested, reviewed, released, run or read on its own. It need not be all of
those. A log of decisions is read and reviewed, and never built. Each of
those is a lifecycle of its own.

Some directories rarely have a reason of their own to give:

- tooling configuration, scratch directories, `scripts/` and `assets/`
  usually serve the pile around them;
- a generated or vendored directory has the reason of whatever produced it
  or pulled it in;
- a directory that only groups other directories says nothing of its own.

Declare one anyway when somebody arrives at it with a question of its own.

## The test is one sentence

If one sentence cannot say what the subtree is for without turning into a
list, it is two piles, or one pile with its boundary in the wrong place. A
headline that needs "and" twice is the sign.

No number of piles is the right one. A repository may hold three piles, or
one for nearly every directory. Either is right when each intent serves the
people who read it.

## Signs that a boundary has stopped being true

- The intent needs "and" twice to cover what is under it now.
- Two piles always change together in `pile log`, and never apart in
  `pile status --all`. They may be one thing.
- Nobody arrives with the question the closing pair names.

## Declare a pile, or unmake one

Propose a boundary to whoever owns the repository before you declare it.

```sh
cd packages/lib && pile init --set name=lib
```

`pile init` declares the directory it runs in, and moves no file. Inside
another pile, the new pile is that pile's child. Its intent is a question,
and `pile guide intent` says how it is answered.

A pile is unmade by hand, and never on your call alone. First look at what
its `.pile/` holds, because another program may keep a file there. Then
remove the directory, with `git rm -r packages/lib/.pile`, or `rm -r` for
one nobody committed. The pile's paths are its parent's again, or in no
pile. Its id goes with it: declared again, the directory is a new pile with
a new id.
