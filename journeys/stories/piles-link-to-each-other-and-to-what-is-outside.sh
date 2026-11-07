#!/usr/bin/env bash
# A repository's piles depend on each other, and on git, which is outside
# it. Somebody gives the repository words for that: its aux pile creates the
# edge types uses and exercises, and the node type external. Then each pile
# writes links, and gets three of them wrong. doctor names each mistake
# where it is written: a reference to a pile that has no such name, an end
# its edge type does not allow, and one edge written in two piles. Each is
# fixed, doctor runs clean, and show, which and list print the links: show
# each one whole, which the nodes at the other ends, and list a count each
# way.
#
# A link is an [[edges]] table, in any pile. It gives its source, its
# target, or both, and an end it leaves out is the pile that writes it. Its
# reference names a node by name, @name for a pile and type:name for any
# other node. An edge is written once. A pile's links include those that
# end at a node at home in it. (pile guide links, pile show --help,
# pile which --help, pile list --help, pile doctor --help)
. "$JOURNEY_LIB"

repo
within . pp init --set name=top
explain . 'The whole repository.'
for p in cli storage journeys; do
	mkdir -p $p
	within $p pp init --set name=$p
done
explain cli 'The command line.'
explain storage 'Every read of a repository.'
explain journeys 'Sessions with the binary, told as scripts.'
mkdir -p piles
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

[[edge-types]]
name = "exercises"
title = "one pile runs another whole, and says whether it works"
source = ["pile"]
target = ["pile"]
'
write .pile/nodes.toml 'version = "gitpile/1"

[[nodes]]
type = "external"
name = "git"
title = "the git binary every read of a repository runs"
'
commit "Declare the piles, and the words to link them with"

say "I. each pile writes its links, three of them wrong"

write cli/.pile/links.toml 'version = "gitpile/1"

[[edges]]
type = "uses"
target = "@store"
title = "reads every repository through"

[[edges]]
type = "exercises"
target = "external:git"
title = "asks every question about a repository of"
'
write journeys/.pile/links.toml 'version = "gitpile/1"

[[edges]]
type = "exercises"
target = "@cli"
title = "holds what each command prints to what its help claims"
'
write cli/.pile/more.toml 'version = "gitpile/1"

[[edges]]
type = "exercises"
source = "@journeys"
title = "is held to its help by"
'

say "II. doctor names each mistake where it is written"

refuses doctor
exits 1 "doctor exits 1"
says ":/cli/.pile/links.toml:5:10: no pile is named @store" \
	"a reference to a name no pile claims, at the reference"
says ":/cli/.pile/links.toml:10:10: the target of a link of type exercises is pile, and this one's is external" \
	"an end its edge type does not allow, at the reference"
says ":/cli/.pile/more.toml:3:1: the link @journeys exercises @cli is written here and at :/journeys/.pile/links.toml:3:1" \
	"one edge written in two piles, at each table, naming the other"
says "Remove all but one" "and what fixes it"
says "4 problems" "four problems, the edge written twice counting at each table"
ask doctor --format json
field '[.result.piles[] | .problems[] | .code] | sort | join(" ")' \
	"declared-twice declared-twice no-such-name wrong-end" \
	"in json, each problem has its code"
pp list
says "@cli" "list reads on past every mistake"

say "III. each is fixed, and doctor runs clean"

write cli/.pile/links.toml 'version = "gitpile/1"

[[edges]]
type = "uses"
target = "@storage"
title = "reads every repository through"

[[edges]]
type = "uses"
target = "external:git"
title = "asks every question about a repository of"
'
rm cli/.pile/more.toml
pp doctor
exits 0 "doctor exits 0"
says "5 piles, nothing wrong" "and finds nothing wrong"
commit "Link the piles"

say "IV. show prints each link whole, both ways"

pp show @cli
says "links     -> uses       @storage      reads every repository through" \
	"a link from this pile: its type, the node at the other end, its title"
says "-> uses       external:git  asks every question about a repository of" \
	"the other end may be outside the repository"
says "<- exercises  @journeys     holds what each command prints to what its help claims  (written in @journeys)" \
	"a link to this pile, saying where it is written"
ask show @cli --format json
field '.result.links[0] | "\(.direction) \(.type) \(.other) \(.at)"' \
	"to uses @storage :/cli/.pile/links.toml:3:1" \
	"in json, each link has its direction, type, other end and place"
field '[.next[].command] | join(", ")' "pile show @storage, pile show @journeys, pile show @words" \
	"and next names each pile at the other end, then the aux pile their types come from"
pp show @top
says "links     -- uses  @cli -> external:git  asks every question about a repository of  (written in @cli)" \
	"the top holds external:git, so the link to it is the top's too, with both ends shown"

say "V. which names the other ends, and list counts them"

pp which cli/src/main.rs
says "links     -> @storage, external:git" "which names the nodes the pile links to"
says "<- @journeys" "and the piles that link to it"
pp list
says "@cli       :/cli " "list prints every pile"
says "-> 2  <- 1" "and counts each pile's links, each way"
ask list --format json
field -c '[.result.piles[] | select(.name == "journeys") | .links[]]' \
	'[{"direction":"to","type":"exercises","other":"@cli","source":"@journeys","target":"@cli"}]' \
	"in json, each pile has its links, each end as it is typed"

verdict
