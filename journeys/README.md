# Journeys

A journey is one session somebody has with `pile`, as a bash script in
`stories/`. It makes a git repository they could arrive at, runs the built
binary the way they would type it, and holds what came back to what the
command's `--help` says. The check runs every journey.

```bash
journeys/run                           # every journey
journeys/run a-mistyped-command-and-what-the-refusal-tells-you
SHOW=1 journeys/run <journey>          # print the session as it goes
KEEP=1 journeys/run <journey>          # keep the repository it made
PILE_BIN=$(which pile) journeys/run    # ask another binary the same things
QUIET=1 journeys/run                   # print a journey only when it fails
journeys/record [<journey>...]         # record into journeys/casts/
```

Run them in the flake's shell, which brings jq and asciinema:
`nix develop -c journeys/run`.
`run` builds `pile` from this checkout, unless `PILE_BIN` names a binary. It
gives each journey a fresh repository in a temporary directory, and deletes
it afterwards unless `KEEP` is set. It exits non-zero when any journey failed.

A journey runs as a person, with `AI_AGENT` and `AGENT` unset, unless it is
about an agent and sets one itself. It runs with no git configuration and
none of the variables a git hook sets, so a run from inside a hook reads
only its own repository.

## Watching one

A run prints verdicts, and nothing of the session. `SHOW=1` prints the
session too: each command as it would be typed, and what came back.

`journeys/record` runs each journey under asciinema with `SHOW` on, and
writes `journeys/casts/<journey>.cast`. `asciinema play <cast>` plays it
back as it went. A failing journey fails its recording too. git ignores
`casts/`, because a cast is of one run of one checkout.

## Writing one

A journey sources `$JOURNEY_LIB`, which is `lib.sh`, and ends with
`verdict`. Every word it is written in is defined in `lib.sh`, with what it
does above it.

- **The story comes first.** The comment at the top says who is doing what,
  and which help the claims come from. That is what makes the assertions
  readable when the journey fails a year later.
- **An assertion is a claim.** Write its message as the claim, so that a
  `FAIL` line reads as the thing that is untrue. Claim what the help says,
  not what the binary happens to print.
- **An assertion records and carries on.** One run reports every claim that
  is untrue, not only the first.
- **`pp` is a step that should work, and `refuses` one that should be
  refused.** Each fails the journey when the other happens. `ask` runs a
  step and judges nothing.

A change to what a user of `pile` sees comes with a new journey, or a
changed one, in the same commit. A journey written for a defect fails until
the fix lands: watch it fail for the reason it states, then let the fix turn
it green.
