+++
role = start
summary = where a session begins, and how to read the rest of the guide
needs = piles paths
see = adopt start-a-change write-an-intent commit-a-change read-a-change review reading intent boundaries doctor quality declarations types aux links answers agent-mode
+++
This is `pile guide`: pages of prose that the `pile` binary carries, about
the binary that printed them. Read this page and the two printed under it
whole, before anything else. They say what a pile is, and how to name one.

## Ask for a page by its name

```sh
pile guide                  # this page, and the pages it needs
pile guide adopt            # one page, and the pages it needs
pile guide intent doctor    # several pages, each printed once
pile guide --list           # every page, a line each
pile guide --agents         # the section for a rules file
```

A page has a name, and no number. Under each page are its references,
under two keys:

- `needs` names a page you cannot act on this one without. `pile guide`
  prints it after the page, then the pages it needs in turn, each once.
- `see` names a page you may want next. It is named, and not printed.

An agent gets the guide in an envelope, as it gets every answer
(`pile guide answers`). Each page is one item of `result.pages`, with its
text whole, and a workflow has its parts beside the text, under
`workflow`. The section is the exception: `pile guide --agents` prints it
as text in every format, because it is written into a file.

## Carry out a workflow from the top

A workflow is a page for accomplishing one thing. It says when to use it,
what to have first, the steps with the exact commands to run and what to
look for in each answer, what is true once it is done, and what to hand
back.
Print it, and follow it in order. Do not carry one out from memory.

- `pile guide adopt`: a project that declares no pile yet.
- `pile guide start-a-change`: before you write in a file, to find the pile
  it is in and read that pile first.
- `pile guide write-an-intent`: a pile whose intent is missing, still a
  question, or no longer true.
- `pile guide commit-a-change`: before every commit.
- `pile guide read-a-change`: a change somebody made, read pile by pile.
- `pile guide review`: a pull request, held to its task and each intent.

## Read a reference page when it applies

- `pile guide reading`: which command reads what, and when to reach for it.
- `pile guide intent`: before you write or change an intent.
- `pile guide boundaries`: when you decide whether a subtree is one pile.
- `pile guide doctor`: what doctor's findings and severities mean.
- `pile guide quality`: the words a review judges a change by, and its shape.
- `pile guide declarations`: the `.toml` files in a `.pile/`, and what
  each table in them declares.
- `pile guide types`: before you write a type, or a node or link of one.
- `pile guide aux`: before you make an aux pile, or change one.
- `pile guide links`: before you write a link, or follow one.
- `pile guide answers`: the envelope, exit codes and refusal codes.
- `pile guide agent-mode`: what changes when an agent runs `pile`.

## The help is the home of each command

What a command takes and what it prints is in its help:
`pile <command> --help`. A page says what holds across commands, and
points to the help rather than repeating it. Each help ends with the
refusals that command can give, and `pile guide answers` lists the ones
any command can.
