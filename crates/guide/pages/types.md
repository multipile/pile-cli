+++
role = reference
summary = node types and edge types, and when a type exists
see = aux declarations links
+++
A type names what a node or an edge is. A **node type** is a name. An
**edge type** also says which node types each end of its edges may be. A
type is not a node: it is what a node or an edge is.

## A type exists while an aux pile declares it

A type exists while exactly one table declares it, with no mistake in that
table, in an aux pile whose kind is `auxiliary-gitpile` (`pile guide
aux`). A node or a link of a type with a mistake is no further finding:
the mistake's own finding is the one to fix. The one exception is `pile`:
the format itself creates it, and its nodes are exactly the piles, each
declared by its config and referred to as `@name`.

## A type is a `[[node-types]]` or `[[edge-types]]` table

```toml
# .pile/types.toml, in an aux pile
version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository that a pile relies on"

[[edge-types]]
name = "uses"
title = "one pile relies on another, or on something outside the repository"
source = ["pile"]
target = ["pile", "external"]
do = "say what the source relies on"
dont = "list functions"
```

A type table takes `name`, `title` and `description`. An edge type also
takes:

- `source` and `target`, each a list of the node types that end may be.
  Both are required. An edge goes from its source to its target. A link
  whose end is of a type its edge type does not allow is `wrong-end`
  (`pile guide links`).
- `do` and `dont`: what a link's description has to name, and what it
  must not.

A type table takes no other key. A type says nothing about the keys a
node or a link carries of its own (`pile guide declarations`).

## A type's name is one word

A type's `name` is a word, such as `external` or `service`. One name is
one type across the whole repository, whichever aux pile created it, and
is written once. One name is never both a node
type and an edge type.

## See the types

`pile show` of an aux pile lists the types it creates under `types`: node
or edge, its name, the node types each end of an edge type may be, and
its title. Under an edge type it prints its `do` and `dont`, so that
whoever writes a link's description reads them first.
