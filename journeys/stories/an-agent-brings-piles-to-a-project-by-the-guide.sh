#!/usr/bin/env bash
# An agent is asked to bring piles to a shop's project, which has a history
# and no pile, and it has never heard of pile. It runs a bare pile, is sent
# to the guide, finds the workflow for a project with no pile, and carries
# it out from the top: each step's commands, as the step gives them, then
# what the step says to look for. Where the workflow stops for the person,
# the person here agrees at once. The site reads the app's prices, so the
# layout it proposes has a link, and an aux pile for the link's edge type.
# It ends where the workflow says it ends: doctor exits 0, AGENTS.md holds
# the section, and one commit holds it all. (pile guide --help,
# pile guide adopt, pile guide proposal)
. "$JOURNEY_LIB"

repo
write README.md 'The shop: the app that sells, and the site that explains it.
'
write app/Cargo.toml '[package]
name = "shop"
'
write app/src/main.rs 'fn main() {}
'
write app/prices.toml 'tea = 4
'
write site/index.md '# The shop
'
commit "Start the shop"
write app/src/main.rs 'fn main() { println!("open"); }
'
commit "Open the shop"
write site/index.md '# The shop, open

Prices are read from app/prices.toml.
'
commit "Say the shop is open"

export AGENT=its-harness

# The commands one step of the workflow says to run, a line each.
run_of() { # <step, from 1>
	jq -r ".result.pages[0].workflow.steps[$(($1 - 1))].run[]" <<<"$adopt"
}

say "I. the agent arrives, and is sent to the guide"

ask
says "pile guide adopt | claude" "a bare pile with no pile to say where it is prints the help, which starts at the adopt workflow"
pp guide
field '.result.pages | map(.name) | join(" ")' "start piles paths" "pile guide is the start page, then the pages it needs"
field '.result.pages[0].text | contains("pile guide adopt")' true "and the start page names the workflow for a project with no pile"

say "II. it finds the workflow, and reads it whole"

pp guide --list
field '[.result.pages[] | select(.role == "workflow") | .name] | join(" ")' \
	"adopt commit-a-change read-a-change refine review start-a-change write-an-intent" \
	"the list gives the workflows a role of their own"
pp guide adopt
adopt=$stdout
field '.result.pages | map(.name) | join(" ")' "adopt boundaries proposal" "the workflow comes with the pages it needs"
field '.result.pages[0].workflow | keys | join(" ")' "done_when first hand_back steps when" "and its parts, each under its own key"
field '.result.pages[0].workflow.steps | length' 6 "six steps"

say "III. step 1: no pile is declared yet"

same "$(run_of 1 | paste -sd ' ')" "pile version pile list" "the step names the commands to run"
pp version
field .result.agent AGENT "the binary answers, and says it is answering an agent"
pp list
field '.result.piles | length' 0 "there is no pile yet, so the workflow is the right one"

say "IV. step 2: the evidence, and the link it shows"

same "$(run_of 2 | head -1)" "git ls-files" "the step reads the project with git"
files=$(git ls-files)
same "$(grep -c Cargo.toml <<<"$files")" 1 "a build file of its own marks the app"
git log --stat -n 50 >/dev/null || bad "git log --stat works"
same "$(git log --format=%s -- app | paste -sd '|')" "Open the shop|Start the shop" \
	"and the history changed the app apart from the site"
same "$(run_of 2 | tail -1)" "git grep -n -F '<dir>/' -- ':!<dir>'" "then it searches for what names each candidate from outside it"
same "$(git grep -n -F 'app/' -- ':!app' | cut -d: -f1)" "site/index.md" \
	"the site names a path in the app, so the site relies on the app"

say "V. step 3: what is left outside the candidates, and a proposal the person agrees to"

same "$(run_of 3)" "git ls-files -- ':(exclude)<dir>'" "the step lists what no candidate holds"
same "$(git ls-files -- ':(exclude)app' ':(exclude)site')" "README.md" \
	"only the README is left, and it is about the whole shop: no move to propose"
pp guide proposal
field '.result.pages[0].text | contains("relies on @target")' true "the proposal page says how each link is listed"
# The proposal: the shop at the top, the app, the site, an aux pile at
# :/piles for the edge type relies-on, and "@site relies on @app: reads
# its prices from". No move. Agreed.

say "VI. step 4: no move was accepted, so no file moves"

same "$(run_of 4 | head -1)" "git mv <old path> <new path>" "the step moves a file with git, and is skipped with no move"

say "VII. step 5: each agreed pile is declared, the top first, and its intent drafted"

same "$(run_of 5 | paste -sd '|')" \
	"pile init --set name=<name>|cd <dir> && pile init --set name=<name>|cd <dir> && pile init --set name=<name> --set kind=auxiliary-gitpile|pile doctor" \
	"the step gives init at the top, in each directory, and for the aux pile, then doctor"
pp init --set name=shop
field .result.path ":/" "the top is declared first"
field '.next | map(.command) | join(" ")' "pile guide write-an-intent" "and init names the workflow that answers its question"
within app pp init --set name=app
field .result.inside ":/" "the app is declared inside the shop"
within site pp init --set name=site
field .result.path ":/site" "and so is the site"
mkdir -p piles
within piles pp init --set name=piles --set kind=auxiliary-gitpile
field .result.path ":/piles" "the aux pile is declared in a directory of its own"
write piles/.pile/types.toml 'version = "gitpile/1"

[[edge-types]]
name = "relies-on"
title = "one pile relies on what another is for"
source = ["pile"]
target = ["pile"]
do = "say what the source relies on, in words of the target'"'"'s intent"
dont = "name a file or a path"
'
refuses doctor
field '[.result.piles[].problems[].code] | join(" ")' \
	"intent-unanswered intent-unanswered intent-unanswered intent-unanswered" "doctor reports each new intent as a question still to answer"
explain . 'The shop: what is sold, and what is said about it, released together.'
explain app 'The app that takes the orders, and the prices it charges, shipped as one binary.'
explain site 'The site that explains the shop to whoever is about to buy.'
explain piles 'The words the shop'"'"'s piles say what they rely on of each other in.'
pp doctor
field '[.result.piles[] | .intent] | join(" | ")' \
	"The shop: what is sold, and what is said about it, released together. | The app that takes the orders, and the prices it charges, shipped as one binary. | The words the shop's piles say what they rely on of each other in. | The site that explains the shop to whoever is about to buy." \
	"with the drafts written, doctor prints each headline beside its pile, and the app's names the prices the site relies on"

say "VIII. step 6: the link and the section are written, and one commit holds it all"

same "$(run_of 6 | paste -sd '|')" "pile guide --agents|pile doctor|pile status --all --brief" \
	"the step prints the section, then asks doctor, then status"
write site/.pile/links.toml 'version = "gitpile/1"

[[edges]]
type = "relies-on"
target = "@app"
title = "reads its prices from"
description = """
The site shows the prices the app charges, and never keeps its own.
"""
'
ask guide --agents
printf '%s\n' "$stdout" >AGENTS.md
same "$(head -1 AGENTS.md | cut -c1-10)" "## Piles (" "the section is text an agent's json default leaves alone, ready for the file"
same "$(head -1 AGENTS.md | grep -cE '^## Piles \([0-9]{4}-[0-9]{2}-[0-9]{2}\)$')" 1 "its heading carries the date its text last changed"
same "$(grep -c 'pile guide' AGENTS.md)" 1 "and it sends the next session to pile guide"
pp doctor
exits 0 "doctor exits 0"
field .result.total.problems 0 "with no problem"
pp status --all --brief
field '[.result.piles[] | .changed] | add' 0 "no file the project had is changed"
field '[.result.piles[] | .untracked] | add' 11 "only the four .pile/ directories and AGENTS.md are new"
commit "Lay the shop out in piles"
same "$(git show --name-only --format= HEAD | grep -c '\.pile/')" 10 "one commit holds every config, every intent, the type and the link"

say "IX. what is true once it is done, and what goes back"

same "$(jq '.result.pages[0].workflow.done_when | length' <<<"$adopt")" 4 \
	"the workflow says what is true once it is done"
pp doctor
field '[.result.piles[].notes[].code] | length' 0 "a clone has every pile now"
pp show @site --format text
says "reads its prices from" "the link goes back to the person, with its title"
pp list --format text
says "@shop" "and the piles, as list prints them"
says "@app" "the app"
says "@site" "the site"
says "@piles" "and the aux pile"

verdict
