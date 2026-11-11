#!/usr/bin/env bash
# A repository keeps two diagrams. On a branch, a change edits one of them
# and changes the code of a pile the other shows. Reviewing the branch, an
# agent asks which diagrams the change touches, and draws each with its
# change: the edited one as its two versions in one, the other with the
# changed pile's box marked.
#
# --since counts a change from a revision to the working copy, and main...
# from where the branch left main. A diagram is added, edited, removed, or
# touched through the piles it shows. (pile guide diagrams, pile diagram
# list --help, pile diagram render --help)
. "$JOURNEY_LIB"

repo
pp init --set name=app
explain . 'The application.'
mkdir -p lib && within lib pp init --set name=lib
explain lib 'The library the application is built on.'
write lib/code.rs 'one
'
write .pile/diagrams/layers.diagram 'diagram dependency version=1 {
  title "What the application is built on"
  desc "The application depends on its library."
  box app pile="@app"
  box lib pile="@lib"
  line app lib
}
'
write .pile/diagrams/flow.diagram 'diagram dependency version=1 {
  title "How a request flows"
  desc "A request reaches the handler."
  box request "request"
  box handler "handler"
  line request handler
}
'
commit "Declare the application, its library and two diagrams"

export AGENT=its-harness

say "I. a branch changes the library and edits a diagram"

git switch -q -c feature
write lib/code.rs 'two
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
field '.result.diagrams[1].change' piles "layers did not, and it shows a pile that did"
field '.result.diagrams[1].piles[0]' @lib "the library"

say "III. it draws each with its change"

pp diagram render --since main... -o flow.gif flow
field .result.change edited "the edited diagram"
field -c '.result.duration_ms > 0' true "moves from what it was to what it is"
pp diagram render --since main... --as png layers
field .result.change piles "the other is drawn with the library marked"
refuses diagram render --since main... --as gif layers
field .error.code diagram-still "and it is still, so it is not a gif"

verdict
