#!/usr/bin/env bash
# Somebody arrives in a repository whose piles nest three deep, and reads
# them as a tree before reading one of them.
#
# Each pile is mounted in its parent, the nearest pile above it, so the
# root pile is the top of a tree. pile list --tree prints that tree, each
# pile at its path relative to the pile it is mounted in, and pile show
# names the pile one is mounted in and the path within it, beside the path
# from the top. Nothing new is written for it: the tree is what the
# directories already say. (pile list --help, pile show --help)
. "$JOURNEY_LIB"

repo
pp init --set name=shop
explain . 'An online shop.'
mkdir -p services/billing/invoices services/catalog docs
within services/billing pp init --set name=billing
explain services/billing 'Charges customers and keeps what they owe.'
within services/billing/invoices pp init --set name=invoice-rendering
explain services/billing/invoices 'Renders an invoice as a PDF.'
within services/catalog pp init --set name=catalog
explain services/catalog 'What the shop sells.'
within docs pp init --set name=docs
explain docs 'How the shop is run.'
commit "Lay the shop out in piles"

say "I. the tree shows which pile each one is mounted in"

pp list --tree
says "@shop" "the root pile is the top of the tree"
says "├── @docs" "a pile under the root sits under it"
says "│   └── @invoice-rendering  invoices" \
	"a pile two deep is printed at its path relative to the pile it is mounted in"
lacks ":/services" "and no pile below the top is printed with its path from the top"
says "├── @billing" "a pile is mounted in the nearest pile above it, past a directory that is none"
# Each branch drawn as one character, so a column is counted the same on
# every line.
cols=$(head -5 <<<"$out" | sed -e 's/│/x/g; s/├/x/g; s/└/x/g; s/─/x/g' |
	awk '{ print index($0, $NF) }' | sort -u | wc -l)
same "$cols" 1 "every id starts at the same column, the longest name two deep included"
says "5 piles" "the tree holds every pile the list does"

pp list --tree --format json
field '.result.piles | length' 1 "the result holds one top"
field '.result.piles[0].piles[1].piles[0].mount' invoices \
	"and each pile under the pile it is mounted in, at its relative path"
field '.result.piles[0].piles[1].piles[0].mounted_in' @billing "named as it is typed"

say "II. show gives both paths of one pile"

pp show @invoice-rendering
says "@invoice-rendering  :/services/billing/invoices" "the heading keeps the path from the top"
says "   mounted   invoices in @billing" "and mounted names the pile it is in, and the path within it"
pp show @billing
says "   children  @invoice-rendering  invoices" "each child is named at its path relative to the pile"
pp show @shop
lacks "mounted" "the top is mounted in no pile"

verdict
