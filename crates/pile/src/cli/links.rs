//! A pile's links, as `show`, `which` and `list` print them: every link its
//! `.pile/` writes, whatever its ends, and every link elsewhere whose end is
//! the pile or a node at home in it.
//!
//! An arrow says which way an edge goes, seen from the pile: `->` for an
//! edge whose source is the pile, `<-` for one whose target is, and `--` for
//! one with the pile at neither end, which shows both. An end is named as a
//! reference is written, `@name` or `type:name`, and a pile with no name by
//! its path, which is how it is typed. In a result the arrow is `direction`,
//! `to`, `from` or `between`.

use serde_json::{Map, Value, json};

use graph::{Edge, Node, Relations};
use piles::{Pile, Structure};

/// Which way an edge goes, seen from a pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Direction {
    /// The pile is its source.
    To,
    /// The pile is its target.
    From,
    /// The pile is at neither end.
    Between,
}

/// One link of a pile.
pub(super) struct Link {
    pub direction: Direction,
    pub of_type: String,
    /// The other end, as it is typed: `@name`, a pile's path when it has no
    /// name, or `type:name`. For a link between two other nodes, both ends,
    /// `type:a -> type:b`, as text prints them; a result gives null.
    pub other: String,
    /// Each end, as it is typed.
    pub source: String,
    pub target: String,
    /// The other end, when it is a pile.
    pub other_pile: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    /// Every other property the edge gives, with its values.
    pub values: Vec<(String, Vec<String>)>,
    /// Where its table starts, as `:/path:line:column`.
    pub at: Option<String>,
    /// The pile whose `.pile/` writes the link, as it is typed.
    pub written_in: Option<String>,
}

impl Link {
    pub fn arrow(&self) -> &'static str {
        match self.direction {
            Direction::To => "->",
            Direction::From => "<-",
            Direction::Between => "--",
        }
    }

    fn direction(&self) -> &'static str {
        match self.direction {
            Direction::To => "to",
            Direction::From => "from",
            Direction::Between => "between",
        }
    }

    /// The other end in a result: null for a link with this pile at
    /// neither end, whose ends are `source` and `target`.
    fn other_in_result(&self) -> Option<&str> {
        (self.direction != Direction::Between).then_some(self.other.as_str())
    }

    /// The link as `which` and `list` give it in a result.
    pub fn brief(&self) -> Value {
        json!({
            "direction": self.direction(),
            "type": self.of_type,
            "other": self.other_in_result(),
            "source": self.source,
            "target": self.target,
        })
    }

    /// The link as `show` gives it in a result.
    pub fn whole(&self) -> Value {
        let mut values = Map::new();
        for (property, said) in &self.values {
            values.insert(property.clone(), json!(said));
        }
        json!({
            "direction": self.direction(),
            "type": self.of_type,
            "other": self.other_in_result(),
            "source": self.source,
            "target": self.target,
            "title": self.title,
            "description": self.description,
            "values": values,
            "at": self.at,
            "written_in": self.written_in,
        })
    }
}

/// A node as it is typed: a pile by its handle, and any other node by its
/// reference.
fn typed(s: &Structure, node: &Node) -> String {
    match node {
        Node::Pile(id) => s
            .ordered()
            .into_iter()
            .find(|p| &p.id == id)
            .map_or_else(|| id.clone(), |p| p.handle()),
        Node::Named { of_type, name } => format!("{of_type}:{name}"),
    }
}

/// The pile whose `.pile/` holds a file printed from the top.
fn pile_of<'a>(s: &'a Structure, file: &str) -> Option<&'a Pile> {
    let file = file.strip_prefix(':').unwrap_or(file);
    let (dir, _) = piles::doctor::split(file)?;
    s.find(&dir)
}

/// The links of a pile: the edges from it, then the edges to it, then the
/// others it writes or that end at a node at home in it, each in the order
/// they are written, by file and line.
pub(super) fn of(rows: &Relations, s: &Structure, pile: &Pile) -> Vec<Link> {
    let me = Node::Pile(pile.id.clone());
    let home = |n: &Node| rows.node(n).is_some_and(|r| r.home == me);
    let mut mine: Vec<(Direction, &Edge)> = rows
        .edge
        .iter()
        .filter_map(|e| {
            let direction = if e.source == me {
                Direction::To
            } else if e.target == me {
                Direction::From
            } else if home(&e.source)
                || home(&e.target)
                || rows
                    .edge_at(e)
                    .and_then(|a| pile_of(s, &a.file))
                    .is_some_and(|p| p.id == pile.id)
            {
                Direction::Between
            } else {
                return None;
            };
            Some((direction, e))
        })
        .collect();
    let order = |d: Direction| match d {
        Direction::To => 0,
        Direction::From => 1,
        Direction::Between => 2,
    };
    mine.sort_by_key(|(d, e)| (order(*d), rows.edge_at(e).map(|a| (a.file.clone(), a.at))));
    mine.into_iter()
        .map(|(direction, e)| {
            let first = |p: &str| rows.edge_values(e, p).first().map(|v| v.to_string());
            let mut values: Vec<(String, Vec<String>)> = Vec::new();
            for p in rows.edge_prop.iter().filter(|p| &p.edge == e) {
                if p.property == "title" || p.property == "description" {
                    continue;
                }
                match values.iter_mut().find(|(name, _)| *name == p.property) {
                    Some((_, said)) => said.push(p.value.to_string()),
                    None => values.push((p.property.clone(), vec![p.value.to_string()])),
                }
            }
            let (source, target) = (typed(s, &e.source), typed(s, &e.target));
            let (other, other_node) = match direction {
                Direction::To => (target.clone(), Some(&e.target)),
                Direction::From => (source.clone(), Some(&e.source)),
                Direction::Between => (format!("{source} -> {target}"), None),
            };
            let at = rows.edge_at(e);
            Link {
                direction,
                of_type: e.of_type.clone(),
                other_pile: other_node
                    .filter(|n| matches!(n, Node::Pile(_)))
                    .map(|_| other.clone()),
                other,
                source,
                target,
                title: first("title"),
                description: first("description"),
                values,
                at: at.map(|a| format!("{}:{}:{}", a.file, a.at.line, a.at.column)),
                written_in: at.and_then(|a| pile_of(s, &a.file)).map(|p| p.handle()),
            }
        })
        .collect()
}

/// The other ends of a pile's links, once each, `->` first: what `which`
/// prints on its links lines.
pub(super) fn ends(links: &[Link]) -> Vec<String> {
    let mut lines = Vec::new();
    for (direction, arrow) in [(Direction::To, "->"), (Direction::From, "<-")] {
        let mut others: Vec<&str> = Vec::new();
        for l in links.iter().filter(|l| l.direction == direction) {
            if !others.contains(&l.other.as_str()) {
                others.push(&l.other);
            }
        }
        if !others.is_empty() {
            lines.push(format!("{arrow} {}", others.join(", ")));
        }
    }
    lines
}
