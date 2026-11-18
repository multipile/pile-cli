#!/usr/bin/env bash
# A shop keeps a diagram of what relies on its ledger. Its author wants it
# to stay true, so the diagram says what it should show: every pile with a
# relies-on link to the ledger. They try the program with pile query, put
# it in the diagram, and mistype a name on the way, which doctor finds at
# its line and column. Later somebody adds an admin service that relies on
# the ledger, and never looks at the diagram: doctor fails until the
# diagram draws the new pile or ignores it with a reason. The author
# ignores it, then draws it after all, and doctor notes the ignore that is
# no longer needed.
#
# A box stands for a pile by pile:id and a line for a link by pile:edge.
# In meta pile, include takes a Datalog program, as pile query takes one:
# under boxes each query binds Node, and under lines Edge. ignore leaves
# one node out, and says why. A node the diagram should show and does not
# is a problem at the include, and does not stop it being drawn. (pile
# guide diagrams, pile guide diagram-includes, pile doctor --help, pile
# diagram list --help)
. "$JOURNEY_LIB"

repo
within . pp init --set name=shop
explain . 'The whole shop.'
for p in ledger billing reports; do
	mkdir -p services/$p
	within services/$p pp init --set name=$p
	explain services/$p "The $p service."
done
mkdir -p piles
within piles pp init --set kind=auxiliary-gitpile --set name=words
explain piles 'The words this repository talks about its piles in.'
write piles/.pile/types.toml 'version = "gitpile/1"

[[edge-types]]
name = "relies-on"
title = "one cannot work without the other"
source = ["pile"]
target = ["pile"]
'
relies='version = "gitpile/1"

[[edges]]
type = "relies-on"
target = "@ledger"
title = "writes every entry through"
'
write services/billing/.pile/links.toml "$relies"
write services/reports/.pile/links.toml "$relies"
commit "Declare the shop, its services and what relies on the ledger"

# The id of a pile, as pile list --full prints it.
id() { "$PILE" list --full --format json | jq -r --arg n "$1" '.result.piles[] | select(.name == $n) | .id'; }

# The diagram, with these boxes and lines after the three it starts with,
# and this beside its include.
diagram() { # <more boxes and lines> <beside the include> [<the pile the include names>]
	write .pile/diagrams/ledger.diagram "diagram dependency version=1 {
  title \"What relies on the ledger\"
  desc \"Billing and the reports both write through the ledger.\"
  box billing pile:id=\"$(id billing)\"
  box reports pile:id=\"$(id reports)\"
  box ledger pile:id=\"$(id ledger)\" focal=#true
  line billing ledger pile:edge=\"relies-on\"
  line reports ledger pile:edge=\"relies-on\"
$1  meta pile {
    boxes {
      include #\"?- edge(_, Node, \"relies-on\", @${3:-ledger}).\"#
$2    }
    lines {
      include #\"?- edge(Edge, _, \"relies-on\", _).\"#
    }
  }
}
"
}

export AGENT=its-harness

say "I. the author tries the program, and puts it in the diagram"

pp query '?- edge(_, Node, "relies-on", @ledger).'
field -c '.result.answers[0].rows' '[["@billing"],["@reports"]]' "the program finds the two piles that rely on the ledger"

diagram '' '' leger
refuses doctor --diagrams
field '.result.diagrams[0].problems[0].code' diagram-include-invalid "a program that names no pile is refused in the diagram as pile query refuses it"
field -c '.result.diagrams[0].problems[0] | [.line, .column]' '[11,47]' "at the line and the column of the name, in the diagram's file"
says "no pile is named @leger" "with what pile query says of it"

diagram '' ''
pp doctor --diagrams
field '.result.total.problems' 0 "with the name right, the diagram shows everything it should"
commit "Say what the ledger diagram should show"

say "II. somebody adds a service that relies on the ledger"

mkdir -p services/admin
within services/admin pp init --set name=admin
explain services/admin 'The back office.'
write services/admin/.pile/links.toml "$relies"
commit "Add the back office"

refuses doctor
field .error.code problems-found "doctor fails, though nobody touched the diagram"
refuses doctor --diagrams
field '.result.diagrams[0].problems[0].code' diagram-misses-node "the diagram should show a node no box draws"
field '.result.diagrams[0].problems[0].line' 11 "the finding is at the include that found it"
says "@admin" "and names the pile"
says "ignore \\\"admin\\\" id=\\\"$(id admin)\\\"" "its fix writes out the ignore, beside the box to draw"

pp diagram list --since HEAD~1
field '.result.diagrams[0].change' null "the change touched nothing the diagram draws"
field -c '.result.diagrams[0].misses' '["@admin"]' "and the list says what the diagram misses, for a review to show"
pp diagram render ledger
field .result.out ledger.html "a diagram that misses a node is still drawn"

say "III. the author leaves it out, and has to say why"

diagram '' "      ignore \"admin\" id=\"$(id admin)\"
"
refuses doctor --diagrams
field '.result.diagrams[0].problems[0].code' diagram-ignore-no-reason "an ignore with no reason is a problem"

diagram '' "      ignore \"admin\" id=\"$(id admin)\" reason=\"the back office gets a diagram of its own\"
"
pp doctor
field '.result.total.problems' 0 "with a reason, doctor passes"
pp diagram list --since HEAD~1
field -c '.result.diagrams[0].misses' '[]' "and the diagram misses nothing"
commit "Leave the back office out of the ledger diagram"

say "IV. later the author draws it after all"

diagram "  box admin pile:id=\"$(id admin)\"
  line admin ledger pile:edge=\"relies-on\"
" "      ignore \"admin\" id=\"$(id admin)\" reason=\"the back office gets a diagram of its own\"
"
pp doctor --diagrams
field '.result.total.problems' 0 "the diagram is true, so doctor exits 0"
field '.result.diagrams[0].notes[0].code' diagram-ignore-unneeded "and notes the ignore of a node the diagram now draws"

diagram "  box admin pile:id=\"$(id admin)\"
  line admin ledger pile:edge=\"relies-on\"
" ''
pp doctor --diagrams
field '.result.total.notes' 0 "with the ignore removed, there is nothing to report"
pp diagram list --since HEAD
field '.result.diagrams[0].change' edited "and a review shows the diagram as edited, to draw before and after"

verdict
