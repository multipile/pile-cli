#!/usr/bin/env bash
# An agent asked to explain how a project's piles fit together writes a
# diagram in the top pile's .pile/diagrams/, mistypes a pile's name, lets
# doctor find it, fixes it, and draws the diagram as a page, a picture and
# an animation.
#
# A diagram says what it shows and never where, and a box stands for a pile
# by pile="@name". doctor reads every diagram, and doctor --diagrams reads
# only the diagrams; each finding is a problem at its line and column.
# diagram render draws one: --as says what as, and without it -o's ending
# does. A diagram doctor finds a problem in is not drawn. (pile guide
# diagrams, pile doctor --help, pile diagram render --help)
. "$JOURNEY_LIB"

repo
pp init --set name=app
explain . 'The application.'
mkdir -p lib && within lib pp init --set name=lib
explain lib 'The library the application is built on.'
commit "Declare the application and its library"

export AGENT=its-harness

say "I. the agent writes a diagram, and mistypes a pile"

write .pile/diagrams/layers.diagram 'diagram dependency version=1 {
  title "What the application is built on"
  desc "The application depends on its library."
  box app pile="@app"
  box lib pile="@libary" focal=#true
  line app lib
}
'

refuses diagram render layers
field .error.code diagram-invalid "a diagram that names a pile the repository lacks is not drawn"
field '.next[0].command' "pile doctor --diagrams" "and the refusal points at doctor"

say "II. doctor finds it at its line"

refuses doctor --diagrams
field .error.code problems-found "doctor --diagrams exits 1 over a diagram's problem"
field '.result.diagrams[0].problems[0].code' diagram-no-such-pile "the finding is a pile no pile answers to"
field '.result.diagrams[0].problems[0].line' 5 "at the line of the box"
refuses doctor
field '.result.total.problems' 1 "the whole doctor reports it too"

say "III. it fixes the name, and draws the diagram"

sed -i.was 's/@libary/@lib/' .pile/diagrams/layers.diagram && rm .pile/diagrams/layers.diagram.was
pp doctor --diagrams
field '.result.total.diagrams' 1 "with the name fixed, doctor finds one diagram"
field '.result.total.problems' 0 "and nothing wrong with it"

pp diagram render layers
field .result.as html "without --as or -o, it is drawn as a page"
field .result.out layers.html "named after the diagram"
same "$(head -c 15 layers.html)" "<!DOCTYPE html>" "the page is on disk"

pp diagram render --as png layers
field .result.out layers.png "--as png draws a picture"
pp diagram render -o docs/layers.gif layers
field .result.as gif "-o's ending says gif when --as is not given"
field -c '.result.duration_ms > 0' true "and the animation moves"

say "IV. the person beside it reads text"

AGENT= pp diagram render --format text --as png layers
says "wrote     layers.png" "a person reads which file was written"
commit "Explain how the application is built"

verdict
