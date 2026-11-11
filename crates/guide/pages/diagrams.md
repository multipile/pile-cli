+++
role = reference
summary = what a pile's diagrams are, where they are kept, and how one is drawn
see = piles doctor declarations
+++
A pile can keep diagrams that explain it: how its parts fit together, how
something moves through it, what it holds. Each is a file in the pile's
`.pile/diagrams/`, named `<name>.diagram`, and the name is what the diagram
is typed by. Most repositories keep theirs in the top pile.

## A diagram says what it shows, never where

A diagram file is written in KDL. It names its type and the version of the
format it is written in, then says what the diagram shows: its boxes and
lines, and which one or two are focal. It never says where anything goes,
or what colour or size it is. The diagram's type places everything by its
own rules, and holds the diagram to its limits, such as 9 boxes, so that it
shows one idea.

```kdl
diagram dependency version=1 {
  title "How the crates depend on one another"
  desc "The cli depends on the library, and both on storage."
  box cli pile="@cli"
  box lib pile="@lib"
  box storage pile="@storage" focal=#true
  line cli lib
  line cli storage
  line lib storage
}
```

The types are architecture, sequence, state, swimlane, process, tree,
dependency, nested and treemap. What each takes, and its limits, are
multipile's diagrams', at https://github.com/multipile/diagrams.

## A box can stand for a pile

`pile="@lib"` makes a box stand for the pile named lib, and
`pile=":/docs"` for the pile at `:/docs`. The box shows the pile's name.
Give the pile's id too, as `id="..."` from `pile list --full`, and a pile
renamed later is still found by it.

## doctor checks each diagram

`pile doctor` reads every diagram, and `pile doctor --diagrams` reads the
diagrams and nothing else. Each finding is a problem, at the diagram's file,
line and column:

- `diagram-invalid`: the diagram breaks a rule of its format, or of its
  type's limits. The finding says which, and what fixes it.
- `diagram-no-such-pile`: a box names a pile that no pile answers to. When
  it gives an id that a pile has, the finding names what that pile is
  called now.
- `diagram-pile-id-differs`: the name and the id it gives are two piles.

A file in `.pile/diagrams/` that does not end in `.diagram` is an unread
file, as any other file in a `.pile/` is.

## Draw one

`pile diagram render <name>` draws a diagram and writes it to a file:

- `--as html`, the default: a page that loads and runs nothing, with its
  fonts in it, to open in a browser or put in another page;
- `--as png`: a picture;
- `--as gif`: the diagram revealed, element by element.

Without `--as`, the name `-o` is given decides by its ending. Where two
piles keep a diagram of one name, type it after its pile: `@lib/flow`.
A diagram doctor finds a problem in is not drawn.
