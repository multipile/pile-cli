# Fixtures

Each directory here is a repository's piles, laid out as files. A test
copies one into a repository, and renames each `_pile/` directory to
`.pile/` as it does. Named `_pile/`, they declare no pile in this
repository, so `pile doctor` here does not read them.

`store/` is a small repository that a test can lay out whole and then
change, one case at a time. It is a key-value store: an interface, the
tests every backend is held to, two backends, the app that serves them,
and stories told against the app. Its aux pile, `@words`, creates the edge
types `implements`, `uses`, `conforms-to`, `stores-in`, `exercises` and
`replicates-to`, and the node type `external`. Every pile links to its
neighbours. One link is written in the pile at its target, one ends at an external
the top declares, one carries a key of its own, and one joins the top's
two externals, written in the top. The top has no name.

As it is laid out, every link reads as an edge and `pile doctor` finds
nothing wrong but the top's missing name. A test that needs a mistake
writes it in after laying the repository out.
