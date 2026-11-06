//! A pile's config: the `.pile/config` file that declares it.
//!
//! The file is one `key = value` to a line. Whitespace around each is
//! trimmed, blank lines and lines starting with `#` are skipped, and nothing
//! nests. A `#` after a value is part of the value.
//! There are four keys, each given at most once, and no other:
//!
//! - `kind` says the file is a pile's: `gitpile`, or `auxiliary-gitpile` for
//!   an aux pile.
//! - `version` is the format, a number no newer than [`VERSION`].
//! - `id` is 32 lowercase hex characters, minted once ([`new_id`]).
//! - `name` is the word the pile is typed and shown by. It is optional.
//!
//! The config holds only what finds a pile and tells it apart. Its `kind`
//! and `version` say how it is read, and are not properties of the pile.
//! Anything else a pile says about itself is given in a `[pile]` table, in
//! one of the `.toml` declaration files beside the config.
//!
//! A file that breaks any of that is refused, never read as no config. Read
//! as none, it would hand every path under it to the pile above, which is a
//! confident wrong answer in place of a refusal. The refusal names its line
//! as data ([`Refused::line`]), and the config that reads says where each
//! value is written ([`Config::at`]), so a finding or a relation can point
//! at it.
//!
//! [`render`] writes a new config whole, and [`set_line`] rewrites one line
//! of an existing one and leaves every other line as it was.

use std::collections::BTreeMap;
use std::fmt;

use crate::Position;

/// The kind of an ordinary pile, and the one `pile init` writes.
pub const KIND: &str = "gitpile";
/// The kind of an aux pile: the one kind of pile that may create types.
pub const AUX: &str = "auxiliary-gitpile";
/// The kinds a config may say. An aux pile reads as a pile like any other.
pub const KINDS: &[&str] = &[KIND, AUX];
/// The newest format this binary reads, and the one it writes.
pub const VERSION: i64 = 1;
/// How long an id is, in hex characters.
pub const ID_LEN: usize = 32;

pub const KEY_KIND: &str = "kind";
pub const KEY_VERSION: &str = "version";
pub const KEY_ID: &str = "id";
pub const KEY_NAME: &str = "name";

/// What a config says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub kind: String,
    pub version: i64,
    pub id: String,
    /// Empty for a pile with no name.
    pub name: String,
    /// Where each key's value is written, by key: its line, and the column
    /// the value starts at. A key the file has no line for is not here.
    pub at: BTreeMap<&'static str, Position>,
}

/// A config that cannot be read: why, as a sentence, and the line that is
/// wrong, where one is. A config with no `id` line has no line to name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub line: Option<usize>,
    pub why: String,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.why)
    }
}

impl std::error::Error for Refused {}

/// Reads a config. The refusal says which line is wrong, and how.
pub fn parse(content: &[u8]) -> Result<Config, Refused> {
    let mut cfg = Config::default();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();

    let text = String::from_utf8_lossy(content);
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let refuse = |why: String| Refused {
            line: Some(line),
            why,
        };
        let text = raw.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        let Some((key, value)) = text.split_once('=') else {
            return Err(refuse(format!(
                "line {line} is {text:?}, and a line of a config is `key = value`"
            )));
        };
        let (key, value) = (key.trim(), value.trim());
        if seen.insert(key.to_string(), line).is_some() {
            return Err(refuse(format!("line {line} names {key} a second time")));
        }
        let at = Position {
            line,
            column: value_column(raw),
        };
        match key {
            KEY_KIND => {
                if !KINDS.contains(&value) {
                    return Err(refuse(format!(
                        "line {line} says this is a {value:?}, and a pile is a {}",
                        quoted_or(KINDS)
                    )));
                }
                cfg.kind = value.to_string();
                cfg.at.insert(KEY_KIND, at);
            }
            KEY_VERSION => {
                cfg.version = value.parse().map_err(|_| {
                    refuse(format!(
                        "line {line}: the version is {value:?}, which is not a number"
                    ))
                })?;
                cfg.at.insert(KEY_VERSION, at);
            }
            KEY_ID => {
                check_id(value).map_err(|e| refuse(format!("line {line}: {e}")))?;
                cfg.id = value.to_string();
                cfg.at.insert(KEY_ID, at);
            }
            KEY_NAME => {
                check_name(value).map_err(|e| refuse(format!("line {line}: {e}")))?;
                cfg.name = value.to_string();
                cfg.at.insert(KEY_NAME, at);
            }
            _ => {
                return Err(refuse(format!(
                    "line {line} names {key:?}, and a config holds only {KEY_KIND}, \
                     {KEY_VERSION}, {KEY_ID} and {KEY_NAME}"
                )));
            }
        }
    }

    let refuse = |why: String| Refused { line: None, why };
    if !seen.contains_key(KEY_KIND) {
        return Err(refuse(format!(
            "it names no {KEY_KIND}, so nothing says it is a pile's"
        )));
    }
    if !seen.contains_key(KEY_VERSION) {
        return Err(refuse(format!("it names no {KEY_VERSION}")));
    }
    if cfg.version > VERSION {
        return Err(Refused {
            line: seen.get(KEY_VERSION).copied(),
            why: format!(
                "it was written by a newer binary (format {}, and this one reads {VERSION})",
                cfg.version
            ),
        });
    }
    if !seen.contains_key(KEY_ID) {
        return Err(refuse(format!("it names no {KEY_ID}")));
    }
    Ok(cfg)
}

/// The column a line's value starts at, counting from one: the first
/// character after the `=` and the spaces after it.
fn value_column(raw: &str) -> usize {
    let Some(eq) = raw.find('=') else {
        return 1;
    };
    let after = &raw[eq + 1..];
    let start = eq + 1 + (after.len() - after.trim_start().len());
    raw[..start].chars().count() + 1
}

/// `"a"`, `"a" or "b"`, `"a", "b" or "c"`.
fn quoted_or(words: &[&str]) -> String {
    let q: Vec<String> = words.iter().map(|w| format!("{w:?}")).collect();
    match q.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// Whether s is a word: lowercase letters, digits and hyphens, starting with
/// a letter, as a name is.
pub fn is_word(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Refuses a string that is not a name.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("a name cannot be empty".into());
    }
    if !is_word(name) {
        return Err(format!(
            "the name {name:?} is not lowercase letters, digits and hyphens beginning with a letter"
        ));
    }
    Ok(())
}

/// Refuses a string that is not an id.
pub fn check_id(id: &str) -> Result<(), String> {
    if id.len() != ID_LEN {
        return Err(format!(
            "the id {id:?} is {} characters, and an id is {ID_LEN}",
            id.len()
        ));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    {
        return Err(format!("the id {id:?} is not lowercase hex"));
    }
    Ok(())
}

/// A fresh id: sixteen random bytes from the operating system, as lowercase
/// hex.
pub fn new_id() -> std::io::Result<String> {
    use std::io::Read as _;
    let mut b = [0u8; ID_LEN / 2];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// A new pile's config, written whole: two comment lines, then `kind`,
/// `version` and `id`, then `name` when there is one.
pub fn render(id: &str, name: &str) -> String {
    render_kind(KIND, id, name)
}

/// [`render`], for a pile of another kind: an aux pile.
pub fn render_kind(kind: &str, id: &str, name: &str) -> String {
    let mut b = String::new();
    b.push_str("# What this subtree is. One property per line; # is a comment.\n");
    b.push_str("# Committing this file is what carries the pile to a clone.\n");
    b.push_str(&format!("{KEY_KIND} = {kind}\n"));
    b.push_str(&format!("{KEY_VERSION} = {VERSION}\n"));
    b.push_str(&format!("{KEY_ID} = {id}\n"));
    if !name.is_empty() {
        b.push_str(&format!("{KEY_NAME} = {name}\n"));
    }
    b
}

/// A config with one key's line rewritten, or removed when value is empty.
/// None when the file already says that, so nothing needs writing.
///
/// Every other line is left exactly as it was, comments and order included,
/// because they are somebody's. A key the file has no line for gets one
/// under the first key in `after` that the file has, and at the end when it
/// has none of them.
pub fn set_line(content: &str, key: &str, value: &str, after: &[&str]) -> Option<String> {
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();
    let mut at = None;
    let mut keyed: Vec<(String, usize)> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let text = line.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        let Some((name, _)) = text.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name == key {
            at = Some(i);
        } else {
            keyed.push((name.to_string(), i));
        }
    }

    let line = format!("{key} = {value}");
    match at {
        None if value.is_empty() => return None,
        Some(i) if value.is_empty() => {
            lines.remove(i);
        }
        Some(i) if lines[i] == line => return None,
        Some(i) => lines[i] = line,
        None => {
            let place = after
                .iter()
                .find_map(|k| keyed.iter().find(|(n, _)| n == k).map(|(_, i)| *i + 1))
                .unwrap_or_else(|| {
                    // At the end, before the empty string a final newline
                    // leaves.
                    if lines.last().is_some_and(String::is_empty) {
                        lines.len() - 1
                    } else {
                        lines.len()
                    }
                });
            lines.insert(place, line);
        }
    }
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "9f8e7d6c5b4a39281706f5e4d3c2b1a0";

    fn config(body: &str) -> Result<Config, Refused> {
        parse(body.as_bytes())
    }

    #[test]
    fn a_config_is_read_with_where_each_value_is_written() {
        let c = config(&format!(
            "# a comment\n\n  kind = gitpile \nversion=1\nid = {ID}\nname = lib\n"
        ))
        .unwrap();
        assert_eq!(c.kind, "gitpile");
        assert_eq!(c.version, 1);
        assert_eq!(c.id, ID);
        assert_eq!(c.name, "lib");
        let at = |key| c.at.get(key).map(|p: &Position| (p.line, p.column));
        assert_eq!(at(KEY_KIND), Some((3, 10)));
        assert_eq!(at(KEY_VERSION), Some((4, 9)));
        assert_eq!(at(KEY_ID), Some((5, 6)));
        assert_eq!(at(KEY_NAME), Some((6, 8)));
        let aux = config(&format!(
            "kind = auxiliary-gitpile\nversion = 1\nid = {ID}\n"
        ))
        .unwrap();
        assert_eq!(aux.name, "");
        assert_eq!(aux.at.get(KEY_NAME), None);
    }

    #[test]
    fn a_config_that_breaks_a_rule_is_refused_naming_the_line() {
        let base = format!("kind = gitpile\nversion = 1\nid = {ID}\n");
        for (body, line, says) in [
            (
                format!("{base}just words\n"),
                Some(4),
                "line 4 is \"just words\"",
            ),
            (
                format!("{base}name = a\nname = b\n"),
                Some(5),
                "line 5 names name a second time",
            ),
            (
                format!("{base}colour = red\n"),
                Some(4),
                "line 4 names \"colour\", and a config holds only kind, version, id and name",
            ),
            (
                format!("{base}labels = core\n"),
                Some(4),
                "line 4 names \"labels\", and a config holds only",
            ),
            (
                "kind = other\nversion = 1\n".to_string(),
                Some(1),
                "line 1 says this is a \"other\"",
            ),
            (format!("version = 1\nid = {ID}\n"), None, "names no kind"),
            (
                format!("kind = gitpile\nid = {ID}\n"),
                None,
                "names no version",
            ),
            (
                format!("kind = gitpile\nversion = x\nid = {ID}\n"),
                Some(2),
                "is \"x\", which is not a number",
            ),
            (
                format!("kind = gitpile\nid = {ID}\nversion = 2\n"),
                Some(3),
                "written by a newer binary (format 2",
            ),
            (
                "kind = gitpile\nversion = 1\n".to_string(),
                None,
                "names no id",
            ),
            (
                "kind = gitpile\nversion = 1\nid = ABC\n".to_string(),
                Some(3),
                "is 3 characters",
            ),
            (
                format!("kind = gitpile\nversion = 1\nid = {}\n", ID.to_uppercase()),
                Some(3),
                "not lowercase hex",
            ),
            (
                format!("{base}name = Lib\n"),
                Some(4),
                "the name \"Lib\" is not",
            ),
            (
                format!("{base}name = 2d\n"),
                Some(4),
                "the name \"2d\" is not",
            ),
        ] {
            let err = config(&body).unwrap_err();
            assert!(err.why.contains(says), "{body:?}: {err}");
            assert_eq!(err.line, line, "{body:?}: {err}");
        }
    }

    #[test]
    fn a_name_is_a_word_that_starts_with_a_letter() {
        for ok in ["lib", "build-tools", "v2", "e2e"] {
            check_name(ok).unwrap();
        }
        for bad in ["", "-x", "2d", "Lib", "a b", "a/b", "a:b", "a_b"] {
            assert!(check_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn an_aux_pile_s_config_reads_as_any_other() {
        let c = parse(render_kind(AUX, ID, "words").as_bytes()).unwrap();
        assert_eq!(c.kind, AUX);
    }

    #[test]
    fn a_new_id_is_one_and_differs_each_time() {
        let (a, b) = (new_id().unwrap(), new_id().unwrap());
        check_id(&a).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn a_rendered_config_reads_back() {
        let c = parse(render(ID, "lib").as_bytes()).unwrap();
        assert_eq!(
            (c.id.as_str(), c.name.as_str(), c.kind.as_str()),
            (ID, "lib", KIND)
        );
        assert!(!render(ID, "").contains("name"));
    }

    #[test]
    fn setting_a_line_leaves_every_other_line_as_it_was() {
        let file = format!("# mine\nkind = gitpile\nversion = 1\nid = {ID}\n\n# end\n");
        let named = set_line(&file, KEY_NAME, "lib", &[KEY_ID]).unwrap();
        assert_eq!(
            named,
            format!("# mine\nkind = gitpile\nversion = 1\nid = {ID}\nname = lib\n\n# end\n")
        );
        assert_eq!(set_line(&named, KEY_NAME, "lib", &[KEY_ID]), None);
        let renamed = set_line(&named, KEY_NAME, "core", &[KEY_ID]).unwrap();
        assert!(renamed.contains("\nname = core\n"), "{renamed}");
        let forgot = set_line(&named, KEY_NAME, "", &[KEY_ID]).unwrap();
        assert_eq!(forgot, file);
        assert_eq!(set_line(&file, KEY_NAME, "", &[KEY_ID]), None);
        let nowhere = set_line("kind = gitpile\n", KEY_NAME, "x", &[KEY_ID]).unwrap();
        assert_eq!(nowhere, "kind = gitpile\nname = x\n");
    }
}
