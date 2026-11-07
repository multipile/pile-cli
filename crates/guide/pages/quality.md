+++
role = reference
summary = the words a review judges a change by, and the shape it hands back
see = review intent links boundaries
+++
A review judges a change, pile by pile, by three things: each pile's
intent as the branch leaves it, the task, and what the change itself
does. A branch may rewrite an intent, and the rest of the change is held
to what the intent says now. A pile the change did not touch is not
judged. This is not a code review: a bug, a security hole or a style
problem is never a finding here.

Each changed pile gets two answers. The first is the task:

- **in the task**: the task needs this pile's part of the change.
- **not in the task**: nothing in the task needs it.

The second is the intent. A sentence of an intent admits a change when
the change is the kind of thing it says the pile holds, and refuses it
when it says that kind of thing does not belong. The closing pair, "Read
this pile… Change it when…", admits nothing.

- **fits the intent**: a sentence admits the change.
- **outside the intent**: no sentence admits the change, or one refuses
  it.
- **makes the intent untrue**: the change belongs, but a sentence is false
  after it, and the change did not rewrite that sentence.

A rewritten intent is held to its base too. It is **worse** when it is:

1. vaguer: the headline became a topic, a list, or says "and" twice;
2. wider only to admit this change: it adds ground nothing else in the
   pile holds;
3. claiming another pile's ground: what its parent, a child or a sibling
   claims;
4. without a refusal it had: a sentence that said what does not belong was
   dropped or weakened, and nothing it refused moved out.

Any other rewrite is no worse, and is not reported.

Links are held to their own type, whatever the repository named it:

- **link made untrue**: after the change, a link to or from a changed pile
  no longer holds: its title is false, or its type no longer describes its
  ends, and the change did not fix it.
- **link against its type**: a link the change added or rewrote breaks its
  type's `do` or `dont`.
- **missing link**: the change makes one pile rely on another the way one
  of the repository's types describes, and no link says so.

## Findings, and their two levels

A finding of a review is one thing the author has to act on: the pile,
what is wrong, the sentence, title or headlines it is judged by, and one
action that fixes it. A review reports findings, and nothing else. A
finding's level comes from its kind: never raise or lower it.

- **blocking**: something in the change must change before it merges:
  makes the intent untrue, an intent rewritten worse, a link made untrue,
  a link against its type.
- **consider**: there are two honest fixes, and only the author knows
  which: not in the task, outside the intent, a missing link. Outside the
  intent is fixed by moving the change, or by rewriting the intent to
  admit it.

A pile that is in the task and fits its intent, with no link finding, has
no finding, and a review does not list it.

## How a review is written

````markdown
**Pile review** · 1 blocking · 1 to consider

#### Blocking

- **`@posts`** · makes the intent untrue: the list is sorted by length.
  > the published ones, newest first
  **Fix:** keep the list newest first, or rewrite the intent.

#### Consider

- **`@auth`** · not in the task: sessions last 30 days, not 7.
  **Fix:** move it to its own pull request, or say why it is here.

<details><summary>How each pile was judged</summary>
…a line per changed pile: its two answers, and what admits each part…
</details>
````

An entry is at most three lines: the pile, the finding and what happened
in one sentence, the quoted sentence, title or both headlines, and one
fix. Blocking comes first. With three findings or more, a table goes
above the entries, a row each: level, pile, finding, fix. With none, the
first line is the whole review: `**Pile review** · nothing to act on`.
