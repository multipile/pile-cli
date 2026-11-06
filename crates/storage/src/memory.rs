//! A repository held in memory, for unit tests.
//!
//! A test builds one the way a developer builds a repository: it writes
//! files into the working copy, removes and moves them, and records
//! revisions. Then it asks the [`Storage`] questions of it. Each revision's
//! id is `r` and its number, from `r1`, so no id here looks like git's.
//!
//! It answers each question the way the git backend does, with three
//! differences:
//!
//! - It finds a move only when the file arrived holding exactly what it held
//!   before. Git also finds a move whose file changed a little on the way.
//! - Its history is one line. Each revision has the head before it as its
//!   only parent, so no revision joins two lines.
//! - Nothing is ignored. A file a test does not want in the working copy is
//!   a file it does not write.
//!
//! The bases it offers are the ones a test names with [`Memory::base`]. It
//! does not work out a main branch on its own.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use crate::{
    Base, BaseRole, Bases, Change, ChangeKind, Content, Error, File, FileKind, History, Result,
    Revision, RevisionInfo, State, Storage, Subtree, Touched, compare, is_declaration, sort,
};

/// The author of every revision, unless a test sets another.
pub const AUTHOR: &str = "pile test";
/// When the first revision was made, in seconds since the Unix epoch. Each
/// revision after it is a minute later.
pub const FIRST_AT: i64 = 1_700_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Held {
    kind: FileKind,
    bytes: Vec<u8>,
}

type Files = BTreeMap<String, Held>;

#[derive(Debug, Clone)]
struct Recorded {
    info: RevisionInfo,
    parent: Option<usize>,
    files: Files,
}

/// A repository held in memory. See the module for what it does.
#[derive(Debug, Clone)]
pub struct Memory {
    working: Files,
    /// The paths in the working copy the next revision would carry.
    tracked: BTreeSet<String>,
    conflicted: BTreeSet<String>,
    revisions: Vec<Recorded>,
    head: Option<usize>,
    names: BTreeMap<String, usize>,
    branch: Option<String>,
    bases: Vec<(String, BaseRole, usize)>,
    on_disk: Option<PathBuf>,
    author: String,
    next_at: i64,
    offset: i32,
}

impl Default for Memory {
    fn default() -> Memory {
        Memory::new()
    }
}

impl Memory {
    /// An empty repository, with no revision and an empty working copy.
    pub fn new() -> Memory {
        Memory {
            working: Files::new(),
            tracked: BTreeSet::new(),
            conflicted: BTreeSet::new(),
            revisions: Vec::new(),
            head: None,
            names: BTreeMap::new(),
            branch: None,
            bases: Vec::new(),
            on_disk: None,
            author: AUTHOR.to_string(),
            next_at: FIRST_AT,
            offset: 0,
        }
    }

    /// Writes a file into the working copy, and tracks it.
    pub fn write(&mut self, path: &str, bytes: impl AsRef<[u8]>) -> &mut Memory {
        self.write_kind(path, FileKind::File, bytes)
    }

    /// Writes a file of a kind into the working copy, and tracks it. A
    /// symbolic link holds the path it points to.
    pub fn write_kind(
        &mut self,
        path: &str,
        kind: FileKind,
        bytes: impl AsRef<[u8]>,
    ) -> &mut Memory {
        self.put(path, kind, bytes);
        self.tracked.insert(path.to_string());
        self
    }

    /// Writes a file into the working copy, and does not track it. A file
    /// that was tracked stays tracked.
    pub fn write_untracked(&mut self, path: &str, bytes: impl AsRef<[u8]>) -> &mut Memory {
        self.put(path, FileKind::File, bytes);
        self
    }

    fn put(&mut self, path: &str, kind: FileKind, bytes: impl AsRef<[u8]>) {
        let held = Held {
            kind,
            bytes: bytes.as_ref().to_vec(),
        };
        self.working.insert(path.to_string(), held);
    }

    /// Makes a file in the working copy executable.
    pub fn make_executable(&mut self, path: &str) -> &mut Memory {
        if let Some(held) = self.working.get_mut(path) {
            held.kind = FileKind::Executable;
        }
        self
    }

    /// Removes a file from the working copy, and stops tracking it.
    pub fn remove(&mut self, path: &str) -> &mut Memory {
        self.working.remove(path);
        self.tracked.remove(path);
        self
    }

    /// Stops tracking a file, and leaves it in the working copy.
    pub fn untrack(&mut self, path: &str) -> &mut Memory {
        self.tracked.remove(path);
        self
    }

    /// Moves a file in the working copy, tracked at its new path.
    pub fn rename(&mut self, from: &str, to: &str) -> &mut Memory {
        if let Some(held) = self.working.remove(from) {
            self.tracked.remove(from);
            self.working.insert(to.to_string(), held);
            self.tracked.insert(to.to_string());
        }
        self
    }

    /// Marks a path of the working copy as in a conflict.
    pub fn conflict(&mut self, path: &str) -> &mut Memory {
        self.conflicted.insert(path.to_string());
        self
    }

    /// Records a revision of every tracked file in the working copy, after
    /// the head, and makes it the head.
    pub fn commit(&mut self, subject: &str) -> Revision {
        let n = self.revisions.len() + 1;
        let id = format!("r{n}");
        let files = self
            .working
            .iter()
            .filter(|(path, _)| self.tracked.contains(*path))
            .map(|(path, held)| (path.clone(), held.clone()))
            .collect();
        let info = RevisionInfo {
            revision: Revision::new(&id),
            short: id,
            author: self.author.clone(),
            at: self.next_at,
            offset: self.offset,
            subject: subject.to_string(),
        };
        self.next_at += 60;
        self.revisions.push(Recorded {
            info,
            parent: self.head,
            files,
        });
        self.head = Some(n - 1);
        self.revisions[n - 1].info.revision.clone()
    }

    /// Sets who makes the revisions recorded from now on.
    pub fn author(&mut self, name: &str) -> &mut Memory {
        self.author = name.to_string();
        self
    }

    /// Sets how far the author's clock is from UTC, in seconds, for the
    /// revisions recorded from now on. It is 0 until set.
    pub fn offset(&mut self, seconds: i32) -> &mut Memory {
        self.offset = seconds;
        self
    }

    /// Gives a revision a name that [`Storage::resolve`] reads.
    pub fn name(&mut self, name: &str, revision: &Revision) -> &mut Memory {
        if let Some(i) = self.index(revision) {
            self.names.insert(name.to_string(), i);
        }
        self
    }

    /// Offers a revision as a base, by a name. The name resolves to nothing
    /// unless [`Memory::name`] gives it to a revision too.
    pub fn base(&mut self, name: &str, role: BaseRole, revision: &Revision) -> &mut Memory {
        if let Some(i) = self.index(revision) {
            self.bases.push((name.to_string(), role, i));
        }
        self
    }

    /// Names the branch the working copy is on.
    pub fn branch(&mut self, name: &str) -> &mut Memory {
        self.branch = Some(name.to_string());
        self
    }

    /// Says where the working copy is on disk. Nothing is read from there.
    pub fn on_disk(&mut self, path: impl Into<PathBuf>) -> &mut Memory {
        self.on_disk = Some(path.into());
        self
    }

    fn index(&self, revision: &Revision) -> Option<usize> {
        self.revisions
            .iter()
            .position(|r| &r.info.revision == revision)
    }

    fn recorded(&self, revision: &Revision) -> Result<&Recorded> {
        self.index(revision)
            .map(|i| &self.revisions[i])
            .ok_or_else(|| Error::UnknownRevision(revision.to_string()))
    }

    /// The files at a state. In the working copy, only the tracked ones,
    /// unless untracked ones are asked for too.
    fn files_at(&self, at: &State, untracked: bool) -> Result<Vec<Content>> {
        let files = match at {
            State::Working => &self.working,
            State::At(r) => &self.recorded(r)?.files,
        };
        Ok(files
            .iter()
            .filter(|(path, _)| untracked || *at != State::Working || self.tracked.contains(*path))
            .map(|(path, held)| Content {
                path: path.clone(),
                kind: held.kind,
                bytes: held.bytes.clone(),
            })
            .collect())
    }

    /// The revisions back from start: start, then each parent, as indexes.
    fn back_from(&self, start: Option<usize>) -> Vec<usize> {
        let mut out = Vec::new();
        let mut at = start;
        while let Some(i) = at {
            out.push(i);
            at = self.revisions[i].parent;
        }
        out
    }

    /// Every path a revision changed, against its parent, with a move as
    /// both of its paths.
    fn touched(&self, i: usize) -> Vec<String> {
        let r = &self.revisions[i];
        let parent = r.parent.map(|p| &self.revisions[p].files);
        let empty = Files::new();
        let before = parent.unwrap_or(&empty);
        let mut paths: BTreeSet<String> = BTreeSet::new();
        for (path, held) in before {
            if r.files.get(path) != Some(held) {
                paths.insert(path.clone());
            }
        }
        for (path, held) in &r.files {
            if before.get(path) != Some(held) {
                paths.insert(path.clone());
            }
        }
        paths.into_iter().collect()
    }
}

impl Storage for Memory {
    fn working_copy(&self) -> Option<&Path> {
        self.on_disk.as_deref()
    }

    fn head(&self) -> Result<Option<Revision>> {
        Ok(self.head.map(|i| self.revisions[i].info.revision.clone()))
    }

    fn resolve(&self, name: &str) -> Result<Revision> {
        if let Some(r) = self
            .revisions
            .iter()
            .find(|r| r.info.revision.as_str() == name)
        {
            return Ok(r.info.revision.clone());
        }
        match self.names.get(name) {
            Some(&i) => Ok(self.revisions[i].info.revision.clone()),
            None => Err(Error::UnknownRevision(name.to_string())),
        }
    }

    fn declarations(&self, at: &State) -> Result<Vec<Content>> {
        Ok(self
            .files_at(at, true)?
            .into_iter()
            .filter(|c| is_declaration(&c.path) && c.kind != FileKind::Submodule)
            .collect())
    }

    fn files(&self, at: &State, of: &Subtree) -> Result<Vec<File>> {
        Ok(self
            .files_at(at, true)?
            .into_iter()
            .filter(|c| of.holds(&c.path))
            .map(|c| File {
                path: c.path,
                kind: c.kind,
            })
            .collect())
    }

    fn read(&self, at: &State, paths: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let files: BTreeMap<String, Content> = self
            .files_at(at, true)?
            .into_iter()
            .map(|c| (c.path.clone(), c))
            .collect();
        Ok(paths
            .iter()
            .map(|p| {
                files
                    .get(*p)
                    .filter(|c| c.kind != FileKind::Submodule)
                    .map(|c| c.bytes.clone())
            })
            .collect())
    }

    fn changes(&self, from: Option<&Revision>, to: &State) -> Result<Vec<Change>> {
        let before = match from {
            Some(r) => self.files_at(&State::At(r.clone()), false)?,
            None => Vec::new(),
        };
        let after = self.files_at(to, false)?;
        let mut out = compare(&before, &after);
        if *to == State::Working {
            for path in self.working.keys() {
                if !self.tracked.contains(path) {
                    out.push(Change::new(path, ChangeKind::Untracked));
                }
            }
            out.retain(|c| !self.conflicted.contains(&c.path));
            for path in &self.conflicted {
                out.push(Change::new(path, ChangeKind::Conflicted));
            }
        }
        sort(&mut out);
        Ok(out)
    }

    fn history(
        &self,
        of: &Subtree,
        after: Option<&Revision>,
        limit: Option<usize>,
    ) -> Result<History> {
        let seen: HashSet<usize> = match after {
            Some(r) => {
                let i = self
                    .index(r)
                    .ok_or_else(|| Error::UnknownRevision(r.to_string()))?;
                self.back_from(Some(i)).into_iter().collect()
            }
            None => HashSet::new(),
        };
        let mut history = History::default();
        for i in self.back_from(self.head) {
            if seen.contains(&i) {
                continue;
            }
            let paths = self.touched(i);
            if !paths.iter().any(|p| of.holds(p)) {
                continue;
            }
            history.total += 1;
            if limit.is_none_or(|n| history.revisions.len() < n) {
                history.revisions.push(Touched {
                    info: self.revisions[i].info.clone(),
                    paths,
                });
            }
        }
        Ok(history)
    }

    fn merge_base(&self, a: &Revision, b: &Revision) -> Result<Option<Revision>> {
        let a = self
            .index(a)
            .ok_or_else(|| Error::UnknownRevision(a.to_string()))?;
        let b = self
            .index(b)
            .ok_or_else(|| Error::UnknownRevision(b.to_string()))?;
        let behind_a: HashSet<usize> = self.back_from(Some(a)).into_iter().collect();
        Ok(self
            .back_from(Some(b))
            .into_iter()
            .find(|i| behind_a.contains(i))
            .map(|i| self.revisions[i].info.revision.clone()))
    }

    fn bases(&self, recent: usize) -> Result<Bases> {
        let Some(head) = self.head else {
            return Ok(Bases::default());
        };
        Ok(Bases {
            head: Some(self.revisions[head].info.clone()),
            branch: self.branch.clone(),
            recent: self
                .back_from(Some(head))
                .into_iter()
                .take(recent)
                .map(|i| self.revisions[i].info.clone())
                .collect(),
            named: self
                .bases
                .iter()
                .map(|(name, role, i)| Base {
                    name: name.clone(),
                    role: *role,
                    info: self.revisions[*i].info.clone(),
                })
                .collect(),
        })
    }
}
