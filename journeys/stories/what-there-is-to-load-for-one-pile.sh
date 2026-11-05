#!/usr/bin/env bash
# Somebody, or an agent with a small context, is about to read one pile, and
# asks first which files are its own and how long each is.
#
# A pile's own files are its directory, less every pile it holds, which git
# ls-files of the directory cannot give. files measures each one as it is on
# disk now, and names each pile it held back as the command that lists it.
# (pile files --help)
. "$JOURNEY_LIB"

repo
write site/index.md '# Home
two
three
'
# A NUL is what makes a file binary, and a shell string cannot hold one.
printf '\x89PNG\x00\x00\x00' >site/logo.png
write site/blog/post.md 'a post
'
write site/blog/draft.md 'a draft, never committed'
ln -s index.md site/start.md
mkdir -p site && within site pp init --set name=site
mkdir -p site/blog && within site/blog pp init --set name=blog
explain site 'The site, as it is published.'
explain site/blog 'The blog inside the site.'
commit "Declare the site and its blog" site/.pile site/blog/.pile site/index.md site/logo.png site/start.md site/blog/post.md

pp files @site
says "@site  :/site  " "files heads the listing with the pile"
says "       3      17  :/site/index.md" "each file has its lines, as wc -l counts them, and its bytes"
says "  binary       7  :/site/logo.png" "a binary file has no lines"
says "    link       8  :/site/start.md" "a link is as long as the path it points to"
lacks "post.md" "and a file in a pile it holds is not its own"
says "not counted, being piles of their own: pile files @blog" "the pile it holds is named as the command that lists it"

pp files @site --format json
field .result.total.files 5 "the totals count the config, the intent and the three files"
field '.next[0].command' "pile files @blog" "and the command for the held pile is next"

within site/blog pp files
says "draft.md" "with no pile named, files lists the pile it runs in, untracked files too"

verdict
