//! What a diagram says in the metadata `pile` owns.
//!
//! The format keeps metadata as it is written and never reads it. This
//! module reads the words under the owner `pile`, and nothing of any other
//! owner:
//!
//! - on an element, `pile:id`, with `pile:node-type` and `pile:name` beside
//!   it, which say the node the element stands for.
//! - on a line, `pile:edge`, the type of the edge the line stands for.
//! - in `meta pile`, `boxes` and `lines`, each holding `include`, a Datalog
//!   program that says what the diagram should show, and `ignore`, one node
//!   or edge it leaves out, with the reason.
//!
//! Nothing here knows the graph. Whatever is read is handed on with where
//! it is written, and whatever is not a word `pile` reads is a [`Wrong`].

use diagram_format::finding::{did_you_mean, nearest};
use diagram_format::{Body, Diagram, Item, Kind, Line, Meta, MetaNode, MetaValue, Text};

/// Whose metadata this crate reads.
pub const OWNER: &str = "pile";

/// The keys an element takes.
const ON_ELEMENT: [&str; 3] = ["id", "node-type", "name"];
/// The key a line takes.
const ON_LINE: [&str; 1] = ["edge"];
/// What `meta pile` holds, and what each of those holds.
const FRAMES: [&str; 2] = ["boxes", "lines"];
const IN_FRAME: [&str; 2] = ["include", "ignore"];

/// The node type every pile is, which no element writes.
const PILE: &str = "pile";

/// How an include is written, as a fix says it.
const RAW_STRING: &str = "write the program in a raw string: include #\"?- node(Node, pile, _).\"#";

/// Something wrong with a diagram, before it has a line and a column: what
/// the file alone shows is wrong in the metadata, and what holding it to
/// the graph finds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wrong {
    pub code: &'static str,
    /// The byte offset into the file of what is wrong.
    pub offset: usize,
    pub message: String,
    pub fix: String,
}

impl Wrong {
    pub fn new(code: &'static str, offset: usize, message: String, fix: String) -> Wrong {
        Wrong {
            code,
            offset,
            message,
            fix,
        }
    }
}

/// A node as a box names one: its name, the id of the pile it is, or is at
/// home in, and its type when it is not a pile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedAs {
    pub name: String,
    pub id: String,
    pub node_type: Option<String>,
    /// Where it is written.
    pub offset: usize,
}

/// One element, and the node it says it stands for: by the id of a pile,
/// that pile when there is no node type, and otherwise the node of that
/// type and this name at home in it.
#[derive(Debug, Clone)]
pub struct Element<'d> {
    pub item: &'d Item,
    pub names: Option<NamedAs>,
    /// It says it stands for a node, in a way that names none: the finding
    /// is already made.
    pub broken: bool,
}

/// One line, and the type of the edge it says it stands for.
#[derive(Debug, Clone)]
pub struct Wire<'d> {
    pub line: &'d Line,
    pub edge: Option<String>,
}

/// One `include`: its arguments, which joined by newlines are one program.
#[derive(Debug, Clone)]
pub struct Include<'d> {
    pub texts: Vec<&'d Text>,
    pub offset: usize,
}

impl Include<'_> {
    /// The program: the arguments, joined by newlines.
    pub fn program(&self) -> String {
        self.texts
            .iter()
            .map(|text| text.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The offset into the file of a place in the program: a line and a
    /// column of it, each from one, the column in characters. A line of the
    /// program is a line of one argument, since newlines join them.
    pub fn offset_of(&self, line: usize, column: usize) -> Option<usize> {
        let rows: Vec<(&Text, usize, &str)> = self
            .texts
            .iter()
            .flat_map(|text| {
                text.text
                    .split('\n')
                    .enumerate()
                    .map(move |(i, row)| (*text, i, row))
            })
            .collect();
        let (text, i, row) = rows.get(line.saturating_sub(1)).or(rows.last())?;
        let bytes = row
            .char_indices()
            .nth(column.saturating_sub(1))
            .map_or(row.len(), |(at, _)| at);
        text.offset(*i, bytes)
    }
}

/// Where a node is named by an id, and the word that names it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place<'w> {
    /// On an element, by the word its type writes one with: `box`, say.
    Element(&'w str),
    /// In a node of `meta pile`: `ignore`, `from` or `to`.
    Meta(&'w str),
}

impl Place<'_> {
    pub fn word(&self) -> &str {
        match self {
            Place::Element(word) | Place::Meta(word) => word,
        }
    }

    /// How the id is written there.
    pub fn id_key(&self) -> &'static str {
        match self {
            Place::Element(_) => "pile:id",
            Place::Meta(_) => "id",
        }
    }
}

/// A node as an element writes it: `box <key> pile:id="…"`, with
/// `pile:node-type=` for a node that is not a pile. A type whose elements
/// have no key of their own writes the name in quotes.
pub fn as_element(
    word: &str,
    keyed: bool,
    name: &str,
    id: &str,
    node_type: Option<&str>,
) -> String {
    let named = if keyed {
        name.to_string()
    } else {
        format!("\"{name}\"")
    };
    let of_type = node_type.map_or(String::new(), |t| format!(" pile:node-type={t}"));
    format!("{word} {named} pile:id=\"{id}\"{of_type}")
}

/// A node as a node of `meta pile` writes it, after its word: `"<name>"
/// id="…"`, with `node-type=` for a node that is not a pile.
pub fn as_meta(name: &str, id: &str, node_type: Option<&str>) -> String {
    let of_type = node_type.map_or(String::new(), |t| format!(" node-type={t}"));
    format!("\"{name}\" id=\"{id}\"{of_type}")
}

/// One `ignore` under `lines`: an edge by its type and its two ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredEdge {
    pub of_type: String,
    pub from: NamedAs,
    pub to: NamedAs,
    pub offset: usize,
}

/// Everything a diagram says in `pile`'s metadata.
#[derive(Debug, Clone, Default)]
pub struct Said<'d> {
    /// Every element a type names by a key, in the order the file writes
    /// them.
    pub elements: Vec<Element<'d>>,
    /// Every line that can stand for an edge, in the order the file writes
    /// them.
    pub wires: Vec<Wire<'d>>,
    pub box_includes: Vec<Include<'d>>,
    pub box_ignores: Vec<NamedAs>,
    pub line_includes: Vec<Include<'d>>,
    pub line_ignores: Vec<IgnoredEdge>,
    pub wrong: Vec<Wrong>,
}

impl Said<'_> {
    /// Whether anything said is held to the graph: a node named by an id,
    /// an edge, an include or an ignore.
    pub fn asks_graph(&self) -> bool {
        self.elements.iter().any(|e| e.names.is_some())
            || self.wires.iter().any(|w| w.edge.is_some())
            || !self.box_includes.is_empty()
            || !self.box_ignores.is_empty()
            || !self.line_includes.is_empty()
            || !self.line_ignores.is_empty()
    }
}

/// Every element of a diagram that may stand for a node, in the order the
/// file writes them.
pub fn elements(body: &Body) -> Vec<&Item> {
    match body {
        Body::Architecture(a) => a
            .boxes
            .iter()
            .flat_map(|held| std::iter::once(&held.item).chain(&held.holds))
            .collect(),
        Body::Sequence(s) => s.actors.iter().collect(),
        Body::State(s) => s.states.iter().map(|state| &state.item).collect(),
        Body::Swimlane(s) => s.lanes.iter().flat_map(|lane| &lane.steps).collect(),
        Body::Process(p) => p
            .lanes
            .iter()
            .flat_map(|lane| lane.tasks.iter().map(|task| &task.item))
            .collect(),
        Body::Tree(t) => t
            .root
            .walk()
            .into_iter()
            .map(|branch| &branch.item)
            .collect(),
        Body::Dependency(d) => d.boxes.iter().collect(),
        Body::Nested(n) => n.levels.iter().collect(),
        Body::Treemap(t) => t.parts.iter().map(|part| &part.item).collect(),
    }
}

/// Every line of a diagram that may stand for an edge, in the order the
/// file writes them. Only three types have one: None for any other.
pub fn lines(body: &Body) -> Option<Vec<&Line>> {
    match body {
        Body::Architecture(a) => Some(a.lines.iter().collect()),
        Body::Swimlane(s) => Some(s.lines.iter().collect()),
        Body::Dependency(d) => Some(d.lines.iter().chain(&d.cycle).collect()),
        _ => None,
    }
}

/// The word a type writes an element with.
pub fn element_word(kind: Kind) -> &'static str {
    match kind {
        Kind::Architecture | Kind::Dependency => "box",
        Kind::Sequence => "actor",
        Kind::State => "state",
        Kind::Swimlane => "step",
        Kind::Process => "task",
        Kind::Tree => "item",
        Kind::Nested => "level",
        Kind::Treemap => "part",
    }
}

fn text(value: &MetaValue) -> Option<&str> {
    value.as_text().map(|t| t.text.as_str())
}

/// "did you mean x?", or the words there are.
fn meant(word: &str, known: &[&str], otherwise: &str) -> String {
    did_you_mean(word, known.iter().copied()).unwrap_or_else(|| otherwise.to_string())
}

/// The one text argument of a node of `meta pile`. None when it has none,
/// several, or one that is not text.
fn one_text(node: &MetaNode) -> Option<String> {
    match node.args.as_slice() {
        [(value, _)] => text(value).map(str::to_string),
        _ => None,
    }
}

/// Reads everything a diagram says in `pile`'s metadata.
pub fn read(diagram: &Diagram) -> Said<'_> {
    let mut said = Said::default();
    let word = element_word(diagram.kind());
    for item in elements(&diagram.body) {
        let element = element(item, word, &mut said.wrong);
        said.elements.push(element);
    }
    let lines = lines(&diagram.body);
    for line in lines.iter().flatten() {
        let wire = wire(line, &mut said.wrong);
        said.wires.push(wire);
    }
    for block in diagram.meta.iter().filter(|b| b.owner == OWNER) {
        for node in &block.nodes {
            frame(node, diagram.kind(), lines.is_some(), &mut said);
        }
    }
    said
}

/// The metadata of `pile`'s on an element or a line, each with its text,
/// or a [`Wrong`] for one whose value is not text.
fn mine<'m>(meta: &'m [Meta], wrong: &mut Vec<Wrong>) -> Vec<(&'m Meta, &'m str)> {
    let mut out = Vec::new();
    for m in meta.iter().filter(|m| m.owner == OWNER) {
        match text(&m.value) {
            Some(value) => out.push((m, value)),
            None => wrong.push(unknown(
                m.span.offset,
                format!("pile:{} takes text, and was given {}", m.key, m.value),
                format!("write it in quotes: pile:{}=\"{}\"", m.key, m.value),
            )),
        }
    }
    out
}

fn element<'d>(item: &'d Item, word: &str, wrong: &mut Vec<Wrong>) -> Element<'d> {
    let (mut id, mut node_type, mut name) = (None, None, None);
    let mut broken = false;
    for (m, value) in mine(&item.meta, wrong) {
        match m.key.as_str() {
            "id" => id = Some(value),
            "node-type" => node_type = Some((m, value)),
            "name" => name = Some((m, value)),
            "edge" => wrong.push(unknown(
                m.span.offset,
                format!("pile:edge is a line's, and this is a {word}"),
                format!(
                    "write pile:edge on the line that draws the link, and pile:id on the {word}"
                ),
            )),
            other => wrong.push(unknown(
                m.span.offset,
                format!("pile:{other} is not a word pile reads on a {word}"),
                // `pile:type` and its misspellings mean `pile:node-type`.
                match nearest(other, ON_ELEMENT)
                    .or_else(|| nearest(&format!("node-{other}"), ["node-type"]))
                {
                    Some(near) => format!("did you mean pile:{near}?"),
                    None => format!("a {word} takes pile:id, pile:node-type and pile:name"),
                },
            )),
        }
    }
    if let Some((m, PILE)) = node_type {
        wrong.push(unknown(
            m.span.offset,
            format!(
                "pile:node-type is never pile: a {word} with pile:id alone stands for the pile"
            ),
            "leave pile:node-type out".to_string(),
        ));
        return Element {
            item,
            names: None,
            broken: true,
        };
    }
    let Some(id) = id else {
        if let Some((m, _)) = node_type.or(name) {
            wrong.push(Wrong {
                code: "diagram-needs-id",
                offset: m.span.offset,
                message: format!(
                    "pile:{} says which node the {word} stands for, and the {word} gives no pile:id",
                    m.key
                ),
                fix: "add pile:id with the id of the pile it is, or is at home in: pile list \
                      --full prints each pile's id"
                    .to_string(),
            });
            broken = true;
        }
        return Element {
            item,
            names: None,
            broken,
        };
    };
    Element {
        item,
        names: Some(NamedAs {
            name: name.map_or(item.name.as_str(), |(_, n)| n).to_string(),
            id: id.to_string(),
            node_type: node_type.map(|(_, t)| t.to_string()),
            offset: item.span.offset,
        }),
        broken,
    }
}

fn wire<'d>(line: &'d Line, wrong: &mut Vec<Wrong>) -> Wire<'d> {
    let mut edge = None;
    for (m, value) in mine(&line.meta, wrong) {
        match m.key.as_str() {
            "edge" => edge = Some(value.to_string()),
            other => wrong.push(unknown(
                m.span.offset,
                format!("pile:{other} is not a word pile reads on a line"),
                match nearest(other, ON_LINE) {
                    Some(near) => format!("did you mean pile:{near}?"),
                    None => "a line takes pile:edge, the type of the link it draws".to_string(),
                },
            )),
        }
    }
    Wire { line, edge }
}

fn unknown(offset: usize, message: String, fix: String) -> Wrong {
    Wrong::new("diagram-meta-unknown", offset, message, fix)
}

/// `boxes { … }` or `lines { … }`.
/// `lined` says whether the diagram's type has lines that can stand for an
/// edge.
fn frame<'d>(node: &'d MetaNode, kind: Kind, lined: bool, said: &mut Said<'d>) {
    let name = node.name.as_str();
    if !FRAMES.contains(&name) {
        said.wrong.push(unknown(
            node.span.offset,
            format!("{name} is not a word pile reads in meta pile"),
            meant(name, &FRAMES, "meta pile holds boxes { … } and lines { … }"),
        ));
        return;
    }
    if name == "lines" && !lined {
        said.wrong.push(unknown(
            node.span.offset,
            format!(
                "lines says which links a diagram should draw, and no line of a {} diagram stands for one",
                kind.word()
            ),
            "leave lines out: only an architecture, a swimlane and a dependency diagram have a line that takes pile:edge".to_string(),
        ));
        return;
    }
    let given = node
        .args
        .iter()
        .map(|(_, span)| span.offset)
        .chain(node.props.iter().map(|(_, _, span)| span.offset))
        .next();
    if let Some(offset) = given {
        said.wrong.push(unknown(
            offset,
            format!("{name} takes include and ignore between braces, and nothing beside its name"),
            format!("write it as: {name} {{ include … }}"),
        ));
    }
    for child in &node.children {
        match child.name.as_str() {
            "include" => {
                if let Some(include) = include(child, &mut said.wrong) {
                    match name {
                        "boxes" => said.box_includes.push(include),
                        _ => said.line_includes.push(include),
                    }
                }
            }
            "ignore" if name == "boxes" => {
                reason(child, &mut said.wrong);
                if let Some(node) = named_as(child, "ignore", &["reason"], &mut said.wrong) {
                    said.box_ignores.push(node);
                }
            }
            "ignore" => {
                reason(child, &mut said.wrong);
                if let Some(edge) = ignored_edge(child, &mut said.wrong) {
                    said.line_ignores.push(edge);
                }
            }
            other => said.wrong.push(unknown(
                child.span.offset,
                format!("{other} is not a word pile reads in {name}"),
                meant(
                    other,
                    &IN_FRAME,
                    &format!("{name} holds include and ignore"),
                ),
            )),
        }
    }
}

/// `include <program>…`: one program, in one string or several.
fn include<'d>(node: &'d MetaNode, wrong: &mut Vec<Wrong>) -> Option<Include<'d>> {
    let mut fine = true;
    for (key, _, span) in &node.props {
        wrong.push(unknown(
            span.offset,
            format!("include takes a program, and no property: {key} is not read"),
            RAW_STRING.to_string(),
        ));
        fine = false;
    }
    if let Some(child) = node.children.first() {
        wrong.push(unknown(
            child.span.offset,
            "include takes a program, and nothing between braces".to_string(),
            RAW_STRING.to_string(),
        ));
        fine = false;
    }
    let mut texts = Vec::new();
    for (value, span) in &node.args {
        match value.as_text() {
            Some(text) => texts.push(text),
            None => {
                wrong.push(unknown(
                    span.offset,
                    format!("include takes a program in a string, and was given {value}"),
                    RAW_STRING.to_string(),
                ));
                fine = false;
            }
        }
    }
    if fine && texts.is_empty() {
        wrong.push(Wrong {
            code: "diagram-include-invalid",
            offset: node.span.offset,
            message: "the include has no program, so it says nothing the diagram should show"
                .to_string(),
            fix: "give it a program in a raw string, or remove it: include #\"?- node(Node, \
                  pile, _).\"#"
                .to_string(),
        });
        fine = false;
    }
    fine.then_some(Include {
        texts,
        offset: node.span.offset,
    })
}

/// An `ignore` says why: `reason="…"`, which is not empty.
fn reason(node: &MetaNode, wrong: &mut Vec<Wrong>) {
    let given = node
        .props
        .iter()
        .find(|(key, _, _)| key == "reason")
        .map(|(_, value, _)| value.to_string());
    if given.is_none_or(|r| r.trim().is_empty()) {
        wrong.push(Wrong {
            code: "diagram-ignore-no-reason",
            offset: node.span.offset,
            message: "the ignore does not say why the diagram leaves this out".to_string(),
            fix: "add reason=\"…\", a sentence that says why".to_string(),
        });
    }
}

/// A node named as a box names one, in a node of `meta pile`: `<word>
/// "<name>" id="<id>"`, with `node-type=` for a node that is not a pile.
/// `also` are the properties its caller reads beside those.
fn named_as(node: &MetaNode, word: &str, also: &[&str], wrong: &mut Vec<Wrong>) -> Option<NamedAs> {
    let usage = format!("write it as: {word} \"<name>\" id=\"<the pile's id>\"");
    let mut fine = true;
    let name = one_text(node);
    if name.is_none() {
        wrong.push(unknown(
            node.span.offset,
            format!("{word} takes one name, in a string"),
            usage,
        ));
        fine = false;
    }
    let mut known: Vec<&str> = vec!["id", "node-type"];
    known.extend_from_slice(also);
    let (mut id, mut node_type) = (None, None);
    for (key, value, span) in &node.props {
        let Some(said) = text(value) else {
            wrong.push(unknown(
                span.offset,
                format!("{key} takes text, and was given {value}"),
                format!("write it in quotes: {key}=\"{value}\""),
            ));
            fine = false;
            continue;
        };
        match key.as_str() {
            "id" => id = Some(said.to_string()),
            "node-type" if said == PILE => {
                wrong.push(unknown(
                    span.offset,
                    "node-type is never pile: an id alone names the pile".to_string(),
                    "leave node-type out".to_string(),
                ));
                fine = false;
            }
            "node-type" => node_type = Some(said.to_string()),
            other if also.contains(&other) => {}
            other => {
                wrong.push(unknown(
                    span.offset,
                    format!("{other} is not a word pile reads on {word}"),
                    meant(other, &known, &format!("{word} takes {}", known.join(", "))),
                ));
                fine = false;
            }
        }
    }
    if id.is_none() && fine {
        wrong.push(Wrong {
            code: "diagram-needs-id",
            offset: node.span.offset,
            message: format!("{word} names a node, and gives no id"),
            fix: "add id= with the id of the pile it is, or is at home in: pile list --full \
                  prints each pile's id"
                .to_string(),
        });
    }
    if !fine {
        return None;
    }
    Some(NamedAs {
        name: name?,
        id: id?,
        node_type,
        offset: node.span.offset,
    })
}

/// `ignore "<edge type>" reason="…" { from "<name>" id="<id>"; to "<name>"
/// id="<id>" }`.
fn ignored_edge(node: &MetaNode, wrong: &mut Vec<Wrong>) -> Option<IgnoredEdge> {
    let usage = "write it as: ignore \"<edge type>\" reason=\"…\" { from \"<name>\" id=\"…\"; to \
                 \"<name>\" id=\"…\" }";
    let mut fine = true;
    let of_type = one_text(node);
    if of_type.is_none() {
        wrong.push(unknown(
            node.span.offset,
            "an ignore under lines takes one edge type, in a string".to_string(),
            usage.to_string(),
        ));
        fine = false;
    }
    for (key, _, span) in node.props.iter().filter(|(key, _, _)| key != "reason") {
        wrong.push(unknown(
            span.offset,
            format!("{key} is not a word pile reads on an ignore under lines"),
            meant(key, &["reason"], usage),
        ));
        fine = false;
    }
    let (mut from, mut to) = (None, None);
    for child in &node.children {
        let end = match child.name.as_str() {
            "from" => &mut from,
            "to" => &mut to,
            other => {
                wrong.push(unknown(
                    child.span.offset,
                    format!("{other} is not a word pile reads in an ignore under lines"),
                    meant(other, &["from", "to"], usage),
                ));
                fine = false;
                continue;
            }
        };
        if end.is_some() {
            wrong.push(unknown(
                child.span.offset,
                format!("the ignore gives {} twice", child.name),
                usage.to_string(),
            ));
            fine = false;
            continue;
        }
        match named_as(child, &child.name, &[], wrong) {
            Some(named) => *end = Some(named),
            None => fine = false,
        }
    }
    if fine && (from.is_none() || to.is_none()) {
        wrong.push(unknown(
            node.span.offset,
            "an ignore under lines names a link by its two ends, and one is not given".to_string(),
            usage.to_string(),
        ));
    }
    if !fine {
        return None;
    }
    Some(IgnoredEdge {
        of_type: of_type?,
        from: from?,
        to: to?,
        offset: node.span.offset,
    })
}
