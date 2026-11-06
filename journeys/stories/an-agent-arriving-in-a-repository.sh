#!/usr/bin/env bash
# An agent is dropped into a repository that declares piles, and finds its
# way around without being told a single flag.
#
# Its harness says it is an agent the way harnesses do, with AGENT in the
# environment. So a bare `pile` answers where it is instead of printing the
# help, every answer is one JSON envelope, and a refusal reaches it on stdout
# with a code to branch on and the command that fixes it. The person beside
# it, in the same shell with the variable emptied, still reads text.
# (pile --help, pile version --help, pile guide agent-mode)
. "$JOURNEY_LIB"

repo
write lib/index.ts 'export const answer = 42
'
pp init
explain . 'The project, released as one thing.'
mkdir -p lib && within lib pp init --set name=lib
explain lib 'The library, released on its own.'
pp set name app .
commit "Declare the project and its library"

export AGENT=its-harness

say "I. a bare pile says where the agent is"

ask
exits 0 "a bare pile is not a mistake to have typed"
field .ok true "it is one JSON envelope, without --format json"
field .result.version.agent AGENT "it says which binary answers, and which variable said an agent is asking"
field '.result.piles | map(.path) | join(" ")' ":/ :/lib" "it lists every pile, each by its path from the top"
field '.result.status.total.piles' 2 "and how much each pile changed, a line each"
field '.next[0].command' "pile guide" "and names the guide as where a session begins"
field '.next[1].command' "pile show <pile>" "then reading a pile whole"
field '.next[2].command' "pile doctor" "and doctor, which must exit 0 before a commit"
lacks "Usage: pile" "it is not the help, which answers a question it did not ask"

say "II. every answer is JSON, unless it asks for text"

ask show @lib
field .command "pile show @lib" "the envelope names the command as it was run"
field .result.path :/lib "the pile is the result"
field '.result.files[0].text | contains("released on its own")' true "with its intent whole"

ask show @lib --format text
says "@lib  :/lib  " "--format text is what a person reads"
lacks '"ok"' "with no envelope around it"

say "III. a refusal comes on stdout, with what fixes it"

refuses show @nope
exits 1 "a name nothing claims is refused, and exits 1"
same "$stderr" "" "nothing is written to stderr, where an agent is not reading"
field .error.code unknown-name "the refusal has a code to branch on"
field '.error.message | contains("@app @lib")' true "and lists the names that are claimed"

refuses show nowhere --format text
same "$stderr" "" "in text too, an agent's refusal is on stdout"
says "pile: :/nowhere is not a pile. It is in @app  :/  " "and it reads as it would for a person"

refuses lsit
exits 2 "a mistyped command exits 2"
field '.next[0].command' "pile --help" "and the envelope says what lists the commands"

say "IV. the person in the same shell"

AGENT='' ask
says "Usage: pile <command>" "an empty variable is a person, and a bare pile prints them the help"
says "pile guide adopt | claude" "and the help sends them to the guide too, by an agent"
AGENT='' ask list
says "@lib  :/lib" "and a person reads text"

verdict
