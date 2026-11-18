+++
role = reference
summary = ask a question of what the piles declare, as a Datalog program
see = declarations links types reading
+++
`pile query` asks a question of what the repository declares: its piles,
the nodes and links in their `.toml` files, and the types
(`pile guide declarations`). The question is a Datalog program. Ask one
when `pile show` and `pile list` do not answer: which piles rely on this
one, which have no link at all, which carry a label.

```sh
pile query '?- edge(_, Node, "relies-on", @ledger).'
pile query --file covers.dl
```

## A query is a clause that starts with `?-`

```prolog
% every pile, and the pile it sits in
?- parent(Child, Parent).

% what relies on @ledger, straight or through another pile
needs(A, B) :- edge(_, A, "relies-on", B).
needs(A, C) :- needs(A, B), edge(_, B, "relies-on", C).
?- needs(Node, @ledger).

% the nodes labelled service that no link ends at
?- prop(Node, labels, service), !edge(_, _, _, Node).
```

- A word that starts with a capital letter is a variable, and `_` is any
  value. A query is answered with a column for each variable it binds, and
  a row for each answer.
- A clause with `:-` is a rule: it defines a relation of the program's
  own, which its queries read. It is gone when the command returns.
- `!` before an atom says no such row exists. `=`, `!=`, `<`, `<=`, `>`
  and `>=` compare two values.
- `%` starts a comment. Every clause ends with a `.`.

## The relations a program reads

| Relation | One row is |
|---|---|
| `node(Node, Type, Home)` | a node, its type, and the pile it is at home in |
| `prop(Node, Property, Value)` | one value of a node's property |
| `edge(Edge, Source, Type, Target)` | an edge, and the nodes it joins |
| `edge_prop(Edge, Property, Value)` | one value of an edge's property |
| `type(Type, Sort, Home)` | a type, `node` or `edge`, and its aux pile |
| `type_prop(Type, Property, Value)` | one value of a type's property |
| `allows(EdgeType, End, NodeType)` | a node type an edge type takes at an end |
| `at(Item, File, Line, Column)` | where a node, an edge or a type is written |
| `prop_at(Item, Property, File, Line, Column)` | where one value is written |
| `parent(Child, Parent)` | a pile, and the pile it sits in |

A pile is a node of type `pile`, at home in itself. Its properties are
`id`, `name`, `path` and `headline`, and each key of its `[pile]` table. A
list gives a row for each item, so `labels = ["service", "ui"]` is two
rows of `prop`. A property that is not given has no row.

## How a value is written

- **A pile** is `@name`, as a link writes it. `@name` in a program is the
  pile of that name, and a name no pile claims is refused. A pile with no
  name is its path, in quotes: `":/services/billing"`.
- **Any other node** is `type:name` in an answer: `external:postgres`. A
  program cannot write that form yet. It reaches such a node through the
  relations: `node(Node, external, _)`.
- **An edge** is one value, the first column of `edge`. It is how
  `edge_prop` and `at` name an edge, and an answer prints it as its
  source, its type and its target.
- **A type, a property or a text** is written bare when it is letters,
  digits and `_` after a lowercase letter: `pile`, `labels`. Anything
  else is in double quotes: `"relies-on"`, `"the git binary"`.
- **An integer** is written bare, and compares as a number: `port = 8080`
  in a link is found by `edge_prop(Edge, port, Port), Port > 1024`. A line
  and a column are integers too.

## When a program is refused

A program that does not read, or that names a relation or a pile there is
none of, is refused. The refusal says the line and the column of the
mistake, counted in the program, and nothing is answered. A relation is
always written with all of its columns, with `_` for each one that does
not matter. A program with no query is answered with nothing.

`pile query --help` says what the command prints, in text and as a
result.
