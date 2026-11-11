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
//! can say: that every pile it names is one of the repository's piles.

pub use diagram_format as format;
use diagram_format::{Body, Item};
use piles::doctor::{Finding, Severity, Survey};
use piles::{FROM_TOP, ROOT, SIGIL, shown};
use storage::Content;

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
/// row of the `.pile/` it is in.
pub fn check(survey: &mut Survey, files: &[Content]) {
    let piles = named(survey);
    let findings = found(files)
        .iter()
        .flat_map(|d| findings(d, &piles))
        .collect();
    survey.add(findings);
}

/// What is wrong with one diagram: each rule of its format it breaks, and
/// each pile it names that the repository does not have. Every one is a
/// problem.
pub fn findings(diagram: &Diagram, piles: &[Named]) -> Vec<Finding> {
    let Some(source) = &diagram.source else {
        return vec![problem(
            "diagram-invalid",
            &diagram.path,
            None,
            "the diagram is not UTF-8 text, so it cannot be read".to_string(),
            "write it again as text: pile guide diagrams says what a diagram holds".to_string(),
        )];
    };
    let read = match diagram_format::read(source) {
        Ok(read) => read,
        Err(found) => {
            return found
                .0
                .into_iter()
                .map(|f| {
                    problem(
                        "diagram-invalid",
                        &diagram.path,
                        Some((f.line, f.column)),
                        format!("{} ({})", f.message, f.code),
                        f.help.unwrap_or_else(|| {
                            "pile guide diagrams says what a diagram holds".to_string()
                        }),
                    )
                })
                .collect();
        }
    };
    let mut out = Vec::new();
    for item in with_piles(&read.diagram.body) {
        let Some(pile) = &item.pile else { continue };
        let at = Some(diagram_format::finding::locate(source, item.span.offset));
        let by_reference = resolve(&pile.reference, piles);
        let by_id = pile
            .id
            .as_deref()
            .and_then(|id| piles.iter().find(|p| p.id == id));
        match (by_reference, by_id, &pile.id) {
            (Some(_), None, None) => {}
            (Some(named), Some(with_id), _) if named.path == with_id.path => {}
            (Some(named), Some(with_id), _) => out.push(problem(
                "diagram-pile-id-differs",
                &diagram.path,
                at,
                format!(
                    "{} is the pile at {}, and the id given is the pile at {}",
                    pile.reference,
                    shown(&named.path),
                    shown(&with_id.path)
                ),
                format!(
                    "write the id of {} (pile list --full prints it), or the name of the pile the id is",
                    pile.reference
                ),
            )),
            (Some(_), None, Some(id)) => out.push(problem(
                "diagram-pile-id-differs",
                &diagram.path,
                at,
                format!("{} is a pile, and no pile has the id {id}", pile.reference),
                "copy the pile's id from pile list --full, or leave id= out".to_string(),
            )),
            (None, Some(with_id), _) => out.push(problem(
                "diagram-no-such-pile",
                &diagram.path,
                at,
                format!(
                    "no pile answers to {}, and the pile with its id is now {}",
                    pile.reference,
                    with_id.handle()
                ),
                format!("write pile=\"{}\"", with_id.handle()),
            )),
            (None, None, _) => {
                let handles: Vec<String> = piles.iter().map(Named::handle).collect();
                let near = diagram_format::finding::nearest(&pile.reference, handles.iter().map(String::as_str));
                let fix = match near {
                    Some(near) => format!("did you mean {near}? Write pile=\"{near}\""),
                    None => "name one of the piles pile list prints, as @name or as its :/ path, or leave pile= out"
                        .to_string(),
                };
                out.push(problem(
                    "diagram-no-such-pile",
                    &diagram.path,
                    at,
                    format!("no pile answers to {}", pile.reference),
                    fix,
                ))
            }
        }
    }
    out
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

/// Every element of a diagram that may stand for a pile, in the order the
/// file writes them.
pub fn with_piles(body: &Body) -> Vec<&Item> {
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

fn problem(
    code: &'static str,
    file: &str,
    at: Option<(usize, usize)>,
    message: String,
    fix: String,
) -> Finding {
    Finding {
        code,
        severity: Severity::Problem,
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

/// Draws a diagram that reads.
pub fn draw(diagram: &diagram_format::Diagram, as_: As) -> Result<Drawn, String> {
    use diagram_document::Animation;
    let still = diagram_document::document(diagram, Animation::None);
    let (width, height) = (still.width, still.height);
    match as_ {
        As::Html => Ok(Drawn {
            bytes: diagram_html::page(&still, &diagram_html::Options::default()).into_bytes(),
            width,
            height,
            duration_ms: None,
        }),
        As::Png => diagram_render::Renderer::new()
            .still(&still, diagram_render::StillFormat::Png, None)
            .map(|bytes| Drawn {
                bytes,
                width,
                height,
                duration_ms: None,
            })
            .map_err(|e| e.to_string()),
        As::Gif => {
            let revealed = diagram_document::document(diagram, Animation::Reveal);
            diagram_render::Renderer::new()
                .animation(&revealed, diagram_render::MotionFormat::Gif, 25)
                .map(|bytes| Drawn {
                    bytes,
                    width: revealed.width,
                    height: revealed.height,
                    duration_ms: Some(revealed.duration_ms),
                })
                .map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests;
