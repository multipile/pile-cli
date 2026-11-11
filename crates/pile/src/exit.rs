//! What the process exits with, and the refusals that make it exit non-zero.
//!
//! There are three exit codes, because a script has to tell "you typed it
//! wrong" apart from "it went wrong". The two want different responses.
//!
//! A refusal is a command declining to act. It carries a [`Code`], the
//! sentence a person reads, and the commands that fix it. The sentence may be
//! reworded in any release. The code may not.

use std::fmt;

/// The command did what was asked.
pub const OK: i32 = 0;
/// The command failed. Look at the repository.
pub const FAILURE: i32 = 1;
/// The invocation was wrong. Fix what you typed.
pub const USAGE: i32 = 2;

/// Declares [`Code`]: each code's variant, its word, and the exit code it
/// exits with, on one line, so the three cannot drift apart.
macro_rules! codes {
    ($($(#[doc = $doc:literal])* $variant:ident = $word:literal, $exit:ident;)+) => {
        /// The word naming which refusal it is. A structured answer prints it
        /// as `error.code`, and a reader may branch on it.
        ///
        /// The words are lowercase and hyphenated, and stable: renaming one
        /// breaks whoever branches on it. A command that has codes of its own
        /// adds them here as it lands, and lists them under `Refusals:` in its
        /// help.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Code {
            $($(#[doc = $doc])* $variant,)+
        }

        impl Code {
            /// The word itself.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Code::$variant => $word,)+
                }
            }

            /// The exit code a refusal with this code exits with.
            pub fn exit(self) -> i32 {
                match self {
                    $(Code::$variant => $exit,)+
                }
            }
        }
    };
}

codes! {
    /// `pile` has no such command.
    UnknownCommand = "unknown-command", USAGE;
    /// The command does not take the flag.
    UnknownFlag = "unknown-flag", USAGE;
    /// `--format` was given something other than `text`, `json` or
    /// `markdown`.
    UnknownFormat = "unknown-format", USAGE;
    /// `--format markdown` was given to a command that does not answer in
    /// Markdown.
    FormatNotAnswered = "format-not-answered", USAGE;
    /// Any other invocation the command cannot take.
    Usage = "usage", USAGE;
    /// The command reads a repository, and was run outside one.
    NotARepository = "not-a-repository", FAILURE;
    /// There is no `git` on PATH to read the repository with.
    NoGit = "no-git", FAILURE;
    /// A name was given that names no revision of the repository.
    UnknownRevision = "unknown-revision", USAGE;
    /// The repository could not be read. The message says what was being
    /// asked, and what the backend said.
    StorageFailed = "storage-failed", FAILURE;
    /// A pile's config could not be read. The message names the file and
    /// the line.
    UnreadableConfig = "unreadable-config", FAILURE;
    /// The repository declares no pile, and the command needs one.
    NoPiles = "no-piles", FAILURE;
    /// The command is about the pile it is run in, and no pile is at or
    /// above the directory it was run in.
    NoPileHere = "no-pile-here", FAILURE;
    /// The directory `pile` runs in cannot be read: it was removed, say.
    UnresolvablePath = "unresolvable-path", FAILURE;
    /// A path that ends up outside the repository: a path on disk, or a `:/`
    /// path that climbs above the top with `..`.
    PathOutsideRepository = "path-outside-repository", FAILURE;
    /// A command that takes a pile was given a path that is not one.
    NotAPile = "not-a-pile", FAILURE;
    /// No pile claims the name given.
    UnknownName = "unknown-name", FAILURE;
    /// Two or more piles claim the name given, so it names neither.
    NameClaimedTwice = "name-claimed-twice", FAILURE;
    /// The name given could never be a name.
    InvalidName = "invalid-name", USAGE;
    /// `init` was given a directory that is a pile already.
    AlreadyAPile = "already-a-pile", FAILURE;
    /// A `--set` that is not `key=value`.
    MalformedSet = "malformed-set", USAGE;
    /// One key given twice to `init --set`.
    KeyGivenTwice = "key-given-twice", USAGE;
    /// A key that is not a key of a config.
    UnknownKey = "unknown-key", USAGE;
    /// A key of a config that `set` does not write.
    NotSettable = "not-settable", USAGE;
    /// A value the key can never take.
    InvalidValue = "invalid-value", USAGE;
    /// The name is claimed by another pile already.
    NameTaken = "name-taken", FAILURE;
    /// `log -n` was given a number of commits that is not one.
    InvalidCount = "invalid-count", USAGE;
    /// `guide` was given a name that is no page of the guide.
    UnknownPage = "unknown-page", USAGE;
    /// `gitdiff` was given `a...b`, and a and b share no revision, so
    /// there is no merge base to compare from.
    NoSharedHistory = "no-shared-history", FAILURE;
    /// `doctor` found a problem. The report is the result, whole.
    ProblemsFound = "problems-found", FAILURE;
    /// A file could not be written: a `.pile/` file, or a drawing. The
    /// message says which, and what the operating system said.
    WriteFailed = "write-failed", FAILURE;
    /// `diagram render` was given a name no pile keeps a diagram by.
    NoSuchDiagram = "no-such-diagram", FAILURE;
    /// Two piles keep a diagram of the name given, so it names neither.
    DiagramNameAmbiguous = "diagram-name-ambiguous", FAILURE;
    /// The diagram breaks a rule of its format, or names a pile the
    /// repository does not have, so it is not drawn.
    DiagramInvalid = "diagram-invalid", FAILURE;
    /// `diagram render` was asked to draw as something other than html,
    /// png or gif.
    UnknownOutput = "unknown-output", USAGE;
    /// A diagram that reads could not be drawn. The message says why.
    DrawFailed = "draw-failed", FAILURE;
    /// A refusal that has no code of its own yet. Every refusal `pile` means
    /// to give has one, so this one is worth reporting.
    Failed = "failed", FAILURE;
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A command worth running: what to type, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Next {
    pub command: String,
    pub why: String,
}

impl Next {
    pub fn new(command: impl Into<String>, why: impl Into<String>) -> Next {
        Next {
            command: command.into(),
            why: why.into(),
        }
    }
}

/// A command declining to act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub code: Code,
    /// The sentence, without the `pile:` prefix and without the commands
    /// that fix it.
    pub message: String,
    /// The commands that fix it, in the order the text prints them.
    pub next: Vec<Next>,
}

impl Refusal {
    pub fn new(code: Code, message: impl Into<String>) -> Refusal {
        Refusal {
            code,
            message: message.into(),
            next: Vec::new(),
        }
    }

    /// Adds a command that fixes it.
    pub fn next(mut self, command: impl Into<String>, why: impl Into<String>) -> Refusal {
        self.next.push(Next::new(command, why));
        self
    }

    /// The exit code it exits with.
    pub fn exit(&self) -> i32 {
        self.code.exit()
    }

    /// Whether the refusal is about what was typed.
    pub fn usage(&self) -> bool {
        self.exit() == USAGE
    }

    /// The refusal as the text format prints it: `pile:` and the sentence,
    /// then each command that fixes it on a line of its own, indented.
    pub fn text(&self) -> String {
        let mut out = format!("pile: {}", self.message);
        out.push('\n');
        let width = self.next.iter().map(|n| n.command.len()).max().unwrap_or(0);
        for n in &self.next {
            let line = format!("  {:<width$}  {}", n.command, n.why);
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_usage_code_exits_2_and_any_other_exits_1() {
        assert_eq!(Refusal::new(Code::UnknownFlag, "x").exit(), USAGE);
        assert!(Refusal::new(Code::Usage, "x").usage());
        assert_eq!(Refusal::new(Code::Failed, "x").exit(), FAILURE);
    }

    #[test]
    fn the_text_indents_what_fixes_it_under_the_sentence() {
        let r = Refusal::new(Code::UnknownCommand, "unknown command \"lsit\"")
            .next("pile --help", "lists every command")
            .next("pile version", "");
        assert_eq!(
            r.text(),
            "pile: unknown command \"lsit\"\n  pile --help   lists every command\n  pile version\n"
        );
        assert_eq!(
            Refusal::new(Code::Failed, "it broke").text(),
            "pile: it broke\n"
        );
    }
}
