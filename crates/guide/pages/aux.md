+++
role = reference
summary = what an aux pile is, how to make one, and what it may declare
see = types declarations links piles
+++
An aux pile, an auxiliary pile, is a pile whose config says
`kind = auxiliary-gitpile`. It is the only place a type is created. A
repository that wants its own words, such as a type for what lives
outside it, holds an aux pile that declares them.

## Make one with `pile init`

```sh
mkdir -p piles && cd piles
pile init --set name=piles --set kind=auxiliary-gitpile
```

`pile init` writes the config and the intent, as for any pile. Answer the
intent: say what words the aux pile gives the repository, and why.
`pile init --help` says what else it takes.

## What it declares, and where

An aux pile creates a type with a `[[node-types]]` or `[[edge-types]]`
table in any `.toml` file in its `.pile/`, by convention `types.toml`
(`pile guide types`). It may not create `pile`, or write a type another
aux pile created.

No table declares the keys a node, a link or `[pile]` carries of its own
(`pile guide declarations`). When the repository agrees on what one
holds, the aux pile's intent says so: "a pile's `labels` are `core`,
`docs` or `tooling`."

An aux pile is a pile, so it writes nodes and links as any pile does
(`pile guide links`).

A `[[node-types]]` or `[[edge-types]]` table in a pile that is not an aux
pile declares nothing, and `pile doctor` reports it as
`types-outside-aux`.

## See what an aux pile declares

```sh
pile show @piles     # its types, each a line
pile doctor          # every type, node and link checked
```
