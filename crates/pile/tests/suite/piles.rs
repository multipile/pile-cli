//! A repository of nested piles the read commands' tests share.

use crate::harness::*;

/// `/` (@top), `/lib` (@lib, with an intent), `/lib/sub` (no name, only a
/// config), and `/lib-v2` (@v2), with a file in each, committed. The top's
/// intent has no heading, as an intent is written; /lib's has the `# /lib`
/// heading line an older intent opens with.
pub fn nested() -> Repo {
    let r = Repo::new();
    r.declare(".", "a", "top");
    r.intent(".", "The whole repository.\n");
    r.declare("lib", "b", "lib");
    r.intent("lib", "# /lib\n\nThe parser, and only the parser.\n\nRead this pile to find out how text is parsed. Change it when the grammar does.\n");
    r.declare("lib/sub", "c", "");
    r.declare("lib-v2", "d", "v2");
    r.write("README", "top\n");
    r.write("lib/parse.rs", "fn parse() {}\n");
    r.write("lib/sub/deep.rs", "one\ntwo\n");
    r.write("lib-v2/x.rs", "x\n");
    r.commit("lay out");
    r
}
