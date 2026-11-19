//! The diagrams a repository keeps, what doctor finds wrong with them, and
//! how one is drawn.
//!
//! A pile keeps its diagrams in its `.pile/diagrams/`, a file
//! `<name>.diagram` each, written in the format of multipile's diagrams
//! (github.com/multipile/diagrams): what a diagram shows, and never where.
//! That project reads a file, holds it to the rules every diagram and its
//! type are held to, paints it into a document, and writes the document as
//! a page or renders it as a picture. This crate finds the files among a
//! repository's declarations, and holds each to what only the repository
//! can say.
//!
//! A diagram says it in metadata, which the format keeps and never reads,
//! under the owner `pile` ([`meta`]): which node a box stands for, which
//! edge a line does, what the diagram should show, as Datalog programs, and
//! what it leaves out on purpose. Held to the graph, that gives doctor's
//! findings ([`findings`]), what a change touched of what a diagram stands
//! for ([`changes`]), what an edited diagram stands for that it did not
//! ([`swapped`]), and what it should show and does not ([`misses`]).

use std::collections::BTreeSet;

use diagram_document::Mark;
pub use diagram_format as format;
use diagram_format::finding::locate;
use piles::doctor::{Finding, Severity, Survey};
use piles::{FROM_TOP, ROOT, SIGIL, shown};
use storage::Content;

mod checks;
pub mod meta;

pub use checks::{Draws, Graph, Standing, Stands, Swapped, swapped};

/// One diagram a pile keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagram {
    /// What it is typed by: its file's name, without `.diagram`.
    pub name: String,
    /// The directory whose `.pile/` holds it, from the top.
    pub dir: String,
    /// Its file, from the top.
    pub path: String,
    /// What the file holds, or None when it is not UTF-8 text.
    pub source: Option<String>,
}

/// Every diagram among a repository's declaration files, in path order.
pub fn found(files: &[Content]) -> Vec<Diagram> {
    let mut all: Vec<Diagram> = files
        .iter()
        .filter_map(|c| {
            let (dir, name) = piles::diagram_of(&c.path)?;
            Some(Diagram {
                name,
                dir,
                path: c.path.clone(),
                source: String::from_utf8(c.bytes.clone()).ok(),
            })
        })
        .collect();
    all.sort_by(|a, b| a.path.cmp(&b.path));
    all
}

/// A pile a diagram may name: where it is, its name, and its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub path: String,
    pub name: String,
    pub id: String,
}

impl Named {
    /// How the pile is typed: `@name`, or its `:/` path when it has none.
    pub fn handle(&self) -> String {
        if self.name.is_empty() {
            shown(&self.path)
        } else {
            format!("{SIGIL}{}", self.name)
        }
    }
}

/// The piles of a survey whose config reads.
pub fn named(survey: &Survey) -> Vec<Named> {
    survey
        .rows
        .iter()
        .filter_map(|row| {
            row.config.as_ref().map(|config| Named {
                path: row.path.clone(),
                name: config.name.clone(),
                id: config.id.clone(),
            })
        })
        .collect()
}

/// Adds what is wrong with each diagram to the survey, each finding to the
/// row of the `.pile/` it is in. `graph` is the graph of the same files,
/// given only when every pile's config reads: without it, a diagram is held
/// to what its file alone shows, and to the piles it names the old way.
pub fn check(survey: &mut Survey, files: &[Content], graph: Option<Graph>) {
    let piles = named(survey);
    let findings = found(files)
        .iter()
        .flat_map(|d| findings(d, &piles, graph))
        .collect();
    survey.add(findings);
}

/// What is wrong with one diagram, in the order of the file: each rule of
/// its format it breaks; each word of `pile`'s metadata that is not one;
/// each box and line that stands for nothing; and, of what its includes
/// say it should show, each node and edge it does not. Every one is a
/// problem but `diagram-ignore-unneeded`, a note.
///
/// Without a graph, only what the file alone shows is found, and a pile
/// named as `pile="@name"` that the repository does not have.
pub fn findings(diagram: &Diagram, piles: &[Named], graph: Option<Graph>) -> Vec<Finding> {
    match held(diagram, piles, graph) {
        Ok(held) => held.findings,
        Err(unread) => unread,
    }
}

/// A diagram that reads, held to the repository once: its findings, in the
/// order of the file, what it stands for, and what it misses.
struct Held {
    findings: Vec<Finding>,
    standing: Standing,
    misses: Vec<String>,
}

/// Reads one diagram and holds it to the repository. A diagram that does
/// not read is the findings that say why.
fn held(diagram: &Diagram, piles: &[Named], graph: Option<Graph>) -> Result<Held, Vec<Finding>> {
    let Some(source) = &diagram.source else {
        return Err(vec![finding(
            "diagram-invalid",
            &diagram.path,
            None,
            "the diagram is not UTF-8 text, so it cannot be read".to_string(),
            "write it again as text: pile guide diagrams says what a diagram holds".to_string(),
        )]);
    };
    let read = diagram_format::read(source).map_err(|found| {
        found
            .0
            .into_iter()
            .map(|f| {
                finding(
                    "diagram-invalid",
                    &diagram.path,
                    Some((f.line, f.column)),
                    format!("{} ({})", f.message, f.code),
                    f.help.unwrap_or_else(|| {
                        "pile guide diagrams says what a diagram holds".to_string()
                    }),
                )
            })
            .collect::<Vec<_>>()
    })?;
    let checks::Held {
        wrong,
        standing,
        misses,
    } = checks::hold(&read.diagram, piles, graph);
    let mut findings: Vec<Finding> = wrong
        .into_iter()
        .map(|w| {
            let at = locate(source, w.offset);
            finding(w.code, &diagram.path, Some(at), w.message, w.fix)
        })
        .collect();
    findings.sort_by_key(|f| (f.line, f.column));
    Ok(Held {
        findings,
        standing,
        misses,
    })
}

/// How a finding of each code counts: its severity, and whether it stops
/// the diagram being drawn. A diagram that misses what it should show is
/// still true, only incomplete, so it is drawn. One with a box or a line
/// that stands for nothing, or any other problem, is not.
fn counts(code: &str) -> (Severity, bool) {
    match code {
        "diagram-ignore-unneeded" => (Severity::Note, false),
        "diagram-misses-node" | "diagram-misses-link" => (Severity::Problem, false),
        _ => (Severity::Problem, true),
    }
}

/// Whether one of a diagram's findings stops the diagram being drawn.
pub fn stops_drawing(finding: &Finding) -> bool {
    counts(finding.code).1
}

/// What a diagram stands for: the node of each element that names one, and
/// the edge of each line that names one. Nothing for a diagram that does
/// not read.
pub fn standing(diagram: &Diagram, piles: &[Named], graph: Option<Graph>) -> Standing {
    held(diagram, piles, graph)
        .map(|held| held.standing)
        .unwrap_or_default()
}

/// The nodes and the edges a diagram should show and does not, each as an
/// answer of a program writes it: `@name`, `type:name`, or an edge's
/// source, type and target. None for a diagram that does not read.
pub fn misses(diagram: &Diagram, piles: &[Named], graph: Graph) -> Vec<String> {
    held(diagram, piles, Some(graph))
        .map(|held| held.misses)
        .unwrap_or_default()
}

/// Whether a diagram that reads says anything of the graph. One that does
/// not has the same findings with a graph and without one.
pub fn asks_graph(diagram: &diagram_format::Diagram) -> bool {
    meta::read(diagram).asks_graph()
}

/// The pile a diagram's reference names: `@name` by its name, or a `:/`
/// path by where it is.
pub fn resolve<'p>(reference: &str, piles: &'p [Named]) -> Option<&'p Named> {
    if let Some(name) = reference.strip_prefix(SIGIL) {
        return piles.iter().find(|p| p.name == name);
    }
    let path = reference.strip_prefix(FROM_TOP)?;
    let path = format!("{ROOT}{}", path.trim_end_matches('/'));
    piles.iter().find(|p| p.path == path)
}

/// One of a diagram's findings, at a line and a column of its file, or at
/// the file alone. Its severity is its code's.
fn finding(
    code: &'static str,
    file: &str,
    at: Option<(usize, usize)>,
    message: String,
    fix: String,
) -> Finding {
    Finding {
        code,
        severity: counts(code).0,
        file: file.to_string(),
        line: at.map(|(line, _)| line),
        column: at.map(|(_, column)| column),
        message,
        fix,
    }
}

/// What a diagram is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum As {
    /// A page that loads and runs nothing, with its fonts in it.
    Html,
    /// A picture, at twice the diagram's size.
    Png,
    /// The diagram revealed, element by element.
    Gif,
}

impl As {
    pub const WORDS: [&'static str; 3] = ["html", "png", "gif"];

    pub fn word(self) -> &'static str {
        match self {
            As::Html => "html",
            As::Png => "png",
            As::Gif => "gif",
        }
    }

    pub fn from_word(word: &str) -> Option<As> {
        match word {
            "html" => Some(As::Html),
            "png" => Some(As::Png),
            "gif" => Some(As::Gif),
            _ => None,
        }
    }
}

/// A diagram drawn: the file's bytes, the size of the diagram, and how long
/// it moves, for a drawing that does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawn {
    pub bytes: Vec<u8>,
    pub width: i32,
    pub height: i32,
    pub duration_ms: Option<u32>,
}

/// Draws a diagram that reads: still as a page or a picture, and revealed
/// as a gif.
pub fn draw(diagram: &diagram_format::Diagram, as_: As) -> Result<Drawn, String> {
    use diagram_document::Animation;
    let still = diagram_document::document(diagram, Animation::None);
    let moving = (as_ == As::Gif).then(|| diagram_document::document(diagram, Animation::Reveal));
    draw_documents(&still, moving.as_ref(), as_)
}

/// Draws a diagram's two versions as one: still, the second with what was
/// added, removed and changed marked, and listed under it; as a gif, the
/// first becoming the second. None when the two are of different types.
///
/// What both versions have is marked as changed when it is drawn
/// differently, which multipile's diagrams find, and when it is in
/// `swapped`: drawn the same, it stands for something else. Metadata that
/// is only written differently marks nothing.
pub fn draw_diff(
    before: &diagram_format::Diagram,
    after: &diagram_format::Diagram,
    swapped: &Swapped,
    as_: As,
) -> Option<Result<Drawn, String>> {
    let also = |mark: Mark| match mark {
        Mark::Element(key) => swapped.elements.iter().any(|e| e == key),
        Mark::Line { from, to } => swapped.lines.iter().any(|(f, t)| f == from && t == to),
    };
    let still = diagram_document::diff_document(before, after, false, also).ok()?;
    let moving = match as_ {
        As::Gif => Some(diagram_document::diff_document(before, after, true, also).ok()?),
        _ => None,
    };
    Some(draw_documents(&still, moving.as_ref(), as_))
}

/// Draws a diagram with what a change touched marked: each element and
/// each line of `touched`, by its key. It is still, so it is drawn as a
/// page or a picture, and never as a gif.
pub fn draw_marked(
    diagram: &diagram_format::Diagram,
    touched: &Touched,
    as_: As,
) -> Result<Drawn, String> {
    let still = diagram_document::changed_document(diagram, |mark| match mark {
        Mark::Element(key) => touched.elements.iter().any(|e| e == key),
        Mark::Line { from, to } => touched.lines.iter().any(|(f, t)| f == from && t == to),
    });
    draw_documents(&still, None, as_)
}

/// A document written as a page, rendered as a picture, or, given the
/// document that moves, as a gif.
fn draw_documents(
    still: &diagram_document::Document,
    moving: Option<&diagram_document::Document>,
    as_: As,
) -> Result<Drawn, String> {
    let (width, height) = (still.width, still.height);
    match (as_, moving) {
        (As::Html, _) => Ok(Drawn {
            bytes: diagram_html::page(still, &diagram_html::Options::default()).into_bytes(),
            width,
            height,
            duration_ms: None,
        }),
        (As::Png, _) => diagram_render::Renderer::new()
            .still(still, diagram_render::StillFormat::Png, None)
            .map(|bytes| Drawn {
                bytes,
                width,
                height,
                duration_ms: None,
            })
            .map_err(|e| e.to_string()),
        (As::Gif, Some(moving)) => diagram_render::Renderer::new()
            .animation(moving, diagram_render::MotionFormat::Gif, 25)
            .map(|bytes| Drawn {
                bytes,
                width: moving.width,
                height: moving.height,
                duration_ms: Some(moving.duration_ms),
            })
            .map_err(|e| e.to_string()),
        (As::Gif, None) => {
            Err("it is still, so it is drawn as html or png, not as a gif".to_string())
        }
    }
}

/// What a change touched of what a diagram stands for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Touched {
    /// The piles alone, each once, in the order the file names them: as
    /// the file names it where it says `pile="@name"`, and as an answer
    /// writes it otherwise.
    pub piles: Vec<String>,
    /// Every node and then every edge, each once, as an answer of a
    /// program writes it: `@name`, `type:name`, or an edge's source, type
    /// and target.
    pub references: Vec<String>,
    /// The keys of the elements that stand for a touched node.
    pub elements: Vec<String>,
    /// The two keys of each line that stands for a touched edge.
    pub lines: Vec<(String, String)>,
}

impl Touched {
    pub fn is_empty(&self) -> bool {
        self.references.is_empty()
    }
}

/// How a diagram changed since a revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Its file is new.
    Added,
    /// Its file changed.
    Edited,
    /// Its file did not change, and something it stands for was touched: a
    /// pile, another node, or an edge.
    Piles(Touched),
    /// Its file is gone.
    Removed,
}

impl Change {
    pub fn word(&self) -> &'static str {
        match self {
            Change::Added => "added",
            Change::Edited => "edited",
            Change::Piles(_) => "piles",
            Change::Removed => "removed",
        }
    }

    /// What the change touched of what the diagram stands for, for a
    /// diagram whose file did not change.
    pub fn touched(&self) -> Option<&Touched> {
        match self {
            Change::Piles(touched) => Some(touched),
            _ => None,
        }
    }
}

/// One diagram, how a change touched it, and what it should show and does
/// not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub diagram: Diagram,
    pub change: Option<Change>,
    /// As [`misses`] gives them, in the working copy. Empty for a diagram
    /// that is gone, and when there is no graph.
    pub misses: Vec<String>,
}

/// Each diagram there is now, and each that is gone, in path order, with
/// how it changed since a revision, or None. `then` are the diagrams at the
/// revision, and `now` those of the working copy. `piles` and `graph` are
/// the working copy's. What the change touched is its caller's to say:
/// `touched_piles` the ids of the piles whose files changed, and `touched`
/// the other nodes and the edges that were added, removed or changed.
pub fn changes(
    then: &[Diagram],
    now: &[Diagram],
    piles: &[Named],
    graph: Option<Graph>,
    touched_piles: &BTreeSet<String>,
    touched: &graph::Touched,
) -> Vec<Listed> {
    let mut out: Vec<Listed> = now
        .iter()
        .map(|diagram| listed(then, diagram, piles, graph, touched_piles, touched))
        .collect();
    for gone in then
        .iter()
        .filter(|d| !now.iter().any(|n| n.path == d.path))
    {
        out.push(Listed {
            diagram: gone.clone(),
            change: Some(Change::Removed),
            misses: Vec::new(),
        });
    }
    out.sort_by(|a, b| a.diagram.path.cmp(&b.diagram.path));
    out
}

/// One diagram of the working copy, as [`changes`] lists it. The diagram is
/// held to the repository once, for what the change touched of it and for
/// what it misses.
pub fn listed(
    then: &[Diagram],
    diagram: &Diagram,
    piles: &[Named],
    graph: Option<Graph>,
    touched_piles: &BTreeSet<String>,
    touched: &graph::Touched,
) -> Listed {
    let held = held(diagram, piles, graph).ok();
    let change = match then.iter().find(|d| d.path == diagram.path) {
        None => Some(Change::Added),
        Some(before) if before.source != diagram.source => Some(Change::Edited),
        Some(_) => {
            let touched = held.as_ref().map_or_else(Touched::default, |held| {
                touched_in(&held.standing, piles, graph, touched_piles, touched)
            });
            (!touched.is_empty()).then_some(Change::Piles(touched))
        }
    };
    Listed {
        diagram: diagram.clone(),
        change,
        misses: held.map_or_else(Vec::new, |held| held.misses),
    }
}

/// What a change touched of what one diagram stands for. Nothing for a
/// diagram that does not read.
pub fn touched_of(
    diagram: &Diagram,
    piles: &[Named],
    graph: Option<Graph>,
    touched_piles: &BTreeSet<String>,
    touched: &graph::Touched,
) -> Touched {
    held(diagram, piles, graph).map_or_else(
        |_| Touched::default(),
        |held| touched_in(&held.standing, piles, graph, touched_piles, touched),
    )
}

/// What a change touched of what a diagram stands for.
fn touched_in(
    standing: &Standing,
    piles: &[Named],
    graph: Option<Graph>,
    touched_piles: &BTreeSet<String>,
    touched: &graph::Touched,
) -> Touched {
    let mut out = Touched::default();
    // Without a graph a diagram stands for piles alone, each named as it
    // is typed.
    let symbol = |node: &graph::Node| match (graph, node) {
        (Some(g), _) => g.db.symbol(node),
        (None, graph::Node::Pile(id)) => piles
            .iter()
            .find(|p| &p.id == id)
            .map_or_else(|| id.clone(), Named::handle),
        (None, graph::Node::Named { of_type, name }) => format!("{of_type}:{name}"),
    };
    let add = |list: &mut Vec<String>, said: String| {
        if !list.contains(&said) {
            list.push(said);
        }
    };
    for element in &standing.elements {
        let is_touched = match &element.node {
            graph::Node::Pile(id) => touched_piles.contains(id),
            node => touched.nodes.contains(node),
        };
        if !is_touched {
            continue;
        }
        let said = symbol(&element.node);
        if matches!(element.node, graph::Node::Pile(_)) {
            add(
                &mut out.piles,
                element.written.clone().unwrap_or_else(|| said.clone()),
            );
        }
        add(&mut out.references, said);
        out.elements.push(element.key.clone());
    }
    for line in &standing.lines {
        if !touched.edges.contains(&line.edge) {
            continue;
        }
        // A line stands for an edge only when there is a graph.
        if let Some(g) = graph {
            add(&mut out.references, g.db.written(&line.edge));
        }
        out.lines.push((line.from.clone(), line.to.clone()));
    }
    out
}

#[cfg(test)]
mod tests;
