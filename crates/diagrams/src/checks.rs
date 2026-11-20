//! A diagram held to the graph: which node each element stands for, which
//! edge each line does, what the diagram should show and does not, and
//! every finding on the way.
//!
//! [`crate::meta`] reads what the file says. Here each thing it says is
//! looked up in the graph. Without a graph, only what the file alone shows
//! is reported, and the diagram stands for nothing.

use std::collections::BTreeSet;

use diagram_format::Kind;
use diagram_format::finding::nearest;
use graph::{Edge, Node, Relations};
use query::{Database, Value};

use crate::meta::{self, Include, NamedAs, Place, Wrong};

/// The graph a diagram is held to: the rows of one state of a repository,
/// and the database made of them, which a program is asked of.
#[derive(Debug, Clone, Copy)]
pub struct Graph<'g> {
    pub rows: &'g Relations,
    pub db: &'g Database,
}

impl Graph<'_> {
    /// A pile's name. None for a pile with no name, and for any other node.
    fn pile_name(&self, pile: &Node) -> Option<&str> {
        match self.rows.value(pile, graph::NAME) {
            Some(graph::Value::Text(name)) => Some(name),
            _ => None,
        }
    }

    /// The ids of the piles of a name.
    fn ids_named(&self, name: &str) -> Vec<&str> {
        self.rows
            .prop
            .iter()
            .filter(|p| {
                p.property == graph::NAME && matches!(&p.value, graph::Value::Text(n) if n == name)
            })
            .filter_map(|p| match &p.node {
                Node::Pile(id) => Some(id.as_str()),
                Node::Named { .. } => None,
            })
            .collect()
    }

    /// A node as a box names one: its name, the id of the pile it is or is
    /// at home in, and its type when it is not a pile.
    fn named_as(&self, node: &Node) -> (String, String, Option<String>) {
        match node {
            Node::Pile(id) => (
                self.pile_name(node).unwrap_or("<name>").to_string(),
                id.clone(),
                None,
            ),
            Node::Named { of_type, name } => {
                let home = match self.rows.home(node) {
                    Some(Node::Pile(id)) => id.clone(),
                    _ => "<id>".to_string(),
                };
                (name.clone(), home, Some(of_type.clone()))
            }
        }
    }
}

/// An element that stands for a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stands {
    /// The element's key, which a mark names it by.
    pub key: String,
    pub node: Node,
}

/// A line that stands for an edge, whether or not a link declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draws {
    /// The keys of the two elements the line joins.
    pub from: String,
    pub to: String,
    pub edge: Edge,
}

/// What a diagram stands for: a node for each element that names one, and
/// an edge for each line that names one between two such elements.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Standing {
    pub elements: Vec<Stands>,
    pub lines: Vec<Draws>,
}

impl Standing {
    fn node(&self, key: &str) -> Option<&Node> {
        self.elements.iter().find(|e| e.key == key).map(|e| &e.node)
    }

    fn draws(&self, node: &Node) -> Option<&Stands> {
        self.elements.iter().find(|e| &e.node == node)
    }
}

/// The elements and the lines of a diagram's second version that stand for
/// something other than they did in its first, each by its key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Swapped {
    /// The keys of the elements that stand for another node.
    pub elements: Vec<String>,
    /// The two keys of each line that stands for another edge.
    pub lines: Vec<(String, String)>,
}

/// What of a diagram's second version stands for something other than it
/// did in its first: an element of the same key that stood for a node and
/// now stands for another, and a line between the same two keys that stood
/// for an edge and now stands for another. A pile is the same node
/// whatever its name and wherever it is, any other node is the same by its
/// type and its name, and an edge by its source, its type and its target.
/// How the file writes any of it does not count. Neither does an element
/// or a line that stands for nothing in one version: it gained or lost
/// what it stands for, and shows what it did.
pub fn swapped(before: &Standing, after: &Standing) -> Swapped {
    Swapped {
        elements: after
            .elements
            .iter()
            .filter(|now| before.node(&now.key).is_some_and(|was| was != &now.node))
            .map(|now| now.key.clone())
            .collect(),
        lines: after
            .lines
            .iter()
            .filter(|now| {
                before
                    .lines
                    .iter()
                    .find(|was| was.from == now.from && was.to == now.to)
                    .is_some_and(|was| was.edge != now.edge)
            })
            .map(|now| (now.from.clone(), now.to.clone()))
            .collect(),
    }
}

/// A diagram held to the repository.
#[derive(Debug, Clone, Default)]
pub(crate) struct Held {
    /// Everything wrong with it, in the order it is found.
    pub wrong: Vec<Wrong>,
    pub standing: Standing,
    /// The nodes and the edges the diagram should show and does not, each
    /// as an answer writes it.
    pub misses: Vec<String>,
}

/// The node named by an id, at a place. One that is not found is reported
/// to `wrong`, where it is written, and the error is the node that was
/// meant, when the id finds a pile.
fn node_of(
    g: Graph,
    n: &NamedAs,
    place: Place,
    wrong: &mut Vec<Wrong>,
) -> Result<Node, Option<Node>> {
    let (word, id_key) = (place.word(), place.id_key());
    let mut not = |code: &'static str, message: String, fix: String, meant: Option<Node>| {
        wrong.push(Wrong::new(code, n.offset, message, fix));
        Err(meant)
    };
    let pile = Node::Pile(n.id.clone());
    if g.rows.node(&pile).is_none() {
        let fix = match &n.node_type {
            None => match g.ids_named(&n.name).as_slice() {
                [one] => format!(
                    "the pile named {} has the id {one}: write {id_key}=\"{one}\"",
                    n.name
                ),
                _ => "copy the pile's id from pile list --full".to_string(),
            },
            Some(of_type) => match g.rows.named(of_type, &n.name).map(|row| &row.home) {
                Some(home @ Node::Pile(id)) => format!(
                    "{of_type}:{} is at home in {}: write {id_key}=\"{id}\"",
                    n.name,
                    g.db.symbol(home)
                ),
                _ => "copy the id of the pile the node is at home in from pile list --full"
                    .to_string(),
            },
        };
        return not(
            "diagram-no-such-pile",
            format!("no pile has the id {}", n.id),
            fix,
            None,
        );
    }
    let Some(of_type) = &n.node_type else {
        return match g.pile_name(&pile) {
            Some(actual) if actual != n.name => not(
                "diagram-name-differs",
                format!(
                    "the pile with this id is named {actual}, and the {word} is named {}",
                    n.name
                ),
                match place {
                    Place::Meta(_) => {
                        format!("name it as the pile is named: {word} \"{actual}\"")
                    }
                    Place::Element(_) => format!(
                        "name the {word} {actual}, as the pile is named, or give \
                         pile:name=\"{actual}\" when it shows another name"
                    ),
                },
                Some(pile),
            ),
            _ => Ok(pile),
        };
    };
    let node = Node::Named {
        of_type: of_type.clone(),
        name: n.name.clone(),
    };
    match g.rows.home(&node) {
        Some(home) if home == &pile => Ok(node),
        Some(home) => not(
            "diagram-no-such-node",
            format!(
                "{of_type}:{} is at home in {}, and the id given is that of {}",
                n.name,
                g.db.symbol(home),
                g.db.symbol(&pile)
            ),
            match home {
                Node::Pile(id) => format!("write {id_key}=\"{id}\""),
                Node::Named { .. } => "write the id of the pile that declares it".to_string(),
            },
            Some(node),
        ),
        None => {
            let names: Vec<&str> = g
                .rows
                .node
                .iter()
                .filter_map(|row| match &row.node {
                    Node::Named { of_type: t, name } if t == of_type => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            let fix = match nearest(&n.name, names.iter().copied()) {
                Some(near) => format!("did you mean {of_type}:{near}?"),
                None if g.rows.type_of(of_type).is_none() => format!(
                    "no aux pile creates the node type {of_type}: name a node pile query '?- \
                     node(Node, Type, Home).' lists"
                ),
                None => format!(
                    "name a node there is, or declare it in a [[nodes]] table of {} (pile guide \
                     declarations)",
                    g.db.symbol(&pile)
                ),
            };
            not(
                "diagram-no-such-node",
                format!(
                    "no node {of_type}:{} is at home in {}",
                    n.name,
                    g.db.symbol(&pile)
                ),
                fix,
                Some(node),
            )
        }
    }
}

/// What the includes of a frame ask for, as their findings word it.
struct Asks {
    /// The variable a query binds to what it finds.
    variable: &'static str,
    /// What one value of it is.
    what: &'static str,
    /// What a program that finds nothing should find.
    finds: &'static str,
    /// What fixes a value that is none.
    bind: &'static str,
}

const NODES: Asks = Asks {
    variable: "Node",
    what: "node",
    finds: "what the diagram shows",
    bind: "bind Node to a node: the first column of node, or an end of edge",
};

const LINKS: Asks = Asks {
    variable: "Edge",
    what: "link",
    finds: "the links the diagram draws",
    bind: "bind Edge to the first column of edge: edge(Edge, _, _, _)",
};

/// What an include finds: the values of one variable, over its queries.
/// None for an include whose program is refused, asks nothing, or has a
/// query that does not bind the variable, each of which is reported.
fn asked(g: Graph, include: &Include, asks: &Asks, wrong: &mut Vec<Wrong>) -> Option<Vec<Value>> {
    let Asks { variable, what, .. } = asks;
    let mut invalid = |offset: usize, message: String, fix: String| {
        wrong.push(Wrong::new("diagram-include-invalid", offset, message, fix));
    };
    let answers = match query::ask(g.db, &include.program(), |name| g.db.pile_named(name)) {
        Ok(answers) => answers,
        Err(refused) => {
            invalid(
                include
                    .offset_of(refused.line, refused.column)
                    .unwrap_or(include.offset),
                format!("the program is refused: {}", refused.message),
                "pile query takes the same program: try it there (pile guide query)".to_string(),
            );
            return None;
        }
    };
    if answers.is_empty() {
        invalid(
            include.offset,
            "the program asks nothing: it has no query".to_string(),
            format!("end it with a query that binds {variable}, a clause that starts with ?-"),
        );
        return None;
    }
    let mut found = Vec::new();
    let mut fine = true;
    for answer in &answers {
        let Some(values) = query::column(answer, variable) else {
            invalid(
                include.offset,
                format!(
                    "the query {} binds no {variable}, so it finds no {what}",
                    answer.query
                ),
                format!(
                    "name the {what} the diagram should show {variable}, with a capital letter"
                ),
            );
            fine = false;
            continue;
        };
        found.extend(values.into_iter().cloned());
    }
    fine.then_some(found)
}

/// What the includes of one frame say a diagram should show: each thing
/// once, in the order it is found, with where the include that found it
/// first is written. `of` leads from a value to what it is. What a program
/// is refused for goes to `refused`, and what is wrong with what it finds
/// to `invalid`.
fn included<'g, T: Clone + Ord + 'g>(
    g: Graph<'g>,
    includes: &[Include],
    asks: &Asks,
    of: impl Fn(&Value) -> Option<&'g T>,
    refused: &mut Vec<Wrong>,
    invalid: &mut Vec<Wrong>,
) -> Vec<(T, usize)> {
    let Asks {
        variable,
        what,
        finds,
        bind,
    } = asks;
    let mut should: Vec<(T, usize)> = Vec::new();
    let mut seen: BTreeSet<&T> = BTreeSet::new();
    for include in includes {
        let Some(values) = asked(g, include, asks, refused) else {
            continue;
        };
        if values.is_empty() {
            invalid.push(Wrong::new(
                "diagram-include-empty",
                include.offset,
                format!("the include finds no {what}, so it says nothing the diagram should show"),
                format!(
                    "correct the program so it finds {finds}, or remove the include: pile query \
                     answers the same program"
                ),
            ));
        }
        for value in values {
            match of(&value) {
                Some(one) if seen.insert(one) => should.push((one.clone(), include.offset)),
                Some(_) => {}
                None => invalid.push(Wrong::new(
                    "diagram-include-invalid",
                    include.offset,
                    format!("the program binds {variable} to {value}, which is no {what}"),
                    bind.to_string(),
                )),
            }
        }
    }
    should
}

/// An `ignore` of what the diagram would not have to show anyway.
fn unneeded(offset: usize, message: String) -> Wrong {
    Wrong::new(
        "diagram-ignore-unneeded",
        offset,
        message,
        "remove the ignore".to_string(),
    )
}

/// Holds one diagram that reads to the repository.
pub(crate) fn hold(read: &diagram_format::Diagram, graph: Option<Graph>) -> Held {
    let mut said = meta::read(read);
    let mut held = Held {
        wrong: std::mem::take(&mut said.wrong),
        ..Held::default()
    };
    let word = meta::element_word(read.kind());

    let Some(g) = graph else {
        return held;
    };

    // The elements: which node each stands for.
    let mut broken: Vec<&str> = Vec::new();
    for element in &said.elements {
        let key = element.item.key.as_str();
        if element.broken {
            broken.push(key);
        }
        let Some(named) = &element.names else {
            continue;
        };
        let node = match node_of(g, named, Place::Element(word), &mut held.wrong) {
            Ok(node) => Some(node),
            Err(meant) => {
                broken.push(key);
                meant
            }
        };
        if let Some(node) = node {
            held.standing.elements.push(Stands {
                key: key.to_string(),
                node,
            });
        }
    }

    // The lines: which edge each stands for.
    for wire in &said.wires {
        let Some(of_type) = &wire.edge else { continue };
        let (from, to) = (wire.line.from.as_str(), wire.line.to.as_str());
        if broken.contains(&from) || broken.contains(&to) {
            // The finding at the box says why the line stands for nothing.
            continue;
        }
        let at = wire.line.span.offset;
        let (source_node, target) = match (held.standing.node(from), held.standing.node(to)) {
            (Some(a), Some(b)) => (a.clone(), b.clone()),
            (a, _) => {
                let bare = if a.is_none() { from } else { to };
                held.wrong.push(Wrong::new(
                    "diagram-edge-needs-nodes",
                    at,
                    format!(
                        "pile:edge says the line stands for a link between two nodes, and {bare} \
                         stands for no node"
                    ),
                    format!("give {bare} a pile:id, or leave pile:edge out of the line"),
                ));
                continue;
            }
        };
        let edge = Edge {
            source: source_node,
            of_type: of_type.clone(),
            target,
        };
        if g.rows
            .edge(&edge.source, &edge.of_type, &edge.target)
            .is_none()
        {
            let (a, b) = (g.db.symbol(&edge.source), g.db.symbol(&edge.target));
            let between: Vec<&str> = g
                .rows
                .edge
                .iter()
                .filter(|e| e.source == edge.source && e.target == edge.target)
                .map(|e| e.of_type.as_str())
                .collect();
            let fix = if g
                .rows
                .edge(&edge.target, &edge.of_type, &edge.source)
                .is_some()
            {
                format!("the link runs from {b} to {a}: write line {to} {from}")
            } else if !between.is_empty() {
                format!(
                    "the links from {a} to {b} are of the types {}: write one of them, or leave \
                     pile:edge out",
                    between.join(", ")
                )
            } else {
                format!(
                    "declare the link in a links.toml of {a} (pile guide links), or leave \
                     pile:edge out of the line"
                )
            };
            held.wrong.push(Wrong::new(
                "diagram-no-such-edge",
                at,
                format!("no link of the type {of_type} runs from {a} to {b}"),
                fix,
            ));
        }
        held.standing.lines.push(Draws {
            from: from.to_string(),
            to: to.to_string(),
            edge,
        });
    }

    // What the diagram should show as elements, and as lines: every edge
    // found between two nodes it draws.
    let mut invalid: Vec<Wrong> = Vec::new();
    let should = included(
        g,
        &said.box_includes,
        &NODES,
        |value| g.db.node(value),
        &mut held.wrong,
        &mut invalid,
    );
    let mut should_lines = included(
        g,
        &said.line_includes,
        &LINKS,
        |value| g.db.edge(value),
        &mut held.wrong,
        &mut invalid,
    );
    held.wrong.append(&mut invalid);
    should_lines.retain(|(edge, _)| {
        held.standing.draws(&edge.source).is_some() && held.standing.draws(&edge.target).is_some()
    });
    let found: BTreeSet<&Node> = should.iter().map(|(node, _)| node).collect();
    let found_lines: BTreeSet<&Edge> = should_lines.iter().map(|(edge, _)| edge).collect();
    let drawn_lines: BTreeSet<&Edge> = held.standing.lines.iter().map(|l| &l.edge).collect();

    // What it leaves out on purpose.
    let mut ignored: BTreeSet<Node> = BTreeSet::new();
    for named in &said.box_ignores {
        let Ok(node) = node_of(g, named, Place::Meta("ignore"), &mut held.wrong) else {
            continue;
        };
        let symbol = g.db.symbol(&node);
        if held.standing.draws(&node).is_some() {
            held.wrong.push(unneeded(
                named.offset,
                format!("the diagram draws {symbol}, so it does not leave it out"),
            ));
        } else if !found.contains(&node) {
            held.wrong.push(unneeded(
                named.offset,
                format!("no include finds {symbol}, so the diagram would not have to show it"),
            ));
        }
        ignored.insert(node);
    }
    let mut ignored_lines: BTreeSet<Edge> = BTreeSet::new();
    for ignore in &said.line_ignores {
        let from = node_of(g, &ignore.from, Place::Meta("from"), &mut held.wrong).ok();
        let to = node_of(g, &ignore.to, Place::Meta("to"), &mut held.wrong).ok();
        let (Some(source_node), Some(target)) = (from, to) else {
            continue;
        };
        let edge = Edge {
            source: source_node,
            of_type: ignore.of_type.clone(),
            target,
        };
        let symbol = g.db.written(&edge);
        if drawn_lines.contains(&edge) {
            held.wrong.push(unneeded(
                ignore.offset,
                format!("a line draws {symbol}, so the diagram does not leave it out"),
            ));
        } else if !found_lines.contains(&edge) {
            held.wrong.push(unneeded(
                ignore.offset,
                format!(
                    "no include finds {symbol} between two nodes the diagram draws, so the \
                     diagram would not have to show it"
                ),
            ));
        }
        ignored_lines.insert(edge);
    }

    // What it should show, and does not.
    let keyed = !matches!(read.kind(), Kind::Tree | Kind::Nested);
    for (node, at) in &should {
        if held.standing.draws(node).is_some() || ignored.contains(node) {
            continue;
        }
        let symbol = g.db.symbol(node);
        let (name, id, of_type) = g.named_as(node);
        let of_type = of_type.as_deref();
        held.wrong.push(Wrong::new(
            "diagram-misses-node",
            *at,
            format!("this include finds {symbol}, and no {word} draws it"),
            format!(
                "draw it, as {}, or leave it out with a reason, as ignore {} reason=\"…\"",
                meta::as_element(word, keyed, &name, &id, of_type),
                meta::as_meta(&name, &id, of_type)
            ),
        ));
        held.misses.push(symbol);
    }
    for (edge, at) in &should_lines {
        if drawn_lines.contains(edge) || ignored_lines.contains(edge) {
            continue;
        }
        let symbol = g.db.written(edge);
        let key = |node: &Node| {
            held.standing
                .draws(node)
                .map_or(String::new(), |e| e.key.clone())
        };
        let end = |node: &Node| {
            let (name, id, of_type) = g.named_as(node);
            meta::as_meta(&name, &id, of_type.as_deref())
        };
        held.wrong.push(Wrong::new(
            "diagram-misses-link",
            *at,
            format!(
                "this include finds {symbol}, between two nodes the diagram draws, and no line \
                 draws it"
            ),
            format!(
                "draw it, as line {} {} pile:edge=\"{}\", or leave it out with a reason, as \
                 ignore \"{}\" reason=\"…\" {{ from {}; to {} }}",
                key(&edge.source),
                key(&edge.target),
                edge.of_type,
                edge.of_type,
                end(&edge.source),
                end(&edge.target)
            ),
        ));
        held.misses.push(symbol);
    }
    held
}
