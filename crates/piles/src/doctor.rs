//! What every `.pile/` directory in a repository is, and what is wrong with
//! any of them: the verdict `pile doctor` gives.
//!
//! Where [`discover`](crate::discover) refuses the whole repository over one
//! config it cannot read, [`survey`] reports it beside every other, because
//! reporting a broken pile is what a verdict is for. It reads the working
//! copy, and the head revision to say whether a clone would have each pile.
//!
//! Each finding has a severity:
//!
//! - A **problem** stops the structure working: a pile nobody can read, two
//!   piles that cannot be told apart, or a pile that says nothing about why
//!   it exists. Any problem fails the verdict.
//! - A **warning** is something to put right that breaks nothing yet.
//! - A **note** is something to know, and breaks nothing.
//!
//! Each finding names its file, from the top, and its line and column where
//! it has them, and says what fixes it. A config's findings have a line and
//! no column. Only structure is checked: what a pile is, where
//! it sits, and what its config and intent say. Nothing here reads what the
//! files a pile holds say, and nothing here reads a `.toml` file in a
//! `.pile/`: crates/graph reads those, and adds its findings to the same
//! survey.

use std::collections::{BTreeMap, BTreeSet};

use storage::{Content, FileKind, State, Storage, Subtree};

use crate::config::{self, Config};
use crate::{
    CONFIG_FILE, INTENT_FILE, PILE_DIR, Position, ROOT, declared_by, intent, is_toml, join, shown,
    typed, under,
};

/// How much a finding weighs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Problem,
    Warning,
    Note,
}

impl Severity {
    pub fn word(self) -> &'static str {
        match self {
            Severity::Problem => "problem",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
}

/// One thing doctor found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The stable word that names which finding it is.
    pub code: &'static str,
    pub severity: Severity,
    /// The file it is about, from the top of the repository.
    pub file: String,
    /// The line of the file, counting from one, where there is one.
    pub line: Option<usize>,
    /// The column of that line, counting characters from one, where there
    /// is one. A finding with a column always has a line.
    pub column: Option<usize>,
    /// What is wrong, as a sentence.
    pub message: String,
    /// What fixes it, as a sentence.
    pub fix: String,
}

/// What a `.pile/` directory is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Is {
    /// A pile, with a config that reads.
    Pile,
    /// It has a config, and the config cannot be read.
    Refused,
    /// It has no config, so nothing declares it a pile.
    NotAPile,
    /// Its config is at the head revision, and gone from the working copy.
    Unmade,
}

impl Is {
    pub fn word(self) -> &'static str {
        match self {
            Is::Pile => "pile",
            Is::Refused => "config refused",
            Is::NotAPile => "not a pile",
            Is::Unmade => "unmade",
        }
    }
}

/// One `.pile/` directory, and what doctor found in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The directory the `.pile/` is in, from the top.
    pub path: String,
    pub is: Is,
    /// What its config says, when it reads.
    pub config: Option<Config>,
    /// The headline of its intent. Empty when it has none.
    pub headline: String,
    /// The pile at the head revision has the same config file, so a clone
    /// would arrive with the pile.
    pub committed: bool,
    /// The nearest pile above it, by path.
    pub parent: Option<String>,
    /// Problems first, then warnings, then notes, each in the order found.
    pub findings: Vec<Finding>,
}

impl Row {
    pub fn has(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }

    /// Whether the row fails the verdict.
    pub fn ailing(&self) -> bool {
        self.has(Severity::Problem) > 0
    }
}

/// Every `.pile/` directory, in path order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Survey {
    pub rows: Vec<Row>,
    /// What was found in no pile: a `.pile` that is a file, or another
    /// repository, with no pile above it.
    pub in_no_pile: Vec<Finding>,
}

impl Survey {
    /// How many rows are piles, refused ones included.
    pub fn piles(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| matches!(r.is, Is::Pile | Is::Refused))
            .count()
    }

    /// How many rows declare no pile.
    pub fn declaring_none(&self) -> usize {
        self.rows.len() - self.piles()
    }

    /// How many rows have a problem.
    pub fn ailing(&self) -> usize {
        self.rows.iter().filter(|r| r.ailing()).count()
    }

    pub fn count(&self, severity: Severity) -> usize {
        self.rows.iter().map(|r| r.has(severity)).sum::<usize>()
            + self
                .in_no_pile
                .iter()
                .filter(|f| f.severity == severity)
                .count()
    }

    /// Adds findings read elsewhere, each to the row of the `.pile/`
    /// directory its file is in, and keeps each row's problems first.
    pub fn add(&mut self, findings: Vec<Finding>) {
        for f in findings {
            let dir = split(&f.file).map(|(dir, _)| dir);
            match self.rows.iter_mut().find(|r| Some(&r.path) == dir.as_ref()) {
                Some(row) => row.findings.push(f),
                None => self.in_no_pile.push(f),
            }
        }
        for row in &mut self.rows {
            row.findings.sort_by_key(|f| f.severity);
        }
    }

    /// Whether the verdict fails: some finding is a problem.
    pub fn failed(&self) -> bool {
        self.count(Severity::Problem) > 0
    }

    pub fn find(&self, path: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.path == path)
    }
}

/// The `.pile/` directory a declaration file is in, and what is after it:
/// `/a/.pile/b/c` is in `/a`, as `b/c`. The first `.pile` decides, so a
/// `.pile/` inside another belongs to the outer one.
pub fn split(path: &str) -> Option<(String, String)> {
    let at = path.find(&format!("/{PILE_DIR}/"))?;
    let dir = if at == 0 {
        ROOT.to_string()
    } else {
        path[..at].to_string()
    };
    Some((dir, path[at + PILE_DIR.len() + 2..].to_string()))
}

/// Where in its file a finding is: nowhere in particular, a line, or a
/// line and a column.
enum At {
    File,
    Line(Option<usize>),
    Position(Position),
}

fn finding(
    code: &'static str,
    severity: Severity,
    file: &str,
    at: At,
    message: impl Into<String>,
    fix: impl Into<String>,
) -> Finding {
    let (line, column) = match at {
        At::File => (None, None),
        At::Line(line) => (line, None),
        At::Position(p) => (Some(p.line), Some(p.column)),
    };
    Finding {
        code,
        severity,
        file: file.to_string(),
        line,
        column,
        message: message.into(),
        fix: fix.into(),
    }
}

/// Reads the working copy's `.pile/` directories and says what is wrong with
/// each. It fails only when the storage cannot answer.
pub fn survey(storage: &dyn Storage) -> Result<Survey, storage::Error> {
    let found = storage.declarations(&State::Working)?;
    survey_of(storage, &found)
}

/// [`survey`], of declaration files already read from the working copy.
pub fn survey_of(storage: &dyn Storage, found: &[Content]) -> Result<Survey, storage::Error> {
    let committed: BTreeMap<String, Vec<u8>> = match storage.head()? {
        Some(head) => storage
            .declarations(&State::At(head))?
            .into_iter()
            .map(|c| (c.path, c.bytes))
            .collect(),
        None => BTreeMap::new(),
    };

    let mut dirs: BTreeMap<String, Vec<&Content>> = BTreeMap::new();
    for c in found {
        if let Some((dir, _)) = split(&c.path) {
            dirs.entry(dir).or_default().push(c);
        }
    }
    // A pile unmade in the working copy, and not yet in a commit.
    for path in committed.keys() {
        if let Some((dir, rest)) = split(path)
            && rest == CONFIG_FILE
            && !dirs.contains_key(&dir)
        {
            dirs.insert(dir, Vec::new());
        }
    }

    let mut rows: Vec<Row> = dirs
        .into_iter()
        .map(|(dir, files)| examine(&dir, &files, &committed))
        .collect();

    // A pile whose config is refused is still a pile above the ones under it.
    let piles: Vec<String> = rows
        .iter()
        .filter(|r| matches!(r.is, Is::Pile | Is::Refused))
        .map(|r| r.path.clone())
        .collect();
    for row in &mut rows {
        row.parent = piles
            .iter()
            .filter(|p| **p != row.path && under(p, &row.path))
            .max_by_key(|p| p.len())
            .cloned();
    }

    claimed_twice(&mut rows);
    let in_no_pile = other_repositories(storage, &mut rows)?;
    for row in &mut rows {
        row.findings.sort_by_key(|f| f.severity);
    }
    Ok(Survey { rows, in_no_pile })
}

/// One `.pile/` directory on its own: its config, its intent, and the files
/// it holds that nothing reads.
fn examine(dir: &str, files: &[&Content], committed: &BTreeMap<String, Vec<u8>>) -> Row {
    let config_file = join(dir, &[PILE_DIR, CONFIG_FILE]);
    let intent_file = join(dir, &[PILE_DIR, INTENT_FILE]);
    let mut row = Row {
        path: dir.to_string(),
        is: Is::NotAPile,
        config: None,
        headline: String::new(),
        committed: committed.contains_key(&config_file),
        parent: None,
        findings: Vec::new(),
    };
    let config = files.iter().find(|c| c.path == config_file);
    let intent = files.iter().find(|c| c.path == intent_file);

    let unmade = || {
        finding(
            "unmade",
            Severity::Note,
            &config_file,
            At::File,
            "the config is in the last commit and gone from the working copy, so this pile is \
             being unmade",
            "commit the removal to finish unmaking it, or restore the config to keep the pile",
        )
    };

    for c in files {
        let Some((_, rest)) = split(&c.path) else {
            continue;
        };
        if declared_by(&c.path).is_none() && rest.ends_with(&format!("/{PILE_DIR}/{CONFIG_FILE}")) {
            row.findings.push(finding(
                "pile-in-pile-directory",
                Severity::Problem,
                &c.path,
                At::File,
                format!(
                    "this config is inside {}, a pile's own directory, and declares nothing: a \
                     pile is a directory of the repository",
                    shown(&join(dir, &[PILE_DIR]))
                ),
                "move the directory out of the .pile/ directory, then run pile init in it",
            ));
        } else if c.path != config_file && c.path != intent_file && !is_toml(&c.path) {
            row.findings.push(finding(
                "unread-file",
                Severity::Note,
                &c.path,
                At::File,
                "pile does not read this file, and refuses nothing for it. Another program may \
                 keep it here",
                "nothing, unless nothing uses it: deleting the .pile/ directory to unmake the \
                 pile would delete it too",
            ));
        }
    }

    let Some(config) = config else {
        if row.committed {
            row.is = Is::Unmade;
            row.findings.push(unmade());
            return row;
        }
        row.findings.push(finding(
            "not-a-pile",
            Severity::Note,
            &config_file,
            At::File,
            "this .pile/ directory has no config, so nothing declares the directory a pile",
            "run pile init in the directory to declare it one, or remove the .pile/ directory",
        ));
        return row;
    };

    row.is = Is::Refused;
    if config.kind == FileKind::Symlink {
        row.findings.push(finding(
            "config-not-a-file",
            Severity::Problem,
            &config_file,
            At::File,
            "the config is a symbolic link, and a config is a file",
            "replace the link with the file it points to",
        ));
        return row;
    }
    let cfg = match config::parse(&config.bytes) {
        Ok(cfg) => cfg,
        Err(why) => {
            row.findings.push(finding(
                "unreadable-config",
                Severity::Problem,
                &config_file,
                At::Line(why.line),
                format!("the config cannot be read: {why}"),
                "correct the line it names. Every command but doctor refuses while it cannot be \
                 read",
            ));
            return row;
        }
    };
    row.is = Is::Pile;

    if cfg.name.is_empty() {
        row.findings.push(finding(
            "no-name",
            Severity::Warning,
            &config_file,
            At::File,
            "the pile has no name, so it is typed and shown by its path",
            format!("pile set name <name> {} gives it one", shown(dir)),
        ));
    }
    row.config = Some(cfg);
    if !row.committed {
        row.findings.push(finding(
            "not-committed",
            Severity::Note,
            &config_file,
            At::File,
            "the config is not in the last commit, so a clone would not have this pile",
            format!(
                "commit {} to carry the pile to a clone",
                shown(&join(dir, &[PILE_DIR]))
            ),
        ));
    }
    intent_findings(&mut row, &intent_file, intent.copied());
    row
}

/// What is wrong with a pile's intent, and its headline.
fn intent_findings(row: &mut Row, file: &str, intent: Option<&Content>) {
    let Some(intent) = intent else {
        row.findings.push(finding(
            "no-intent",
            Severity::Problem,
            file,
            At::File,
            "the pile has no intent, so nothing says why it exists",
            "write it: a headline that says what the pile is for, what belongs in it, and the \
             closing pair. pile guide write-an-intent says how",
        ));
        return;
    };
    if intent.kind == FileKind::Symlink {
        row.findings.push(finding(
            "intent-not-a-file",
            Severity::Problem,
            file,
            At::File,
            "the intent is a symbolic link, and an intent is a file",
            "replace the link with the file it points to",
        ));
        return;
    }
    let Ok(text) = std::str::from_utf8(&intent.bytes) else {
        row.findings.push(finding(
            "intent-not-text",
            Severity::Problem,
            file,
            At::File,
            "the intent is not UTF-8 text",
            "write it again as text",
        ));
        return;
    };
    row.headline = intent::headline(text);
    if text.trim().is_empty() {
        row.findings.push(finding(
            "intent-empty",
            Severity::Problem,
            file,
            At::File,
            "the intent is empty, so nothing says why the pile exists",
            "write a headline that says what the pile is for, then what belongs in it, then the \
             closing pair. pile guide write-an-intent says how",
        ));
    } else if let Some(at) = intent::unclosed_comment(text) {
        row.findings.push(finding(
            "intent-comment-unclosed",
            Severity::Problem,
            file,
            At::Position(at),
            "a comment opens here and is never closed, so it hides everything after it",
            "close it with -->, or remove it",
        ));
    } else if row.headline.is_empty() {
        row.findings.push(finding(
            "intent-unanswered",
            Severity::Problem,
            file,
            At::File,
            "the intent has no headline: it is still the question pile init wrote, or holds no \
             prose",
            "replace the comment with why the pile exists, starting with a headline. pile guide \
             write-an-intent says how",
        ));
    }
    if let Some(at) = intent::heading_at(text) {
        row.findings.push(finding(
            "intent-heading",
            Severity::Warning,
            file,
            At::Position(at),
            "the intent opens with a heading. pile says where the pile is, and a path written \
             here goes stale when the pile moves",
            "remove the heading line, so the intent starts with its headline",
        ));
    }
}

/// An id or a name two or more piles claim is a problem in each of them.
fn claimed_twice(rows: &mut [Row]) {
    let mut ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in rows.iter() {
        if let Some(cfg) = &r.config {
            ids.entry(cfg.id.clone()).or_default().push(r.path.clone());
            if !cfg.name.is_empty() {
                names
                    .entry(cfg.name.clone())
                    .or_default()
                    .push(r.path.clone());
            }
        }
    }
    for row in rows.iter_mut() {
        let Some(cfg) = row.config.clone() else {
            continue;
        };
        let file = join(&row.path, &[PILE_DIR, CONFIG_FILE]);
        let others = |all: &[String]| -> Vec<String> {
            all.iter()
                .filter(|p| **p != row.path)
                .map(|p| shown(p))
                .collect()
        };
        let also = others(&ids[&cfg.id]);
        if !also.is_empty() {
            row.findings.push(finding(
                "id-claimed-twice",
                Severity::Problem,
                &file,
                At::File,
                format!(
                    "its id {} is claimed by {} too. Copying a subtree copies its config",
                    crate::short(&cfg.id),
                    also.join(", ")
                ),
                "in the copy, remove the .pile/config and run pile init there, so it has an id \
                 of its own",
            ));
        }
        if !cfg.name.is_empty() {
            let also = others(&names[&cfg.name]);
            if !also.is_empty() {
                row.findings.push(finding(
                    "name-claimed-twice",
                    Severity::Problem,
                    &file,
                    At::File,
                    format!(
                        "{} is claimed by {} too, so the name names neither",
                        typed(&cfg.name),
                        also.join(", ")
                    ),
                    "choose again: pile set name <other> in one of them",
                ));
            }
        }
    }
}

/// A repository inside the working copy, and a `.pile` that is a file: both
/// read off the files git can see, since neither is a declaration file.
fn other_repositories(
    storage: &dyn Storage,
    rows: &mut [Row],
) -> Result<Vec<Finding>, storage::Error> {
    let mut loose = Vec::new();
    let files = storage.files(&State::Working, &Subtree::whole())?;
    let piles: BTreeSet<String> = rows
        .iter()
        .filter(|r| r.config.is_some())
        .map(|r| r.path.clone())
        .collect();
    let owner = |path: &str| -> Option<String> {
        piles
            .iter()
            .filter(|p| under(p, path))
            .max_by_key(|p| p.len())
            .cloned()
    };
    for f in files {
        let finding = if f.kind == FileKind::Submodule {
            finding(
                "holds-repository",
                Severity::Note,
                &f.path,
                At::File,
                "this is another repository inside this one. pile reads none of it, so a pile it \
                 declares is not this repository's",
                "nothing, unless its piles were meant to be this repository's: then make it a \
                 directory of this repository",
            )
        } else if f.path.rsplit('/').next() == Some(PILE_DIR) {
            finding(
                "pile-directory-is-a-file",
                Severity::Problem,
                &f.path,
                At::File,
                "this .pile is a file, and a pile's .pile is a directory, so it declares nothing",
                "remove it, and run pile init in its directory if that is to be a pile",
            )
        } else {
            continue;
        };
        match owner(&f.path).and_then(|pile| rows.iter_mut().find(|r| r.path == pile)) {
            Some(row) => row.findings.push(finding),
            None => loose.push(finding),
        }
    }
    Ok(loose)
}

#[cfg(test)]
mod tests;
