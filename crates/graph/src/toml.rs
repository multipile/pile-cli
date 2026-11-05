//! The reader of a `.toml` declaration file: its `version`, then each table
//! by its header, and every key and value with the line and column it is
//! written at (design/model/README.md, section 4).
//!
//! The reader knows TOML and the five headers, and nothing of the types. It
//! turns a file into [`Table`]s, each what its header declares and the keys
//! written under it, and leaves every check of what a key holds to
//! [`crate::checks`]. A file that does not open with its version, or a key
//! at the top that no header names, is a finding here, because only the
//! reader sees the file's shape.

use piles::Position;
use piles::doctor::{Finding, Severity};
use toml_edit::{Document, Item, TableLike};

use crate::finding;

/// The version every declaration file opens with.
pub const VERSION: &str = "gitpile/1";

/// What a table declares, by its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Header {
    /// `[[nodes]]`: a node, at home in this pile.
    Node,
    /// `[[edges]]`: an edge, written as a link.
    Edge,
    /// `[pile]`: this pile's own values.
    Pile,
    /// `[[node-types]]`: a node type, in an aux pile.
    NodeType,
    /// `[[edge-types]]`: an edge type, in an aux pile.
    EdgeType,
}

impl Header {
    /// The header as a file writes it.
    pub fn written(self) -> &'static str {
        match self {
            Header::Node => "[[nodes]]",
            Header::Edge => "[[edges]]",
            Header::Pile => "[pile]",
            Header::NodeType => "[[node-types]]",
            Header::EdgeType => "[[edge-types]]",
        }
    }

    fn of(key: &str) -> Option<Header> {
        Some(match key {
            "nodes" => Header::Node,
            "edges" => Header::Edge,
            "pile" => Header::Pile,
            "node-types" => Header::NodeType,
            "edge-types" => Header::EdgeType,
            _ => return None,
        })
    }
}

/// A key as it is written, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub word: String,
    pub at: Position,
}

/// A value as the file writes it, before anything is asked of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw {
    /// Where its first character is.
    pub at: Position,
    pub is: RawIs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawIs {
    Text(String),
    Integer(i64),
    /// A number with a fraction, as it is written, so that it compares.
    Float(String),
    Boolean(bool),
    /// A date or a time, as it is written.
    Datetime(String),
    List(Vec<Raw>),
    /// An inline table.
    Table,
}

impl Raw {
    /// What the value is, as a sentence says it.
    pub fn what(&self) -> String {
        match &self.is {
            RawIs::Text(s) => format!("{s:?}"),
            RawIs::Integer(n) => n.to_string(),
            RawIs::Float(s) | RawIs::Datetime(s) => s.clone(),
            RawIs::Boolean(b) => b.to_string(),
            RawIs::List(_) => "a list".to_string(),
            RawIs::Table => "a table".to_string(),
        }
    }

    /// The text, when the value is a string.
    pub fn text(&self) -> Option<&str> {
        match &self.is {
            RawIs::Text(s) => Some(s),
            _ => None,
        }
    }
}

/// A key written under a header, and its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: Named,
    pub value: Raw,
}

/// One table of a declaration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The declaration file, from the top: `/lib/.pile/types.toml`.
    pub file: String,
    /// Where the table starts: its header's `[`.
    pub at: Position,
    pub header: Header,
    pub entries: Vec<Entry>,
}

impl Table {
    /// The entry of a key, when the table writes it.
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.key.word == key)
    }
}

/// What one file holds: its tables, and what is wrong with its shape.
#[derive(Debug, Default)]
pub struct Read {
    pub tables: Vec<Table>,
    pub findings: Vec<Finding>,
}

/// Where in a text a byte offset is, as a line and a column counting
/// characters, each from one.
fn position(text: &str, offset: usize) -> Position {
    let offset = offset.min(text.len());
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    Position {
        line,
        column: before[start..].chars().count() + 1,
    }
}

/// Reads one file, named from the top. A file that is not TOML gives one
/// finding, where the parser stopped, and declares nothing. So does a file
/// whose first key is not `version = "gitpile/1"`.
pub fn read(file: &str, bytes: &[u8]) -> Read {
    let mut out = Read::default();
    let Ok(text) = std::str::from_utf8(bytes) else {
        out.findings
            .push(unreadable(file, None, "the file is not UTF-8 text"));
        return out;
    };
    let doc = match Document::parse(text) {
        Ok(doc) => doc,
        Err(e) => {
            let at = e.span().map(|s| position(text, s.start));
            let why = e.message().trim_end_matches('.').to_string();
            out.findings.push(unreadable(file, at, &why));
            return out;
        }
    };
    let walk = Walk { text, file };
    if walk.version(doc.as_table(), &mut out) {
        walk.root(doc.as_table(), &mut out);
    }
    // TOML gives a table's keys by where they sit in the tree, so the file's
    // own order is put back.
    out.tables.sort_by_key(|t| t.at);
    out.findings.sort_by_key(|f| (f.line, f.column));
    out
}

fn unreadable(file: &str, at: Option<Position>, why: &str) -> Finding {
    finding(
        "unreadable-declaration",
        Severity::Problem,
        file,
        at,
        format!("the file is not TOML that reads: {why}, so it declares nothing"),
        "correct the TOML where it stopped",
    )
}

struct Walk<'a> {
    text: &'a str,
    file: &'a str,
}

impl<'a> Walk<'a> {
    fn at(&self, span: Option<std::ops::Range<usize>>) -> Position {
        position(self.text, span.map_or(0, |s| s.start))
    }

    fn key_at(&self, t: &dyn TableLike, name: &str) -> Position {
        self.at(t.get_key_value(name).and_then(|(k, _)| k.span()))
    }

    fn unknown(&self, out: &mut Read, at: Position, message: String, fix: &str) {
        out.findings.push(finding(
            "unknown-key",
            Severity::Problem,
            self.file,
            Some(at),
            message,
            fix,
        ));
    }

    /// Whether the file opens with its version. One that does not is a
    /// finding, and nothing else in it is read.
    fn version(&self, doc: &toml_edit::Table, out: &mut Read) -> bool {
        // A key written before any header is a value, or a dotted key such
        // as pile.title, which TOML reads as a table.
        let first = doc
            .iter()
            .filter(|(_, item)| item.is_value() || matches!(item, Item::Table(t) if t.is_dotted()))
            .map(|(name, _)| (self.key_at(doc, name), name))
            .min();
        let fix = format!("write version = \"{VERSION}\" as the first line of the file");
        let (message, at) = match first {
            None => (
                "the file does not open with its version, so it declares nothing".to_string(),
                None,
            ),
            Some((at, name)) if name != "version" => (
                format!(
                    "the first key is {name}, and a declaration file opens with its version, so \
                     it declares nothing"
                ),
                Some(at),
            ),
            Some((at, _)) => match doc.get("version").and_then(Item::as_str) {
                Some(VERSION) => return true,
                _ => (
                    format!(
                        "the version is not \"{VERSION}\", the one version this binary reads, so \
                         the file declares nothing"
                    ),
                    Some(at),
                ),
            },
        };
        out.findings.push(finding(
            "no-version",
            Severity::Problem,
            self.file,
            at,
            message,
            fix,
        ));
        false
    }

    fn root(&self, doc: &'a toml_edit::Table, out: &mut Read) {
        for (name, item) in doc.iter() {
            if name == "version" {
                continue;
            }
            let at = self.key_at(doc, name);
            let Some(header) = Header::of(name) else {
                self.unknown(
                    out,
                    at,
                    format!("{name} is no header a declaration file takes"),
                    "write [[nodes]], [[edges]], [pile], [[node-types]] or [[edge-types]] \
                     (pile guide declarations)",
                );
                continue;
            };
            match (header, item) {
                (Header::Pile, Item::Table(t)) if !t.is_dotted() => {
                    self.table(header, t, self.at(t.span()), out);
                }
                (Header::Pile, _) => self.unknown(
                    out,
                    at,
                    "pile is one table, written [pile]".to_string(),
                    "write it as [pile], and the keys under it",
                ),
                (_, Item::ArrayOfTables(tables)) => {
                    for t in tables.iter() {
                        self.table(header, t, self.at(t.span()), out);
                    }
                }
                _ => self.unknown(
                    out,
                    at,
                    format!(
                        "{name} is an array of tables, each written {}",
                        header.written()
                    ),
                    &format!("write {} above each one", header.written()),
                ),
            }
        }
    }

    fn table(&self, header: Header, t: &'a toml_edit::Table, at: Position, out: &mut Read) {
        let mut entries = Vec::new();
        for (name, item) in t.iter() {
            let key_at = self.key_at(t, name);
            match item {
                Item::Value(v) => entries.push(Entry {
                    key: Named {
                        word: name.to_string(),
                        at: key_at,
                    },
                    value: self.raw(v),
                }),
                _ => self.unknown(
                    out,
                    key_at,
                    format!(
                        "{name} is a table under {}, and a table here holds only keys with values",
                        header.written()
                    ),
                    "write it as a key with a value",
                ),
            }
        }
        out.tables.push(Table {
            file: self.file.to_string(),
            at,
            header,
            entries,
        });
    }

    /// A value as the file writes it, character for character.
    /// `parsed` is the value as the parser prints it, for a value with no
    /// place in the file.
    fn written(&self, v: &toml_edit::Value, parsed: String) -> String {
        v.span()
            .and_then(|s| self.text.get(s))
            .map_or(parsed, |s| s.trim().to_string())
    }

    fn raw(&self, v: &toml_edit::Value) -> Raw {
        use toml_edit::Value as V;
        let is = match v {
            V::String(s) => RawIs::Text(s.value().clone()),
            V::Integer(n) => RawIs::Integer(*n.value()),
            V::Float(f) => RawIs::Float(self.written(v, f.value().to_string())),
            V::Boolean(b) => RawIs::Boolean(*b.value()),
            V::Datetime(d) => RawIs::Datetime(self.written(v, d.value().to_string())),
            V::Array(a) => RawIs::List(a.iter().map(|v| self.raw(v)).collect()),
            V::InlineTable(_) => RawIs::Table,
        };
        Raw {
            at: self.at(v.span()),
            is,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: usize, column: usize) -> Position {
        Position { line, column }
    }

    fn codes(r: &Read) -> Vec<(&str, Option<usize>, Option<usize>)> {
        r.findings
            .iter()
            .map(|f| (f.code, f.line, f.column))
            .collect()
    }

    #[test]
    fn a_position_counts_lines_and_characters_from_one() {
        assert_eq!(position("ab\ncé d", 0), at(1, 1));
        assert_eq!(position("ab\ncé d", 3), at(2, 1));
        assert_eq!(position("ab\ncé d", 7), at(2, 4));
    }

    #[test]
    fn each_table_is_read_as_its_header_says_with_where_it_starts() {
        let r = read(
            "/p/.pile/a.toml",
            br#"version = "gitpile/1"

[[nodes]]
type = "external"
name = "git"

[pile]
title = "the parser"

[[edges]]
type = "uses"
target = "@lib"

[[node-types]]
name = "external"

[[edge-types]]
name = "uses"
source = ["pile"]
target = ["pile", "external"]
"#,
        );
        assert!(r.findings.is_empty(), "{:?}", r.findings);
        let seen: Vec<(Header, Position, usize)> = r
            .tables
            .iter()
            .map(|t| (t.header, t.at, t.entries.len()))
            .collect();
        assert_eq!(
            seen,
            [
                (Header::Node, at(3, 1), 2),
                (Header::Pile, at(7, 1), 1),
                (Header::Edge, at(10, 1), 2),
                (Header::NodeType, at(14, 1), 1),
                (Header::EdgeType, at(17, 1), 3),
            ]
        );
        let target = r.tables[4].get("target").unwrap();
        assert_eq!(target.key.at, at(20, 1));
        assert_eq!(target.value.at, at(20, 10));
    }

    #[test]
    fn a_file_that_does_not_open_with_its_version_declares_nothing() {
        for (text, place) in [
            ("", (None, None)),
            ("[[nodes]]\ntype = \"a\"\n", (None, None)),
            ("name = 1\nversion = \"gitpile/1\"\n", (Some(1), Some(1))),
            ("version = \"gitpile/2\"\n", (Some(1), Some(1))),
            ("version = 1\n", (Some(1), Some(1))),
            (
                "pile.title = \"x\"\nversion = \"gitpile/1\"\n",
                (Some(1), Some(1)),
            ),
        ] {
            let r = read("/p/.pile/a.toml", text.as_bytes());
            assert_eq!(codes(&r), [("no-version", place.0, place.1)], "{text:?}");
            assert!(r.tables.is_empty());
        }
    }

    #[test]
    fn a_file_that_is_not_toml_is_one_finding_where_the_parser_stopped() {
        let r = read("/p/.pile/a.toml", b"version = \"gitpile/1\"\n[[nodes]\n");
        assert_eq!(codes(&r), [("unreadable-declaration", Some(2), Some(9))]);
        let r = read("/p/.pile/a.toml", b"\xff");
        assert_eq!(codes(&r), [("unreadable-declaration", None, None)]);
    }

    #[test]
    fn a_header_the_format_does_not_take_is_an_unknown_key_at_it() {
        let r = read(
            "/p/.pile/a.toml",
            br#"version = "gitpile/1"

[[links]]
type = "uses"

[nodes]
type = "a"

[[pile]]
title = "x"

[[edges]]
type = "uses"
[edges.more]
x = 1
"#,
        );
        assert_eq!(
            codes(&r),
            [
                ("unknown-key", Some(3), Some(3)),
                ("unknown-key", Some(6), Some(2)),
                ("unknown-key", Some(9), Some(3)),
                ("unknown-key", Some(14), Some(8)),
            ]
        );
        assert_eq!(r.tables.len(), 1);
    }

    #[test]
    fn every_value_toml_writes_is_read_as_it_is() {
        let r = read(
            "/p/.pile/a.toml",
            br#"version = "gitpile/1"
[pile]
a = 1.5
b = 1979-05-27
c = { x = 1 }
d = [1, "two"]
"#,
        );
        let is: Vec<&RawIs> = r.tables[0].entries.iter().map(|e| &e.value.is).collect();
        assert_eq!(
            is,
            [
                &RawIs::Float("1.5".into()),
                &RawIs::Datetime("1979-05-27".into()),
                &RawIs::Table,
                &RawIs::List(vec![
                    Raw {
                        at: at(6, 6),
                        is: RawIs::Integer(1)
                    },
                    Raw {
                        at: at(6, 9),
                        is: RawIs::Text("two".into())
                    },
                ]),
            ]
        );
    }
}
