+++
role = reference
summary = what an intent says, the shape it takes, and how it is kept true
see = write-an-intent boundaries links doctor
+++
A pile's intent, its `.pile/intent.md`, says why the subtree is a pile. It
is the one thing about a pile that neither git nor `pile` can work out, so
only a person can say it. `pile init` writes the intent as a question, in a
comment, and `pile doctor` reports the pile as a problem until somebody
answers it.

## An intent opens with its headline and ends with its closing pair

```markdown
One claim about what the pile is for. This paragraph is its headline.

What belongs in it, and what does not, if the headline needs more.

Read this pile to find out <X>. Change it when <Y>.
```

- **The headline** is the first paragraph of prose. `pile doctor` and
  `pile which` print it beside the pile. Make it a claim from which a
  stranger could say whether a new file belongs here, not a topic, an
  address or a list. It runs to the first blank line, or to a line that
  opens a block of its own: a heading, a rule, a bullet, a quote or a
  numbered item. Its lines are joined into one, so wrap where you like.
  Text inside `<!-- -->` is not prose.
- **What belongs** is the paragraphs between, if the headline needs them.
  Say what sort of thing does not belong ("anything that knows git"),
  and not which pile holds it.
- **The closing pair** is the last two sentences. The first names what a
  reader opens the pile to find out: a question somebody arrives with. The
  second names when the pile changes: an event somebody could see happen,
  not "when needed". The words are the pile's own, and nothing follows
  them.

## An intent has no heading

An intent starts with its headline. `pile` says where the pile is, and a
path written in the file goes stale when the pile moves. A heading line on
top is skipped when the headline is read, and `pile doctor` warns of it.

## What an intent leaves out

- The piles it holds: `pile list` shows them.
- What it is to another pile, such as what it calls there. A link says
  that, at one of its ends (`pile guide links`).
- How the pile is built inside, and why. That goes in a document beside
  the pile's files, and the intent points to it.
- Anything that would change when the code does and the purpose does not:
  a file name, a function, a count.

## Hold a draft to four tests

- **The headline alone.** Read it with nothing else. A stranger can say
  from it whether a new file belongs.
- **Placement.** Take four plausible changes: a new file, a new
  dependency, a feature somebody would ask for, and something that belongs
  in the parent. Each has a sentence that admits it or refuses it.
- **Survival.** No sentence needs editing when the code changes and the
  purpose does not.
- **The pair is the pile's.** Pasted under a sibling pile, it would be
  untrue there.

## Keep it true

Read a pile's intent before you change the pile. When a change makes the
intent untrue, rewrite it in the same commit. Replace the sentence that
became untrue, and do not add one beside it.

`pile show` says how many commits have changed the pile since its intent
was written. The count is not a verdict. But an intent many commits behind
its pile is one to read again, asking whether it is still true.

An intent is drafted, then handed back to a person, and committed only once
they approve it. A plausible sentence nobody approved is worse than none:
it makes `pile doctor` pass while the file says nothing.
`pile guide write-an-intent` is how a draft is made.
