#!/usr/bin/env bash
# Somebody opens a pull request from a branch, and wants it read as the piles
# it changed, not as a list of files: on their machine first, then as the
# comment CI posts on the pull request.
#
# gitdiff names its revisions as git diff does. main...HEAD compares where
# HEAD left main with HEAD, so what reached main since is not part of the
# change. It lists only the changed piles, each with its intent's headline,
# and says when the change rewrote an intent. --format markdown is the
# comment. pile show --at reads a rewritten intent as the base had it.
# (pile gitdiff --help, pile show --help)
. "$JOURNEY_LIB"

repo
write blog/ui/page.tsx 'export const page = 1
'
write blog/auth/session.ts 'export const session = 1
'
write db/schema.sql 'create table posts (id int);
'
mkdir -p blog/ui && within blog/ui pp init --set name=ui
mkdir -p blog/auth && within blog/auth pp init --set name=auth
mkdir -p db && within db pp init --set name=db
explain blog/ui 'Shows the blog to a reader.'
explain blog/auth 'Proves who a user is.'
explain db 'The schema every service reads.'
commit "Lay the blog out in piles"

say "I. a branch that changed two piles, while main moved on"

git switch -q -c work
write blog/ui/page.tsx 'export const page = 2
'
write db/schema.sql 'create table posts (id int, title text);
'
explain db 'The schema every service reads, and its migrations.'
commit "Show titles, and keep them in the schema"
git switch -q main
write blog/auth/session.ts 'export const session = 2
'
commit "Meanwhile, on main"
git switch -q work

pp gitdiff main...HEAD
says "where HEAD left main" "three dots compare from where HEAD left main"
says "@ui  :/blog/ui  " "a changed pile is a heading"
says "   intent    Shows the blog to a reader." "with what the pile is for under it"
says "   M  :/blog/ui/page.tsx" "and each path with its letter"
says "The schema every service reads, and its migrations.  (intent rewritten)" "a pile whose intent the change wrote says so"
lacks "@auth" "what reached main after the branch left it is not part of the change"
says "2 of 3 piles changed, 3 paths" "the last line counts the changed piles against all of them"

pp gitdiff main..HEAD
says "@auth  :/blog/auth  " "two dots compare main itself, so main's own change is in it"

pp gitdiff main... --all
says "@auth  :/blog/auth  " "--all names the unchanged piles too"
says "   (unchanged)" "as unchanged"

say "II. the comment CI posts on the pull request"

git update-ref refs/remotes/origin/main main
pp gitdiff origin/main...HEAD --format json
field .result.from.name origin/main "the base is whatever revision is named"
field '.result.piles | map(.name) | join(" ")' "ui db" "and only the changed piles are listed"
field '.result.piles[1].intent_changed' true "each says whether its intent was rewritten"
field .result.total.unchanged 1 "and the rest are counted"

pp gitdiff origin/main...HEAD --format markdown
says "<!-- pile gitdiff -->" "the comment starts with a marker CI can find it again by"
says "### 2 of 3 piles changed" "then sums the change up"
says "| Pile | Path | What it is for | Paths |" "then a table of the changed piles"
says "| \`@db\` | \`:/db\` | The schema every service reads, and its migrations. **Intent rewritten.** | 2 |" "each with what it is for"
says "<summary><code>@ui</code> · 1 path</summary>" "then each pile's paths, folded"
says "- \`M\` \`:/blog/ui/page.tsx\`" "a path with its letter"
lacks "@auth" "and the unchanged piles are left out"

refuses status --format markdown
exits 2 "only gitdiff answers in markdown"
says "only pile gitdiff answers in markdown" "and the refusal says which does"

say "III. what the branch rewrote, as the base had it"

pp show @db --at origin/main
says "   at        origin/main " "--at shows the pile as that revision declares it"
says "The schema every service reads." "so the intent the branch rewrote reads as it was"
lacks "and its migrations" "and not as the branch left it"
lacks "written" "with no written line, which counts from HEAD"

pp show @db
says "The schema every service reads, and its migrations." "without --at, it is the branch's"

say "IV. before a commit, and what it refuses"

write blog/auth/login.ts 'export {}
'
write blog/ui/page.tsx 'export const page = 3
'
pp gitdiff
says "to    the working copy" "with no revision named, it is HEAD against the working copy"
says "   M  :/blog/ui/page.tsx" "a change not committed yet"
lacks "login.ts" "an untracked file is not listed, as in git diff"

refuses gitdiff origin/release...HEAD
exits 2 "a branch that is not fetched names no revision"

verdict
