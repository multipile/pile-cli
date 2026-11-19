+++
role = reference
summary = what a pile's diagrams are, where they are kept, and how one is drawn
needs = diagram-includes
see = piles doctor declarations links query
+++
A pile can keep diagrams that explain it: how its parts fit together, how
something moves through it, what it holds. Each is a file in the pile's
`.pile/diagrams/`, named `<name>.diagram`, and the name is what the diagram
is typed by. Most repositories keep theirs in the top pile.

## A diagram says what it shows, never where

A diagram file is written in KDL. It names its type and the version of
its format, then says what it shows: its boxes and lines, and which one
or two are focal. It never says where anything goes. Its type places
everything, and holds it to limits, such as 9 boxes, so it shows one idea.

```kdl
diagram dependency version=1 {
  title "How the crates depend on one another"
  desc "The cli depends on the library, and both on storage."
  box cli     pile:id="42498648a7753fb45d2ec5d33b97376b"
  box lib     pile:id="9a7c1e0b5d3f4a62b8c4d6e8f0a1b2c3"
  box storage pile:id="3823eb7be13988395e3a856f282fc7dd" focal=#true
  line cli lib     pile:edge="uses"
  line cli storage pile:edge="uses"
  line lib storage pile:edge="uses"
}
```

The types are architecture, sequence, state, swimlane, process, tree,
dependency, nested and treemap. What each takes, and its limits, are
multipile's diagrams', at https://github.com/multipile/diagrams.

## A box stands for a node, and a line for a link

A property written `pile:<word>` is metadata. The format keeps it, and
only `pile` reads it: it ties what is drawn to what the repository
declares, so that a diagram does not go out of date unnoticed.

- `pile:id="<id>"` on a box makes it stand for the pile with that id, and
  the box's name is the pile's name. `pile list --full` prints every id.
- `pile:node-type=<type>` beside it makes the box stand for a node that
  is not a pile: the one of that type and the box's name. The id is then
  that of its home, the pile whose `.pile/` declares it. `external:git`
  is `box git pile:id="<its home's id>" pile:node-type=external`.
- `pile:name="<name>"` gives the node's name where the box shows another:
  `box db "The database" pile:id="<id>" pile:name="db"`.
- `pile:edge="<type>"` on a line makes it stand for the link of that
  type from its first box's node to its second's (`pile guide links`).

A box or a line with none of them stands for nothing, and is only drawn.
`pile guide diagram-includes` says how a diagram says what it should
show, so that a pile added later is not left out of it.

## doctor checks each diagram

`pile doctor` reads every diagram, and `pile doctor --diagrams` reads
only the diagrams: run it after you write one. Each finding is at the
diagram's file, line and column, and says what fixes it:

- `diagram-invalid`: it breaks a rule of its format, or a limit.
- `diagram-no-such-pile`, `diagram-name-differs`, `diagram-no-such-node`
  and `diagram-needs-id`: a box does not stand for what it says. A pile
  renamed since is `diagram-name-differs`, which says its name now.
- `diagram-no-such-edge` and `diagram-edge-needs-nodes`: no link of a
  line's `pile:edge` runs between the nodes of its two boxes.
- `diagram-meta-unknown`: a `pile:` word `pile` does not read. It says
  the nearest one.

A file in `.pile/diagrams/` that does not end in `.diagram` is an unread
file, as any other file in a `.pile/` is.

## Draw one

`pile diagram render <name>` draws a diagram and writes it to a file:
`--as html`, the default, a page that loads and runs nothing; `--as png`,
a picture; or `--as gif`, the diagram revealed element by element.
Without `--as`, the name `-o` is given decides by its ending. Where two
piles keep a diagram of one name, type it after its pile: `@lib/flow`.
A diagram with a box or a line that stands for nothing is not drawn.
`pile diagram list` lists the diagrams, each with the name render takes.

## Show what a change touched

`--since <revision>` counts a change from a revision to the working
copy, and `origin/main...` from where the branch left main, as a pull
request into main does. `pile diagram list --since <revision>` says how
the change touches each diagram:

- `added`, `edited` or `removed`: what the change did to its file;
- `piles`: its file did not change, and something it stands for was
  touched. `touched` lists each: a pile whose files changed, and a node
  or a link that was added or removed, or that declares something else.

`pile diagram render --since <revision> <name>` draws a diagram with its
change: an edited one as its two versions in one, and a `piles` one with
each touched box and line marked. In an edited one, a box is marked as
changed when it is drawn differently or stands for another node, and a
line when it is drawn differently or stands for another link. How the file
writes what one stands for marks nothing: `pile:name` written beside the
same pile's `pile:id` is no change, nor is a box that gains `pile:id` or
a line that gains `pile:edge`. An edited diagram whose earlier version
does not read is refused as `diagram-not-comparable`.
