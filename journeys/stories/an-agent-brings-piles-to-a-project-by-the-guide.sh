#!/usr/bin/env bash
# An agent is asked to bring piles to a shop's project, which has a history
# and no pile, and it has never heard of pile. It runs a bare pile, is sent
# to the guide, finds the workflow for a project with no pile, and carries
# it out from the top: each step's commands, as the step gives them, then
# what the step says to look for. Where the workflow stops for the person,
# the person here agrees at once. It ends where the workflow says it ends:
# doctor exits 0, AGENTS.md opens with the section, and one commit holds it
# all. (pile guide --help, pile guide adopt)
. "$JOURNEY_LIB"

repo
write README.md 'The shop: the app that sells, and the site that explains it.
'
write app/Cargo.toml '[package]
name = "shop"
'
write app/src/main.rs 'fn main() {}
'
write site/index.md '# The shop
'
commit "Start the shop"
write app/src/main.rs 'fn main() { println!("open"); }
'
commit "Open the shop"
write site/index.md '# The shop, open
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
	"adopt commit-a-change read-a-change review start-a-change write-an-intent" \
	"the list gives the workflows a role of their own"
pp guide adopt
adopt=$stdout
field '.result.pages | map(.name) | join(" ")' "adopt boundaries intent" "the workflow comes with the pages it needs"
field '.result.pages[0].workflow | keys | join(" ")' "done_when first hand_back steps when" "and its parts, each under its own key"
field '.result.pages[0].workflow.steps | length' 6 "six steps"

say "III. step 1: no pile is declared yet"

same "$(run_of 1 | paste -sd ' ')" "pile version pile list" "the step names the commands to run"
pp version
field .result.agent AGENT "the binary answers, and says it is answering an agent"
pp list
field '.result.piles | length' 0 "there is no pile yet, so the workflow is the right one"

say "IV. step 2: the evidence, and a proposal the person agrees to"

same "$(run_of 2 | head -1)" "git ls-files" "the step reads the project with git"
files=$(git ls-files)
same "$(grep -c Cargo.toml <<<"$files")" 1 "a build file of its own marks the app"
git log --stat -n 50 >/dev/null || bad "git log --stat works"
same "$(git log --format=%s -- app | paste -sd '|')" "Open the shop|Start the shop" \
	"and the history changed the app apart from the site"
# The proposal: the shop at the top, the app, and the site. Agreed.

say "V. step 3: each agreed pile is declared, the top first"

same "$(run_of 3 | paste -sd '|')" "pile init --set name=<name>|cd <dir> && pile init --set name=<name>" \
	"the step gives init at the top, then in each directory"
pp init --set name=shop
field .result.path ":/" "the top is declared first"
field '.next | map(.command) | join(" ")' "pile guide write-an-intent" "and init names the workflow that answers its question"
within app pp init --set name=app
field .result.inside ":/" "the app is declared inside the shop"
within site pp init --set name=site
field .result.path ":/site" "and so is the site"

say "VI. step 4: each intent is drafted, and the person approves the drafts"

same "$(run_of 4)" "pile doctor" "the step asks doctor"
refuses doctor
field '[.result.piles[].problems[].code] | join(" ")' \
	"intent-unanswered intent-unanswered intent-unanswered" "doctor reports each new intent as a question still to answer"
explain . 'The shop: what is sold, and what is said about it, released together.'
explain app 'The app that takes the orders, built and shipped as one binary.'
explain site 'The site that explains the shop to whoever is about to buy.'
pp doctor
field '[.result.piles[] | .intent] | join(" | ")' \
	"The shop: what is sold, and what is said about it, released together. | The app that takes the orders, built and shipped as one binary. | The site that explains the shop to whoever is about to buy." \
	"with the drafts written, doctor prints each headline beside its pile"

say "VII. step 5: the next session is sent to the guide"

same "$(run_of 5)" "pile guide --agents >> AGENTS.md" "the step appends the section to AGENTS.md"
ask guide --agents
printf '%s\n' "$stdout" >>AGENTS.md
same "$(head -1 AGENTS.md)" "## Piles" "the section is text an agent's json default leaves alone, ready for the file"
same "$(grep -c 'pile guide' AGENTS.md)" 1 "and it sends the next session to pile guide"

say "VIII. step 6: doctor exits 0, and one commit holds it all"

same "$(run_of 6 | paste -sd '|')" "pile doctor|pile status --all --brief" "the step asks doctor, then status"
pp doctor
exits 0 "doctor exits 0"
field .result.total.problems 0 "with no problem"
pp status --all --brief
field '[.result.piles[] | .changed] | add' 0 "no file the project had is changed"
field '[.result.piles[] | .untracked] | add' 7 "only the three .pile/ directories and AGENTS.md are new"
commit "Lay the shop out in piles"
same "$(git show --name-only --format= HEAD | grep -c '\.pile/')" 6 "one commit holds every config and every intent"

say "IX. what is true once it is done, and what goes back"

same "$(jq '.result.pages[0].workflow.done_when | length' <<<"$adopt")" 4 \
	"the workflow says what is true once it is done"
pp doctor
field '[.result.piles[].notes[].code] | length' 0 "a clone has every pile now"
pp list --format text
says "@shop  :/      " "and the piles go back to the person, as list prints them"
says "@app   :/app   " "the app"
says "@site  :/site  " "and the site"

verdict
