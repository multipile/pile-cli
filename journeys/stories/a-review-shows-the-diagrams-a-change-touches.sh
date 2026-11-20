#!/usr/bin/env bash
# A repository keeps two diagrams. On a branch, a change edits one of them,
# changes the code of a pile the other shows, and says more in the link the
# other draws as a line. Reviewing the branch, an agent asks which diagrams
# the change touches, and draws each with its change: the edited one as its
# two versions in one, the other with the changed pile's box and the
# changed link's line marked.
#
# --since counts a change from a revision to the working copy, and main...
# from where the branch left main. A diagram is added, edited, removed, or
# touched through what it stands for: a box stands for a pile by pile:id,
# and a line for a link by pile:edge. A pile is touched by its files, and a
# link by what it declares. An edited diagram marks a box as changed when
# it is drawn differently or stands for another node, and never for how
# its file writes what it stands for. (pile guide diagrams, pile diagram
# list --help, pile diagram render --help)
. "$JOURNEY_LIB"

repo
pp init --set name=app
explain . 'The application.'
mkdir -p lib && within lib pp init --set name=lib
explain lib 'The library the application is built on.'
mkdir -p words && within words pp init --set kind=auxiliary-gitpile --set name=words
explain words 'The words this repository talks about its piles in.'
write words/.pile/types.toml 'version = "gitpile/1"

[[edge-types]]
name = "uses"
title = "one pile calls the code of another"
source = ["pile"]
target = ["pile"]
'
write .pile/links.toml 'version = "gitpile/1"

[[edges]]
type = "uses"
target = "@lib"
title = "is built on"
'
write lib/code.rs 'one
'
pp list --full --format json
app=$(jq -r '.result.piles[] | select(.name == "app") | .id' <<<"$stdout")
lib=$(jq -r '.result.piles[] | select(.name == "lib") | .id' <<<"$stdout")
words=$(jq -r '.result.piles[] | select(.name == "words") | .id' <<<"$stdout")
write .pile/diagrams/layers.diagram "diagram dependency version=1 {
  title \"What the application is built on\"
  desc \"The application depends on its library.\"
  box app pile:id=\"$app\"
  box lib pile:id=\"$lib\"
  line app lib pile:edge=\"uses\"
}
"
write .pile/diagrams/flow.diagram 'diagram dependency version=1 {
  title "How a request flows"
  desc "A request reaches the handler."
  box request "request"
  box handler "handler"
  line request handler
}
'
pp doctor --diagrams
says "2 diagrams, nothing wrong" "each box stands for a pile, and the line for the link between them"
commit "Declare the application, its library and two diagrams"

export AGENT=its-harness

say "I. a branch changes the library, the link to it, and a diagram"

git switch -q -c feature
write lib/code.rs 'two
'
write .pile/links.toml 'version = "gitpile/1"

[[edges]]
type = "uses"
target = "@lib"
title = "is built on, and asks for every stored answer"
'
write .pile/diagrams/flow.diagram 'diagram dependency version=1 {
  title "How a request flows"
  desc "A request reaches the handler, which asks the store."
  box request "request"
  box handler "handler"
  box store "store"
  line request handler
  line handler store
}
'
commit "Store what the handler is asked"

say "II. the agent asks which diagrams the change touches"

pp diagram list --since main...
field '.result.total.changed' 2 "both diagrams are touched"
field '.result.diagrams[0].name' flow "flow comes first, in path order"
field '.result.diagrams[0].change' edited "its file changed"
field '.result.diagrams[1].change' piles "layers did not, and it stands for what did"
field -c '.result.diagrams[1].piles' '["@app","@lib"]' "piles keeps the piles alone: the two whose files changed"
field -c '.result.diagrams[1].touched | index("@app uses @lib") != null' true "touched has the link too, as its source, its type and its target"
field -c '.result.diagrams[1].misses' '[]' "and it misses nothing it should show"

say "III. it draws each with its change"

pp diagram render --since main... -o flow.gif flow
field .result.change edited "the edited diagram"
field -c '.result.duration_ms > 0' true "moves from what it was to what it is"
pp diagram render --since main... --as png layers
field .result.change piles "the other is drawn with what the change touched marked"
pp diagram render --since main... layers
same "$(grep -c 'data-changed="3"' layers.html)" 1 "both boxes and the line between them are marked"
refuses diagram render --since main... --as gif layers
field .error.code diagram-still "and it is still, so it is not a gif"

say "IV. an edited diagram marks what stands for another node, and not what is only written differently"

write .pile/diagrams/layers.diagram "diagram dependency version=1 {
  title \"What the application is built on\"
  desc \"The application depends on its library.\"
  box app pile:id=\"$app\" pile:name=\"app\"
  box lib pile:id=\"$lib\" pile:name=\"lib\"
  line app lib
}
"
pp diagram render --since main... layers
field .result.change edited "the diagram now writes each pile's name beside its id, and its line stands for nothing"
same "$(grep -c 'data-changed="0"' layers.html)" 1 "each box stands for the pile it stood for, so nothing is marked"
write .pile/diagrams/layers.diagram "diagram dependency version=1 {
  title \"What the application is built on\"
  desc \"The application depends on its library.\"
  box app pile:id=\"$app\"
  box lib pile:id=\"$words\" pile:name=\"words\"
  line app lib
}
"
pp diagram render --since main... layers
same "$(grep -c 'data-changed="1"' layers.html)" 1 "the box drawn as it was now stands for another pile, and is marked"
pp diagram render --since main... -o layers.gif layers
field -c '.result.duration_ms > 0' true "and the edited diagram moves, as a gif"

verdict
