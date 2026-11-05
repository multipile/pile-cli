//! What a pile is, read from a repository: its config, its id, name and
//! path, which piles it sits in and holds, and its intent.
//!
//! A pile is declared by its `.pile/config` and by nothing else. So
//! [`discover`] asks the storage for every declaration file and builds the
//! [`Structure`] from the configs it finds. Nothing lists the piles, and
//! nothing records which pile holds which: both are worked out again on
//! every read.
//!
//! A path is in the nearest pile at or above it ([`Structure::owner`]).
//! Every question about piles rests on that one rule: which pile a file is
//! in, which pile holds which, what a pile's own files are, and where a
//! change landed ([`changes`]).
//!
//! Nothing here writes, and nothing here knows how a repository is stored.
//! The command line writes `.pile/` files with what [`config`] and
//! [`intent`] render.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

use storage::{Content, FileKind, State, Storage, Subtree};

pub mod changes;
pub mod config;
pub mod doctor;
pub mod intent;
pub mod measure;

pub use config::Config;

/// The path of the pile at the top of a repository.
pub const ROOT: &str = "/";
/// The directory that makes a subtree a pile.
pub const PILE_DIR: &str = ".pile";
/// The file in it that declares the pile.
pub const CONFIG_FILE: &str = "config";
/// The file in it that says why the pile exists.
pub const INTENT_FILE: &str = "intent.md";
/// How a name is typed when it must be read as a name: `@lib`.
pub const SIGIL: &str = "@";
/// What a declaration file's name ends in when it holds tables of nodes,
/// types and values. crates/graph reads each one, so none is an unread file.
pub const TOML: &str = ".toml";

/// Whether a declaration file holds tables, which crates/graph reads:
/// `/lib/.pile/types.toml`.
pub fn is_toml(path: &str) -> bool {
    path.ends_with(TOML)
}

/// Where something is written in a file: its line and its column, each
/// counting from one. A column counts characters, not bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// What stops the piles being read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The storage could not answer.
    Storage(storage::Error),
    /// A pile's config could not be read. `file` is where it is, from the
    /// top of the repository, and `why` says which line is wrong and how.
    Config { file: String, why: config::Refused },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Storage(e) => e.fmt(f),
            Error::Config { file, why } => write!(f, "{}: {why}", shown(file)),
        }
    }
}

impl std::error::Error for Error {}

impl From<storage::Error> for Error {
    fn from(e: storage::Error) -> Error {
        Error::Storage(e)
    }
}

/// One pile: where it is, what its config says, and its intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pile {
    pub path: String,
    pub config: Config,
    /// What its `.pile/intent.md` holds. None when it has none.
    pub intent: Option<String>,
}

impl Deref for Pile {
    type Target = Config;
    fn deref(&self) -> &Config {
        &self.config
    }
}

impl Pile {
    /// The first eight characters of the id, which is how a report prints it.
    pub fn short_id(&self) -> &str {
        short(&self.id)
    }

    /// The pile as every report heads it: the name with its `@` where it has
    /// one, then the path, then the short id.
    pub fn heading(&self) -> String {
        heading(&self.name, &self.path, self.short_id())
    }

    /// What to type to name the pile: `@` and its name where it has one, and
    /// its path, written from the top, where it does not.
    pub fn handle(&self) -> String {
        if self.name.is_empty() {
            shown(&self.path)
        } else {
            typed(&self.name)
        }
    }

    /// The headline of its intent. Empty when it has no intent, or one with no
    /// prose yet.
    pub fn headline(&self) -> String {
        self.intent
            .as_deref()
            .map(intent::headline)
            .unwrap_or_default()
    }

    /// Where its config is, from the top of the repository.
    pub fn config_path(&self) -> String {
        join(&self.path, &[PILE_DIR, CONFIG_FILE])
    }

    /// Where its intent is, from the top of the repository.
    pub fn intent_path(&self) -> String {
        join(&self.path, &[PILE_DIR, INTENT_FILE])
    }
}

/// The first eight characters of an id.
pub fn short(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// A name as it is typed: `@lib`.
pub fn typed(name: &str) -> String {
    format!("{SIGIL}{name}")
}

/// A pile as a report heads it: `@lib  /packages/lib  9f8e7d6c`, or
/// `/packages/lib  9f8e7d6c` for a pile with no name.
pub fn heading(name: &str, path: &str, id: &str) -> String {
    let path = shown(path);
    if name.is_empty() {
        format!("{path}  {id}")
    } else {
        format!("{}  {path}  {id}", typed(name))
    }
}

/// How a path from the top of the repository is written when it is shown or
/// typed: `:/` and the path, so `:/packages/lib`, and `:/` for the top. The
/// form says the path is from the top, wherever it is read or pasted.
/// Inside this crate a path is held as `/packages/lib`.
pub fn shown(path: &str) -> String {
    format!(":{path}")
}

/// The form in which a path from the top is typed: `:/`.
pub const FROM_TOP: &str = ":/";

/// A path under a pile's path: `join("/a", &[".pile", "config"])` is
/// `/a/.pile/config`, and the root gives `/.pile/config`.
pub fn join(base: &str, rest: &[&str]) -> String {
    let mut out = base.trim_end_matches('/').to_string();
    for part in rest {
        out.push('/');
        out.push_str(part);
    }
    out
}

/// Whether path is under the pile at prefix: the same path, or one that
/// continues it past a `/`. So `/packages/lib` does not hold
/// `/packages/library`.
pub fn under(prefix: &str, path: &str) -> bool {
    if prefix == ROOT {
        return true;
    }
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// How many segments down a path is. The root is at none.
fn depth(p: &str) -> usize {
    if p == ROOT { 0 } else { p.matches('/').count() }
}

/// The pile a declaration file at path declares, when it is a config:
/// `/a/b/.pile/config` declares `/a/b`.
/// A config inside another `.pile/` directory, such as
/// `/a/.pile/b/.pile/config`, declares nothing: a pile is a directory of the
/// repository, and a `.pile/` is not one.
pub(crate) fn declared_by(path: &str) -> Option<String> {
    let rest = path.strip_suffix(&format!("/{PILE_DIR}/{CONFIG_FILE}"))?;
    if rest.split('/').any(|part| part == PILE_DIR) {
        return None;
    }
    Some(if rest.is_empty() {
        ROOT.to_string()
    } else {
        rest.to_string()
    })
}

/// Every pile in a repository, and the declaration files nothing here reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Structure {
    /// Deepest first, and by path among equals. So the first pile that holds
    /// a path is the nearest one above it.
    piles: Vec<Pile>,
    /// Every file in a `.pile/` directory that is not a pile's config, its
    /// intent or a `.toml` file, such as `notes.md`, sorted. Each is skipped,
    /// and nothing is refused for it.
    pub unread: Vec<String>,
}

impl Structure {
    /// A structure of these piles, held deepest first.
    pub fn new(mut piles: Vec<Pile>, mut unread: Vec<String>) -> Structure {
        piles.sort_by(|a, b| {
            depth(&b.path)
                .cmp(&depth(&a.path))
                .then_with(|| a.path.cmp(&b.path))
        });
        unread.sort();
        Structure { piles, unread }
    }

    pub fn is_empty(&self) -> bool {
        self.piles.is_empty()
    }

    pub fn len(&self) -> usize {
        self.piles.len()
    }

    /// The piles in path order, which is how a person reads them. A pile
    /// comes before the piles under it, though not always right before them:
    /// `/lib-v2` sorts between `/lib` and `/lib/sub`.
    pub fn ordered(&self) -> Vec<&Pile> {
        let mut out: Vec<&Pile> = self.piles.iter().collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    /// The pile at a path.
    pub fn find(&self, path: &str) -> Option<&Pile> {
        self.piles.iter().find(|p| p.path == path)
    }

    /// The pile a path is in: the nearest pile at or above it.
    pub fn owner(&self, path: &str) -> Option<&Pile> {
        self.piles.iter().find(|p| under(&p.path, path))
    }

    /// The pile a pile sits in: the nearest pile strictly above it.
    pub fn parent(&self, path: &str) -> Option<&Pile> {
        self.piles
            .iter()
            .find(|p| p.path != path && under(&p.path, path))
    }

    /// The piles a pile holds directly, in path order. A pile inside one of
    /// them is that one's.
    pub fn children(&self, path: &str) -> Vec<&Pile> {
        let mut out: Vec<&Pile> = self
            .piles
            .iter()
            .filter(|q| q.path != path && under(path, &q.path))
            .filter(|q| self.parent(&q.path).is_some_and(|p| p.path == path))
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    /// The pile at a path and every pile under it, in path order.
    pub fn from(&self, prefix: &str) -> Vec<&Pile> {
        let mut out: Vec<&Pile> = self
            .piles
            .iter()
            .filter(|p| under(prefix, &p.path))
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    /// A pile's own files, as a subtree: its directory, less every pile it
    /// holds.
    pub fn own(&self, path: &str) -> Subtree {
        self.children(path)
            .into_iter()
            .fold(Subtree::new(path), |s, c| s.except(&c.path))
    }

    /// The piles that claim a name, in path order.
    pub fn by_name(&self, name: &str) -> Vec<&Pile> {
        self.ordered()
            .into_iter()
            .filter(|p| !p.name.is_empty() && p.name == name)
            .collect()
    }

    /// Every name a pile claims, sorted, each once.
    pub fn names(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .piles
            .iter()
            .filter(|p| !p.name.is_empty())
            .map(|p| p.name.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Each id two or more piles claim, with the piles that claim it, in path
    /// order.
    pub fn duplicate_ids(&self) -> BTreeMap<String, Vec<&Pile>> {
        claimed_twice(self.ordered(), |p| Some(p.id.as_str()))
    }

    /// Each name two or more piles claim, with the piles that claim it, in
    /// path order.
    pub fn duplicate_names(&self) -> BTreeMap<String, Vec<&Pile>> {
        claimed_twice(self.ordered(), |p| {
            (!p.name.is_empty()).then_some(p.name.as_str())
        })
    }

    /// The declaration files nothing reads that are inside a pile's own
    /// `.pile/` directory, from the top: `/lib/.pile/notes.md`.
    pub fn unread_in(&self, path: &str) -> Vec<String> {
        let dir = join(path, &[PILE_DIR]);
        self.unread
            .iter()
            .filter(|f| under(&dir, f))
            .cloned()
            .collect()
    }
}

fn claimed_twice(
    piles: Vec<&Pile>,
    key: impl Fn(&Pile) -> Option<&str>,
) -> BTreeMap<String, Vec<&Pile>> {
    let mut by: BTreeMap<String, Vec<&Pile>> = BTreeMap::new();
    for p in piles {
        if let Some(k) = key(p) {
            by.entry(k.to_string()).or_default().push(p);
        }
    }
    by.retain(|_, v| v.len() >= 2);
    by
}

/// Every pile the repository declares at a state: the working copy, where a
/// pile declared a minute ago counts, or a revision.
///
/// It fails whole when one config cannot be read. Read as no config, that
/// pile's paths would go to the pile above it without a word.
pub fn discover(storage: &dyn Storage, at: &State) -> Result<Structure, Error> {
    structure(&storage.declarations(at)?)
}

/// The piles these declaration files declare, as [`discover`] reads them. A
/// reader that needs the files too asks the storage for them once, and
/// passes them here.
pub fn structure(found: &[Content]) -> Result<Structure, Error> {
    let mut configs: Vec<(String, &Content)> = Vec::new();
    let mut intents: BTreeMap<String, String> = BTreeMap::new();
    let mut unread = Vec::new();
    for c in found {
        if let Some(pile) = declared_by(&c.path) {
            configs.push((pile, c));
        } else if let Some(pile) = c.path.strip_suffix(&format!("/{PILE_DIR}/{INTENT_FILE}"))
            && c.kind != FileKind::Symlink
        {
            let pile = if pile.is_empty() { ROOT } else { pile };
            intents.insert(
                pile.to_string(),
                String::from_utf8_lossy(&c.bytes).into_owned(),
            );
        } else if !is_toml(&c.path) {
            unread.push(c.path.clone());
        }
    }

    let mut piles = Vec::new();
    for (path, c) in configs {
        let file = c.path.clone();
        if c.kind == FileKind::Symlink {
            return Err(Error::Config {
                file,
                why: config::Refused {
                    line: None,
                    why: "it is a symbolic link, and a config is a file".to_string(),
                },
            });
        }
        let config = config::parse(&c.bytes).map_err(|why| Error::Config { file, why })?;
        let intent = intents.remove(&path);
        piles.push(Pile {
            path,
            config,
            intent,
        });
    }
    // An intent in a .pile/ directory that declares no pile is read by
    // nothing.
    unread.extend(
        intents
            .into_keys()
            .map(|p| join(&p, &[PILE_DIR, INTENT_FILE])),
    );
    Ok(Structure::new(piles, unread))
}

#[cfg(test)]
mod tests;
