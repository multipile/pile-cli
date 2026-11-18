The diagrams a repository keeps in its piles' `.pile/diagrams/`: which
files they are, what each says of the graph, what doctor finds wrong with
each, and how one is drawn as a page or a picture.

What belongs here is what ties a diagram to the repository: finding the
files among the declarations, and holding each to the rules of its format
and to what its metadata says of the graph. That metadata is the words
written under the owner `pile`, which this crate alone reads: the node a
box stands for, the edge a line stands for, the Datalog programs that say
what the diagram should show, and what it leaves out on purpose, with the
reason. So does what follows from them: which nodes and edges a diagram
should show and does not, and which of those it stands for a change
touched. Handing a diagram that reads to multipile's diagrams to draw
belongs here too. What a diagram may say, how it is laid out, how it looks
and how it is rendered do not belong here: they are multipile's diagrams',
pinned in this crate's manifest, which keep metadata and never read it.
Neither does which rows a declaration becomes, how a program is evaluated,
a command, or how an answer is printed. Nothing here knows git, and
nothing here writes.

Read this pile to find out where a repository's diagrams are, which words
a diagram says of the graph, and what is checked of them. Change it when
where a diagram is kept, a word of its metadata, what doctor holds it to,
or which commit of multipile's diagrams it is drawn with changes.
