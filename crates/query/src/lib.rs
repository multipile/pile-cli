//! What a repository declares, as a Datalog database, and a program asked of
//! it.
//!
//! [`database`] holds the graph's base relations by their names, each row a
//! row of values, and `parent`, which is derived from where the piles sit.
//! [`ask`] parses a program and evaluates it against a copy of the database,
//! so a relation the program defines is the program's alone. It returns an
//! [`Answer`] for each query, or says why the program is [`Refused`], and
//! where.
//!
//! The relations, and the columns of each:
//!
//! ```text
//! node(Node, Type, Home)
//! prop(Node, Property, Value)
//! edge(Edge, Source, Type, Target)
//! edge_prop(Edge, Property, Value)
//! type(Type, Sort, Home)
//! type_prop(Type, Property, Value)
//! allows(EdgeType, End, NodeType)
//! at(Item, File, Line, Column)
//! prop_at(Item, Property, File, Line, Column)
//! parent(Child, Parent)
//! ```
//!
//! A value is an integer or a symbol. Every node is one symbol, and so is
//! every edge:
//!
//! - A pile is `@name`. A pile with no name, or with a name another pile
//!   claims too, is its path from the top, `:/lib/sub`.
//! - Any other node is `type:name`: `external:git`.
//! - An edge is its source, its type and its target, with a space between:
//!   `@app uses external:git`. It is the first column of `edge`.
//!
//! [`Database::node`] and [`Database::edge`] lead from a value of an answer
//! back to the graph's node or edge, and [`Database::symbol`] and
//! [`Database::written`] write a node and an edge as an answer does.
//!
//! A property's value is an integer when it is written as one, and a symbol
//! otherwise: text as it is, a number with a fraction as it is written, and
//! a boolean as `true` or `false`. A line and a column are integers.
//!
//! `@word` in a program is a node the caller names. [`Database::pile_named`]
//! is what every caller passes so far: the pile of that name.
//!
//! Nothing here reads a repository, and nothing here writes.

use std::collections::BTreeMap;

pub use datalog::{Answer, Value};
use graph::{Edge, Item, Node, Relations};
use piles::Structure;

/// Whose the relations of a database are, as a program that defines one of
/// them again is told.
const HELD_BY: &str = "pile";
/// Whose the relations a program defines are. No later program reads them,
/// since each is asked of a copy.
const PROGRAM: &str = "the program";

/// The relations a program reads, in the order the guide lists them: each
/// one's name, its columns, and what one row of it says.
pub const RELATIONS: &[(&str, &[&str], &str)] = &[
    (
        "node",
        &["Node", "Type", "Home"],
        "a node, its type, and the pile it is at home in",
    ),
    (
        "prop",
        &["Node", "Property", "Value"],
        "one value of a node's property",
    ),
    (
        "edge",
        &["Edge", "Source", "Type", "Target"],
        "an edge, the node it is from, its type, and the node it is to",
    ),
    (
        "edge_prop",
        &["Edge", "Property", "Value"],
        "one value of an edge's property",
    ),
    (
        "type",
        &["Type", "Sort", "Home"],
        "a type, node or edge as its sort, and the aux pile that creates it",
    ),
    (
        "type_prop",
        &["Type", "Property", "Value"],
        "one value of a type's property",
    ),
    (
        "allows",
        &["EdgeType", "End", "NodeType"],
        "a node type an edge type allows at its source or its target",
    ),
    (
        "at",
        &["Item", "File", "Line", "Column"],
        "where a node, an edge or a type is written",
    ),
    (
        "prop_at",
        &["Item", "Property", "File", "Line", "Column"],
        "where one property's value is written",
    ),
    (
        "parent",
        &["Child", "Parent"],
        "a pile, and the pile it sits in",
    ),
];

/// The graph of one state of a repository, as a program reads it: the
/// relations, and which symbol is which node and which edge.
#[derive(Debug, Clone, Default)]
pub struct Database {
    held: datalog::Database,
    symbols: BTreeMap<Node, String>,
    nodes: BTreeMap<String, Node>,
    /// Each edge's symbol, written once.
    edge_symbols: BTreeMap<Edge, String>,
    edges: BTreeMap<String, Edge>,
    /// Each name exactly one pile claims, and that pile.
    names: BTreeMap<String, Node>,
}

impl Database {
    /// The symbol a node is written as. None for a node the graph does not
    /// hold.
    pub fn node_symbol(&self, node: &Node) -> Option<&str> {
        self.symbols.get(node).map(String::as_str)
    }

    /// The node a value of an answer is. None for an integer, and for a
    /// symbol that is no node's.
    pub fn node(&self, value: &Value) -> Option<&Node> {
        match value {
            Value::Symbol(s) => self.nodes.get(s),
            Value::Integer(_) => None,
        }
    }

    /// The edge a value of an answer is: a value of the first column of
    /// `edge`. None for an integer, and for a symbol that is no edge's.
    pub fn edge(&self, value: &Value) -> Option<&Edge> {
        match value {
            Value::Symbol(s) => self.edges.get(s),
            Value::Integer(_) => None,
        }
    }

    /// The pile of a name, which is what `@name` in a program is. None for
    /// a name no pile claims, and for one two piles claim, which names
    /// neither.
    pub fn pile_named(&self, name: &str) -> Option<Node> {
        self.names.get(name).cloned()
    }

    /// The rows of a relation, sorted.
    ///
    /// # Panics
    ///
    /// If it is none of [`RELATIONS`].
    pub fn rows(&self, relation: &str) -> Vec<Vec<Value>> {
        self.held.rows(relation)
    }

    /// An edge's symbol, the first column of `edge`: its source, its type
    /// and its target. An edge the graph does not hold is written as it
    /// would be.
    pub fn written(&self, edge: &Edge) -> String {
        match self.edge_symbols.get(edge) {
            Some(s) => s.clone(),
            None => format!(
                "{} {} {}",
                self.symbol(&edge.source),
                edge.of_type,
                self.symbol(&edge.target)
            ),
        }
    }

    /// A node's symbol, or how it would be written if the graph held it.
    pub fn symbol(&self, node: &Node) -> String {
        match self.symbols.get(node) {
            Some(s) => s.clone(),
            None => match node {
                Node::Pile(id) => id.clone(),
                Node::Named { of_type, name } => format!("{of_type}:{name}"),
            },
        }
    }

    fn item(&self, item: &Item) -> String {
        match item {
            Item::Node(n) => self.symbol(n),
            Item::Edge(e) => self.written(e),
            Item::Type(t) => t.clone(),
        }
    }
}

fn symbol(s: impl Into<String>) -> Value {
    Value::Symbol(s.into())
}

fn integer(n: usize) -> Value {
    Value::Integer(i64::try_from(n).unwrap_or(i64::MAX))
}

/// A property's value as a program compares it: an integer stays one, and
/// everything else is a symbol, as it is written.
fn value(v: &graph::Value) -> Value {
    match v {
        graph::Value::Integer(n) => Value::Integer(*n),
        other => symbol(other.to_string()),
    }
}

/// The database of a graph: the rows of each base relation, and `parent`
/// from where the piles sit. The rows and the piles are of one state of one
/// repository: what `graph::read` and `piles::structure` give of the same
/// files, or what `graph::check` returns for doctor.
pub fn database(rows: &Relations, s: &Structure) -> Database {
    let mut db = Database::default();
    for (name, columns, title) in RELATIONS {
        db.held.declare(name, columns, HELD_BY, title);
    }

    // A pile is written by its name where the name is its alone, and by its
    // path where it is not, so no two piles share a symbol.
    let twice = s.duplicate_names();
    for pile in s.ordered() {
        let node = Node::Pile(pile.id.clone());
        let named = !pile.name.is_empty() && !twice.contains_key(&pile.name);
        let symbol = if named {
            db.names.insert(pile.name.clone(), node.clone());
            piles::typed(&pile.name)
        } else {
            piles::shown(&pile.path)
        };
        db.symbols.entry(node).or_insert(symbol);
    }
    for n in &rows.node {
        let symbol = db.symbol(&n.node);
        db.symbols.entry(n.node.clone()).or_insert(symbol);
    }
    for (node, symbol) in &db.symbols {
        db.nodes.insert(symbol.clone(), node.clone());
    }
    for e in &rows.edge {
        let symbol = db.written(e);
        db.edge_symbols.insert(e.clone(), symbol.clone());
        db.edges.insert(symbol, e.clone());
    }

    for n in &rows.node {
        let row = vec![
            symbol(db.symbol(&n.node)),
            symbol(&n.of_type),
            symbol(db.symbol(&n.home)),
        ];
        db.held.add_values("node", &row);
    }
    for p in &rows.prop {
        let row = vec![
            symbol(db.symbol(&p.node)),
            symbol(&p.property),
            value(&p.value),
        ];
        db.held.add_values("prop", &row);
    }
    for e in &rows.edge {
        let row = vec![
            symbol(db.written(e)),
            symbol(db.symbol(&e.source)),
            symbol(&e.of_type),
            symbol(db.symbol(&e.target)),
        ];
        db.held.add_values("edge", &row);
    }
    for p in &rows.edge_prop {
        let row = vec![
            symbol(db.written(&p.edge)),
            symbol(&p.property),
            value(&p.value),
        ];
        db.held.add_values("edge_prop", &row);
    }
    for t in &rows.types {
        let row = vec![
            symbol(&t.name),
            symbol(t.sort.word()),
            symbol(db.symbol(&t.home)),
        ];
        db.held.add_values("type", &row);
    }
    for p in &rows.type_prop {
        let row = vec![symbol(&p.of_type), symbol(&p.property), value(&p.value)];
        db.held.add_values("type_prop", &row);
    }
    for a in &rows.allows {
        let row = vec![
            symbol(&a.edge_type),
            symbol(a.end.word()),
            symbol(&a.node_type),
        ];
        db.held.add_values("allows", &row);
    }
    for a in &rows.at {
        let row = vec![
            symbol(db.item(&a.item)),
            symbol(&a.file),
            integer(a.at.line),
            integer(a.at.column),
        ];
        db.held.add_values("at", &row);
    }
    for p in &rows.prop_at {
        let row = vec![
            symbol(db.item(&p.item)),
            symbol(&p.property),
            symbol(&p.file),
            integer(p.at.line),
            integer(p.at.column),
        ];
        db.held.add_values("prop_at", &row);
    }
    for pile in s.ordered() {
        if let Some(parent) = s.parent(&pile.path) {
            let row = vec![
                symbol(db.symbol(&Node::Pile(pile.id.clone()))),
                symbol(db.symbol(&Node::Pile(parent.id.clone()))),
            ];
            db.held.add_values("parent", &row);
        }
    }
    db
}

/// Why a program was refused, and where: the line and the column in the
/// program's text, each counting from one, the column in characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl From<datalog::Error> for Refused {
    fn from(e: datalog::Error) -> Refused {
        Refused {
            line: e.span.line,
            column: e.span.column,
            message: e.message,
        }
    }
}

/// Asks a program of a database: an answer for each of its queries, in the
/// order they are written. A program with no query has no answer.
///
/// The program is evaluated against a copy, so what it defines does not
/// stay, and the next program asked of the same database sees the base
/// relations and `parent` alone.
///
/// `names` says which node each `@word` is, given the word without its `@`:
/// pass [`Database::pile_named`] for the pile of that name. A word it names
/// nothing for is refused, where it is written, and so is a node the graph
/// does not hold. Whatever the parser or the evaluator refuses is refused
/// the same way, with what it said.
pub fn ask(
    db: &Database,
    program: &str,
    names: impl Fn(&str) -> Option<Node>,
) -> Result<Vec<Answer>, Refused> {
    let parsed = datalog::parse(program)?;
    let resolve = |word: &str| -> Result<Value, String> {
        names(word)
            .and_then(|node| db.node_symbol(&node).map(symbol))
            .ok_or_else(|| format!("no pile is named @{word}"))
    };
    let mut copy = db.held.clone();
    Ok(datalog::evaluate(&mut copy, &parsed, PROGRAM, &resolve)?)
}

/// The values of one column of an answer, a value per row, by the name of
/// the variable the query binds: `Node`, say. None when the query binds no
/// such variable.
pub fn column<'a>(answer: &'a Answer, variable: &str) -> Option<Vec<&'a Value>> {
    let i = answer.columns.iter().position(|c| c == variable)?;
    Some(answer.rows.iter().map(|row| &row[i]).collect())
}

#[cfg(test)]
mod tests;
