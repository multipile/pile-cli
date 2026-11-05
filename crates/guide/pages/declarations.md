+++
role = reference
summary = the .toml files in a .pile/: their version, and what each table declares
see = types aux links doctor piles
+++
Every `.toml` file in a pile's `.pile/` is a declaration file `pile` reads,
whatever its name. A table says what it declares by its header, so the
file it sits in decides nothing. By convention an aux pile writes its
types in `types.toml`, and a pile writes its nodes in `nodes.toml`, its
links in `links.toml` and its own values in `pile.toml`. Any other file in
a `.pile/`, besides the config and the intent, is an unread file.

What a repository declares is a graph. Each pile is a **node**, and a pile
may declare other nodes, such as something outside the repository it
relies on. An **edge** joins one node to another, and is written as a
**link**. Every node and every edge has a **type**. An aux pile creates
each one, except `pile`, the type of every pile (`pile guide aux`).

A **word**, wherever these pages ask for one, is lowercase letters, digits
and hyphens, starting with a letter: `postgres`, `relies-on`.

## Every file starts with its version

```toml
version = "gitpile/1"
```

The first key of every declaration file is `version`, before any table.
It names the format the file is written in. `gitpile/1` is the only
version so far. A file with no `version`, or with another, is not read,
and `pile doctor` reports it as `no-version`. It is not the config's
`version`, which says the format of the config (`pile guide piles`).

## A table's header says what it declares

| Table | Declares | In |
|---|---|---|
| `[[nodes]]` | a node, at home in this pile | any pile |
| `[[edges]]` | an edge, written as a link (`pile guide links`) | any pile |
| `[pile]` | this pile's own values; one table, not an array | any pile |
| `[[node-types]]` | a node type (`pile guide types`) | an aux pile |
| `[[edge-types]]` | an edge type | an aux pile |

A file may hold any of them, each as many times as it needs. What a table
is about is in its keys.

## A node is a `[[nodes]]` table in its home

```toml
# .pile/nodes.toml
version = "gitpile/1"

[[nodes]]
type = "external"
name = "postgres"
title = "the database every invoice is kept in"
port = 5432
```

The pile whose `.pile/` holds the table is the node's home. A node is
declared once, in any pile. A node several piles rely on belongs in the
top pile, which holds them all, or in an aux pile. A node table takes:

- `type`, the node's type, by its name. It is required, and is never
  `pile`: a pile is declared by its config.
- `name`, a word. It is required.
- `title` and `description`.
- keys of its own, such as `port`.

A node is referred to as `"external:postgres"` (`pile guide links`).
`pile show` lists the nodes at home in a pile.

## `[pile]` gives a pile's own values

`[pile]` takes `title`, `description`, and keys of its own, such as
`labels = ["core"]`. A title sits beside the intent's headline, and does
not replace it. A pile's id and name stay in its config, so `[pile]` never
gives `id`, `name`, `path` or `headline`. `pile show` prints each value a
pile gives.

## A node, a link or `[pile]` may carry keys of its own

Beyond the keys it takes, a `[[nodes]]`, `[[edges]]` or `[pile]` table
may carry any key named by a word, with any TOML value: `port = 5432`,
`labels = ["core"]`, `through = "http"`. No type declares these keys, and
nothing checks their values. When a repository agrees on what one holds,
its aux pile's intent says so (`pile guide aux`). A key one letter from a
key the table takes, such as `targte` or `titel`, is refused as a
mistake, so a misspelled end is never read as an end left out.

Such a value is never a reference: `owner = "@ledger"` is a string, not a
link to `@ledger`. To join one node to another, write a link.

## Each thing is written once

- A node, by its type and its name, once in the repository.
- A type, by its name, once in the repository.
- An edge, by its source, its type and its target, once in the repository.
- A key of `[pile]`, once in its pile, though `[pile]` may span files.

Two tables or keys that name one thing are each `declared-twice`, and
neither is read until one is removed.

`pile doctor` reports each mistake in a declaration file at its line and
column. `pile doctor --help` lists every code.
