The read-only interface every read of a repository goes through, so that
no command knows which backend answers it.

The interface names what a command needs to know, not how git finds it
out: the piles' files and the files under a path, in the working copy or
at a revision, what changed, and which revisions changed it. A revision
is an id the backend chose, and nothing outside the backend reads into
it. Nothing here writes, because `pile` makes no commit. The backends
belong here too, with the suite every backend has to pass, and one
backend held in memory that proves the interface has no git in it. What
a command makes of an answer, such as which pile owns a path, does not
belong here.

Read this pile to find out what a command may ask of a repository, and
what each backend promises in its answer. Change it when a command needs
to ask something the interface cannot, or when a backend is added.
