+++
role = workflow
summary = review a pull request by pile, against its task and each intent
needs = reading
see = read-a-change links boundaries
+++
## When to use

A branch is to be merged into another, through a pull request or not yet,
and you are to review it. A list of paths says which files changed. This
review says which piles changed, what each is for, and whether each
belongs to the task the branch was made for. It fixes nothing.

## What to have first

- The branch, checked out, with the history it shares with its base. A
  checkout of one commit, as CI often makes, has no merge base.
- The task the branch was made for: the pull request's title and
  description, or the issue it closes. Without one, ask for it.

## Steps

### 1. Find the base

```sh
gh pr view --json title,body,baseRefName
git fetch origin <base>
```

Look for the branch the change is to merge into, and the task. In GitHub
Actions, `$GITHUB_BASE_REF` names the base too. With no pull request, the
base is the branch the work goes into, such as `main`.

### 2. See which piles the branch changed

```sh
pile gitdiff origin/<base>...HEAD
```

Look for a block per changed pile, with the headline of its intent and
each path with its letter. Three dots compare from where HEAD left the
base, so what reached the base since is not part of the change. A path
that moved from one pile to another shows in both. A path under
`in no pile` is in no pile.

### 3. Hold each pile to the task and to its intent

```sh
pile show <pile>
```

Look for the sentence of the intent each part of the change is held to,
and quote it. Then give each changed pile one verdict:

- **In the task**: the task needs this pile to change.
- **Not in the task**: nothing in the task needs it. It is a question for
  the author, not a verdict that the change is wrong.
- **Intent untrue**: the change made a sentence untrue, and did not
  rewrite it.

Under `links`, read the pile at the other end of each link whose title
the change touches.

### 4. Read the lines

```sh
git diff origin/<base>...HEAD -- <path>
```

Look for what each change does, a pile at a time, now that you know what
the pile is for and whether the task asked for it.

### 5. Ask doctor

```sh
pile doctor
```

Look for its exit code, and for findings in the piles the change touched.

### 6. Print the view by pile

```sh
pile gitdiff origin/<base>...HEAD --format markdown
```

Look for a table of the changed piles, then each one's paths, folded. It
opens the review.

## Done when

- Every changed pile was read whole, and has one verdict.
- Each part of the change is held to a quoted sentence of its intent.
- Every path in no pile, and every path that crossed a boundary, is named.

## What to hand back

The table by pile, as it was printed. Then a line per changed pile: its
verdict, and the sentence of its intent the change is held to, quoted,
with the piles not in the task and the intents made untrue first. Then
each link the change touched, the paths in no pile, the paths that crossed
a boundary, and what doctor reported. Change nothing, and post the review
only where you were asked to.
