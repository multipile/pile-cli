+++
role = reference
summary = how a diagram says what it should show, and what it leaves out
see = diagrams query links doctor
+++
A diagram of what relies on the ledger goes out of date when one more
pile comes to rely on it. So a diagram can say what it should show, as a
Datalog program, and `pile doctor` reports each node and each link the
program finds that the diagram does not draw. Nothing is drawn for you:
a program only says what is missing.

```kdl
diagram dependency version=1 {
  title "What relies on the ledger"
  desc "Billing and the reports both write through the ledger."
  box billing pile:id="5b2817e47e4575f8d2fbf781b94c51a5"
  box reports pile:id="1177c060df42448633cf931fad2854bf"
  box ledger  pile:id="0fdcfbf7bb29513f48c076b94e01a314" focal=#true
  line billing ledger pile:edge="relies-on"
  line reports ledger pile:edge="relies-on"
  meta pile {
    boxes {
      include #"?- edge(_, Node, "relies-on", @ledger)."#
      ignore "admin" id="d4e5f60718293a4b5c6d7e8f90a1b2c3" \
        reason="an experiment, removed before the next release"
    }
    lines {
      include #"?- edge(Edge, _, "relies-on", _)."#
    }
  }
}
```

`meta pile` is the diagram's own metadata for `pile`. It holds `boxes`,
and `lines` in a type whose lines take `pile:edge`: architecture,
swimlane and dependency.

## include takes a program

An `include` is a program as `pile query` takes one, over the same
relations (`pile guide query`). Ask it with `pile query` first, and put
it in the diagram once it answers what you mean.

- Under `boxes`, each query binds `Node`, and the diagram should show
  every node found.
- Under `lines`, each query binds `Edge`, the first column of `edge`, and
  the diagram should show every link found between two nodes it draws.
- `@name` is the pile of that name. A rule is that `include`'s alone.
  Several `include` add up, and a box the programs do not find is fine.

Write a program in a raw string, `#"…"#`, so that `"relies-on"` needs
no escape. It is one string, a string for each clause, or one string of
several lines:

```kdl
include #"near(N) :- edge(_, N, "relies-on", @ledger)."# \
        #"near(N) :- edge(_, @ledger, "relies-on", N)."# \
        #"?- near(Node)."#
include #"""
  service(P) :- prop(P, labels, service).
  ?- service(Node), !edge(_, Node, "relies-on", @ledger).
  """#
```

## ignore leaves one out, and says why

`ignore` names one node or link the diagram should show and leaves out
on purpose, with the reason. Under `boxes` it names a node as a box
does: its name, the id of the pile it is or is at home in, and
`node-type=<type>` for a node that is not a pile. Under `lines` it names
a link by its type and its two ends:

```kdl
ignore "relies-on" reason="drawn in the reports diagram" {
  from "reports" id="1177c060df42448633cf931fad2854bf"
  to "ledger" id="0fdcfbf7bb29513f48c076b94e01a314"
}
```

What a program should never find, say in the program, with `!`.

## What doctor reports

- `diagram-misses-node` and `diagram-misses-link`, at the `include`: it
  finds a node or a link that nothing draws. Draw it, or ignore it with
  a reason: the finding writes out both.
- `diagram-include-invalid`: the program is refused, at its line and
  column in the diagram, or it has no query, or one binds no `Node`.
- `diagram-include-empty`: the program finds nothing, as one with a
  mistyped name does.
- `diagram-ignore-no-reason`: an `ignore` without a `reason`.
- `diagram-ignore-unneeded`, a note: an `ignore` of what the diagram
  draws, or of what no `include` finds. Remove it.

A diagram that misses something is still drawn. `pile diagram list
--since <revision>` gives each diagram's `misses`, whatever the change
touched, so a review can say what a change left out of a diagram.
