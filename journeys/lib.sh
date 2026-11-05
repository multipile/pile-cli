# The words a journey is written in. Every script in stories/ sources this
# file, and nothing runs it alone.
#
# A journey puts a git repository in a state somebody could arrive at, runs
# `pile` the way they would type it, and holds what came back to what the
# command's --help claims. An assertion records its verdict and carries on,
# so one run reports everything that is untrue, not only the first thing.
#
# The state a journey has is what the last command left: out, stdout, stderr
# and code. Every assertion reads them.

failures=0
out=
stdout=
stderr=
code=0

# --- What a run prints ------------------------------------------------------

# say heads one act of the story.
say() { printf '\n  %s\n' "$*"; }

# ok and bad are what every assertion ends in. The text is the claim, so a
# FAIL line reads as the thing that is untrue.
ok() { printf '  ok    %s\n' "$*"; }
bad() {
	failures=$((failures + 1))
	printf '  FAIL  %s\n' "$*"
}

# skip says which claim nobody here could judge, and why.
skip() { printf '  skip  %s\n' "$*"; }

# show prints what the last command printed, under a FAIL line.
show() { printf '%s\n' "$out" | sed 's/^/          | /'; }

# typed prints the session as it goes, when SHOW is set: the command as a
# person would have typed it, then what came back. A recording sets SHOW,
# because a cast of nothing but verdicts shows nothing of the session.
typed() { # <command line>
	[[ -n ${SHOW:-} ]] || return 0
	printf '\n  $ %s\n' "$1"
	if [[ -n $out ]]; then
		printf '%s\n' "$out" | sed 's/^/    /'
	fi
}

# quoted is the arguments as a person would type them: a word the shell
# would split, or an empty one, in single quotes, and the rest bare.
quoted() {
	local line= word
	for word in "$@"; do
		if [[ -z $word || $word == *[!A-Za-z0-9_./:@=%+,-]* ]]; then
			word="'${word//\'/\'\\\'\'}'"
		fi
		line+=" $word"
	done
	printf '%s' "${line# }"
}

# --- The repository ---------------------------------------------------------

# repo makes the directory the runner put this journey in a fresh git
# repository, on a branch called main, with nothing in it.
repo() {
	cd "$(pwd -P)" || exit 1
	git init -q -b main . || exit 1
	git config user.name 'A Person'
	git config user.email person@example.com
	git config commit.gpgsign false
}

# write puts content in a file, and the directories above it.
write() { # <path> <content>
	mkdir -p "$(dirname "$1")"
	printf '%s' "$2" >"$1"
}

# explain answers the question pile init writes into a pile's intent: a
# headline, then the closing pair, and no heading. What the intent says is not what a journey is about; that it
# says something is.
explain() { # <pile directory> <headline>
	write "$1/.pile/intent.md" "$2

Read this pile to find out what it holds. Change it when that does.
"
}

# commit is a commit made with git, because `pile` makes none. It stages the
# paths given, or everything, untracked files included, and fails the
# journey when git declines.
commit() { # <subject> [path...]
	local subject=$1
	shift
	out=$(git add -A -- "$@" 2>&1 && git commit -q -m "$subject" 2>&1)
	code=$?
	typed "$(quoted git add -A "$@") && $(quoted git commit -m "$subject")"
	if ((code != 0)); then
		bad "git commit of \"$subject\" works"
		show
	fi
}

# --- Running pile -----------------------------------------------------------

# ask runs pile and keeps what it printed: stdout, stderr, both of them in
# out, and the exit code.
ask() {
	local o e
	o=$(mktemp) e=$(mktemp)
	"$PILE" "$@" >"$o" 2>"$e"
	code=$?
	stdout=$(<"$o") stderr=$(<"$e")
	rm -f "$o" "$e"
	out=$stdout
	if [[ -n $stderr ]]; then
		out+=${out:+$'\n'}$stderr
	fi
	typed "$(quoted pile "$@")"
}

# pp is pile, as a step that should work: one that exits non-zero is a
# failure of the journey.
pp() {
	ask "$@"
	if ((code != 0)); then
		bad "pile $* works"
		show
	fi
}

# refuses is pile, as a step that should be refused: one that exits 0 is a
# failure of the journey, because work happened where a refusal was due.
refuses() {
	ask "$@"
	if ((code == 0)); then
		bad "pile $* is refused"
		show
	fi
}

# within runs one step from a directory under the repository, as somebody
# who has cd'd there would, and comes back.
within() { # <directory> <step> [argument...]
	local here=$PWD
	cd "$1" || {
		bad "$1 is a directory to stand in"
		return
	}
	shift
	"$@"
	cd "$here" || exit 1
}

# --- Judging what came back -------------------------------------------------

# says and lacks judge what the last command printed, on either stream.
says() { # <text> <claim>
	if [[ $out == *"$1"* ]]; then ok "$2"; else
		bad "$2"
		show
	fi
}
lacks() { # <text> <claim>
	if [[ $out == *"$1"* ]]; then
		bad "$2"
		show
	else ok "$2"; fi
}

# same judges a value against the one wanted.
same() { # <got> <wanted> <claim>
	if [[ $1 == "$2" ]]; then ok "$3"; else bad "$3 (wanted ${2@Q}, got ${1@Q})"; fi
}

# exits judges the last exit code.
exits() { # <code> <claim>
	same "$code" "$1" "$2"
}

# field judges one value of the last answer, read as JSON with a jq filter.
# With -c first, the value is compared as compact JSON rather than as text.
field() { # [-c] <jq filter> <wanted> <claim>
	local got how=-r
	if [[ $1 == -c ]]; then
		how=-c
		shift
	fi
	if ! got=$(jq "$how" "$1" <<<"$stdout" 2>/dev/null); then
		bad "$3 (stdout is not one JSON document)"
		show
		return
	fi
	same "$got" "$2" "$3"
}

# verdict ends every journey, and exits non-zero when a claim was untrue.
verdict() {
	if ((failures == 0)); then
		printf '\n  passed\n'
		exit 0
	fi
	printf '\n  %s failed\n' "$failures"
	exit 1
}
