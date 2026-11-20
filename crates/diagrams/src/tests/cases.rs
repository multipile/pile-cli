//! Every diagram multipile's diagrams promise something about, taken
//! through this crate: the cases of github.com/multipile/diagrams, which
//! its flake gives as a directory of files with `cases.json`, a manifest
//! that lists them. This repository's flake builds that directory at the
//! commit the crates are pinned to, and says where it is in `DIAGRAM_CASES`,
//! in its shell and in its checks.
//!
//! A case's file is no pile's: it may carry metadata of any owner, `pile`'s
//! among them, naming piles that are in no repository. So these tests
//! cover what needs no graph and nothing else: what a file alone shows of
//! what is wrong with it, and how a diagram that reads is drawn. A diagram
//! held to a graph is `held`'s.
//!
//! Without `DIAGRAM_CASES` every test here fails, and none is skipped.

use std::path::PathBuf;

use serde_json::Value;

use crate::*;

/// Where the cases are, in the environment.
const CASES: &str = "DIAGRAM_CASES";

/// One case of the manifest: its kind, what it is called, and the rest of
/// what the manifest says of it.
struct Case {
    /// `<group>/<name>`, as a failure names it.
    name: String,
    kind: String,
    says: Value,
    dir: PathBuf,
}

impl Case {
    /// A word the manifest says of the case.
    fn word(&self, key: &str) -> &str {
        self.says[key]
            .as_str()
            .unwrap_or_else(|| panic!("{}: cases.json says no {key} of it", self.name))
    }

    /// The file the manifest names under `key`, as a diagram a pile keeps.
    fn diagram(&self, key: &str) -> Diagram {
        let path = self.dir.join(self.word(key));
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {}: {e}", self.name, path.display()));
        Diagram {
            name: self.word("name").to_string(),
            dir: "/".into(),
            path: format!("/.pile/diagrams/{}.diagram", self.word("name")),
            source: Some(source),
        }
    }

    /// The same file, read. An error says what stopped it reading. A hint
    /// is no such finding: a case that reads may be written in an order
    /// that draws worse than another, and is drawn as written. Anything
    /// else found of it fails the case, a note too.
    fn read(&self, key: &str) -> Result<diagram_format::Diagram, String> {
        let diagram = self.diagram(key);
        let found: Vec<Finding> = findings(&diagram, None)
            .into_iter()
            .filter(|f| f.code != "diagram-hint")
            .collect();
        if !found.is_empty() {
            return Err(format!("{key} has findings: {}", told(&found)));
        }
        diagram_format::read(diagram.source.as_deref().unwrap_or_default())
            .map(|read| read.diagram)
            .map_err(|e| format!("{key} does not read: {e}"))
    }
}

/// Every case of one kind. It panics when the cases are not there, when
/// the manifest lacks one of the three kinds, and so when it is empty.
fn cases(kind: &str) -> Vec<Case> {
    let Some(dir) = std::env::var_os(CASES).filter(|dir| !dir.is_empty()) else {
        panic!(
            "{CASES} is not set. It says where the cases of multipile's diagrams are, and the \
             flake's shell sets it: run the tests there, with `nix develop -c cargo test`"
        );
    };
    let dir = PathBuf::from(dir);
    let manifest = dir.join("cases.json");
    let text = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("{CASES}: {}: {e}", manifest.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{CASES}: {}: {e}", manifest.display()));
    let all: Vec<Case> = json["cases"]
        .as_array()
        .unwrap_or_else(|| panic!("{CASES}: {} lists no cases", manifest.display()))
        .iter()
        .map(|says| {
            let word = |key: &str| says[key].as_str().unwrap_or("?").to_string();
            Case {
                name: format!("{}/{}", word("group"), word("name")),
                kind: word("kind"),
                says: says.clone(),
                dir: dir.clone(),
            }
        })
        .collect();
    for case in &all {
        assert!(
            KINDS.contains(&case.kind.as_str()),
            "{}: cases.json says it is of kind {:?}, which no test here runs",
            case.name,
            case.kind
        );
    }
    for needed in KINDS {
        assert!(
            all.iter().any(|case| case.kind == needed),
            "{CASES}: {} lists no {needed} case",
            manifest.display()
        );
    }
    all.into_iter().filter(|case| case.kind == kind).collect()
}

/// The kinds a case is of. A manifest must have each, and no other.
const KINDS: [&str; 3] = ["valid", "invalid", "diff"];

fn told(findings: &[Finding]) -> String {
    findings
        .iter()
        .map(|f| {
            format!(
                "{} at {}:{}: {}",
                f.code,
                f.line.unwrap_or(0),
                f.column.unwrap_or(0),
                f.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Runs `check` on every case of a kind, and fails with every case it
/// failed on, each by its name. Each case has a thread of its own, because
/// a picture is slow to draw in a build that is not optimized.
fn every(kind: &str, check: impl Fn(&Case) -> Result<(), String> + Sync) {
    let cases = cases(kind);
    let failed: Vec<String> = std::thread::scope(|scope| {
        let running: Vec<_> = cases
            .iter()
            .map(|case| (case, scope.spawn(|| check(case))))
            .collect();
        running
            .into_iter()
            .filter_map(|(case, thread)| {
                let why = match thread.join() {
                    Ok(result) => result.err()?,
                    Err(_) => "the check panicked".to_string(),
                };
                Some(format!("{}: {why}", case.name))
            })
            .collect()
    });
    assert!(
        failed.is_empty(),
        "{} of {} {kind} cases failed:\n{}",
        failed.len(),
        cases.len(),
        failed.join("\n")
    );
}

/// A drawing is of the format asked for, by how its bytes start.
fn starts(drawn: Result<Drawn, String>, as_: As, with: &[u8]) -> Result<Drawn, String> {
    let drawn = drawn.map_err(|e| format!("it is not drawn as {}: {e}", as_.word()))?;
    if drawn.bytes.starts_with(with) {
        Ok(drawn)
    } else {
        Err(format!("drawn as {}, it is no {}", as_.word(), as_.word()))
    }
}

const HTML: &[u8] = b"<!DOCTYPE html>";
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
const GIF: &[u8] = b"GIF89a";

#[test]
fn every_valid_case_has_no_finding_and_is_drawn_as_a_page_and_a_picture() {
    every("valid", |case| {
        let diagram = case.read("path")?;
        let kind = diagram.kind().word();
        if kind != case.word("type") {
            return Err(format!(
                "it is a {kind} diagram, and cases.json says {}",
                case.word("type")
            ));
        }
        starts(draw(&diagram, As::Html), As::Html, HTML)?;
        starts(draw(&diagram, As::Png), As::Png, PNG)?;
        Ok(())
    });
}

/// The hints of a valid case, and nothing else found of it: each a note at
/// its line and column, which names the library's code and says what to
/// write. Written as `better` rewrites it, which follows one of them, the
/// same diagram has no finding at all.
fn hinted(name: &str, code: &str, hints_at: &[(usize, usize, &str)], better: (&str, &str)) {
    let case = cases("valid")
        .into_iter()
        .find(|case| case.name == name)
        .unwrap_or_else(|| panic!("{CASES}: cases.json lists no valid case {name}"));
    let mut diagram = case.diagram("path");
    let found = findings(&diagram, None);
    assert_eq!(found.len(), hints_at.len(), "{name}: {}", told(&found));
    for (hint, (line, column, fix)) in found.iter().zip(hints_at) {
        assert_eq!(hint.code, "diagram-hint");
        assert_eq!(hint.severity, Severity::Note);
        assert_eq!((hint.line, hint.column), (Some(*line), Some(*column)));
        assert!(
            hint.message.ends_with(&format!("({code})")),
            "{}",
            hint.message
        );
        assert_eq!(hint.fix, *fix);
        assert!(!stops_drawing(hint), "a hint stops nothing being drawn");
    }
    assert_eq!(hints(&diagram), found, "a hint needs no graph");

    let source = diagram.source.take().unwrap();
    let rewritten = source.replace(better.0, better.1);
    assert_ne!(rewritten, source, "{name}: nothing was rewritten");
    diagram.source = Some(rewritten);
    assert_eq!(findings(&diagram, None), vec![], "{name}, as hinted");
}

#[test]
fn a_tier_whose_order_makes_lines_meet_is_hinted_a_better_order() {
    // Either tier, in the other order, keeps the two lines apart.
    hinted(
        "architecture/a-line-shares-a-channel",
        "tier-order-crosses",
        &[
            (14, 3, "write `tier auth posts`, and they meet nowhere"),
            (15, 3, "write `tier cache db`, and they meet nowhere"),
        ],
        ("tier db cache", "tier cache db"),
    );
}

#[test]
fn a_rank_whose_order_makes_lines_meet_is_hinted_a_better_order() {
    // A rank's hint is at the first box of the rank.
    hinted(
        "dependency/a-rank-written-in-a-crossing-order",
        "rank-order-crosses",
        &[
            (
                7,
                3,
                "write them in the order server, pages, and they meet nowhere",
            ),
            (
                9,
                3,
                "write them in the order icons, db, and they meet nowhere",
            ),
        ],
        (
            "  box db \"db\"\n  box icons \"icons\"\n",
            "  box icons \"icons\"\n  box db \"db\"\n",
        ),
    );
}

#[test]
fn every_invalid_case_is_reported_with_the_finding_it_expects_and_where() {
    every("invalid", |case| {
        let expect = &case.says["expect"];
        let (Some(code), Some(line), Some(column)) = (
            expect["code"].as_str(),
            expect["line"].as_u64(),
            expect["column"].as_u64(),
        ) else {
            return Err("cases.json says no finding it expects".to_string());
        };
        let found = findings(&case.diagram("path"), None);
        if let Some(other) = found.iter().find(|f| f.code != "diagram-invalid") {
            return Err(format!(
                "a file that does not read has only diagram-invalid, and it has {}",
                other.code
            ));
        }
        // The format's code is what the message ends with.
        let expected = found.iter().any(|f| {
            f.message.ends_with(&format!("({code})"))
                && f.line == Some(line as usize)
                && f.column == Some(column as usize)
                && f.severity == Severity::Problem
        });
        if expected {
            Ok(())
        } else {
            Err(format!(
                "no {code} at {line}:{column} among: {}",
                if found.is_empty() {
                    "no finding".to_string()
                } else {
                    told(&found)
                }
            ))
        }
    });
}

#[test]
fn every_diff_case_is_compared_and_drawn_as_a_change_still_and_animated() {
    every("diff", |case| {
        let before = case.read("before")?;
        let after = case.read("after")?;
        let drawn = |as_: As, with: &[u8]| {
            let drawn = draw_diff(&before, &after, &Swapped::default(), as_)
                .ok_or("its two versions are not compared")?;
            starts(drawn, as_, with)
        };
        drawn(As::Html, HTML)?;
        drawn(As::Png, PNG)?;
        let gif = drawn(As::Gif, GIF)?;
        if gif.duration_ms.is_none() {
            return Err("its animation has no duration".to_string());
        }
        Ok(())
    });
}
