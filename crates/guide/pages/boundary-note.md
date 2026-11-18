+++
role = reference
summary = the note that reports a boundary found untrue during a change
see = boundaries relies-on refine commit-a-change
+++
During an ordinary change, a boundary you find untrue is reported in a
boundary note in what you hand back, and never moved. Moving it is for
`pile guide refine`, with the person's agreement.

## When to write one

Write a note when one of these four signs holds, and for nothing else:

1. The change adds a path that points below another pile's root. Look in
   the lines it adds (`git diff HEAD -U0`, and any new file) for the other
   pile's path, as `pile list` prints it, followed by `/`.
2. A link's title or description would have to name a path inside another
   pile.
3. No sentence of the intent covers the change.
4. A directory the change touched has its own build file, or a task that
   runs only it, but is not a pile.

Only what you saw in files the change touched counts. Write at most one
note per pile in a hand-back, never write one into a `.pile/` file, and
write none when the task is itself a refine.

## The template

Always use this exact template:

```markdown
**Boundary note** — no boundary was changed; a person decides.

| | |
|---|---|
| **Piles** | `@<name>` `:/<path>`, `@<other>` `:/<path>` |
| **Sign** | <one of the four above> |
| **Evidence** | `<path:line>`, link or commit ids. Re-run: `<command>` |
| **Intent strained** | "<quoted sentence>", or none, and why |
| **Would suggest** | <operation> → <what it makes> (sure / guess) |
| **Cost** | <what the operation costs>, or none |
| **Decision** | <one yes/no question> |
| **Next** | run refine on `@<name>`, or none |
```

Fill it this way:

- Every row, in this order. Write "none" in a row that has nothing.
- One line per cell. Separate items with `;`, and put commands in inline
  backticks: a code block does not fit in a cell.
- Write a `|` inside a cell as `\|`.
- **Would suggest** names one operation: split, merge, move, unmake,
  rename or declare. After the arrow come the names, the paths and a
  one-line draft intent. Or it is a question about code.
- **Cost** counts piles declared, ids minted or retired, files moved and
  references rewritten.
- The note is complete on its own, because a hand-back is not carried
  into the next session.

## An example

After a task "let a customer download every invoice as a CSV file" that
edited files of `@billing`, one of them under
`services/billing/export/`, the rows read:

- **Piles**: `@billing` `:/services/billing`
- **Sign**: a directory the change touched has its own build file, but
  is not a pile
- **Evidence**: `services/billing/export/package.json`, a workspace
  member in `package.json`. Re-run:
  `git ls-files services/billing/export`
- **Intent strained**: "Charges each customer once a month and records
  it in `@ledger`." Writing export files is not that.
- **Would suggest**: split → `@export` `:/services/billing/export`:
  "Turns an invoice into a file a customer can download." (guess)
- **Cost**: one pile declared; one id minted; no links to rewrite
- **Decision**: Should `:/services/billing/export` be its own pile?
- **Next**: run refine on `@billing`
