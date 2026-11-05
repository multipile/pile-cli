+++
role = workflow
summary = commit a change once doctor, status and each intent say it is ready
needs = doctor
see = write-an-intent read-a-change start-a-change links
+++
## When to use

A change is ready to commit, in a repository that declares piles, whoever
made it. `pile` makes no commit. The last step makes it with git, as the
repository's own rules say.

## What to have first

- The change, in the working copy.
- The repository's own rules for a commit, in its AGENTS.md or wherever it
  keeps them.

## Steps

### 1. Ask doctor

```sh
pile doctor
```

Look for its exit code. 1 means a problem: fix each one as its line says,
and run it again until it exits 0 (`pile guide doctor`). A problem in a
pile you did not touch is one to raise, not to commit past.

### 2. See which piles the change touched

```sh
pile status --all --brief
```

Look for the piles you meant to change, and no others. A pile changed that
nobody meant to touch is a boundary somebody crossed. A path that crossed
from one pile to another counts in both: commit both halves together.

### 3. Read each changed pile's intent again

```sh
pile show <pile>
```

Look for a sentence the change made untrue. If there is one, rewrite it in
this commit (`pile guide write-an-intent`), and say so in the message.

Where the change added, changed or removed a link, run `pile show` for the
pile at each end of it. Look for a sentence of either intent that the link
now contradicts, and for a link title the change made untrue.

### 4. Commit with git

```sh
git add <path>
git commit
```

Look for git's own answer. In the message, say which piles the change
touched when it touched more than one, and why it was not split.

## Done when

- `pile doctor` exits 0.
- `pile status --all --brief` showed only the piles meant to change.
- Every changed pile's intent is still true, or rewritten in the commit.
- The intents at both ends of every changed link are still true.
- A path that crossed a boundary is committed whole, both halves together.

## What to hand back

The commit's id, the piles it changed, and every warning or note doctor
gave that is still there, with why it stays.
