#!/usr/bin/env bash
# Somebody halfway through a change asks what it touched, pile by pile, and
# after the commit asks the pile's history the same question.
#
# status is git's answer arranged by pile: a pile nothing touched is named,
# a file moved from one pile to another shows in both, and a change in no
# pile is kept. log is one pile's own commits, less the piles it holds, with
# what else each commit changed. (pile status --help, pile log --help)
. "$JOURNEY_LIB"

repo
write server/main.rs 'fn main() {}
'
write server/api/routes.rs 'pub fn routes() {}
'
write client/view.ts 'export const view = 1
'
write notes.txt 'loose notes
'
mkdir -p server && within server pp init --set name=server
mkdir -p server/api && within server/api pp init --set name=api
mkdir -p client && within client pp init --set name=client
explain server 'The server.'
explain server/api 'The routes the server answers.'
explain client 'The client.'
commit "Declare the server, its api, and the client"

say "I. a change in two piles, a move between piles, a new file"

write server/main.rs 'fn main() { serve() }
'
git mv server/api/routes.rs client/routes.rs
write client/new.ts 'export {}
'
write notes.txt 'more notes
'

pp status --all
says "@server  :/server  " "every pile is a heading"
says "   M  :/server/main.rs" "a change is under the pile it is in, by its path from the top"
says "   D  :/server/api/routes.rs  (left this pile, now :/client/routes.rs)" "a move out of a pile is a deletion there, saying where it went"
says "   A  :/client/routes.rs  (arrived from :/server/api/routes.rs)" "and an addition where it arrived, saying where from"
says "   ?  :/client/new.ts" "an untracked file is ?"
says "in no pile
   M  :/notes.txt" "a change no pile holds is kept, under in no pile"
says "3 piles, 3 changed, 4 paths, 2 paths crossed a boundary, 1 path in no pile" "and the last line counts all of it"

pp status --all --brief
says "@api     :/server/api  " "--brief is a line per pile"
says "1 path changed, 1 path crossed a boundary" "with what changed in it"

say "II. from inside a pile, status is that pile and the piles it holds"

within server pp status --format json
field '.result.piles | map(.path) | join(" ")' ":/server :/server/api" "standing in the server reports it and its api, and not the client"
field '.result.in_no_pile | length' 0 "and leaves out a change in no pile, which is outside where it stands"

commit "Serve, and give the routes to the client"
pp status --all
says "3 piles, none changed" "after the commit, nothing is changed"
says "@client  :/client  " "and every pile is still named"

say "III. the history of one pile"

write server/api/health.rs 'pub fn health() {}
'
commit "Add a health route"

pp log @server
says "@server  :/server  " "log heads the history with the pile"
says "2 commits to it" "and counts the commits to its own files"
lacks "Add a health route" "a commit only to a pile it holds is not its own"
says "also   @client :/client  @api :/server/api  in no pile" "each commit says what else it changed, the piles in path order"
says "intent written here" "and whether it wrote the pile's intent"
says "2 commits, 0 to this pile alone; also :/client 2, :/server/api 2, in no pile 2" "the counts say what the listed commits changed together"
says "by A Person 2" "and who made them"

pp log -n 1 @server --format json
field '.result.commits | length' 1 "-n limits the commits listed"
field .result.total 2 "and the total still counts every one"
refuses log -n -1 @server
exits 2 "a count that is not one is a mistake in what was typed"
says "pile log -n 0" "and the refusal names the count for every commit"

verdict
