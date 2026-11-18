#!/usr/bin/env bash
# A repository of services has links between its piles, and somebody is
# about to change the ledger. pile show says who relies on the ledger
# directly. It does not say who relies on it through another pile, or which
# services nothing relies on. So they ask, with pile query: first every
# pile, then the links to the ledger, then a rule of their own that follows
# the links as far as they go. They mistype a name and a relation, and each
# refusal says the line and the column. Last they keep the program in a
# file, and a script reads its answer as JSON.
#
# A program reads the relations of what the repository declares: node,
# prop, edge and the rest, each always written with all its columns. @name
# is the pile of that name. A query is answered with a column for each
# variable it binds, and a row for each answer. A rule is the program's
# own. A program that is refused answers nothing, and says where its
# mistake is. (pile query --help, pile guide query)
. "$JOURNEY_LIB"

repo
within . pp init --set name=shop
explain . 'The whole shop.'
for p in ledger billing checkout reports; do
	mkdir -p services/$p
	within services/$p pp init --set name=$p
	explain services/$p "The $p service."
done
mkdir -p piles
within piles pp init --set kind=auxiliary-gitpile --set name=words
explain piles 'The words this repository talks about its piles in.'
write piles/.pile/types.toml 'version = "gitpile/1"

[[node-types]]
name = "external"
title = "something outside the repository that a pile relies on"

[[edge-types]]
name = "relies-on"
title = "one cannot work without the other"
source = ["pile"]
target = ["pile", "external"]
'
write services/ledger/.pile/nodes.toml 'version = "gitpile/1"

[[nodes]]
type = "external"
name = "postgres"
title = "the database every entry is kept in"

[[edges]]
type = "relies-on"
target = "external:postgres"
title = "keeps every entry in"
port = 5432

[pile]
labels = ["service", "money"]
'
write services/billing/.pile/links.toml 'version = "gitpile/1"

[[edges]]
type = "relies-on"
target = "@ledger"
title = "records every charge in"

[pile]
labels = ["service", "money"]
'
write services/checkout/.pile/links.toml 'version = "gitpile/1"

[[edges]]
type = "relies-on"
target = "@billing"
title = "charges each order through"

[pile]
labels = ["service"]
'
write services/reports/.pile/pile.toml 'version = "gitpile/1"

[pile]
labels = ["service"]
'
pp doctor
exits 0 "doctor finds nothing wrong with what is declared"
commit "Declare the services, and what each relies on"

say "I. every pile, and what relies on the ledger"

pp query '?- node(Node, pile, _).'
says '?- node(Node, pile, _).' "the query is printed as it was read"
says "   Node" "then the variable it binds, as the column"
says "   @ledger" "a pile is written as a link writes it"
says "   6 rows" "and the rows are counted"

pp query '?- edge(_, Node, "relies-on", @ledger).'
says "   @billing" "@ledger in the program is the pile of that name"
says "   1 row" "one pile relies on the ledger directly"
lacks "@checkout" "and checkout, which relies on billing, is not among them"

say "II. a rule of their own follows the links as far as they go"

pp query 'needs(A, B) :- edge(_, A, "relies-on", B).
needs(A, C) :- needs(A, B), edge(_, B, "relies-on", C).
?- needs(Node, @ledger).
?- prop(Node, labels, service), !needs(_, Node).
?- needs(@checkout, @ledger).
?- needs(@ledger, @checkout).'
says "   @billing
   @checkout
   2 rows" "the first query finds checkout too, through billing"
says "?- prop(Node, labels, service), !needs(_, Node).
   Node
   @checkout
   @reports
   2 rows" "the second finds the services nothing relies on, with a negation"
says "?- needs(@checkout, @ledger).
   yes" "a query that binds no variable is answered yes"
says "?- needs(@ledger, @checkout).
   no" "or no"

refuses query '?- needs(Node, @ledger).'
says "needs is no relation" "the rule was that program's alone: the next one does not have it"

say "III. a node that is not a pile, an edge, and an integer"

pp query '?- edge(Edge, Source, Type, Target), node(Target, external, Home).'
says "   Edge                                 Source   Type       Target             Home" \
	"a column for each variable, in the order they are first written"
says "   @ledger relies-on external:postgres  @ledger  relies-on  external:postgres  @ledger" \
	"an edge is one value: its source, its type and its target"

pp query '?- edge_prop(Edge, port, Port), Port > 1024, at(Edge, File, Line, _).'
says "@ledger relies-on external:postgres  5432  :/services/ledger/.pile/nodes.toml  8" \
	"an integer compares as a number, and at says where the link is written"

say "IV. a mistake is refused where it is"

refuses query '?- edge(_, Node, "relies-on", @leger).'
exits 1 "a name no pile claims exits 1"
says "pile: the program is refused at 1:31: no pile is named @leger" \
	"and the refusal says the line and the column of the name"
says "pile guide query" "and where a program is explained"

refuses query '?- node(Node, pile, _),
   edges(_, Node, _, _).'
says "refused at 2:4: edges is no relation" "a relation there is none of, on the second line"

refuses query '?- edge(Node, "relies-on", @ledger).'
says "refused at 1:4: edge has 4 columns and is written here with 3 columns" \
	"a relation is written with all its columns"

refuses query '?- node(Node pile, _).'
says "refused at 1:14" "what does not read as a program is refused where reading stopped"

refuses query
exits 2 "no program at all is a usage refusal"

say "V. the program is kept in a file, and a script reads the answer"

write queries/around-the-ledger.dl '% Everything that stops working when the ledger does.
needs(A, B) :- edge(_, A, "relies-on", B).
needs(A, C) :- needs(A, B), edge(_, B, "relies-on", C).
?- needs(Node, @ledger), prop(Node, path, Path).
'
pp query --file queries/around-the-ledger.dl
says "@billing   :/services/billing" "--file reads the program from the file"

ask query --file queries/around-the-ledger.dl --format json
field '.result.answers[0].query' '?- needs(Node, @ledger), prop(Node, path, Path).' \
	"in json, each answer has its query"
field -c '.result.answers[0].columns' '["Node","Path"]' "its columns"
field -c '.result.answers[0].rows' \
	'[["@billing",":/services/billing"],["@checkout",":/services/checkout"]]' \
	"and its rows, a value per column"

ask query '?- edge_prop(_, port, Port).' --format json
field -c '.result.answers[0].rows' '[[5432]]' "an integer is a number in json"

write queries/around-the-ledger.dl '?- needs(Node, @ledger).
'
ask query --file queries/around-the-ledger.dl --format json
exits 1 "a refused program exits 1 in json too"
field '.error.code' "program-refused" "with its code"
field -c '.result.refused' \
	'{"file":"queries/around-the-ledger.dl","line":1,"column":4,"message":"needs is no relation: nothing provides or defines it"}' \
	"and the file, line, column and message as data"

verdict
