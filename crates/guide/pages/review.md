+++
role = workflow
summary = review a pull request by pile, against its task and each intent
needs = reading quality
see = read-a-change links boundaries intent
+++
## When to use

A branch is to be merged into another, through a pull request or not yet,
and you are to review it. A list of paths says which files changed. This
review says, for each pile the branch changed, whether the task needed it
and whether its intent admits it, and reports only what the author has to
act on. It fixes nothing.

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

Look for a block per changed pile, with its intent's headline and each
path with its letter. `intent rewritten` and `declarations rewritten` mark
a pile whose intent or links the branch wrote. Three dots compare from
where HEAD left the base. A path that moved between piles shows in both,
and one under `in no pile` is in no pile.

### 3. Read each changed pile, and answer for it

```sh
pile show <pile>
git diff origin/<base>...HEAD -- <path>
```

Look for the sentence of the intent each part of the change is held to,
and quote it. Then give the pile its two answers. Read the lines a pile at
a time, now that you know what the pile is for.

### 4. Compare what the branch rewrote

```sh
pile show <pile> --at origin/<base>
```

For a pile marked `intent rewritten`, look for whether the new intent is
worse than this one (`pile guide quality`). For one marked `declarations
rewritten`, look for each link the branch added, removed or retitled.

### 5. Hold the links

```sh
pile show <aux pile>
```

Look under each changed pile's `links` for each link to or from it, and
hold it to what the change did. `next` names the aux pile that creates
each type: show it for the type's `do` and `dont`. Where the change makes
one pile use another, look for a link that says so.

### 6. Ask doctor

```sh
pile doctor
```

Look for its exit code, and for findings in the piles the change touched.

## Done when

- Every changed pile was read whole, and has its two answers.
- Each part of the change is held to a quoted sentence of its intent.
- Every rewritten intent was compared with its base, and every link to or
  from a changed pile was held to its type.

## What to hand back

Findings, and nothing else, each blocking or a question (`pile guide
quality`). Open with one line that counts them: `Pile review: 1 blocking,
1 question, 3 piles fine`. Then an entry per finding, blocking first: its
level, the pile, its two answers, why in one sentence, the quoted
sentence, title or headlines it is judged by, and one action that fixes
it. Then one line naming each pile in the task that fits its intent. Then,
folded, each pile's answers with the sentence that admits each part.

Leave out the table `pile gitdiff` prints, which is posted on its own, and
doctor unless it found something. With no finding, the first line is the
whole review. Change nothing, and post only where you were asked to.
