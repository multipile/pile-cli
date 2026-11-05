#!/usr/bin/env bash
# A pile deep in the tree is given a name to type, and then moved with
# git mv. The name and the id go with it; only the path goes stale.
#
# A pile has three spellings: an id nobody types, a name a person chose,
# and a path that is where it sits. The name and the id are in the pile's own
# config, so a move carries them, and every command typed at the name still
# finds the pile. When two piles claim one name, it names neither until
# somebody chooses again. (pile set --help, pile show --help, pile log --help)
. "$JOURNEY_LIB"

repo
write packages/parser/lib/source.txt 'the source
'
mkdir -p packages/parser/lib && within packages/parser/lib pp init
explain packages/parser/lib 'The parser library.'
# A line somebody wrote in the config, which a command setting one key must
# leave alone.
printf '\n# split out of the parser in the spring\n' >>packages/parser/lib/.pile/config
commit "Declare the parser library"
pp list --full --format json
id=$(jq -r '.result.piles[0].id' <<<"$stdout")

say "I. the pile is given a name, and nothing else in its config changes"

pp set name lib packages/parser/lib
says "@lib  :/packages/parser/lib  ${id:0:8}" "set says which pile answers to the name now"
same "$(git diff --numstat -- packages/parser/lib/.pile/config | cut -f1,2)" "1	0" \
	"setting a name adds one line and removes none"
says ".pile/config is changed, and should be committed" "and says to commit it"
pp set name lib @lib
says "is @lib already, and nothing was written" "setting what the line already says writes nothing"
commit "Name the parser library"

say "II. the name is typed wherever a pile is"

pp show lib
says "@lib  :/packages/parser/lib  ${id:0:8}  gitpile/1" "a bare word that a pile claims is that pile"
pp show @lib --format json
field .result.id "$id" "and @lib is the same pile, by its id"

say "III. the pile moves, and the path is what goes stale"

mkdir -p libraries
git mv packages/parser/lib libraries/parser
commit "Move the parser library up"

refuses show :/packages/parser/lib
says ":/packages/parser/lib is not a pile" "the path written down before the move names no pile now"
pp show @lib --format json
field .result.path :/libraries/parser "the name finds the pile where it sits now"
field .result.id "$id" "and it is the same pile, by its id"
field .result.written.since 0 "the written count starts again at the move, which wrote the intent at its new path"
pp log @lib
says "1 commit to it" "the pile's history starts at the move, as git log of a directory does"
same "$(grep -c '^# split out' libraries/parser/.pile/config)" 1 "the line somebody wrote moved with the config"

say "IV. two piles claim one name, and it names neither"

mkdir -p tools/lib
mkdir -p tools/lib && within tools/lib pp init --set name=tools
refuses set name lib @tools
exits 1 "a name another pile claims is not given"
says "@lib is claimed by :/libraries/parser already" "and the refusal names the pile that has it"
# A config edited by hand is how two piles come to claim one name.
sed -i.was 's/^name = tools$/name = lib/' tools/lib/.pile/config && rm tools/lib/.pile/config.was
pp list
says "@lib is claimed by 2 piles: :/libraries/parser :/tools/lib" "list names every pile that claims it, and still exits 0"
refuses show @lib
says "@lib is claimed by 2 piles" "the name answers for neither pile"
pp show :/tools/lib
says "@lib  :/tools/lib  " "and each still answers to its path"
pp set --forget name :/tools/lib
says "it is typed and shown by its path now, which is :/tools/lib." "taking the name away leaves the pile its path"
pp show @lib --format json
field .result.path :/libraries/parser "and the name names one pile again"

verdict
