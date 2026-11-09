#!/usr/bin/env bash
# Somebody wants to label a pile. They write the labels in its config
# first, find every command refusing, and give them in [pile] instead,
# where show prints them.
#
# A config holds kind, version, id and name, and no other key. Any other
# line is refused, and the refusal names its line. Anything else a pile says
# about itself is given in [pile], in a .toml file in its .pile/, which
# opens with version = "gitpile/1". show prints each value a pile gives,
# its key in a column, and a key too long for the column still apart from
# its value.
# (pile guide piles, pile guide declarations, pile show --help,
# pile doctor --help)
. "$JOURNEY_LIB"

repo
write src/main.rs 'fn main() {}
'
within src pp init --set name=app
explain src 'The application.'
commit "Declare the application"

say "I. a labels line in the config stops every command"

printf 'labels = core, docs\n' >>src/.pile/config
refuses list
exits 1 "a config that cannot be read refuses the command, exiting 1"
says ":/src/.pile/config: line 7 names \"labels\", and a config holds only kind, version, id and name" \
	"the refusal names the config, the line, and the keys a config holds"
refuses doctor
exits 1 "doctor exits 1 over it"
says "problem   :/src/.pile/config:7: the config cannot be read" \
	"the problem names the config's line, and no column"

say "II. given in [pile], the labels read"

sed -i.was '/^labels = /d' src/.pile/config && rm src/.pile/config.was
write src/.pile/pile.toml 'version = "gitpile/1"

[pile]
labels = ["core", "docs"]
visibility = "public"
'
pp doctor
exits 0 "with the labels in [pile], doctor exits 0"
says "1 pile, nothing wrong" "and nothing is wrong"
pp show @app
says "labels    core, docs" "show prints the labels the pile gives"
says "visibility public" "a key wider than the column still has a space before its value"
ask show @app --format json
field '.result.values.labels | join(",")' "core,docs" "in json, the labels are a list under values"

say "III. a file that does not open with its version declares nothing"

write src/.pile/pile.toml '[pile]
labels = ["core", "docs"]
'
refuses doctor
says "problem   :/src/.pile/pile.toml: the file does not open with its version" \
	"doctor says the file declares nothing without its version"

verdict
