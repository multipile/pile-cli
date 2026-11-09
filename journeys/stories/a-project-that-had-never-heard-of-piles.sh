#!/usr/bin/env bash
# A project with years of history and no piles is given its first ones.
#
# Somebody reads the history, decides the project is two things, and
# declares them, standing in each: the whole of it, and the parser inside
# it. Nothing moves, no history is rewritten, and what pile writes is a
# .pile/ directory per pile, to be committed like any other change. (pile init --help, pile set --help,
# pile list --help, pile show --help)
. "$JOURNEY_LIB"

repo
write README 'The project.
'
write parser/grammar.txt 'expr = term
'
commit "Start the project"
write parser/grammar.txt 'expr = term { "+" term }
'
commit "Add sums to the grammar"

say "I. a repository with no pile says so"

pp list
says "no pile in " "list says there is no pile, and exits 0, because listing judges nothing"
refuses status
exits 1 "status needs a pile to report by"
says "pile: no pile in " "and says there is none"
says "pile init" "and names the command that declares one"
says "pile guide adopt" "and the workflow that brings piles to a project"

say "II. the top is declared, and asked why it exists"

pp init
says "pile      :/" "init declares the directory it runs in, the top, written :/"
says "id        " "with an id it minted"
says "it has no name yet" "and says the pile has no name"
same "$(sed -n 's/^kind = //p' .pile/config)" gitpile "the config says it is a pile's"
same "$(head -1 .pile/intent.md)" "<!--" "the intent init writes has no heading, and starts with the question it asks"
says ".pile/intent.md is a question so far" "and init says the intent is a question, not an answer"
says "pile guide write-an-intent says how to answer it" "and names the workflow that answers it"

pp show
says ":/.pile/intent.md" "show prints the intent init wrote"
says "written   not committed yet" "and that no commit holds it yet"

pp set name project
says "@project  :/  " "set says which pile answers to the name now"
says ".pile/config is changed, and should be committed" "and that the change is to commit"

say "III. the parser is declared inside it, with its name"

mkdir -p parser && within parser pp init --set name=parser
says "name      @parser" "init gives the name it was asked to"
says "inside    :/, whose paths under :/parser are now this pile's" "and says the parser's paths moved from the top to it"
within parser refuses init
exits 1 "a directory that is a pile already is refused"
says ":/parser is a pile already" "and says so"
refuses init parser
exits 2 "init takes no path, because it declares only the directory it runs in"
says "cd to the directory first" "and the refusal says what to do instead"

explain . 'The project, released as one thing.'
explain parser 'The grammar and the parser built from it.'
commit "Declare the project and its parser"

say "IV. the two piles, as list and show see them"

pp list
says "@project  :/       " "list prints the name, then the path from the top, in a column each"
says "@parser   :/parser " "for every pile"
says "2 piles" "and counts them"

pp show @parser
says "@parser  :/parser  " "show is typed at the name"
says "   parent    @project  :/  " "and says the pile it sits in"
says "   children  none" "and that it holds none"
says ", nothing to this pile since" "and that nothing changed the pile since its intent was written"
says "The grammar and the parser built from it." "and prints the intent whole"

pp show
says "   children  @parser  parser  " "the top holds the parser, mounted at parser"

pp doctor
says "2 piles, nothing wrong" "doctor finds nothing wrong with the piles just declared"

verdict
