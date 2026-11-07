//! `pile gitdiff`: what changed between two revisions, by pile.

use serde_json::json;

use crate::harness::*;
use crate::piles::nested;

/// nested(), on a branch `work` that left `main` after "lay out".
fn on_work() -> Repo {
    let r = nested();
    r.git(&["switch", "-q", "-c", "work"]);
    r
}

fn head(r: &Repo) -> String {
    r.git(&["rev-parse", "HEAD"]).trim().to_string()
}

/// The first eight characters of an id, as text and Markdown print it.
fn short(id: &str) -> &str {
    &id[..8]
}

#[test]
fn only_the_changed_piles_are_listed_each_with_its_headline() {
    let r = on_work();
    let parted = head(&r);
    r.write("lib/parse.rs", "fn parse() { todo!() }\n");
    r.write("lib/sub/fresh.rs", "new\n");
    r.commit("work");
    let now = head(&r);
    assert_eq!(
        r.stdout(&["gitdiff", "main...HEAD"]),
        format!(
            "from  {}  where HEAD left main\n\
             to    {}  HEAD\n\n\
             @lib  :/lib  bbbbbbbb\n   intent    The parser, and only the parser.\n   M  :/lib/parse.rs\n\n\
             :/lib/sub  cccccccc\n   intent    (no headline)\n   A  :/lib/sub/fresh.rs\n\n\
             2 of 4 piles changed, 2 paths\n",
            short(&parted),
            short(&now)
        )
    );
    let v = r.result(&["gitdiff", "main...HEAD"]);
    assert_eq!(v["from"], json!({"name": "main", "revision": parted}));
    assert_eq!(v["merge_base"], json!(parted));
    assert_eq!(v["to"], json!({"name": "HEAD", "revision": now}));
    assert_eq!(v["piles"][0]["name"], "lib");
    assert_eq!(
        v["piles"][0]["headline"],
        "The parser, and only the parser."
    );
    assert_eq!(v["piles"][0]["intent_changed"], false);
    assert_eq!(v["piles"][1]["name"], json!(null));
    assert_eq!(
        v["total"],
        json!({"piles": 4, "changed": 2, "unchanged": 2, "paths": 2, "crossed": 0, "in_no_pile": 0})
    );
}

#[test]
fn two_revisions_are_named_spaced_or_with_two_dots() {
    let r = on_work();
    let parted = head(&r);
    r.write("lib/parse.rs", "edited\n");
    r.commit("work");
    let now = head(&r);
    let spaced = r.result(&["gitdiff", "main", "work"]);
    assert_eq!(spaced["from"], json!({"name": "main", "revision": parted}));
    assert_eq!(spaced["merge_base"], json!(null));
    assert_eq!(spaced["to"], json!({"name": "work", "revision": now}));
    assert_eq!(spaced["total"]["paths"], 1);
    let dotted = r.result(&["gitdiff", "main..work"]);
    assert_eq!(dotted, spaced);
    // A side left out is HEAD.
    assert_eq!(r.result(&["gitdiff", "main.."])["to"]["name"], "HEAD");
    assert_eq!(r.result(&["gitdiff", "..main"])["from"]["name"], "HEAD");
    assert_eq!(
        r.result(&["gitdiff", "main..."])["merge_base"],
        json!(parted)
    );
    contains!(
        r.stdout(&["gitdiff", "main", "work"]),
        format!(
            "from  {}  main\nto    {}  work\n",
            short(&parted),
            short(&now)
        )
    );
}

#[test]
fn all_lists_the_unchanged_piles_too() {
    let r = on_work();
    r.write("README", "changed\n");
    r.commit("work");
    let out = r.stdout(&["gitdiff", "--all", "main..."]);
    contains!(
        out,
        "@top  :/  aaaaaaaa\n   intent    The whole repository.\n   M  :/README\n",
        "@lib  :/lib  bbbbbbbb\n   intent    The parser, and only the parser.\n   (unchanged)\n",
        "1 of 4 piles changed, 1 path\n"
    );
    assert_eq!(
        r.result(&["gitdiff", "--all", "main..."])["piles"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        r.result(&["gitdiff", "main..."])["piles"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn with_three_dots_what_reached_the_base_after_head_left_it_is_not_part_of_the_change() {
    let r = on_work();
    r.write("lib/parse.rs", "on work\n");
    r.commit("work");
    r.git(&["switch", "-q", "main"]);
    r.write("lib-v2/x.rs", "on main\n");
    r.commit("main moved on");
    r.git(&["switch", "-q", "work"]);
    let paths = |v: &serde_json::Value| -> Vec<String> {
        v["piles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["path"].as_str().unwrap().to_string())
            .collect()
    };
    let v = r.result(&["gitdiff", "main...work"]);
    assert_eq!(paths(&v), [":/lib"]);
    assert_ne!(v["from"]["revision"], v["merge_base"]);
    // With two dots, main's own commit is part of the difference.
    let v = r.result(&["gitdiff", "main..work"]);
    assert_eq!(paths(&v), [":/lib", ":/lib-v2"]);
}

#[test]
fn a_move_across_a_boundary_is_a_half_in_each_and_a_rewritten_intent_says_so() {
    let r = on_work();
    r.git(&["mv", "lib/parse.rs", "lib-v2/parse.rs"]);
    r.intent("lib", "The parser, which moved to v2.\n");
    r.commit("move");
    let out = r.stdout(&["gitdiff", "main..."]);
    contains!(
        out,
        "@lib  :/lib  bbbbbbbb\n   intent    The parser, which moved to v2.  (intent rewritten)\n   \
         M  :/lib/.pile/intent.md\n   D  :/lib/parse.rs  (left this pile, now :/lib-v2/parse.rs)\n",
        "   A  :/lib-v2/parse.rs  (arrived from :/lib/parse.rs)\n",
        "2 of 4 piles changed, 3 paths, 2 paths crossed a boundary\n"
    );
    let v = r.result(&["gitdiff", "main..."]);
    assert_eq!(v["piles"][0]["intent_changed"], true);
    assert_eq!(v["piles"][1]["intent_changed"], false);
    assert_eq!(v["total"]["crossed"], 2);
}

#[test]
fn a_rewritten_declaration_file_says_declarations_rewritten() {
    let r = on_work();
    r.write(
        "lib/.pile/links.toml",
        "[[edges]]\ntype = \"relies-on\"\ntarget = \"@v2\"\ntitle = \"reads what v2 writes\"\n",
    );
    r.commit("link");
    contains!(
        r.stdout(&["gitdiff", "main..."]),
        "   intent    The parser, and only the parser.  (declarations rewritten)\n"
    );
    let v = r.result(&["gitdiff", "main..."]);
    assert_eq!(v["piles"][0]["declarations_changed"], true);
    assert_eq!(v["piles"][0]["intent_changed"], false);
    let md = r.stdout(&["gitdiff", "main...", "--format", "markdown"]);
    contains!(md, "only the parser. **Declarations rewritten.** |");
}

#[test]
fn a_change_in_no_pile_is_kept() {
    let r = Repo::new();
    r.declare("a", "a", "a");
    r.commit("a");
    r.git(&["switch", "-q", "-c", "work"]);
    r.write("stray", "s\n");
    r.commit("stray");
    contains!(
        r.stdout(&["gitdiff", "main..."]),
        "in no pile\n   A  :/stray\n",
        "0 of 1 pile changed, 0 paths, 1 path in no pile\n"
    );
    assert_eq!(
        r.result(&["gitdiff", "main..."])["in_no_pile"][0]["path"],
        ":/stray"
    );
    let md = r.stdout(&["gitdiff", "main...", "--format", "markdown"]);
    contains!(
        md,
        "### 0 of 1 pile changed, and 1 path in no pile\n",
        "| _In no pile_ | | | 1 |\n",
        "<details>\n<summary>In no pile · 1 path</summary>\n\n- `A` `:/stray`\n\n</details>\n"
    );
    assert!(!md.contains("`@a`"), "an unchanged pile is left out: {md}");
}

#[test]
fn one_revision_or_none_is_compared_with_the_working_copy() {
    let r = on_work();
    r.write("lib/parse.rs", "edited\n");
    r.write("lib/sub/draft.rs", "draft\n");
    let out = r.stdout(&["gitdiff"]);
    contains!(out, "to    the working copy\n", "   M  :/lib/parse.rs\n");
    assert!(
        !out.contains("draft.rs"),
        "an untracked file is not listed, as in git diff: {out}"
    );
    let v = r.result(&["gitdiff"]);
    assert_eq!(v["from"]["name"], "HEAD");
    assert_eq!(v["to"], json!(null));
    assert_eq!(v["total"]["paths"], 1);
    assert_eq!(r.result(&["gitdiff", "main"])["total"]["paths"], 1);
    // Committed against itself, nothing changed.
    assert_eq!(
        r.result(&["gitdiff", "HEAD", "HEAD"])["total"]["changed"],
        0
    );
}

#[test]
fn markdown_is_a_table_of_the_changed_piles_then_their_paths() {
    let r = on_work();
    r.intent(
        "lib",
        "Parses `<text>` into *trees*, [see](http://x) <b>bold</b> | piped.\n",
    );
    r.git(&["mv", "lib/parse.rs", "lib-v2/parse.rs"]);
    r.write("stray", "s\n");
    r.commit("work");
    let parted = r.git(&["rev-parse", "main"]).trim().to_string();
    let now = head(&r);
    let md = r.stdout(&["gitdiff", "main...HEAD", "--format", "markdown"]);
    assert!(md.starts_with("<!-- pile gitdiff -->\n"), "{md}");
    contains!(
        md,
        "### 3 of 4 piles changed\n\n",
        format!(
            "From `{}`, where `HEAD` left `main`, to `{}` (`HEAD`): 4 paths changed.\n",
            short(&parted),
            short(&now)
        ),
        "| Pile | Path | What it is for | Paths |\n| --- | --- | --- | ---: |\n",
        "| `@lib` | `:/lib` | Parses `<text>` into \\*trees\\*, \\[see\\](http://x) \\<b\\>bold\\</b\\> \\| piped. **Intent rewritten.** | 2 |\n",
        "| `@v2` | `:/lib-v2` | _No headline yet._ | 1 |\n",
        "| `@top` | `:/` | The whole repository. | 1 |\n",
        "<details>\n<summary><code>@lib</code> · 2 paths</summary>\n\n\
         - `M` `:/lib/.pile/intent.md`\n\
         - `D` `:/lib/parse.rs` (left this pile, now `:/lib-v2/parse.rs`)\n\n\
         </details>\n"
    );
    assert!(
        !md.contains(":/lib/sub"),
        "an unchanged pile is left out: {md}"
    );
    // A refusal in markdown is printed as it is in text.
    let a = r.answered(&[], &["gitdiff", "nope", "--format", "markdown"]);
    assert_eq!(a.code, 2);
    assert!(a.stdout.is_empty());
    assert!(
        a.stderr.starts_with("pile: \"nope\" names no revision"),
        "{}",
        a.stderr
    );
}

#[test]
fn a_path_in_markdown_is_shown_as_it_is() {
    let r = on_work();
    r.write("lib/a`b|<i>.rs", "x\n");
    r.commit("odd name");
    contains!(
        r.stdout(&["gitdiff", "main...", "--format", "markdown"]),
        "- `A` ``:/lib/a`b|<i>.rs``\n"
    );
}

#[test]
fn only_gitdiff_answers_in_markdown() {
    let r = nested();
    for args in [&["status"][..], &["list"], &["init", "x"]] {
        let mut all = args.to_vec();
        all.extend(["--format", "markdown"]);
        let a = r.answered(&[], &all);
        assert_eq!(a.code, 2, "pile {}", args.join(" "));
        assert_eq!(
            a.stderr,
            format!(
                "pile: pile {} answers in text or json, and only pile gitdiff answers in markdown\n",
                args[0]
            )
        );
    }
    assert!(!r.path().join("x/.pile").exists(), "init wrote nothing");
}

#[test]
fn what_it_cannot_compare_it_refuses() {
    let r = on_work();
    let v = r.refused(&["gitdiff", "gone"], 2);
    assert_eq!(v["error"]["code"], "unknown-revision");
    let v = r.refused(&["gitdiff", "main", "work", "main"], 2);
    assert_eq!(v["error"]["code"], "usage");
    let v = r.refused(&["gitdiff", "main..work", "main"], 2);
    assert_eq!(v["error"]["code"], "usage");

    r.git(&["switch", "-q", "--orphan", "alone"]);
    r.declare(".", "a", "top");
    r.commit("a history of its own");
    let v = r.refused(&["gitdiff", "main...alone"], 1);
    assert_eq!(v["error"]["code"], "no-shared-history");
    // Two dots need no shared history.
    assert_eq!(
        r.result(&["gitdiff", "main..alone"])["merge_base"],
        json!(null)
    );
}

#[test]
fn a_repository_with_no_pile_or_no_revision_is_refused() {
    let r = Repo::new();
    r.write("a", "a\n");
    r.commit("a");
    let v = r.refused(&["gitdiff"], 1);
    assert_eq!(v["error"]["code"], "no-piles");
    let e = Repo::empty();
    let v = e.refused(&["gitdiff"], 2);
    assert_eq!(v["error"]["code"], "unknown-revision");
}
