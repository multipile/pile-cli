The `pile` binary: the command line a person or an agent types, and how
every answer it gives is printed.

What belongs here is each command, its help, its refusals and their codes,
the envelope and agent mode, and the suite that drives the built binary
against a real git repository. Every command comes with at least one test
in that suite. How a repository is read does not belong here: a command
asks the storage interface, and never asks git itself. Neither does what
holds across commands, or how a whole piece of work is carried out: that
is the guide's to say, and a help points to it.

Read this pile to find out what a command does and how its answer is
printed. Change it when a command, a flag, a refusal code or the shape of
an answer is added, changed or removed.
