//! What changed, arranged by the pile each change is in.
//!
//! Every change goes to the nearest pile above its path, so a change under a
//! child pile is the child's and not also its parent's. That is decided
//! against every pile in the repository, even when only some are reported.
//!
//! A move inside one pile is one entry, with where it came from. A move
//! across a boundary is two entries, one half in each pile: an addition
//! where it arrived and a deletion where it left, each naming the far end.
//! A change in no pile is kept, never dropped. Every path in an entry is
//! from the top of the repository.

use storage::{Change, ChangeKind};

use crate::{PILE_DIR, Pile, Structure, is_toml, join, under};

/// One change, as one pile sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub kind: ChangeKind,
    /// The file's kind changed too: it became executable, or a symbolic
    /// link, or the reverse.
    pub mode: bool,
    /// Where a moved path came from. Empty otherwise.
    pub from: String,
    /// Where a path that left this pile went. Empty otherwise.
    pub to: String,
    /// It is one half of a move across a boundary.
    pub crossed: bool,
}

impl Entry {
    fn new(path: &str, kind: ChangeKind, mode: bool) -> Entry {
        Entry {
            path: path.to_string(),
            kind,
            mode,
            from: String::new(),
            to: String::new(),
            crossed: false,
        }
    }

    /// The letter a change is printed as: `A`, `M`, `D`, `R` for a move,
    /// `?` for untracked and `U` for a conflict.
    pub fn letter(&self) -> &'static str {
        letter(self.kind)
    }
}

/// The letter a kind of change is printed as.
pub fn letter(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "A",
        ChangeKind::Modified => "M",
        ChangeKind::Deleted => "D",
        ChangeKind::Moved => "R",
        ChangeKind::Untracked => "?",
        ChangeKind::Conflicted => "U",
    }
}

/// One pile, and what changed in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group<'a> {
    pub pile: &'a Pile,
    /// Sorted by path.
    pub entries: Vec<Entry>,
}

impl Group<'_> {
    pub fn changed(&self) -> bool {
        !self.entries.is_empty()
    }

    /// How many entries are untracked files.
    pub fn untracked(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.kind == ChangeKind::Untracked)
            .count()
    }

    /// How many entries are halves of a move across a boundary.
    pub fn crossed(&self) -> usize {
        self.entries.iter().filter(|e| e.crossed).count()
    }

    /// The change wrote the pile's intent: added it, changed it, removed
    /// it, or moved it here.
    pub fn intent_written(&self) -> bool {
        let intent = self.pile.intent_path();
        self.entries.iter().any(|e| e.path == intent)
    }

    /// The change wrote one of the pile's declaration files, the `.toml`
    /// files in its `.pile/` that hold its links, types and nodes: added
    /// one, changed one, removed one, or moved one here.
    pub fn declarations_written(&self) -> bool {
        let dir = join(&self.pile.path, &[PILE_DIR]);
        self.entries.iter().any(|e| {
            is_toml(&e.path)
                && e.path
                    .strip_prefix(&dir)
                    .and_then(|rest| rest.strip_prefix('/'))
                    .is_some_and(|name| !name.contains('/'))
        })
    }
}

/// The changes, arranged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View<'a> {
    /// The piles in scope, in path order, each with its changes.
    pub groups: Vec<Group<'a>>,
    /// The changes in scope that are in no pile, from the top, sorted.
    pub in_no_pile: Vec<Entry>,
}

/// Arranges changes by pile, for the piles under scope. A change in no pile
/// is kept only when it is under scope too, so standing in one pile does not
/// list every stray path in the repository.
pub fn arrange<'a>(s: &'a Structure, scope: &str, changes: &[Change]) -> View<'a> {
    let mut groups: Vec<Group> = s
        .from(scope)
        .into_iter()
        .map(|pile| Group {
            pile,
            entries: Vec::new(),
        })
        .collect();
    let mut in_no_pile = Vec::new();
    let mut add = |pile: &str, e: Entry| {
        if let Some(g) = groups.iter_mut().find(|g| g.pile.path == pile) {
            g.entries.push(e);
        }
    };

    for c in changes {
        let path = c.path.as_str();
        let from = c.from.as_deref().unwrap_or("");
        let left = (!from.is_empty()).then(|| s.owner(from)).flatten();

        let Some(owner) = s.owner(path) else {
            if under(scope, path) {
                let mut e = Entry::new(path, c.kind, c.retyped);
                if left.is_some() {
                    e.kind = ChangeKind::Added;
                    e.crossed = true;
                }
                e.from = from.to_string();
                in_no_pile.push(e);
            }
            if let Some(was) = left {
                let mut e = Entry::new(from, ChangeKind::Deleted, false);
                e.to = path.to_string();
                e.crossed = true;
                add(&was.path, e);
            }
            continue;
        };

        if from.is_empty() {
            add(&owner.path, Entry::new(path, c.kind, c.retyped));
            continue;
        }

        if left.is_some_and(|w| w.path == owner.path) {
            let mut e = Entry::new(path, c.kind, c.retyped);
            e.from = from.to_string();
            add(&owner.path, e);
            continue;
        }

        let mut arrived = Entry::new(path, ChangeKind::Added, c.retyped);
        arrived.from = from.to_string();
        arrived.crossed = true;
        add(&owner.path, arrived);

        let mut gone = Entry::new(from, ChangeKind::Deleted, false);
        gone.to = path.to_string();
        gone.crossed = true;
        match left {
            Some(was) => add(&was.path, gone),
            None if under(scope, from) => in_no_pile.push(gone),
            None => {}
        }
    }

    for g in &mut groups {
        g.entries.sort_by(|a, b| a.path.cmp(&b.path));
    }
    in_no_pile.sort_by(|a, b| a.path.cmp(&b.path));
    View { groups, in_no_pile }
}
