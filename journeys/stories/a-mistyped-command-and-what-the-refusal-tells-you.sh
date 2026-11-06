#!/usr/bin/env bash
# Somebody mistypes, and reads the refusal to find out what to type instead.
#
# A mistake in what was typed exits 2, and anything that went wrong in the
# repository exits 1, so a script can tell the two apart. Every refusal says
# what fixes it: the command's help, the commands there are, or the names
# there are. (pile --help, pile help --help, pile guide --help)
. "$JOURNEY_LIB"

repo
pp init --set name=core
explain . 'The core.'
commit "Declare the core"

say "I. a command that is not one"

refuses lsit
exits 2 "a mistyped command exits 2"
says "pile: \"lsit\" is not a command. Did you mean set or list?" "and offers the commands it could have been"
says "pile --help  lists every command" "and where to read them all"

say "II. a flag, or an argument, that the command does not take"

refuses list --long
exits 2 "a flag the command does not take exits 2"
says "pile list has no flag --long" "and says which flag, and which command"
says "pile list --help  says what this command takes" "and sends the reader to that command's help"
refuses show one two
exits 2 "two piles for a command that takes one is a usage refusal"
says "pile show takes one pile at most, and was given 2" "saying how many it takes"
refuses list --format yaml
says "--format is text, json or markdown, not \"yaml\"" "a format that is not one names the ones there are"

say "III. a name that is not one, and the names that are"

refuses show @cor
exits 1 "a name nothing claims exits 1, because the repository was asked"
says "no pile is named @cor. The names in this repository are @core" "and lists the names that are claimed"
refuses show @Core
exits 2 "a name that could never be one is a typing mistake, and exits 2"
refuses show /core
says "pile guide paths" "a path outside the repository points to the page on the forms a path is typed in"
refuses guide intnet
exits 2 "a page the guide does not carry is a typing mistake, and exits 2"
says "\"intnet\" is not a page of the guide. The pages are" "and the refusal lists the pages there are"
says "intent" "the one meant among them"

say "IV. the help a refusal points to is there"

pp help show
help=$stdout
pp show --help
same "$stdout" "$help" "pile help show prints what pile show --help prints"
says "Refusals:" "and the help lists the command's refusals"
says "not-a-pile" "each by its code"

verdict
