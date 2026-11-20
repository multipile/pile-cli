#!/usr/bin/env bash
# An agent asked to explain how a project's piles fit together writes a
# diagram in the top pile's .pile/diagrams/. It copies each pile's id from
# pile list --full, misnames a box, lets doctor find it, fixes it, and
# draws the diagram as a page, a picture and an animation. A diagram
# somebody wrote earlier, whose boxes stand for the same piles, is drawn too.
#
# A diagram says what it shows and never where. A box stands for a pile by
# pile:id, and its name is the pile's name. doctor reads every diagram, and
# doctor --diagrams reads only the diagrams; each finding is at its line
# and column. diagram render draws one: --as says what as, and without it
# -o's ending does. A diagram with a box that stands for nothing is not
# drawn. (pile guide diagrams, pile doctor --help, pile diagram render
# --help)
. "$JOURNEY_LIB"

repo
pp init --set name=app
explain . 'The application.'
mkdir -p lib && within lib pp init --set name=lib
explain lib 'The library the application is built on.'
pp list --full --format json
app=$(jq -r '.result.piles[] | select(.name == "app") | .id' <<<"$stdout")
lib=$(jq -r '.result.piles[] | select(.name == "lib") | .id' <<<"$stdout")
write .pile/diagrams/before.diagram "diagram dependency version=1 {
  title \"What there was before\"
  desc \"The application and its library, as somebody drew them earlier.\"
  box app pile:id=\"$app\"
  box lib pile:id=\"$lib\"
  line app lib
}
"
commit "Declare the application and its library"

export AGENT=its-harness

say "I. the agent copies each pile's id, writes a diagram, and misnames a box"

pp list --full
app=$(jq -r '.result.piles[] | select(.name == "app") | .id' <<<"$stdout")
lib=$(jq -r '.result.piles[] | select(.name == "lib") | .id' <<<"$stdout")
same "${#lib}" 32 "pile list --full gives each pile's whole id"
write .pile/diagrams/layers.diagram "diagram dependency version=1 {
  title \"What the application is built on\"
  desc \"The application depends on its library.\"
  box app pile:id=\"$app\"
  box library pile:id=\"$lib\" focal=#true
  line app library
}
"

refuses diagram render layers
field .error.code diagram-invalid "a diagram with a box that is not named as its pile is not drawn"
field '.next[0].command' "pile doctor --diagrams" "and the refusal points at doctor"

say "II. doctor finds it at its line"

refuses doctor --diagrams
field .error.code problems-found "doctor --diagrams exits 1 over a diagram's problem"
field '.result.diagrams[1].problems[0].code' diagram-name-differs "the box is not named as the pile its id finds"
field '.result.diagrams[1].problems[0].line' 5 "at the line of the box"
says "named lib" "and the finding says the pile's name"
refuses doctor
field '.result.total.problems' 1 "the whole doctor reports it too"

say "III. it fixes the name, and draws the diagram"

sed -i.was 's/library/lib/' .pile/diagrams/layers.diagram && rm .pile/diagrams/layers.diagram.was
pp doctor --diagrams
field '.result.total.diagrams' 2 "with the name fixed, doctor finds both diagrams"
field '.result.total.problems' 0 "and nothing wrong with either"

pp diagram render layers
field .result.as html "without --as or -o, it is drawn as a page"
field .result.out layers.html "named after the diagram"
same "$(head -c 15 layers.html)" "<!DOCTYPE html>" "the page is on disk"
same "$(grep -c "data-pile-id=\"$lib\"" layers.html)" 1 "and the box of the library carries its pile's id"

pp diagram render --as png layers
field .result.out layers.png "--as png draws a picture"
pp diagram render -o docs/layers.gif layers
field .result.as gif "-o's ending says gif when --as is not given"
field -c '.result.duration_ms > 0' true "and the animation moves"

say "IV. the diagram somebody drew earlier is drawn the same way"

pp diagram render before
field .result.out before.html "the earlier diagram is drawn, named after itself"

say "V. the person beside it reads text"

AGENT= pp diagram render --format text --as png layers
says "wrote     layers.png" "a person reads which file was written"
commit "Explain how the application is built"

verdict
