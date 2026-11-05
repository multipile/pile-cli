#!/usr/bin/env bash
# Somebody about to commit runs pile doctor, reads what is wrong with the
# piles, puts each thing right, and runs it again until it exits 0.
#
# doctor is the one command with a verdict. It reads the whole repository,
# reports a config every other command refuses over instead of refusing
# itself, and names the file of each finding, with its line and column
# where it has them, as :/path:line:column, and what fixes it. A problem
# exits 1. A warning and a note do not. (pile doctor --help)
. "$JOURNEY_LIB"

repo
write src/main.rs 'fn main() {}
'
within src pp init --set name=app
explain src 'The application.'
commit "Declare the application"

say "I. three things go wrong on the way to a commit"

# A new pile, its intent never answered.
mkdir -p src/parser
within src/parser pp init --set name=parser
# A subtree copied whole, config and all.
cp -r src/parser src/parser-copy
# A config somebody edited by hand, with a key a config does not hold.
printf 'colour = red\n' >>src/.pile/config

refuses list
says ":/src/.pile/config: line 7 names \"colour\"" "list refuses while a config cannot be read, naming it"

say "II. doctor reports all of it, and refuses nothing"

refuses doctor
exits 1 "a problem makes doctor exit 1"
says ":/src  (config refused)" "the pile whose config cannot be read is still in the report"
says ":/src/.pile/config:7: the config cannot be read: line 7 names \"colour\"" "naming its file and its line"
says "a config holds only kind, version, id and name" "and which keys a config holds"
says ":/src/parser/.pile/intent.md: the intent has no headline" "an intent nobody answered is a problem"
says "pile guide write-an-intent says how" "and its fix names the workflow that answers it"
says "its id" "a copied subtree's id is claimed twice"
says "is claimed by :/src/parser-copy too" "and the copy is named"
says "pile: 7 problems found, so doctor exits 1" "the last line says why it failed: one in the config, three in the parser, three in its copy"

ask doctor --format json
field .error.code problems-found "in json, the refusal is problems-found"
field '.next[0].command' "pile guide doctor" "and it names the page on what each severity means"
field '.result.total.problems' 7 "and the whole report is still the result"
field '[.result.piles[] | select(.path == ":/src/parser") | .problems[].code] | join(" ")' \
	"intent-unanswered id-claimed-twice name-claimed-twice" "each finding has a code"
field '[.result.piles[] | select(.path == ":/src") | .problems[0] | .line, .column] | map(tostring) | join(" ")' \
	"7 null" "a config's finding has its line, and no column"

say "III. each one is put right"

sed -i.was '/^colour = /d' src/.pile/config && rm src/.pile/config.was
explain src/parser 'The grammar, and the parser built from it.'
rm -r src/parser-copy

pp doctor
exits 0 "with every problem fixed, doctor exits 0"
says "will not have this pile until its config is committed" "and says the new pile is not committed yet"
says "2 piles, nothing wrong" "and that nothing else is wrong"
commit "Declare the parser"
pp doctor
says "2 piles, nothing wrong" "after the commit, there is nothing to say"

say "IV. a warning and a note do not fail the verdict"

printf '# :/src\n\nThe application.\n\nRead this pile to find out what it holds. Change it when that does.\n' \
	>src/.pile/intent.md
write src/.pile/notes.md '# kept by another program
'
pp doctor
exits 0 "a leftover heading and a file pile does not read leave doctor at 0"
says "warning   :/src/.pile/intent.md:1:1: the intent opens with a heading" "the heading is a warning, at its line and column"
says "Remove the heading line" "with what fixes it"
says "note      :/src/.pile/notes.md: pile does not read this file" "the file nothing reads is a note"

verdict
