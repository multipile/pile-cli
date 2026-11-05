+++
role = workflow
summary = find the piles a change belongs in, and read each before writing
needs = reading
see = write-an-intent commit-a-change paths links
+++
## When to use

Before you write in a repository that declares piles: a fix, a feature,
or any other file. It says which pile each file is in, and holds the
change to what that pile says it is for, before any of it is written.

## What to have first

- What the change is for, in a sentence.
- The paths you expect to write. Files that do not exist yet count.

## Steps

### 1. Ask which pile each path is in

```sh
pile which <path> <path>
```

Look for a block per pile, each with its headline and the paths given
that are in it. A path under `in no pile` is in no pile: say so to whoever
asked, rather than choose one.

### 2. Read each pile whole

```sh
pile show <pile>
```

Look for the sentence in the intent that admits the change. If none does,
the change may belong in another pile, or the intent may be out of date.
Under `children`, a pile it holds may be where the change belongs. The
`written` line says how many commits the intent is behind.

Under `links`, look for each link whose title the change touches: what this
pile relies on of another, or what another relies on of it. Read the pile
at the other end of each of those with `pile show` too, because the change
may be owed there.

### 3. See what reading the pile will cost

```sh
pile files <pile>
```

Look for the lines and bytes of each file, and the totals, before you load
any of them. The piles it holds are named at the end, each with the
command that lists its own files.

### 4. See what is changed already

```sh
pile status --all --brief
```

Look for piles that are changed already, by somebody else or by an
earlier step. Your change should not be mixed with theirs in one commit
without saying so.

## Done when

- Every path you will write is in a pile you have read whole.
- Every pile at the other end of a link the change touches is read too.
- For each pile, a sentence of its intent admits the change. Quote it.
- No pile you will write in has an intent you do not believe. If one does,
  `pile guide write-an-intent` comes first.

## What to hand back

Before you write anything: the piles the change is in, the sentence of
each intent that admits it, the links it touches with the pile at each
other end, and any intent that looked out of date. Then
make the change, and commit it as `pile guide commit-a-change` says.
