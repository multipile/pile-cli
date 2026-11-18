+++
role = reference
summary = which directories are piles: a lifecycle of its own, and a leaf
see = boundaries relies-on paths intent
+++
A pile is a leaf of the project: a directory that is something, with a
lifecycle of its own. `pile guide boundaries` holds each to the
one-sentence test.

## A pile has a lifecycle of its own

A pile is for something with its own lifecycle: something designed, built,
tested, reviewed, released, run or read on its own. It need not be all of
those. A log of decisions is read and reviewed, and never built. A design
is drafted, decided and superseded. Each of those is a lifecycle of its
own.

Two parts share a lifecycle when they are built and released as one unit,
like the modules of a library. Code, data, the schema for that data, and
documents are different kinds of thing. A schema, a folder of data and a
set of documents are never built, so they share no lifecycle with each
other or with a tool that reads them, even when none has a build file.

Judge a lifecycle by what a part does: what it builds, runs or drives,
who runs it, and why it changes. Do not judge it by how the build files
are laid out today. One build file shared by two parts is a sign, not
proof: shared for convenience by parts that do different things, it is a
build file to copy, and the parts are two piles. Changing in the same
commits is not sharing a lifecycle either: that is history, and history
is only a sign.

Three rules turn on whether two parts share a lifecycle: whether a part
another pile relies on stays or is split out, whether a part reached past
a pile's entry point is exposed or split (`pile guide relies-on`), and
whether a container's shared build file is copied or kept once.

Some directories rarely have a reason of their own to give:

- tooling configuration, scratch directories, `scripts/` and `assets/`
  usually serve the pile around them;
- a generated or vendored directory has the reason of whatever produced it
  or pulled it in.

Declare one anyway when it has a claim of its own.

## A good pile is a leaf

Look for the leaves of the project: the directories that are something,
like a package, a service, a tool or a body of documents. A directory that
only holds others, such as `packages/`, `apps/` or `libs/`, is there for
tidiness. Its sentence would be a list of its children. Don't make it a
pile. Its children are the piles, and its own few files belong to the
pile above it.

Go down through containers until you reach a directory with a claim of its
own. Inside a leaf, go deeper where a subdirectory has a claim of its own.

A subdirectory has a claim of its own when somebody arrives at it with a
question of its own. That is the question its intent's closing pair would
name: "Read this pile to find out …" (`pile guide intent`). If you can
write that sentence for the subdirectory, and it would be untrue of the
leaf around it, the subdirectory is a pile, even when it serves the leaf.

Three cases show such a question:

- The project builds or runs it on its own: it has its own build file,
  such as `package.json` or `Cargo.toml`, that a task, a CI step or a
  workspace builds, or a task that runs only it.
- It is a different kind of thing from its siblings: code, data, the
  schema for that data, or documents. Each kind has a lifecycle of its
  own, though none of them is built. A directory of test cases, the
  schema they follow, and the tool that checks them are three piles.
- People call it by a name of its own. A subject talked about as one
  thing, such as "the malleability design" or "the billing rules", whose
  files are proposed, decided and superseded together, is a leaf, even
  when its siblings are the same kind of thing. Hold the name to the name
  test (`pile guide paths`).

None of these is one: test fixtures and sample inputs that only the
leaf's own tests read, even with a build file; documents about how the
leaf is built inside; and `src/` and `tests/`. Nobody builds, runs or
reads them apart from the leaf, and nobody arrives at them with a
question the leaf's intent does not answer. A body of documents with a
subject of its own is not about how the leaf is built inside, even when
it sits inside the leaf: it is the third case.
