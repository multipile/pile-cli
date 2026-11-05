+++
role = workflow
summary = write a pile's intent, or rewrite one that is no longer true
needs = intent
see = commit-a-change boundaries links
+++
## When to use

A pile has no intent, its intent is still the question `pile init` wrote,
or a change has made it untrue. `pile doctor` reports the first two as
`no-intent`, `intent-empty` or `intent-unanswered`. Only a reader notices
the third.

## What to have first

- The pile, by its name or its path.
- A person who can say what the pile is for. You draft, and they approve.

## Steps

### 1. Read the pile, and where it sits

```sh
pile show <pile>
```

Look for its parent and its children: the intent says nothing of the
children, and must not claim what the parent claims. Under `links`, what
the pile is to other piles is said already, and the intent does not
repeat it. Read the intent as it stands. Under `written`, the count of
commits since the intent last changed says how far behind it may be.

### 2. Read the intents around it

```sh
pile show <parent>
pile list
```

Look for how the parent and the piles beside this one write a headline and
a closing pair. Match their shape, and do not claim what one of them
claims.

### 3. Read what the pile holds, and what has changed it

```sh
pile files <pile>
pile log -n 20 <pile>
```

Look for the sorts of file it holds, the changes it has taken, and the
piles it changes together with. The authors are who to ask what it is for.

### 4. Write the draft, and ask doctor about it

Write `.pile/intent.md`: the headline, what belongs, and the closing pair,
with no heading. When you rewrite, replace the sentences that became
untrue.

```sh
pile doctor
```

Look for the pile with no finding on its intent, and read the headline
doctor prints beside it. Hold the draft to the four tests
(`pile guide intent`).

### 5. Hand it back, and wait

```sh
pile show <pile>
```

Give the person the intent as `show` prints it. Say which sentences you
are sure of and which you guessed at. Wait until they approve it, and
change it as they say.

## Done when

- `pile doctor` reports nothing about this pile's intent.
- The intent starts with its headline, and ends with its closing pair.
- A person approved it.
- A rewrite replaced what became untrue, and added nothing beside it.

## What to hand back

The intent, approved, uncommitted unless you were asked to commit it. The
sentences the person changed, and any question about the boundary itself
that came up (`pile guide boundaries`).
