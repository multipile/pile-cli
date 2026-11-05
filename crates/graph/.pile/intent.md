What a repository declares, read as the base relations: a row for each
node and each edge, for each value of their properties, and for where each
is written.

What belongs here is everything that turns declarations into those rows:
the types the aux piles create, which node a link's reference names, and
the line and column a reader found each one at. A reader of each
declaration file belongs here, and so do the checks, which hold everything
a reader found to the format before it is a row, and give doctor a finding
for each thing that does not check.
What makes a subtree a pile, which pile a path is in and which pile holds
which do not belong here: the rows are read off them as they are found.
Neither does how a repository is read, how a query is evaluated over the
rows, or how an answer is printed. Nothing here knows git, and nothing
here writes.

Read this pile to find out which rows a declaration becomes, and where
each row says it was written. Change it when what a repository may declare
changes, or when a reader is added for a declaration file.
