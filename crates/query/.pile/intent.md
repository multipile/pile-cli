What a repository declares, held as a Datalog database, and a program
asked of it: the rows of each query, or why the program is refused, and
where.

What belongs here is everything between the graph's rows and an answer:
which relations a program reads and what their columns are called, how a
node, an edge and a value are written as the values a program compares,
what an `@name` in a program is, and the way back from an answer's value to
the graph's node or edge. The language a program is written in, how one is
parsed and how one is evaluated do not belong here: they are the Datalog
crate's, pinned in this crate's manifest. Neither does which rows a
declaration becomes, which is the graph's, or how an answer is printed.
What a caller makes of an answer, such as what a diagram should show, is
the caller's. Nothing here reads a repository, and nothing here writes.

Read this pile to find out what a program can ask of a repository, and how
its answer leads back to the graph. Change it when a relation a program
reads is added or changed, when how a node or an edge is written as a value
changes, or when the commit of the Datalog crate it evaluates with does.
