#!/usr/bin/env bash
# Somebody wants a word for the things outside the repository that a pile
# relies on, and labels on its piles. Types are not built in, so they make
# the repository's first aux pile, which creates the node type external. A
# pile gives its labels in [pile], and the top declares an external. show
# prints both, and doctor checks them.
#
# An aux pile is a pile whose config says kind = auxiliary-gitpile, made
# with pile init --set kind=auxiliary-gitpile. A type is not a node: show of
# an aux pile lists its types under types, and under an edge type its do and
# dont. show prints a pile's values and the nodes at home in it, each by its
# reference, type:name. A key of its own, such as labels, is any word with
# any value, and nothing checks it. There are two kinds of pile, gitpile
# and auxiliary-gitpile, and no other: a config that says another is
# refused. (pile init --help, pile show --help, pile guide aux, pile guide
# declarations)
. "$JOURNEY_LIB"

repo
within . pp init --set name=top
explain . 'The whole repository.'
mkdir -p lib
within lib pp init --set name=lib
explain lib 'The parser.'
commit "Declare the piles"

say "I. the aux pile is a pile of its own kind"

mkdir -p piles
within piles pp init --set kind=auxiliary-gitpile --set name=our-words
says "kind      auxiliary-gitpile, an aux pile, which may create types" "init says it made an aux pile"
same "$(grep '^kind' piles/.pile/config)" "kind = auxiliary-gitpile" "the config says kind = auxiliary-gitpile"
explain piles 'The words this repository talks about its piles in.'

say "II. it creates external, which is a type and not a node"

write piles/.pile/types.toml 'version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository that a pile relies on"

[[edge-types]]
name = "uses"
title = "one pile calls the code of another"
source = ["pile"]
target = ["pile", "external"]
do = "say what the source calls"
dont = "list functions"
'
pp show @our-words
says "types     node external  something outside the repository" \
	"show lists the types it creates under types"
says "edge uses      pile -> external,pile  one pile calls the code of another" \
	"an edge type with the node types each end may be"
says "do    say what the source calls" "and, under it, what a link's description has to name"
says "dont  list functions" "and what it must not"
lacks "nodes" "and no node is at home in it"

say "III. a pile gives labels, and the top declares an external"

write lib/.pile/pile.toml 'version = "gitpile/1"

[pile]
title = "the parser"
labels = ["core", "docs"]
'
write .pile/nodes.toml 'version = "gitpile/1"

[[nodes]]
type = "external"
name = "git"
title = "the git binary every read of the repository runs"
'
pp show @lib
says "labels    core, docs" "show prints the labels the pile gives"
says "title     the parser" "and the title it gives beside its intent"
ask show @lib --format json
field -c .result.values '{"labels":["core","docs"],"title":["the parser"]}' "and its result holds them"
pp show
says "nodes     external:git  the git binary every read" "the top's external is at home in it, by its reference"
ask show --format json
field '.result.nodes[0].at' ":/.pile/nodes.toml:3:1" "and its result says where its table starts"

say "IV. doctor checks all of it, and finds nothing wrong"

pp doctor
exits 0 "doctor exits 0"
says "3 piles, nothing wrong" "the aux pile is a pile like any other"
commit "Add the aux pile, labels and the git external"
pp doctor
says "3 piles, nothing wrong" "and after the commit there is nothing to say"

say "V. a pile is one of two kinds, and no other"

mkdir -p more
within more refuses init --set kind=auxiliary-gitpile-disabled
says "a new pile's kind is gitpile, or auxiliary-gitpile for an aux pile" \
	"init makes a pile of one of the two kinds, and no other"
sed -i 's/^kind = .*/kind = auxiliary-gitpile-disabled/' piles/.pile/config
refuses doctor
says 'and a pile is a "gitpile" or "auxiliary-gitpile"' \
	"doctor refuses a config that says any other kind, and names the two"

verdict
