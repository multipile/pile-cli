+++
role = workflow
summary = read what a change touched, pile by pile, against each intent
needs = reading
see = review commit-a-change write-an-intent links
+++
## When to use

Somebody made a change, in the working copy or in recent commits, and you
are to read it before it is reviewed, committed or handed on. It says which
piles the change touched, and holds each part to its pile's intent. It
fixes nothing.

## What to have first

- Where the change is: the working copy, a branch and the branch it goes
  into, or the commits, by their ids.

## Steps

### 1. See which piles the change touched

```sh
pile gitdiff <base>...HEAD   # a branch, or a pull request, against its base
pile status --all            # the working copy alone
```

Look for a block per pile, each path with its letter. A path that moved
from one pile to another shows in both. A path under `in no pile` is in no
pile. `pile gitdiff` lists only the changed piles, each with its intent's
headline, and marks a pile whose intent the change rewrote. `pile status`
marks a pile the change did not touch `unchanged`.

### 2. For commits, see which piles each one changed

```sh
pile log -n 10 <pile>
```

Look for the commits of the change, each with the other piles it changed
under `also`. `intent written here` marks a commit that rewrote the
pile's intent. The count at the end says which piles change together.

### 3. Read each changed pile's intent

```sh
pile show <pile>
```

Look for the sentence each part of the change is held to: one that admits
it, or one it breaks. A pile whose intent the change made untrue needs it
rewritten in the same change.

Under `links`, look for each link whose title the change touches, and read
the pile at its other end too. A changed `.toml` file in a `.pile/` may
add, change or remove a link: read the piles at both its ends.

### 4. Read the lines

```sh
git diff
git diff <base>...HEAD
git show <commit>
```

Look for what each change does, now that you know which pile it is in and
what that pile is for.

### 5. Ask doctor

```sh
pile doctor
```

Look for its exit code, and for findings in the piles the change touched.

## Done when

- Every pile the change touched was read whole, with `pile show`, and so
  was the pile at the other end of each link the change touched.
- Each part of the change is held to a sentence of its pile's intent,
  quoted.
- Every path in no pile, and every path that crossed a boundary, is named.

## What to hand back

For each pile the change touched: what changed, the sentence it is held
to, and whether the intent is still true. Then each link the change
touched, and what it owes the pile at the other end. Then the paths in no
pile, the paths that crossed a boundary, and what doctor reported. Change
nothing.
