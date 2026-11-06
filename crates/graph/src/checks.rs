//! The checks of what a reader found, and the rows of what passes them. A
//! check reads only the tables a reader hands it, each header, key and
//! value with its line and column, and never a file, so the checks are the
//! same whatever syntax a file is written in.
//!
//! The order is the order each part needs the one before it:
//!
//! 1. the types, from each aux pile's `[[node-types]]` and `[[edge-types]]`;
//! 2. each pile's own values, from its `[pile]` tables;
//! 3. the nodes, from `[[nodes]]`, each of a node type that exists;
//! 4. the edges, from `[[edges]]`, each of an edge type that exists, between
//!    two nodes its references name.
//!
//! Every table is checked whole, so each mistake in it is a finding, and
//! one mistake never hides another in the same table.
//!
//! One mistake is one finding. A thing written twice is a finding at each
//! place, and neither is read. A type table with any finding does not
//! exist. Either way, the name is remembered as unread ([`Types::unread`],
//! [`Nodes::unread`]), and whatever names it gets no further finding: the
//! one finding already points at what to fix.
//!
//! A key of its own, on a node, an edge or a pile, holds any value, and
//! nothing checks it. The one exception is a key one letter away from a key
//! the table takes, such as `targte`: read as a key of its own, it would
//! leave the link's target out without a word, so it is refused.

use std::collections::{BTreeMap, BTreeSet};

use piles::config::is_word;
use piles::doctor::{Finding, Severity};
use piles::{Pile, Position, shown};

use crate::toml::{Entry, Header, Raw, RawIs, Table};
use crate::{
    At, Edge, EdgeProp, HEADLINE, ID, Item, NAME, Node, NodeRow, PATH, PILE, Prop, PropAt,
    Relations, Sort, TypeOf, TypeProp, Value, finding,
};

/// A table a reader found, and the pile whose `.pile/` it is in, by its
/// place in the piles handed to [`check`].
pub struct Found {
    pub pile: usize,
    pub table: Table,
}

const TITLE: &str = "title";
const DESCRIPTION: &str = "description";
const TYPE: &str = "type";
const SOURCE: &str = "source";
const TARGET: &str = "target";
const DO: &str = "do";
const DONT: &str = "dont";

/// The keys each table takes, besides keys of its own.
const NODE_KEYS: &[&str] = &[TYPE, NAME, TITLE, DESCRIPTION];
const EDGE_KEYS: &[&str] = &[TYPE, SOURCE, TARGET, TITLE, DESCRIPTION];
const PILE_KEYS: &[&str] = &[TITLE, DESCRIPTION];
const NODE_TYPE_KEYS: &[&str] = &[NAME, TITLE, DESCRIPTION];
const EDGE_TYPE_KEYS: &[&str] = &[NAME, TITLE, DESCRIPTION, SOURCE, TARGET, DO, DONT];

struct Ctx<'a> {
    piles: &'a [&'a Pile],
    findings: &'a mut Vec<Finding>,
    rows: &'a mut Relations,
}

impl Ctx<'_> {
    fn say(&mut self, code: &'static str, file: &str, at: Position, message: String, fix: String) {
        self.findings.push(finding(
            code,
            Severity::Problem,
            file,
            Some(at),
            message,
            fix,
        ));
    }

    fn pile_node(&self, i: usize) -> Node {
        Node::Pile(self.piles[i].id.clone())
    }
}

/// Checks every table found, and adds a row for each thing that passes.
pub fn check(
    piles: &[&Pile],
    found: Vec<Found>,
    findings: &mut Vec<Finding>,
    rows: &mut Relations,
) {
    let mut cx = Ctx {
        piles,
        findings,
        rows,
    };
    let by = |h: Header| -> Vec<&Found> { found.iter().filter(|f| f.table.header == h).collect() };
    let types = types(&mut cx, &by(Header::NodeType), &by(Header::EdgeType));
    pile_values(&mut cx, &by(Header::Pile));
    let nodes = nodes(&mut cx, &by(Header::Node), &types);
    edges(&mut cx, &by(Header::Edge), &types, &nodes);
}

// ---------------------------------------------------------------- values

/// The values a raw value is, as rows hold them: one for a single value, one
/// for each single value in a list, each once, and none for a table.
fn values(raw: &Raw) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let all: Vec<Value> = match &raw.is {
        RawIs::List(items) => items.iter().filter_map(single).collect(),
        _ => single(raw).into_iter().collect(),
    };
    for v in all {
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

fn single(raw: &Raw) -> Option<Value> {
    Some(match &raw.is {
        RawIs::Text(s) | RawIs::Datetime(s) => Value::Text(s.clone()),
        RawIs::Integer(n) => Value::Integer(*n),
        RawIs::Float(s) => Value::Float(s.clone()),
        RawIs::Boolean(b) => Value::Boolean(*b),
        RawIs::List(_) | RawIs::Table => return None,
    })
}

/// A key the format gives as text, such as a title. None, with a finding,
/// when it is not text.
fn text(cx: &mut Ctx, file: &str, e: &Entry) -> Option<String> {
    match e.value.text() {
        Some(s) => Some(s.to_string()),
        None => {
            cx.say(
                "wrong-value",
                file,
                e.value.at,
                format!("{} is {}, and it is text", e.key.word, e.value.what()),
                format!("write {} between double quotes", e.key.word),
            );
            None
        }
    }
}

/// A key the format gives as a word, such as a name.
fn word(cx: &mut Ctx, file: &str, e: &Entry) -> Option<String> {
    let s = text(cx, file, e)?;
    if is_word(&s) {
        return Some(s);
    }
    cx.say(
        "wrong-value",
        file,
        e.value.at,
        format!(
            "{} is {s:?}, and it is a word: lowercase letters, digits and hyphens, starting with \
             a letter",
            e.key.word
        ),
        format!("write {} as a word", e.key.word),
    );
    None
}

fn missing(cx: &mut Ctx, t: &Table, key: &str, why: &str) {
    cx.say(
        "missing-key",
        &t.file,
        t.at,
        format!(
            "this {} table gives no {key}, and {why}",
            t.header.written()
        ),
        format!("give it {key}"),
    );
}

/// Whether two keys are one edit apart: one letter added, removed, changed,
/// or two side by side swapped.
fn one_apart(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a == b || a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[n][m] == 1
}

/// The entries of a node, a link or `[pile]` that become rows: its `title`
/// and `description` when each is text, and each key of its own that is a
/// word and not one letter from a key the table takes. Each other entry is
/// a finding. The keys in `skip`, which the caller reads itself, such as
/// `type`, are skipped. `takes` is every key the table takes.
fn row_entries<'t>(cx: &mut Ctx, t: &'t Table, skip: &[&str], takes: &[&str]) -> Vec<&'t Entry> {
    let mut out = Vec::new();
    for e in &t.entries {
        let key = e.key.word.as_str();
        if key == TITLE || key == DESCRIPTION {
            if text(cx, &t.file, e).is_some() {
                out.push(e);
            }
            continue;
        }
        if skip.contains(&key) {
            continue;
        }
        if !is_word(key) {
            cx.say(
                "unknown-key",
                &t.file,
                e.key.at,
                format!(
                    "{key:?} is not a key the table takes, and a key of its own is a word: \
                     lowercase letters, digits and hyphens, starting with a letter"
                ),
                "rename the key to a word".to_string(),
            );
            continue;
        }
        if let Some(meant) = takes.iter().find(|k| one_apart(key, k)) {
            cx.say(
                "unknown-key",
                &t.file,
                e.key.at,
                format!(
                    "{key} is one letter from {meant}, which {} takes, so it is read as a \
                     mistake and not as a key of its own",
                    t.header.written()
                ),
                format!("write {meant}, or give the key of its own a name further from it"),
            );
            continue;
        }
        out.push(e);
    }
    out
}

/// Adds the rows of one value of a node or an edge: a row for each value,
/// and one of where it is written.
fn add_values(rows: &mut Relations, item: &Item, file: &str, key: &str, raw: &Raw) {
    let vs = values(raw);
    if vs.is_empty() {
        return;
    }
    for value in vs {
        match item {
            Item::Node(node) => rows.prop.push(Prop {
                node: node.clone(),
                property: key.to_string(),
                value,
            }),
            Item::Edge(edge) => rows.edge_prop.push(EdgeProp {
                edge: edge.clone(),
                property: key.to_string(),
                value,
            }),
            Item::Type(name) => rows.type_prop.push(TypeProp {
                of_type: name.clone(),
                property: key.to_string(),
                value,
            }),
        }
    }
    rows.prop_at.push(PropAt {
        item: item.clone(),
        property: key.to_string(),
        file: shown(file),
        at: raw.at,
    });
}

/// Where each place is written, for a finding that names the others.
fn places(of: &[(&str, Position)]) -> String {
    of.iter()
        .map(|(file, at)| format!("{}:{}:{}", shown(file), at.line, at.column))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A finding at each of the places one thing is written, when it is written
/// more than once.
fn twice(cx: &mut Ctx, what: &str, written: &[(&str, Position)]) {
    for (i, (file, at)) in written.iter().enumerate() {
        let others: Vec<(&str, Position)> = written
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, w)| *w)
            .collect();
        cx.say(
            "declared-twice",
            file,
            *at,
            format!(
                "{what} is written here and at {}, and each thing is written once, so neither is \
                 read",
                places(&others)
            ),
            "remove all but one".to_string(),
        );
    }
}

// ---------------------------------------------------------------- types

/// A type while it is checked: what its table says, before it is rows.
#[derive(Debug, Clone)]
pub(crate) struct TypeRow {
    pub name: String,
    pub edge: bool,
    /// The aux pile whose `.pile/` creates it.
    pub home: Node,
    /// Its table: the file, from the top, and where the table starts.
    pub file: String,
    pub at: Position,
    /// Its `title`, `description`, `do` and `dont`, each where it is written.
    pub props: Vec<(&'static str, String, Position)>,
    /// The node types an edge type's source may be, and its target.
    pub source: Vec<String>,
    pub target: Vec<String>,
}

/// The types that exist, by name, and the names of those that are written
/// but not read: written twice, or with a finding in their table.
pub(crate) struct Types {
    pub exist: BTreeMap<String, TypeRow>,
    pub unread: BTreeSet<String>,
}

impl Types {
    fn node_type(&self, name: &str) -> bool {
        name == PILE || self.exist.get(name).is_some_and(|t| !t.edge)
    }
}

/// One type table, read: its row, where its name is, its ends as written,
/// and whether the table had a finding.
struct Candidate<'f> {
    row: TypeRow,
    name_at: Position,
    ends: Vec<(&'static str, Vec<(String, Position)>)>,
    found: &'f Found,
    clean: bool,
}

fn types(cx: &mut Ctx, node_types: &[&Found], edge_types: &[&Found]) -> Types {
    let mut candidates: Vec<Candidate> = Vec::new();
    for f in node_types.iter().chain(edge_types) {
        if let Some(c) = type_table(cx, f) {
            candidates.push(c);
        }
    }

    // A name written in two tables is two, whether either table is clean.
    let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, c) in candidates.iter().enumerate() {
        by_name.entry(c.row.name.clone()).or_default().push(i);
    }
    let mut unread = BTreeSet::new();
    let mut exist: BTreeMap<String, TypeRow> = BTreeMap::new();
    for (name, at) in &by_name {
        if at.len() > 1 {
            let written: Vec<(&str, Position)> = at
                .iter()
                .map(|&i| {
                    (
                        candidates[i].found.table.file.as_str(),
                        candidates[i].name_at,
                    )
                })
                .collect();
            twice(cx, &format!("the type {name}"), &written);
            unread.insert(name.clone());
            continue;
        }
        let c = &candidates[at[0]];
        if c.clean {
            exist.insert(name.clone(), c.row.clone());
        } else {
            unread.insert(name.clone());
        }
    }

    // An edge type's ends name node types, which are all known by now. An
    // end that names none is a finding in the edge type's table, so the edge
    // type does not exist either.
    for c in &candidates {
        if !exist.get(&c.row.name).is_some_and(|t| t.edge) {
            continue;
        }
        let mut clean = true;
        let mut lists: Vec<(&str, Vec<String>)> = Vec::new();
        for (key, list) in &c.ends {
            for (end, at) in list {
                if end == PILE || exist.get(end).is_some_and(|t| !t.edge) {
                    continue;
                }
                clean = false;
                if unread.contains(end) {
                    continue;
                }
                let why = if exist.contains_key(end) {
                    format!("{end} is an edge type, and {key} names node types")
                } else {
                    format!("{key} names {end}, and no aux pile creates a node type of that name")
                };
                cx.say(
                    "unknown-type",
                    &c.found.table.file,
                    *at,
                    why,
                    format!(
                        "name a node type an aux pile creates, or pile, or create {end} \
                         (pile guide types)"
                    ),
                );
            }
            lists.push((key, list.iter().map(|(n, _)| n.clone()).collect()));
        }
        if !clean {
            exist.remove(&c.row.name);
            unread.insert(c.row.name.clone());
            continue;
        }
        let row = exist.get_mut(&c.row.name).expect("the edge type exists");
        for (key, list) in lists {
            if key == SOURCE {
                row.source = list;
            } else {
                row.target = list;
            }
        }
    }
    for t in exist.values() {
        type_rows(cx.rows, t);
    }
    Types { exist, unread }
}

/// The rows of a type that exists: `type`, `type_prop` and `allows`, with
/// where its table and each value are written.
fn type_rows(rows: &mut Relations, t: &TypeRow) {
    rows.types.push(TypeOf {
        name: t.name.clone(),
        sort: if t.edge { Sort::Edge } else { Sort::Node },
        home: t.home.clone(),
    });
    let item = Item::Type(t.name.clone());
    rows.at.push(At {
        item: item.clone(),
        file: shown(&t.file),
        at: t.at,
    });
    for (property, value, at) in &t.props {
        rows.type_prop.push(TypeProp {
            of_type: t.name.clone(),
            property: property.to_string(),
            value: Value::Text(value.clone()),
        });
        rows.prop_at.push(PropAt {
            item: item.clone(),
            property: property.to_string(),
            file: shown(&t.file),
            at: *at,
        });
    }
    for (end, list) in [
        (crate::End::Source, &t.source),
        (crate::End::Target, &t.target),
    ] {
        for node_type in list {
            rows.allows.push(crate::Allows {
                edge_type: t.name.clone(),
                end,
                node_type: node_type.clone(),
            });
        }
    }
}

/// Reads one type table. None when it has no name to remember it by; a
/// finding says why. A candidate that is not clean had a finding, and does
/// not exist.
fn type_table<'f>(cx: &mut Ctx, f: &'f Found) -> Option<Candidate<'f>> {
    let t = &f.table;
    let before = cx.findings.len();
    let edge = t.header == Header::EdgeType;
    let takes = if edge { EDGE_TYPE_KEYS } else { NODE_TYPE_KEYS };
    let mut row = TypeRow {
        name: String::new(),
        edge,
        home: cx.pile_node(f.pile),
        file: t.file.clone(),
        at: t.at,
        props: Vec::new(),
        source: Vec::new(),
        target: Vec::new(),
    };
    let mut name_at = t.at;
    let mut ends = Vec::new();
    for e in &t.entries {
        let key = e.key.word.as_str();
        if !takes.contains(&key) {
            cx.say(
                "unknown-key",
                &t.file,
                e.key.at,
                format!(
                    "{key} is not a key of {}, which takes {}",
                    t.header.written(),
                    takes.join(", ")
                ),
                "remove the key: a type says nothing of the keys its nodes or links carry of \
                 their own"
                    .to_string(),
            );
            continue;
        }
        match key {
            NAME => {
                name_at = e.value.at;
                // The name as written is remembered even when it is not a
                // word, so that a node of it is no further finding.
                row.name = e.value.text().unwrap_or_default().to_string();
                word(cx, &t.file, e);
            }
            TITLE | DESCRIPTION | DO | DONT => {
                if let Some(value) = text(cx, &t.file, e) {
                    let property = takes.iter().find(|k| **k == key).expect("a key it takes");
                    row.props.push((property, value, e.value.at));
                }
            }
            _ => {
                if let Some(list) = end_list(cx, &t.file, e) {
                    ends.push((if key == SOURCE { SOURCE } else { TARGET }, list));
                }
            }
        }
    }
    if edge {
        for key in [SOURCE, TARGET] {
            if t.get(key).is_none() {
                missing(
                    cx,
                    t,
                    key,
                    "an edge type says which node types each end may be",
                );
            }
        }
    }
    if t.get(NAME).is_none() {
        missing(cx, t, NAME, "a type is named by its name");
        return None;
    }
    if row.name == PILE {
        cx.say(
            "reserved-name",
            &t.file,
            name_at,
            "pile is the type of every pile, and the format creates it, so no aux pile does"
                .to_string(),
            "give the type another name".to_string(),
        );
        return None;
    }
    if row.name.is_empty() {
        return None;
    }
    Some(Candidate {
        row,
        name_at,
        ends,
        found: f,
        clean: cx.findings.len() == before,
    })
}

/// An edge type's `source` or `target`: a list of words, each where it is
/// written.
fn end_list(cx: &mut Ctx, file: &str, e: &Entry) -> Option<Vec<(String, Position)>> {
    let bad = |cx: &mut Ctx, at: Position, what: String| {
        cx.say(
            "wrong-value",
            file,
            at,
            format!(
                "{} is {what}, and it is a list of node types, each by its name",
                e.key.word
            ),
            format!(
                "write {} = [\"pile\"], or the node types it allows",
                e.key.word
            ),
        );
    };
    let RawIs::List(items) = &e.value.is else {
        bad(cx, e.value.at, e.value.what());
        return None;
    };
    let mut out = Vec::new();
    for item in items {
        match item.text().filter(|s| is_word(s)) {
            Some(s) => out.push((s.to_string(), item.at)),
            None => {
                bad(cx, item.at, format!("a list that holds {}", item.what()));
                return None;
            }
        }
    }
    Some(out)
}

// ---------------------------------------------------------------- [pile]

fn pile_values(cx: &mut Ctx, tables: &[&Found]) {
    // Each key of a pile's [pile], across its files, with each place it is
    // written.
    let mut keys: BTreeMap<(usize, String), Vec<(&Table, &Entry)>> = BTreeMap::new();
    for f in tables {
        let t = &f.table;
        for e in &t.entries {
            let key = e.key.word.as_str();
            if [ID, NAME, PATH, HEADLINE].contains(&key) {
                let from = match key {
                    HEADLINE => "its intent",
                    PATH => "where its .pile/ is",
                    _ => "its config",
                };
                cx.say(
                    "reserved-name",
                    &t.file,
                    e.key.at,
                    format!("a pile's {key} comes from {from}, and [pile] does not give it"),
                    "remove the key".to_string(),
                );
            }
        }
        for e in row_entries(cx, t, &[ID, NAME, PATH, HEADLINE], PILE_KEYS) {
            keys.entry((f.pile, e.key.word.clone()))
                .or_default()
                .push((t, e));
        }
    }
    for ((pile, key), written) in keys {
        if written.len() > 1 {
            let places: Vec<(&str, Position)> = written
                .iter()
                .map(|(t, e)| (t.file.as_str(), e.key.at))
                .collect();
            twice(cx, &format!("[pile] {key}"), &places);
            continue;
        }
        let (t, e) = written[0];
        let item = Item::Node(cx.pile_node(pile));
        add_values(cx.rows, &item, &t.file, &key, &e.value);
    }
}

// ---------------------------------------------------------------- nodes

/// The nodes that exist, and those written but not read: declared twice,
/// or of a type that is not read. Nothing that names one of those is a
/// further finding.
pub(crate) struct Nodes {
    pub exist: BTreeSet<Node>,
    pub unread: BTreeSet<Node>,
}

/// A node's type, as its table gives it. None, with a finding, when the
/// type is not one a node may be. Some(false) when it is a type that is not
/// read, which is no further finding.
fn node_type_of(cx: &mut Ctx, t: &Table, types: &Types) -> Option<(String, bool)> {
    let Some(ty) = t.get(TYPE) else {
        missing(cx, t, TYPE, "a node is its type and its name");
        return None;
    };
    let of_type = text(cx, &t.file, ty)?;
    if of_type == PILE {
        cx.say(
            "reserved-name",
            &t.file,
            ty.value.at,
            "a pile is declared by its config, and no [[nodes]] table declares one".to_string(),
            "declare the pile with pile init in its directory".to_string(),
        );
        return None;
    }
    if types.unread.contains(&of_type) {
        return Some((of_type, false));
    }
    match types.exist.get(&of_type) {
        Some(row) if !row.edge => Some((of_type, true)),
        found => {
            let why = if found.is_some() {
                format!("{of_type} is an edge type, and a node's type is a node type")
            } else {
                format!("the type {of_type} is created by no aux pile")
            };
            cx.say(
                "unknown-type",
                &t.file,
                ty.value.at,
                why,
                format!(
                    "name a node type an aux pile creates, or create {of_type} in one \
                     (pile guide types)"
                ),
            );
            None
        }
    }
}

fn nodes(cx: &mut Ctx, tables: &[&Found], types: &Types) -> Nodes {
    let mut out = Nodes {
        exist: BTreeSet::new(),
        unread: BTreeSet::new(),
    };
    let mut declared: BTreeMap<Node, Vec<(&Found, Vec<&Entry>)>> = BTreeMap::new();
    for f in tables {
        let t = &f.table;
        let of_type = node_type_of(cx, t, types);
        let name = match t.get(NAME) {
            None => {
                missing(cx, t, NAME, "a node is its type and its name");
                None
            }
            Some(nm) => word(cx, &t.file, nm),
        };
        let entries = row_entries(cx, t, &[TYPE, NAME], NODE_KEYS);
        let (Some((of_type, read)), Some(name)) = (of_type, name) else {
            continue;
        };
        let node = Node::Named { of_type, name };
        if !read {
            out.unread.insert(node);
            continue;
        }
        declared.entry(node).or_default().push((f, entries));
    }

    for (node, written) in declared {
        let Node::Named { of_type, name } = &node else {
            unreachable!("a [[nodes]] table declares no pile");
        };
        if written.len() > 1 {
            let places: Vec<(&str, Position)> = written
                .iter()
                .map(|(f, _)| (f.table.file.as_str(), f.table.at))
                .collect();
            twice(cx, &format!("the node {of_type}:{name}"), &places);
            out.unread.insert(node);
            continue;
        }
        let (f, entries) = &written[0];
        let t = &f.table;
        cx.rows.node.push(NodeRow {
            node: node.clone(),
            of_type: of_type.clone(),
            home: cx.pile_node(f.pile),
        });
        let item = Item::Node(node.clone());
        cx.rows.at.push(At {
            item: item.clone(),
            file: shown(&t.file),
            at: t.at,
        });
        for e in entries {
            add_values(cx.rows, &item, &t.file, &e.key.word, &e.value);
        }
        out.exist.insert(node);
    }
    out
}

// ---------------------------------------------------------------- edges

/// How a reference names a node, read off how it is written.
enum Reference {
    /// `@name`: a pile, by its name.
    Pile(String),
    /// `type:name`: any other node.
    Node { of_type: String, name: String },
}

fn reference(s: &str) -> Option<Reference> {
    if let Some(name) = s.strip_prefix('@') {
        return is_word(name).then(|| Reference::Pile(name.to_string()));
    }
    let (of_type, name) = s.split_once(':')?;
    (is_word(of_type) && is_word(name)).then(|| Reference::Node {
        of_type: of_type.to_string(),
        name: name.to_string(),
    })
}

/// One end of a link: the node, and where it is written, or where the link
/// is when the end is left out.
struct End {
    node: Node,
    at: Position,
    written: bool,
}

/// The node a link's end names. None when it names none; a finding says so
/// unless another finding already does.
fn end(cx: &mut Ctx, f: &Found, key: &str, nodes: &Nodes) -> Option<End> {
    let t = &f.table;
    let Some(e) = t.get(key) else {
        let at = t.get(TYPE).map_or(t.at, |e| e.value.at);
        return Some(End {
            node: cx.pile_node(f.pile),
            at,
            written: false,
        });
    };
    let s = text(cx, &t.file, e)?;
    let at = e.value.at;
    let no_such = |cx: &mut Ctx, message: String, fix: String| {
        cx.say("no-such-name", &t.file, at, message, fix);
        None
    };
    let node = match reference(&s) {
        None => {
            cx.say(
                "wrong-value",
                &t.file,
                at,
                format!(
                    "{key} is {s:?}, and a reference is \"@name\" for a pile, or \"type:name\" \
                     for any other node"
                ),
                "write the reference as \"@name\" or \"type:name\" (pile guide links)".to_string(),
            );
            return None;
        }
        Some(Reference::Pile(name)) => {
            let named: Vec<&&Pile> = cx.piles.iter().filter(|p| p.name == name).collect();
            match named.as_slice() {
                [one] => Node::Pile(one.id.clone()),
                [] => {
                    let unnamed = cx
                        .piles
                        .iter()
                        .find(|p| p.name.is_empty() && p.path.rsplit('/').next() == Some(&name));
                    let fix = match unnamed {
                        Some(p) => format!(
                            "the pile at {} has no name: pile set name {name} {} gives it this one",
                            shown(&p.path),
                            shown(&p.path)
                        ),
                        None => "name a pile that is named: pile list shows each name".to_string(),
                    };
                    return no_such(cx, format!("no pile is named @{name}"), fix);
                }
                // A name two piles claim names neither, and the config's
                // finding says so.
                _ => return None,
            }
        }
        Some(Reference::Node { of_type, name }) if of_type == PILE => {
            return no_such(
                cx,
                format!("a pile is referred to as \"@{name}\", never as \"pile:{name}\""),
                format!("write \"@{name}\""),
            );
        }
        Some(Reference::Node { of_type, name }) => {
            let node = Node::Named {
                of_type: of_type.clone(),
                name: name.clone(),
            };
            if nodes.unread.contains(&node) {
                return None;
            }
            if !nodes.exist.contains(&node) {
                return no_such(
                    cx,
                    format!("no node {of_type}:{name} is declared"),
                    format!(
                        "declare it in a [[nodes]] table, type = \"{of_type}\" and name = \
                         \"{name}\", or name a node that is declared"
                    ),
                );
            }
            node
        }
    };
    Some(End {
        node,
        at,
        written: true,
    })
}

/// A node as a sentence names it: a pile by its name, or by its path when it
/// has none, and any other node by its reference.
fn said(cx: &Ctx, node: &Node) -> String {
    match node {
        Node::Pile(id) => cx
            .piles
            .iter()
            .find(|p| &p.id == id)
            .map_or_else(|| id.clone(), |p| p.handle()),
        Node::Named { of_type, name } => format!("{of_type}:{name}"),
    }
}

fn type_of(node: &Node) -> &str {
    match node {
        Node::Pile(_) => PILE,
        Node::Named { of_type, .. } => of_type,
    }
}

/// A link's edge type, as its table gives it. None, with a finding unless
/// the type is not read, when it is not an edge type that exists.
fn edge_type_of<'t>(cx: &mut Ctx, t: &Table, types: &'t Types) -> Option<&'t TypeRow> {
    let Some(ty) = t.get(TYPE) else {
        missing(cx, t, TYPE, "every link is of an edge type");
        return None;
    };
    let of_type = text(cx, &t.file, ty)?;
    if types.unread.contains(&of_type) {
        return None;
    }
    if let Some(row) = types.exist.get(&of_type).filter(|r| r.edge) {
        return Some(row);
    }
    let why = if types.node_type(&of_type) {
        format!("{of_type} is a node type, and a link's type is an edge type")
    } else {
        format!("the edge type {of_type} is created by no aux pile")
    };
    cx.say(
        "unknown-type",
        &t.file,
        ty.value.at,
        why,
        format!(
            "name an edge type an aux pile creates, or create {of_type} in one (pile guide types)"
        ),
    );
    None
}

fn edges(cx: &mut Ctx, tables: &[&Found], types: &Types, nodes: &Nodes) {
    let mut written: BTreeMap<Edge, Vec<(&Found, Vec<&Entry>)>> = BTreeMap::new();
    for f in tables {
        let t = &f.table;
        let row = edge_type_of(cx, t, types);
        let ends = if t.get(SOURCE).is_none() && t.get(TARGET).is_none() {
            missing(
                cx,
                t,
                "source or target",
                "a link gives at least one of its ends",
            );
            None
        } else {
            Some((end(cx, f, SOURCE, nodes), end(cx, f, TARGET, nodes)))
        };
        let entries = row_entries(cx, t, &[TYPE, SOURCE, TARGET], EDGE_KEYS);
        // A misspelled end, which row_entries refused, would be read as an
        // end left out, so the link would end at the wrong node.
        let misspelled = t
            .entries
            .iter()
            .any(|e| one_apart(&e.key.word, SOURCE) || one_apart(&e.key.word, TARGET));
        let (Some(row), Some((Some(source), Some(target))), false) = (row, ends, misspelled) else {
            continue;
        };
        if source.node == target.node {
            cx.say(
                "self-link",
                &t.file,
                t.at,
                "the link joins a node to itself, and a link joins two nodes".to_string(),
                "remove the link, or name another node at one end".to_string(),
            );
            continue;
        }
        if !ends_allowed(cx, t, row, &source, &target) {
            continue;
        }
        written
            .entry(Edge {
                source: source.node,
                of_type: row.name.clone(),
                target: target.node,
            })
            .or_default()
            .push((f, entries));
    }

    for (edge, at) in written {
        if at.len() > 1 {
            let places: Vec<(&str, Position)> = at
                .iter()
                .map(|(f, _)| (f.table.file.as_str(), f.table.at))
                .collect();
            let what = format!(
                "the link {} {} {}",
                said(cx, &edge.source),
                edge.of_type,
                said(cx, &edge.target)
            );
            twice(cx, &what, &places);
            continue;
        }
        let (f, entries) = &at[0];
        let t = &f.table;
        cx.rows.edge.push(edge.clone());
        let item = Item::Edge(edge);
        cx.rows.at.push(At {
            item: item.clone(),
            file: shown(&t.file),
            at: t.at,
        });
        for e in entries {
            add_values(cx.rows, &item, &t.file, &e.key.word, &e.value);
        }
    }
}

/// Whether each end is of a node type the edge type allows. When neither
/// is, and the other way round both would be, one finding says so.
fn ends_allowed(cx: &mut Ctx, t: &Table, row: &TypeRow, source: &End, target: &End) -> bool {
    let fits = |list: &[String], node: &Node| list.iter().any(|n| n == type_of(node));
    let (s_ok, t_ok) = (
        fits(&row.source, &source.node),
        fits(&row.target, &target.node),
    );
    if s_ok && t_ok {
        return true;
    }
    let either = |list: &[String]| match list {
        [] => "no node type".to_string(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    };
    if fits(&row.source, &target.node) && fits(&row.target, &source.node) {
        let at = if target.written { target.at } else { source.at };
        cx.say(
            "wrong-end",
            &t.file,
            at,
            format!(
                "a link of type {} goes from {} to {}, and this one goes the other way",
                row.name,
                either(&row.source),
                either(&row.target)
            ),
            "exchange its source and its target".to_string(),
        );
        return false;
    }
    for (ok, end, key, list) in [
        (s_ok, source, SOURCE, &row.source),
        (t_ok, target, TARGET, &row.target),
    ] {
        if ok {
            continue;
        }
        let left_out = if end.written {
            String::new()
        } else {
            ", and an end left out is the pile that writes the link".to_string()
        };
        cx.say(
            "wrong-end",
            &t.file,
            end.at,
            format!(
                "the {key} of a link of type {} is {}, and this one's is {}{left_out}",
                row.name,
                either(list),
                type_of(&end.node)
            ),
            format!(
                "name a {} at the {key}, or use an edge type that allows {}",
                either(list),
                type_of(&end.node)
            ),
        );
    }
    false
}

#[cfg(test)]
mod tests {
    use super::one_apart;

    #[test]
    fn one_letter_added_removed_changed_or_swapped_is_one_apart() {
        for (a, b) in [
            ("targte", "target"),
            ("titel", "title"),
            ("descripton", "description"),
            ("sources", "source"),
            ("tyle", "type"),
        ] {
            assert!(one_apart(a, b), "{a} {b}");
        }
        for (a, b) in [
            ("target", "target"),
            ("port", "type"),
            ("labels", "title"),
            ("through", "source"),
        ] {
            assert!(!one_apart(a, b), "{a} {b}");
        }
    }
}
