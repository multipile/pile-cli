+++
role = reference
summary = what one pile relies on of another, and how it tests a boundary
see = boundaries leaves links intent proposal refine
+++
A pile relies on what another pile is for. That is a concept, not a file.
Code, build files and prose are the evidence of it:

- a path named in code or data outside a directory, and its relative
  forms, such as `../schema/`: `git grep -n -F '<dir>/' -- ':!<dir>'`;
- an import, a runtime read, or a path dependency in a build file;
- prose in a README, or in a link, that names a path inside another pile;
- a child pile that reads its parent's own files.

The top pile, CI files, and task runners such as `mise.toml` or a
`Makefile` are not dependents: they name nearly every directory. And
`git grep` misses code that searches for a path or takes it from a
variable, so read the code that loads files, not only the strings.

Evidence shows what a pile relies on, not what it should rely on. Before
you draft a link, rule out two kinds of evidence:

- **What a pile is judged by.** A link records what a pile uses. Prose
  that calls another pile "the contract" or "the spec", with no code that
  reads it, describes a process. When another pile enforces the contract,
  such as a runner that checks an implementation against a set of cases,
  the links go from that runner: to the cases, and to what it drives.
- **Coupling between piles meant to be independent.** Siblings may be
  meant to stand apart, such as alternative implementations of one
  contract. Then the evidence, even a comment like "ported from" that
  names a path in the other, is coupling to remove. Hand it back as a
  question about code, not as a link.

Code that reaches a pile by its public name, such as a package name, its
entry point or its exports, or by its root, relies on the whole pile. Code
that needs a path below the root relies on a part.

## The test is the link's title

For each pile `@X` that relies on another, `@Y`, draft the title of the
link (`pile guide links`). Then ask, from `@X`'s side: if everything in
`@Y` changed except what this title names, would `@X` have to change? If
it would, `@X` relies on `@Y` as a whole, and the title names `@Y`. If
not, the title names a part of `@Y`. Read these in order, and the first
that matches decides:

1. The part is named in `@Y`'s intent, shares `@Y`'s lifecycle, and is
   reached through `@Y`'s entry point: keep the boundary. The title may
   name the part.
2. The part is claimed by `@Y`'s intent and shares its lifecycle, but is
   not named, or is reached past the entry point: expose it. Name it in
   `@Y`'s intent. Publishing it in code is a question for the person.
3. The part has a sentence of its own, or no lifecycle shared with `@Y`:
   split it out.
4. The part is an internal detail nothing claims: ask the person whether
   `@X` should stop relying on it.

A part is claimed when a sentence of the intent covers it, and named when
the intent's paragraphs on what belongs mention it in words
(`pile guide intent`). The entry point is what `@Y` publishes: its
package name, its exports, or `lib.rs`. A change to code is never a
boundary to propose: hand it back as a question.

When you lay a project out, no intent exists yet. Hold the part to the
sentence you propose for `@Y` instead, with two outcomes. A part with a
sentence or a lifecycle of its own is split out. Any other part stays,
and the draft intent names it.

## In this order

1. Look first for the three cases of a leaf (`pile guide leaves`): a
   build file or a task, a kind of thing, and a name people use.
2. Hold what is left in each parent to the one-sentence test.
3. Draft the link titles between the piles you would propose.
4. For each title that names a part, read the list above.
5. After a split, test again only the parent it left behind.

Make one pass, then propose to the person. Going again until nothing
changes risks a chain of splits nobody asked for. When two piles rely on
each other, or the repository has no history to read, say so, and put the
boundary to the person as a question.

The `src/` and `tests/` rule wins over a title that names a part. A parent
left holding only child piles and its own files is a container: it is not
declared, and its own files are held to `pile guide boundaries`. Any
other split that would leave the parent with no true sentence is not
proposed.
