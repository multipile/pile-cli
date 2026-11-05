#!/usr/bin/env bash
# Somebody about to write a file asks which pile it will be in, from wherever
# they are standing, before the file exists.
#
# Piles nest, so the path alone does not say which pile a file is in. which
# answers from the declarations. A path is a path as the shell means it, and
# need not exist yet; a :/ path is from the top of the repository from
# anywhere; and nothing is read as a form it was not typed in.
# (pile --help, pile which --help)
. "$JOURNEY_LIB"

repo
write docs/guide.md '# Guide
'
write src/main.rs 'fn main() {}
'
write src/engine/core.rs 'pub fn run() {}
'
within src pp init --set name=app
within src/engine pp init --set name=engine
explain src 'The application.'
explain src/engine 'The engine the application drives.'
commit "Declare the application and its engine"

say "I. a file that exists, and one that does not yet"

pp which src/main.rs src/engine/core.rs src/engine/next.rs
says "@app  :/src  " "a file is in the nearest pile above it"
says "   paths     :/src/main.rs" "every path is printed from the top, as :/"
says "   paths     :/src/engine/core.rs
             :/src/engine/next.rs" "a file nobody has written yet is answered like one that exists"
says "   intent    The engine the application drives." "with the headline of the pile's intent"
says "   whole     pile show @engine" "and the command that prints the pile whole"

say "II. a path no pile is above"

pp which docs/guide.md
says "in no pile" "a path with no pile at or above it is in no pile"
says ":/docs/guide.md" "named from the top of the repository"

say "III. from a subdirectory, as somebody standing there types it"

within src/engine pp which --path core.rs ../main.rs ../../docs/guide.md
says ":/src/engine
:/src" "--path prints each pile's path alone, a line each, in the order asked"
within src/engine pp which --path core.rs ../main.rs ../../docs/guide.md --format json
field -c .result.paths '[":/src/engine",":/src",null]' "and a path in no pile is null in its result"

say "IV. a :/ path is from the top, from anywhere"

within src/engine pp which --path :/src/engine/later.rs :/src/main.rs :/ --format json
field -c .result.paths '[":/src/engine",":/src",null]' ":/ names a path from the top of the repository, wherever pile runs"
pp which --path "$PWD/src/engine/later.rs"
same "$stdout" ":/src/engine" "an absolute path on disk inside the repository is that path"

say "V. a path is never read as what it looks like"

refuses which /src/engine/next.rs
exits 1 "/src/engine/next.rs is a path on disk, outside the repository, and is refused"
says "is outside the repository" "the refusal says so"
says "A path from the top of the repository is written :/path" "and says how to write one from the top"
lacks "@engine" "and proposes no pile, whatever the path looked like"

say "VI. a symbolic link is followed, as the shell follows it"

ln -s src/engine shortcut
pp which --path shortcut/core.rs
same "$stdout" ":/src/engine" "a path through a link is in the pile the link leads to"

say "VII. a path outside the repository refuses the whole question"

within src/engine refuses which core.rs ../../../elsewhere
exits 1 "a path that ends up outside the repository is refused"
lacks "@engine" "and nothing is answered for the other path, so a script gets one answer"

verdict
