#!/usr/bin/env bash
# A repository with an aux pile gets a round of edits to its .toml files,
# each with a mistake in it. Before the commit, doctor names every one, at
# its file, line and column, and says what fixes it. Each is fixed, and
# doctor runs clean.
#
# doctor checks every table of every .toml file in a .pile/, and each
# finding names its file, line and column as :/path:line:column. A problem
# exits 1. A link is checked too, and once every mistake is fixed it reads
# as an edge. Every other command reads on past a file that does not check.
# (pile doctor --help, pile guide declarations, pile guide types,
# pile guide links)
. "$JOURNEY_LIB"

repo
within . pp init --set name=top
explain . 'The whole repository.'
mkdir -p lib piles
within lib pp init --set name=lib
explain lib 'The parser.'
within piles pp init --set kind=auxiliary-gitpile --set name=words
explain piles 'The words this repository talks about its piles in.'
write piles/.pile/types.toml 'version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository that a pile relies on"

[[edge-types]]
name = "uses"
title = "one pile calls the code of another"
source = ["pile"]
target = ["pile", "external"]
'
commit "Declare the piles and the aux pile"

say "I. a round of edits, each with a mistake"

# A name, which a pile's config gives, in [pile].
write lib/.pile/pile.toml 'version = "gitpile/1"
[pile]
name = "parser"
'
# A node with no name, and a node of a type nobody creates.
write .pile/nodes.toml 'version = "gitpile/1"
[[nodes]]
type = "external"

[[nodes]]
type = "service"
name = "ui"
'
# A type, in a pile that is not an aux pile.
write lib/.pile/types.toml 'version = "gitpile/1"
[[node-types]]
name = "grammar"
'
# An edge type that says its source, and not its target.
write piles/.pile/edges.toml 'version = "gitpile/1"
[[edge-types]]
name = "mirrors"
source = ["external"]
'
# A file that is not TOML, and one that does not open with its version.
write .pile/more.toml 'version = "gitpile/1"
[[nodes]
'
write lib/.pile/old.toml '[pile]
title = "the parser"
'
# The aux pile's own title, written in two files.
write piles/.pile/pile.toml 'version = "gitpile/1"
[pile]
title = "the words"
'
write piles/.pile/also.toml 'version = "gitpile/1"
[pile]
title = "our words"
'
# A link whose target is a path, which is no reference.
write lib/.pile/links.toml 'version = "gitpile/1"
[[edges]]
type = "uses"
target = ":/piles"
'

pp list
says "@lib" "list reads on past every one of them"

say "II. doctor names each, at its line and column"

refuses doctor
exits 1 "doctor exits 1"
says ":/lib/.pile/pile.toml:3:1: a pile's name comes from its config, and [pile] does not give it" \
	"a name in [pile], at the key"
says ":/.pile/nodes.toml:2:1: this [[nodes]] table gives no name" "a node with no name, at the table's header"
says ":/.pile/nodes.toml:6:8: the type service is created by no aux pile" "a type nobody creates, at its name"
says ":/lib/.pile/types.toml:2:1: a type is created only in an aux pile" "a type outside an aux pile, at its header"
says ":/piles/.pile/edges.toml:2:1: this [[edge-types]] table gives no target" "an edge type with no target"
says ":/.pile/more.toml:2:9: the file is not TOML that reads" "a file that is not TOML, where the parser stopped"
says ":/lib/.pile/old.toml: the file does not open with its version, so it declares nothing" \
	"a file with no version, which declares nothing"
says ":/piles/.pile/also.toml:3:1: [pile] title is written here and at :/piles/.pile/pile.toml:3:1" \
	"a key written twice, at each place, naming the other"
says "so neither is read" "and neither copy is read"
says ":/lib/.pile/links.toml:4:10: target is \":/piles\", and a reference is \"@name\"" \
	"a path where a reference is due, at the value"
says "10 problems" "ten problems, each counted"
ask doctor --format json
field '[.result.piles[] | .problems[] | .code] | sort | join(" ")' \
	"declared-twice declared-twice missing-key missing-key no-version reserved-name types-outside-aux unknown-type unreadable-declaration wrong-value" \
	"in json, each problem has its code"
field '[.result.piles[] | .problems[] | select(.code == "wrong-value") | .line, .column] | map(tostring) | join(":")' \
	"4:10" "and its line and column"

say "III. each is fixed, and a link reads"

write lib/.pile/pile.toml 'version = "gitpile/1"
[pile]
title = "the parser"
'
write .pile/nodes.toml 'version = "gitpile/1"
[[nodes]]
type = "external"
name = "git"
title = "the git binary every read of the repository runs"
'
rm lib/.pile/types.toml .pile/more.toml lib/.pile/old.toml piles/.pile/also.toml piles/.pile/edges.toml
write lib/.pile/links.toml 'version = "gitpile/1"
[[edges]]
type = "uses"
target = "external:git"
title = "asks every question about a repository of"
'
pp doctor
exits 0 "with every mistake fixed, doctor exits 0"
says "3 piles, nothing wrong" "and nothing else is wrong, the link included"
pp show @lib
says "links     -> uses  external:git  asks every question about a repository of" \
	"the link reads as an edge, which show prints"

verdict
