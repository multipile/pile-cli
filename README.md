# pile

`pile` is the command-line tool of multipile. multipile declares structure inside a git repository. A **pile** is a subtree with a `.pile/` directory that says what it is and why it is one thing.

```sh
pile guide      # how to work in a repository of piles: start here
pile list       # every pile the repository declares
pile show       # one pile whole: its config, where it sits, its intent
pile status     # what changed since the last commit, by pile
pile doctor     # what every pile is, and what is wrong with any of them
```

`pile --help` lists every command. `pile` makes no commit: git keeps the versions, the content and the history.

## Install

Releases are published on GitHub for macOS and Linux, on arm64 and amd64. With [Homebrew](https://brew.sh):

```sh
brew install multipile/tap/pile
```

With [mise](https://mise.jdx.dev):

```sh
mise use -g github:multipile/pile-cli
```

Or download the archive for your platform from the [releases page](https://github.com/multipile/pile-cli/releases). Each one holds a single `pile` binary.

## Build

The toolchain is pinned in `mise.toml`.

```sh
mise x -- scripts/build    # writes bin/pile
mise x -- scripts/check    # formatting, clippy, tests, journeys and every release platform
```

`AGENTS.md` describes how work is done in this repository.

## License

[MIT](LICENSE)
