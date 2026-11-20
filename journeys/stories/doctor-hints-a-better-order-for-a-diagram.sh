#!/usr/bin/env bash
# Somebody draws how a blog's services talk to one another, as an
# architecture diagram in the top pile. The file reads, and breaks no rule.
# But the boxes of a tier are drawn in the order the file writes them, and
# the order they wrote makes lines meet. doctor says so, as a note at
# the line of the tier, with the order to write instead. The author draws
# the diagram as it is, then writes the order the hint says, and the note
# is gone.
#
# A hint is a note, diagram-hint: doctor still exits 0 with it, and the
# diagram is drawn as written. Its message ends with the code multipile's
# diagrams give the hint, and its fix is what to write. (pile guide
# diagrams, pile guide doctor, pile doctor --help)
. "$JOURNEY_LIB"

repo
pp init --set name=blog
explain . 'A blog: its pages, and the services behind them.'
commit "Declare the blog"

say "I. the author writes the diagram, in an order that makes lines meet"

write .pile/diagrams/services.diagram 'diagram architecture version=1 {
  title "How the parts of the blog talk to one another"
  desc "The pages ask the posts and the auth services, which keep their data in the database."
  flow right
  box ui "blog-ui"
  box posts "posts" focal=#true
  box auth "auth"
  box db "db" as=store
  box cache "cache" as=store
  tier ui
  tier posts auth
  tier db cache
  line ui posts label="posts"
  line ui auth label="sign in"
  line posts db label="read"
  line posts cache label="hot posts"
  line auth db label="sessions"
}
'

say "II. doctor passes it, and hints a better order"

pp doctor --diagrams --format json
field '.result.total.problems' 0 "a diagram that reads, drawn worse than it could be, has no problem"
field '.result.total.notes' 2 "each tier that another order would draw better is a note"
field '.result.diagrams[0].notes[1].code' diagram-hint "the note is a hint"
field '.result.diagrams[0].notes[1].line' 12 "at the line of the tier"
field '.result.diagrams[0].notes[1].column' 3 "and its column"
field '.result.diagrams[0].notes[1].fix' 'write `tier cache db`, and they meet nowhere' "and it says what to write instead"
says "(tier-order-crosses)" "the message ends with the code the hint has in multipile's diagrams"

pp doctor
says "nothing wrong: 2 notes" "the whole doctor exits 0 with the hints, and counts them as notes"
says ":/.pile/diagrams/services.diagram:12:3: in this order, lines meet" "a person reads the hint at its file, line and column"

say "III. a hint stops nothing: the diagram is drawn as written"

pp diagram render services --format json
field .result.out services.html "the diagram is drawn, in the order the file writes"
same "$(head -c 15 services.html)" "<!DOCTYPE html>" "and the page is on disk"

say "IV. the author writes the order the hint says"

sed -i.was 's/tier db cache/tier cache db/' .pile/diagrams/services.diagram && rm .pile/diagrams/services.diagram.was
pp doctor --diagrams --format json
field '.result.total.notes' 0 "with one tier in the hinted order, no lines meet, and no hint is left"
pp doctor
says "nothing wrong" "and doctor has nothing to report"
commit "Draw how the parts of the blog talk to one another"

verdict
