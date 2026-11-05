//! What a repository declares, as the base relations of
//! design/model/README.md: rows of `node`, `prop`, `edge`, `edge_prop`,
//! `type`, `type_prop`, `allows`, `at` and `prop_at`.
//!
//! Every reader of a declaration file produces these rows and nothing else,
//! so whatever is built on them, a check or a query, never reads a file
//! itself. There are two readers:
//!
//! - [`piles`]: each pile, from its config and its intent, as `piles` reads
//!   them.
//! - [`declarations`]: every `.toml` file in a `.pile/`, read by the reader
//!   in [`toml`], then checked in [`checks`]. Only what checks becomes a row,
//!   and only a link whose references name nodes becomes an edge. Each one
//!   that does not is a finding, as `pile doctor` reports it.
//!
//! The types are not nodes, since nothing joins one. They have rows of
//! their own: `type`, `type_prop` and `allows`.
//!
//! What is derived stays derived. Which pile holds which, and which pile a
//! path is in, are worked out as `piles` works them out, and are not rows
//! here.
//!
//! Nothing here writes, and nothing here knows how a repository is stored.

use piles::config::{AUX, KEY_ID, KEY_NAME};
use piles::doctor::{Finding, Severity, Survey, split};
use piles::{Pile, Position, Structure, intent, is_toml, shown, structure};
use storage::{Content, FileKind, State, Storage};

pub mod checks;
pub mod toml;

/// The node type every pile is, and nothing else is. The format creates it,
/// and it is the one type no aux pile does.
pub const PILE: &str = "pile";

/// The properties of a pile.
pub const ID: &str = "id";
pub const NAME: &str = "name";
pub const PATH: &str = "path";
pub const HEADLINE: &str = "headline";

/// A node, by its identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Node {
    /// A pile, by its id. So a pile is the same node after it moves.
    Pile(String),
    /// Any other node, by its type and its name, across the repository.
    /// Where it is declared is its home, a column of `node`, and no part of
    /// its identity.
    Named { of_type: String, name: String },
}

impl Node {
    /// How a file names the node: `@name` for a pile, given its name, and
    /// `type:name` for any other node. None for a pile with no name.
    pub fn reference(&self, pile_name: &str) -> Option<String> {
        match self {
            Node::Pile(_) if pile_name.is_empty() => None,
            Node::Pile(_) => Some(piles::typed(pile_name)),
            Node::Named { of_type, name } => Some(format!("{of_type}:{name}")),
        }
    }
}

/// An edge, by its identity: its source, its type and its target. So there
/// is at most one edge of one type between two nodes, wherever its link is
/// written. It is also the row of `edge(Edge, Source, Type, Target)`, since
/// the three decide the edge. No edge joins a node to itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge {
    pub source: Node,
    pub of_type: String,
    pub target: Node,
}

/// What `at` and `prop_at` say where it is written: a node, an edge or a
/// type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Item {
    Node(Node),
    Edge(Edge),
    /// A type an aux pile creates, by its name.
    Type(String),
}

impl From<Node> for Item {
    fn from(n: Node) -> Item {
        Item::Node(n)
    }
}

impl From<Edge> for Item {
    fn from(e: Edge) -> Item {
        Item::Edge(e)
    }
}

impl Item {
    fn is_type(&self, name: &str) -> bool {
        matches!(self, Item::Type(t) if t == name)
    }

    fn is_node(&self, node: &Node) -> bool {
        matches!(self, Item::Node(n) if n == node)
    }

    fn is_edge(&self, edge: &Edge) -> bool {
        matches!(self, Item::Edge(e) if e == edge)
    }
}

/// A property's value: one TOML value, or one item of a list. A date is
/// held as the text it is written as.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// Any string. A pile's name is one too.
    Text(String),
    Integer(i64),
    /// A number with a fraction, as it is written, so that it compares.
    Float(String),
    Boolean(bool),
}

impl std::fmt::Display for Value {
    /// The value as text prints it: a string as it is, unquoted.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Text(s) | Value::Float(s) => f.write_str(s),
            Value::Integer(n) => n.fmt(f),
            Value::Boolean(b) => b.fmt(f),
        }
    }
}

/// A row of `node(Node, Type, Home)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeRow {
    pub node: Node,
    pub of_type: String,
    pub home: Node,
}

/// A row of `prop(Node, Property, Value)`. An unset property has no row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Prop {
    pub node: Node,
    pub property: String,
    pub value: Value,
}

/// A row of `edge_prop(Edge, Property, Value)`. An unset property has no
/// row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeProp {
    pub edge: Edge,
    pub property: String,
    pub value: Value,
}

/// A row of `at(Item, File, Line, Column)`: where a node or an edge is
/// written. The file is a `:/` path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct At {
    pub item: Item,
    pub file: String,
    pub at: Position,
}

/// A row of `prop_at(Item, Property, File, Line, Column)`: where one
/// property's value is written. A value no file writes, such as a pile's
/// path, has no row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropAt {
    pub item: Item,
    pub property: String,
    pub file: String,
    pub at: Position,
}

/// Whether a type is the type of nodes or of edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sort {
    Node,
    Edge,
}

impl Sort {
    /// The word a query and an answer write it as.
    pub fn word(self) -> &'static str {
        match self {
            Sort::Node => "node",
            Sort::Edge => "edge",
        }
    }
}

/// One end of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum End {
    Source,
    Target,
}

impl End {
    /// The word a query and an answer write it as.
    pub fn word(self) -> &'static str {
        match self {
            End::Source => "source",
            End::Target => "target",
        }
    }
}

/// A row of `type(Type, Sort, Home)`: a type an aux pile creates. A type is
/// not a node: nothing joins one. `pile`, which the format creates, has no
/// row.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeOf {
    pub name: String,
    pub sort: Sort,
    /// The aux pile whose `.pile/` creates it.
    pub home: Node,
}

/// A row of `type_prop(Type, Property, Value)`: a type's `title`,
/// `description`, and an edge type's `do` and `dont`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeProp {
    pub of_type: String,
    pub property: String,
    pub value: Value,
}

/// A row of `allows(EdgeType, End, NodeType)`: a node type one end of an
/// edge type may be. `pile` is one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Allows {
    pub edge_type: String,
    pub end: End,
    pub node_type: String,
}

/// The base relations of a repository, each sorted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Relations {
    pub node: Vec<NodeRow>,
    pub prop: Vec<Prop>,
    pub edge: Vec<Edge>,
    pub edge_prop: Vec<EdgeProp>,
    pub at: Vec<At>,
    pub prop_at: Vec<PropAt>,
    pub types: Vec<TypeOf>,
    pub type_prop: Vec<TypeProp>,
    pub allows: Vec<Allows>,
}

impl Relations {
    /// The value of a node's property. None when it is unset.
    pub fn value(&self, node: &Node, property: &str) -> Option<&Value> {
        self.prop
            .iter()
            .find(|p| &p.node == node && p.property == property)
            .map(|p| &p.value)
    }

    /// Where a node is written.
    pub fn at(&self, node: &Node) -> Option<&At> {
        self.at.iter().find(|a| a.item.is_node(node))
    }

    /// Every value of a node's property, a list's in their sorted order.
    pub fn values(&self, node: &Node, property: &str) -> Vec<&Value> {
        self.prop
            .iter()
            .filter(|p| &p.node == node && p.property == property)
            .map(|p| &p.value)
            .collect()
    }

    /// The nodes at home in a pile, other than the pile itself, in order.
    pub fn at_home(&self, pile: &Node) -> Vec<&NodeRow> {
        self.node
            .iter()
            .filter(|n| &n.home == pile && &n.node != pile)
            .collect()
    }

    /// Where a node's property's value is written.
    pub fn prop_at(&self, node: &Node, property: &str) -> Option<&PropAt> {
        self.prop_at
            .iter()
            .find(|p| p.item.is_node(node) && p.property == property)
    }

    /// The node's row, when it is a node.
    pub fn node(&self, node: &Node) -> Option<&NodeRow> {
        self.node.iter().find(|n| &n.node == node)
    }

    /// The types an aux pile creates, in order.
    pub fn types_of(&self, pile: &Node) -> Vec<&TypeOf> {
        self.types.iter().filter(|t| &t.home == pile).collect()
    }

    /// The row of a type, when an aux pile creates one of that name.
    pub fn type_of(&self, name: &str) -> Option<&TypeOf> {
        self.types.iter().find(|t| t.name == name)
    }

    /// The value of a type's property. None when it is unset.
    pub fn type_value(&self, name: &str, property: &str) -> Option<&Value> {
        self.type_prop
            .iter()
            .find(|p| p.of_type == name && p.property == property)
            .map(|p| &p.value)
    }

    /// The node types one end of an edge type may be, in sorted order.
    pub fn allowed(&self, edge_type: &str, end: End) -> Vec<&str> {
        self.allows
            .iter()
            .filter(|a| a.edge_type == edge_type && a.end == end)
            .map(|a| a.node_type.as_str())
            .collect()
    }

    /// Where a type is written: the table that creates it.
    pub fn type_at(&self, name: &str) -> Option<&At> {
        self.at.iter().find(|a| a.item.is_type(name))
    }

    /// Every value of an edge's property, a list's in their sorted order.
    pub fn edge_values(&self, edge: &Edge, property: &str) -> Vec<&Value> {
        self.edge_prop
            .iter()
            .filter(|p| &p.edge == edge && p.property == property)
            .map(|p| &p.value)
            .collect()
    }

    /// Where an edge is written: the table of its link.
    pub fn edge_at(&self, edge: &Edge) -> Option<&At> {
        self.at.iter().find(|a| a.item.is_edge(edge))
    }

    fn sort(&mut self) {
        self.node.sort();
        self.prop.sort();
        self.edge.sort();
        self.edge_prop.sort();
        self.at.sort();
        self.prop_at.sort();
        self.types.sort();
        self.type_prop.sort();
        self.allows.sort();
    }

    fn extend(&mut self, other: Relations) {
        self.node.extend(other.node);
        self.prop.extend(other.prop);
        self.edge.extend(other.edge);
        self.edge_prop.extend(other.edge_prop);
        self.at.extend(other.at);
        self.prop_at.extend(other.prop_at);
        self.types.extend(other.types);
        self.type_prop.extend(other.type_prop);
        self.allows.extend(other.allows);
        self.sort();
    }
}

/// Every row the repository declares at a state: its piles, and what their
/// `.toml` files declare that checks. It fails when the piles cannot be read,
/// as [`piles::discover`] does. What does not check is left out, and
/// `pile doctor` reports it ([`check`]).
pub fn read(storage: &dyn Storage, at: &State) -> Result<Relations, piles::Error> {
    let found = storage.declarations(at)?;
    let s = structure(&found)?;
    let mut r = piles(&s);
    r.extend(declarations(&s.ordered(), &found).relations);
    Ok(r)
}

/// What the `.toml` files declare, as rows, and what is wrong with them.
#[derive(Debug, Default)]
pub struct Declared {
    pub relations: Relations,
    /// Each one names its file from the top, as doctor's findings do.
    pub findings: Vec<Finding>,
}

/// A finding at a place in a declaration file: a line and a column, or the
/// file alone.
pub(crate) fn finding(
    code: &'static str,
    severity: Severity,
    file: &str,
    at: Option<Position>,
    message: impl Into<String>,
    fix: impl Into<String>,
) -> Finding {
    Finding {
        code,
        severity,
        file: file.to_string(),
        line: at.map(|p| p.line),
        column: at.map(|p| p.column),
        message: message.into(),
        fix: fix.into(),
    }
}

/// Reads every `.toml` file in the `.pile/` of one of these piles, and
/// checks what it declares (design/model/README.md, sections 3 to 5). A file
/// in a `.pile/` that is no pile's here is skipped: doctor says what that
/// directory is.
pub fn declarations(piles: &[&Pile], files: &[Content]) -> Declared {
    let mut findings = Vec::new();
    let mut found = Vec::new();
    for c in files {
        if !is_toml(&c.path) {
            continue;
        }
        let Some((dir, _)) = split(&c.path) else {
            continue;
        };
        let Some(i) = piles.iter().position(|p| p.path == dir) else {
            continue;
        };
        let pile = piles[i];
        if c.kind == FileKind::Symlink {
            findings.push(finding(
                "unreadable-declaration",
                Severity::Problem,
                &c.path,
                None,
                "the file is a symbolic link, and a declaration file is a file",
                "replace the link with the file it points to",
            ));
            continue;
        }
        let read = toml::read(&c.path, &c.bytes);
        findings.extend(read.findings);
        for table in read.tables {
            let declares_type = matches!(
                table.header,
                toml::Header::NodeType | toml::Header::EdgeType
            );
            if declares_type && pile.kind != AUX {
                findings.push(finding(
                    "types-outside-aux",
                    Severity::Problem,
                    &table.file,
                    Some(table.at),
                    "a type is created only in an aux pile, and this pile is not one",
                    format!(
                        "move the table to an aux pile: a pile whose config says kind = {AUX} \
                         (pile guide aux)"
                    ),
                ));
                continue;
            }
            found.push(checks::Found { pile: i, table });
        }
    }
    let mut relations = Relations::default();
    checks::check(piles, found, &mut findings, &mut relations);
    relations.sort();
    findings.sort_by(|a, b| (&a.file, a.line, a.column).cmp(&(&b.file, b.line, b.column)));
    Declared {
        relations,
        findings,
    }
}

/// Adds what is wrong with the `.toml` files of the working copy to a survey
/// `pile doctor` made of its piles, so doctor gives one report. Only the
/// piles whose config reads are read: a pile whose config is refused has its
/// own finding first.
pub fn check(survey: &mut Survey, files: &[Content]) {
    let piles: Vec<Pile> = survey
        .rows
        .iter()
        .filter_map(|r| {
            r.config.clone().map(|config| Pile {
                path: r.path.clone(),
                config,
                intent: None,
            })
        })
        .collect();
    let refs: Vec<&Pile> = piles.iter().collect();
    survey.add(declarations(&refs, files).findings);
}

/// The rows the piles declare: each pile a node of type [`PILE`], at its
/// config, line 1, column 1, with its id, name, path and headline.
pub fn piles(s: &Structure) -> Relations {
    let mut r = Relations::default();
    for pile in s.ordered() {
        add_pile(&mut r, pile);
    }
    r.sort();
    r
}

fn add_pile(r: &mut Relations, pile: &Pile) {
    let node = Node::Pile(pile.id.clone());
    let config = shown(&pile.config_path());
    r.node.push(NodeRow {
        node: node.clone(),
        of_type: PILE.to_string(),
        home: node.clone(),
    });
    r.at.push(At {
        item: node.clone().into(),
        file: config.clone(),
        at: Position { line: 1, column: 1 },
    });

    let mut prop = |property: &str, value: Value, written: Option<(String, Position)>| {
        r.prop.push(Prop {
            node: node.clone(),
            property: property.to_string(),
            value,
        });
        if let Some((file, at)) = written {
            r.prop_at.push(PropAt {
                item: node.clone().into(),
                property: property.to_string(),
                file,
                at,
            });
        }
    };
    let in_config = |key| pile.config.at.get(key).map(|at| (config.clone(), *at));
    prop(ID, Value::Text(pile.id.clone()), in_config(KEY_ID));
    if !pile.name.is_empty() {
        prop(NAME, Value::Text(pile.name.clone()), in_config(KEY_NAME));
    }
    prop(PATH, Value::Text(shown(&pile.path)), None);
    let headline = pile.headline();
    if !headline.is_empty() {
        let written = pile
            .intent
            .as_deref()
            .and_then(intent::headline_at)
            .map(|at| (shown(&pile.intent_path()), at));
        prop(HEADLINE, Value::Text(headline), written);
    }
}

#[cfg(test)]
mod declared_tests;
#[cfg(test)]
mod tests;
