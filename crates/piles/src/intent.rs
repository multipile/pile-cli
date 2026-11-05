//! A pile's intent: the `.pile/intent.md` that says why the subtree is a
//! pile.
//!
//! Nothing in it is parsed but its headline. The rest is prose for whoever
//! reads it, and `pile show` prints it whole.

use crate::Position;

/// The headline: the first paragraph of prose, joined into one line. Empty
/// when the file holds no prose yet.
///
/// An intent has no heading, and starts with its headline. One written with
/// a `# ` heading line on top still reads the same, because a heading is not
/// prose and is skipped.
///
/// Prose is a line that is not blank, not a heading, not a rule, and not
/// inside an `<!-- -->` comment. A comment that is never closed runs to the
/// end of the file. The paragraph runs from its first line to the first
/// blank line, or to the first line that opens a block of its own: a
/// heading, a rule, a bullet, a quote or a numbered item. The lines are
/// trimmed, and joined with one space.
pub fn headline(content: &str) -> String {
    first_paragraph(content)
        .map(|(_, text)| text)
        .unwrap_or_default()
}

/// Where the headline starts: the line and column of its first character.
/// None when the file holds no prose yet.
pub fn headline_at(content: &str) -> Option<Position> {
    first_paragraph(content).map(|(at, _)| at)
}

/// The headline, and where its first character is in the file.
fn first_paragraph(content: &str) -> Option<(Position, String)> {
    let (stripped, from) = without_comments(content);
    let lines: Vec<&str> = stripped.split('\n').collect();
    // How many characters of the stripped text come before this line.
    let mut start = 0;
    for (i, line) in lines.iter().enumerate() {
        let text = line.trim();
        if !prose(text) {
            start += line.chars().count() + 1;
            continue;
        }
        let mut said = vec![text];
        for next in &lines[i + 1..] {
            let more = next.trim();
            if more.is_empty() || opens(more) {
                break;
            }
            said.push(more);
        }
        let lead = line.chars().take_while(|c| c.is_whitespace()).count();
        return Some((from[start + lead], said.join(" ")));
    }
    None
}

/// Where a heading opens the intent: the first line that is not blank,
/// when it starts with `#`, and the column of its `#`. None for an intent
/// that starts with its headline, as an intent is written.
pub fn heading_at(content: &str) -> Option<Position> {
    let (i, line) = content
        .split('\n')
        .enumerate()
        .find(|(_, l)| !l.trim().is_empty())?;
    let lead = line.chars().take_while(|c| c.is_whitespace()).count();
    line.trim_start().starts_with('#').then_some(Position {
        line: i + 1,
        column: lead + 1,
    })
}

/// Where a `<!--` comment that is never closed opens: the line and the
/// column of its `<`. Everything after it is hidden, as a Markdown renderer
/// hides it.
pub fn unclosed_comment(content: &str) -> Option<Position> {
    let mut rest = content;
    let mut at = 0;
    while let Some(open) = rest.find("<!--") {
        let start = at + open;
        match rest[open + 4..].find("-->") {
            Some(close) => {
                let next = open + 4 + close + 3;
                at += next;
                rest = &rest[next..];
            }
            None => {
                let before = &content[..start];
                let line_start = before.rfind('\n').map_or(0, |i| i + 1);
                return Some(Position {
                    line: before.matches('\n').count() + 1,
                    column: before[line_start..].chars().count() + 1,
                });
            }
        }
    }
    None
}

/// A rule: nothing but `-`, `=`, `*` and `_`.
fn rule(text: &str) -> bool {
    text.trim_matches(['-', '=', '*', '_']).is_empty()
}

/// A line that can start a headline. A bullet can: it says something.
fn prose(text: &str) -> bool {
    !(text.is_empty() || text.starts_with('#') || rule(text))
}

/// A line that starts a block of its own, and so ends the paragraph above.
fn opens(text: &str) -> bool {
    if text.starts_with('#') || rule(text) {
        return true;
    }
    if ["- ", "* ", "+ ", "> "].iter().any(|m| text.starts_with(m)) {
        return true;
    }
    let digits = text.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && text[digits..].starts_with(". ")
}

/// The text without its `<!-- -->` comments, and where each of its
/// characters is in the file. A comment goes whole, its line breaks with
/// it. So text right after `-->` carries on the line before `<!--`. A
/// comment that is never closed runs to the end of the file.
fn without_comments(s: &str) -> (String, Vec<Position>) {
    let mut out = String::new();
    let mut from = Vec::new();
    let mut at = Position { line: 1, column: 1 };
    let mut rest = s;
    let mut hidden = false;
    while let Some(c) = rest.chars().next() {
        let skip = if !hidden && rest.starts_with("<!--") {
            hidden = true;
            4
        } else if hidden && rest.starts_with("-->") {
            hidden = false;
            3
        } else {
            if !hidden {
                out.push(c);
                from.push(at);
            }
            1
        };
        for c in rest.chars().take(skip) {
            if c == '\n' {
                at = Position {
                    line: at.line + 1,
                    column: 1,
                };
            } else {
                at.column += 1;
            }
            rest = &rest[c.len_utf8()..];
        }
    }
    (out, from)
}

/// What `pile init` writes as a new pile's intent: a comment asking the
/// question only a person can answer, and no heading. The pile's path is
/// `pile`'s to say, and a path written in the file would go stale when the
/// pile moves. It is a question, not an answer, so its headline is empty
/// until somebody replaces the comment.
pub fn template() -> &'static str {
    r#"<!--
Why this pile exists: the one thing about it that git cannot answer.

Replace this comment with three parts, and no heading:

- First, one claim about what the pile is for: its headline. A reader
  decides from this paragraph alone whether a new file belongs here.
- Then, if the claim needs more, what belongs here, and what sort of thing
  does not.
- Last, two sentences and nothing after them, usually:

      Read this pile to find out <X>. Change it when <Y>.

`pile guide intent` says what each part holds, and
`pile guide write-an-intent` how to write them. Until this comment is
replaced, the pile has no headline, and `pile doctor` reports it as a
problem.
-->
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: usize, column: usize) -> Option<Position> {
        Some(Position { line, column })
    }

    #[test]
    fn the_headline_is_the_first_paragraph_of_prose_on_one_line() {
        let text = "# /lib\n\n<!-- a\nquestion -->\nThe parser, and\n  only the parser.\n\nMore.\n";
        assert_eq!(headline(text), "The parser, and only the parser.");
    }

    #[test]
    fn an_intent_with_a_heading_and_one_without_have_the_same_headline() {
        let bare = "The parser, and\nonly the parser.\n\nRead this pile to find out how. Change it when.\n";
        let headed = format!("# /lib\n\n{bare}");
        assert_eq!(headline(bare), "The parser, and only the parser.");
        assert_eq!(headline(&headed), headline(bare));
    }

    #[test]
    fn a_heading_is_found_on_the_first_line_that_is_not_blank() {
        assert_eq!(heading_at("# /lib\n\nThe parser.\n"), at(1, 1));
        assert_eq!(heading_at("\n\n  ## Lib\nThe parser.\n"), at(3, 3));
        assert_eq!(heading_at("The parser.\n# later\n"), None);
        assert_eq!(heading_at(""), None);
    }

    #[test]
    fn a_comment_never_closed_is_found_on_the_line_it_opens() {
        assert_eq!(
            unclosed_comment("a\n<!-- b -->\nc\n<!-- open\nd\n"),
            at(4, 1)
        );
        assert_eq!(unclosed_comment("a\nnote <!-- open\n"), at(2, 6));
        assert_eq!(unclosed_comment("<!-- a -->\n<!-- b -->\n"), None);
        assert_eq!(unclosed_comment(template()), None);
    }

    #[test]
    fn a_block_of_its_own_ends_the_headline() {
        for (text, want) in [
            ("One claim\n- a bullet\n", "One claim"),
            ("One claim\n> a quote\n", "One claim"),
            ("One claim\n1. first\n", "One claim"),
            ("One claim\n## Heading\n", "One claim"),
            ("One claim\n---\n", "One claim"),
            (
                "- a bullet starts one\nand goes on\n",
                "- a bullet starts one and goes on",
            ),
            ("2024 was a year\nof change\n", "2024 was a year of change"),
        ] {
            assert_eq!(headline(text), want, "{text:?}");
        }
    }

    #[test]
    fn a_file_of_headings_rules_and_comments_has_no_headline() {
        for text in ["", "# /lib\n\n---\n***\n", "<!-- never closed\nSome text\n"] {
            assert_eq!(headline(text), "", "{text:?}");
        }
        assert_eq!(headline(template()), "");
        assert!(template().starts_with("<!--\n"), "no heading");
    }

    #[test]
    fn a_comment_goes_whole_with_its_line_breaks() {
        assert_eq!(headline("First<!--\n-->\n\nSecond\n"), "First");
        assert_eq!(headline("A <!-- aside --> claim.\n"), "A  claim.");
        assert_eq!(headline("Line one<!--\n-->line two\n"), "Line oneline two");
    }

    #[test]
    fn the_headline_is_found_where_it_starts_in_the_file() {
        for (text, want) in [
            ("The parser.\n", at(1, 1)),
            ("# /lib\n\n  The parser.\n", at(3, 3)),
            ("<!-- a\nquestion -->\nThe parser.\n", at(3, 1)),
            ("<!--\n-->Hello\n", at(2, 4)),
            ("<!-- é -->é claim\n", at(1, 11)),
            ("# /lib\n", None),
        ] {
            assert_eq!(headline_at(text), want, "{text:?}");
        }
        assert_eq!(headline_at(template()), None);
    }
}
