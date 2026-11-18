#!/usr/bin/env bash
# A shop's repository declares two piles: the shop at the top, and its
# billing service. Since then somebody added an exporter inside billing,
# with a build file of its own. An agent is asked to refine the layout. It
# finds the workflow, and carries it out from the top: it scopes the
# refine, reads the evidence, proposes one item, a split, and waits. The
# person here accepts it at once. The agent declares the child, drafts its
# intent and rewrites the parent's, and makes the item in one commit, with
# doctor exiting 0 before it. (pile guide refine, pile guide proposal,
# pile guide split-and-merge)
. "$JOURNEY_LIB"

repo
write README.md 'The shop.
'
write services/billing/package.json '{"name": "billing"}
'
write services/billing/src/charge.ts 'export const charge = () => {};
'
within . pp init --set name=shop
explain . 'The shop: every service it runs, released together.'
within services/billing pp init --set name=billing
explain services/billing 'Charges each customer once a month.'
commit "Start the shop, laid out in piles"
write services/billing/export/package.json '{"name": "billing-export"}
'
write services/billing/export/csv.ts 'export const csv = () => "";
'
commit "Let a customer download every invoice"

export AGENT=its-harness

# The commands one step of the workflow says to run, a line each.
run_of() { # <step, from 1>
	jq -r ".result.pages[0].workflow.steps[$(($1 - 1))].run[]" <<<"$refine"
}

say "I. the agent finds the workflow, and reads it whole"

pp guide refine
refine=$stdout
field '.result.pages | map(.name) | join(" ")' "refine boundaries proposal" "the workflow comes with the pages it needs"
field '.result.pages[0].workflow.steps | length' 4 "four steps"

say "II. step 1: the scope, with nothing changed already"

same "$(run_of 1 | paste -sd '|')" "pile status --all --brief|pile list --tree|pile show <pile>" \
	"the step asks status, then the tree, then one pile whole"
pp status --all --brief
field '[.result.piles[] | .changed + .untracked] | add' 0 "nothing is changed already"
pp list --tree --format text
says "@billing" "the tree names the pile to refine"
pp show @billing --format text
says "Charges each customer once a month." "and show prints what it claims"

say "III. step 2: the evidence"

same "$(run_of 2 | head -2 | paste -sd '|')" "pile files <pile>|pile log -n 50 <pile>" \
	"the step lists the pile's own files, and its own history"
pp files @billing
field '[.result.files[].path | select(endswith("package.json"))] | length' 2 \
	"billing holds two build files: a directory inside it builds on its own"
pp log -n 50 @billing
field '.result.commits | length' 2 "and its history has the commit that added the exporter"

say "IV. step 3: one item is proposed, and the person accepts it"

same "$(run_of 3 | paste -sd '|')" "pile files :/|pile list" "the step reads the top pile's own files, and the list"
pp guide proposal
field '.result.pages[0].text | contains("Each item names its operation")' true \
	"the proposal page says what an item names"
# The proposal: 1. split :/services/billing/export out of @billing as
# @export, "Turns an invoice into a file a customer can download.": it
# has a build file of its own. Cost: one pile declared, one id minted, no
# link rewritten. Accepted.

say "V. step 4: the item is made, in one commit, with doctor exiting 0"

same "$(run_of 4 | paste -sd '|')" "pile doctor|git commit" "the step asks doctor, then commits with git"
within services/billing/export pp init --set name=export
field .result.inside ":/services/billing" "the child is declared inside billing"
refuses doctor
field '[.result.piles[].problems[].code] | join(" ")' "intent-unanswered" "its intent is a question until it is answered"
explain services/billing/export 'Turns an invoice into a file a customer can download.'
pp doctor
exits 0 "with the intent approved and written, doctor exits 0"
pp files @billing
field '[.result.files[].path | select(endswith("package.json"))] | length' 1 \
	"billing's own files no longer hold the exporter's"
commit "Split the exporter out of billing"
same "$(git show --name-only --format= HEAD | grep -c '\.pile/')" 2 "one commit holds the item: the child's config and its intent"

verdict
