//! `pile files`: one pile's own files, and how long each is.

use serde_json::json;

use crate::harness::*;
use crate::piles::nested;

#[test]
fn a_pile_owns_its_files_less_the_piles_under_it() {
    let r = nested();
    r.write("lib/new.txt", "untracked\nand more");
    let out = r.stdout(&["files", "@lib"]);
    assert_eq!(
        out,
        "@lib  :/lib  bbbbbbbb\n\n   \
         lines   bytes  path\n       \
         4      76  :/lib/.pile/config\n       \
         5     122  :/lib/.pile/intent.md\n       \
         2      18  :/lib/new.txt\n       \
         1      14  :/lib/parse.rs\n\n\
         4 files, 12 lines, 230 bytes\n\
         not counted, being piles of their own: pile files :/lib/sub\n"
    );
    let v = r.result(&["files", "lib"]);
    assert_eq!(
        v["not_counted"],
        json!([{"name": null, "path": ":/lib/sub"}])
    );
    assert_eq!(v["total"], json!({"files": 4, "lines": 12, "bytes": 230}));
}

#[test]
fn a_binary_file_has_no_lines_and_a_link_is_as_long_as_where_it_points() {
    let r = Repo::new();
    r.declare(".", "a", "");
    r.write("blob.bin", "\u{0}\n\n\n");
    std::os::unix::fs::symlink("blob.bin", r.path().join("link")).unwrap();
    let v = r.result(&["files"]);
    let files = v["files"].as_array().unwrap();
    let by: Vec<_> = files
        .iter()
        .map(|f| (f["path"].clone(), f["lines"].clone(), f["bytes"].clone()))
        .collect();
    assert_eq!(by[1], (json!(":/blob.bin"), json!("binary"), json!(4)));
    assert_eq!(by[2], (json!(":/link"), json!("link"), json!(8)));
}

#[test]
fn a_pile_with_only_a_config_lists_its_config() {
    let r = nested();
    let v = r
        .answered_in("lib/sub", &[], &["files", "--format", "json"])
        .json();
    let paths: Vec<_> = v["result"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].clone())
        .collect();
    assert_eq!(
        paths,
        [json!(":/lib/sub/.pile/config"), json!(":/lib/sub/deep.rs")]
    );
}

#[test]
fn a_path_that_is_not_a_pile_is_refused_naming_the_pile_it_is_in() {
    let r = nested();
    std::fs::create_dir_all(r.path().join("lib/src")).unwrap();
    let v = r.refused(&["files", "lib/src"], 1);
    assert_eq!(v["error"]["code"], "not-a-pile");
    assert_eq!(v["next"][0]["command"], "pile show @lib");
    // Nothing is looked up on disk: a path not there is the repository's
    // path all the same, and it is in the top pile.
    let nowhere = r.refused(&["files", "nowhere"], 1);
    assert_eq!(nowhere["error"]["code"], "not-a-pile");
    assert_eq!(nowhere["next"][0]["command"], "pile show @top");
    assert_eq!(r.refused(&["files", "a", "b"], 2)["error"]["code"], "usage");
}
