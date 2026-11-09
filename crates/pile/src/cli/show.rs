use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value, json};

use graph::{Node, Relations};
use piles::{Pile, Structure};
use storage::{Revision, State, Subtree};

use super::{
    Out, Result, links, name_of, one_path, open, path_arg, pile_value, plural, say, say_all,
    switch, target,
};
use crate::repository::Repository;

pub(super) fn command() -> Command {
    Command::new("show")
        .about("Print one pile whole: its config, where it sits, and its intent")
        .long_about(
            "Print one pile whole: what its config says, the pile it sits in and the\n\
             piles it holds, when its intent was last written, and the intent itself,\n\
             as its author wrote it. Read it before you change the pile.\n\n\
             With no pile named, show prints the pile you are in: the nearest pile at\n\
             or above this directory. A path you name has to be a pile. If it is not,\n\
             show refuses and names the pile the path is in, rather than show that\n\
             pile, so that show can say whether a subtree is declared.\n\n\
             Under the heading, show prints:\n  \
             - mounted: its path relative to the pile it is mounted in, and that\n    \
             pile, unless it is a top of the mount tree. A pile is mounted in its\n    \
             parent: pile list --tree prints the whole tree\n  \
             - parent: the pile it sits in, unless it is at the top\n  \
             - children: the piles it holds directly, each with its path relative\n    \
             to this pile\n  \
             - written: the commit that last changed the intent, its date, and how\n    \
             many commits have changed the pile since. The count is not a verdict on\n    \
             the intent, but an intent many commits behind its pile is one to read\n    \
             asking whether it is still true. The count follows the intent's path,\n    \
             so for a pile moved with git mv it starts again at the move. An intent\n    \
             no commit holds yet is not committed yet. A pile with no intent has no\n    \
             written line.\n  \
             - a line for each value this pile gives in [pile]: its title, its\n    \
             description, and each key of its own, such as labels\n  \
             - types: each type this pile creates, when it is an aux pile, a line\n    \
             each: node or edge, its name, the node types each end of an edge\n    \
             type may be, and its title. Under an edge type, its do and dont:\n    \
             what a link's description has to name, and what it must not\n  \
             - nodes: each node at home in this pile, by its reference, type:name,\n    \
             and its title\n  \
             - links: each link of this pile, a line each: -> for an edge whose\n    \
             source is this pile, <- for one whose target is, and -- for one\n    \
             this pile writes, or that ends at a node at home in it, with this\n    \
             pile at neither end. Then its type, the other end (both ends for --)\n    \
             and its title. An end is named as it is typed: @name, type:name, or\n    \
             the path of a pile with no name. A link written in another pile says\n    \
             so. pile guide links says how a link is written. The answer's next\n    \
             names the aux pile that creates each type the links are of\n  \
             - unread: the files in its .pile/ directory that pile does not read,\n    \
             besides the config, the intent and the .toml files. pile refuses\n    \
             nothing for them. Another program may keep a file there.\n\
             Then it prints the intent whole, under its path, or says it is not there.\n\
             pile guide intent says what an intent holds, and which part is its\n\
             headline.\n\
             The parent and children are worked out from the tree on every run.\n\n\
             show judges nothing. A value, a node or a link that does not check is\n\
             left out, and pile doctor says why. --full prints\n\
             whole ids, which a result always holds.\n\n\
             --at <revision> prints the pile as that revision declares it: its\n\
             config, parent, children, links and intent, read from the revision\n\
             and not the working copy, under an at line naming it. The pile is\n\
             named as it is at that revision, and a pile it does not declare is\n\
             refused. It has no written line, which counts from HEAD. To see how a\n\
             change rewrote a pile's intent or its links, show the pile at the\n\
             change's base and again without --at.",
        )
        .override_usage("pile show [--full] [--at <revision>] [pile]")
        .after_help(format!(
            "Examples:\n  \
             pile show                 the pile you are in\n  \
             pile show :/packages/lib  one named by its path from the top\n  \
             pile show @lib            one named by its name\n  \
             pile show @lib --at main  the same pile, as main declares it\n\n\
             Refusals:\n{}\n  \
             unknown-revision         2  --at names no revision: git fetch, or name another",
            target::REFUSALS
        ))
        .arg(path_arg())
        .arg(switch(
            "full",
            "print whole ids rather than the first eight",
        ))
        .arg(
            Arg::new("at")
                .long("at")
                .value_name("revision")
                .help("show the pile as this revision declares it, not the working copy"),
        )
}

/// The result is the pile, `name`, `path`, `id`, `kind` and `version`;
/// `mount`, as `in` (the pile it is mounted in, as it is typed) and `path`
/// (its path relative to that pile), or `null` at a top of the mount tree;
/// `parent`, `name`, `path` and `id`, or `null` at the top; `children`, the
/// same for each, with its `mount`, its path relative to this pile, in path
/// order; `written`, as `commit` (the whole id, or
/// `null` when no commit holds the intent), `date` and `since`, or `null`
/// for a pile with no intent; `values`, each property the pile gives in
/// `[pile]`, with a list of its values; `types`, each type the pile creates,
/// as `name`, `edge` (true for an edge type), `source` and `target` (lists,
/// empty for a node type, in sorted order), `title`, `description`, `do`
/// and `dont` (each `null` when it is not given) and `at`; `nodes`, each node at home
/// in the pile, as `reference` (`type:name`), `type`,
/// `name`, `title` (or `null`) and `at`, where its table starts as
/// `:/path:line:column`; `links`, each link of the pile, as `direction`
/// (`to` when the pile is its source, `from` when it is its target,
/// `between` when it is neither), `type`, `other` (the other end as it is
/// typed: `@name`, `type:name`, or the path of a pile with no name; `null`
/// for `between`), `source` and `target`, `title` and `description` (or
/// `null`), `values`
/// (each other property it gives, with a list of its values), `at` and
/// `written_in` (the pile whose `.pile/` writes it, as it is typed); `unread`, the other files in its `.pile/`, as
/// the pile names them; and `files`, the intent as `name` and `text`, `text`
/// being `null` when it is not there. With --at, `at` is the revision as
/// `name` and `revision`, and `written` is `null`; without it, `at` is
/// `null`.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    let arg = one_path("show", m)?;
    let full = m.get_flag("full");
    let r = open()?;
    let at: Option<(String, Revision)> = match m.get_one::<String>("at") {
        Some(name) => Some((name.clone(), r.storage().resolve(name)?)),
        None => None,
    };
    let state = match &at {
        Some((_, rev)) => State::At(rev.clone()),
        None => State::Working,
    };
    let s = match &at {
        Some(_) => {
            let s = piles::discover(r.storage(), &state)?;
            if s.is_empty() {
                return Err(target::no_piles(&r));
            }
            s
        }
        None => target::declared(&r)?,
    };
    let pile = target::chosen(&r, &s, arg.as_deref())?;
    let id = |p: &Pile| {
        if full {
            p.id.clone()
        } else {
            p.short_id().to_string()
        }
    };
    let heading = |p: &Pile| piles::heading(&p.name, &p.path, &id(p));

    putln!(out, "{}  {}/{}", heading(pile), pile.kind, pile.version);
    let mut result = Map::new();
    result.insert("name".into(), name_of(&pile.name));
    result.insert("path".into(), json!(piles::shown(&pile.path)));
    result.insert("id".into(), json!(pile.id));
    result.insert("kind".into(), json!(pile.kind));
    result.insert("version".into(), json!(pile.version));
    if let Some((name, rev)) = &at {
        say(
            out,
            "at",
            &format!("{name} {}", &rev.as_str()[..rev.as_str().len().min(8)]),
        );
    }
    result.insert(
        "at".into(),
        at.as_ref().map_or(
            Value::Null,
            |(name, rev)| json!({"name": name, "revision": rev.as_str()}),
        ),
    );

    let mount = s.mount(&pile.path);
    if let Some((within, at)) = &mount {
        say(out, "mounted", &format!("{at} in {}", within.handle()));
    }
    result.insert(
        "mount".into(),
        mount.as_ref().map_or(
            Value::Null,
            |(within, at)| json!({"in": within.handle(), "path": at}),
        ),
    );
    let parent = s.parent(&pile.path);
    if let Some(p) = parent {
        say(out, "parent", &heading(p));
    }
    result.insert("parent".into(), parent.map_or(Value::Null, pile_value));
    let children = s.children(&pile.path);
    if children.is_empty() {
        say(out, "children", "none");
    } else {
        // Each child by its name, its path relative to this pile, and its
        // id, in columns.
        let at = |c: &Pile| s.mount(&c.path).map(|(_, at)| at).unwrap_or_default();
        let names = children
            .iter()
            .filter(|c| !c.name.is_empty())
            .map(|c| c.name.len() + 1)
            .max()
            .unwrap_or(0);
        let paths = children.iter().map(|c| at(c).len()).max().unwrap_or(0);
        let lines: Vec<String> = children
            .iter()
            .map(|c| {
                let name = if c.name.is_empty() {
                    String::new()
                } else {
                    piles::typed(&c.name)
                };
                if names == 0 {
                    format!("{:<paths$}  {}", at(c), id(c))
                } else {
                    format!("{name:<names$}  {:<paths$}  {}", at(c), id(c))
                }
            })
            .collect();
        say_all(out, "children", &lines);
    }
    result.insert(
        "children".into(),
        children
            .iter()
            .map(|c| {
                let mut v = pile_value(c);
                v["mount"] = json!(s.mount(&c.path).map(|(_, at)| at));
                v
            })
            .collect(),
    );

    let mut written = Value::Null;
    if pile.intent.is_some() && at.is_none() {
        let (said, value) = written_line(&r, &s, pile)?;
        say(out, "written", &said);
        written = value;
    }
    result.insert("written".into(), written);

    let rows = graph::read(r.storage(), &state)?;
    let me = Node::Pile(pile.id.clone());
    let mut values = Map::new();
    for (property, said) in pile_values(&rows, &me) {
        say(out, &property, &said.join(", "));
        values.insert(property, json!(said));
    }
    result.insert("values".into(), Value::Object(values));
    let types = shown_types(&rows, &me);
    if !types.is_empty() {
        let names = types.iter().map(|t| t.name.len()).max().unwrap_or(0);
        let mut lines: Vec<String> = Vec::new();
        for t in &types {
            let ends = if t.edge {
                format!("  {} -> {}", t.source.join(","), t.target.join(","))
            } else {
                String::new()
            };
            let what = if t.edge { "edge" } else { "node" };
            lines.push(
                format!(
                    "{what} {:<names$}{ends}  {}",
                    t.name,
                    t.title.as_deref().unwrap_or_default()
                )
                .trim_end()
                .to_string(),
            );
            for (key, said) in [("do", &t.do_), ("dont", &t.dont)] {
                if let Some(said) = said {
                    lines.push(format!("     {key:<4}  {said}"));
                }
            }
        }
        say_all(out, "types", &lines);
    }
    result.insert(
        "types".into(),
        types
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "edge": t.edge,
                    "source": t.source,
                    "target": t.target,
                    "title": t.title,
                    "description": t.description,
                    "do": t.do_,
                    "dont": t.dont,
                    "at": t.at,
                })
            })
            .collect(),
    );
    let nodes = at_home(&rows, &me);
    if !nodes.is_empty() {
        let lines: Vec<String> = nodes
            .iter()
            .map(|n| match &n.title {
                Some(t) => format!("{}  {t}", n.shown),
                None => n.shown.clone(),
            })
            .collect();
        say_all(out, "nodes", &lines);
    }
    result.insert(
        "nodes".into(),
        nodes.iter().map(|n| n.value.clone()).collect(),
    );

    let linked = links::of(&rows, &s, pile);
    if !linked.is_empty() {
        let types = linked.iter().map(|l| l.of_type.len()).max().unwrap_or(0);
        let others = linked.iter().map(|l| l.other.len()).max().unwrap_or(0);
        let me = pile.handle();
        let lines: Vec<String> = linked
            .iter()
            .map(|l| {
                let mut line = format!(
                    "{} {:<types$}  {:<others$}  {}",
                    l.arrow(),
                    l.of_type,
                    l.other,
                    l.title.as_deref().unwrap_or_default()
                )
                .trim_end()
                .to_string();
                if let Some(w) = l.written_in.as_ref().filter(|w| **w != me) {
                    line.push_str(&format!("  (written in {w})"));
                }
                line
            })
            .collect();
        say_all(out, "links", &lines);
    }
    result.insert("links".into(), linked.iter().map(|l| l.whole()).collect());
    let mut named = vec![pile.handle()];
    for l in &linked {
        let Some(other) = l.other_pile.as_ref().filter(|o| !named.contains(o)) else {
            continue;
        };
        named.push(other.clone());
        let why = if l.direction == links::Direction::To {
            format!("the pile this one links to: {}", l.of_type)
        } else {
            format!("the pile that links to this one: {}", l.of_type)
        };
        let also = at
            .as_ref()
            .map(|(name, _)| format!(" --at {name}"))
            .unwrap_or_default();
        out.next(format!("pile show {other}{also}"), why);
    }
    // Each aux pile that creates a type the links are of, once, with the
    // types it creates for them.
    let mut creators: Vec<(&Pile, Vec<&str>)> = Vec::new();
    for l in &linked {
        let Some(Node::Pile(home)) = rows.type_of(&l.of_type).map(|t| &t.home) else {
            continue;
        };
        let Some(aux) = s.ordered().into_iter().find(|p| &p.id == home) else {
            continue;
        };
        if aux.id == pile.id {
            continue;
        }
        match creators.iter_mut().find(|(p, _)| p.id == aux.id) {
            Some((_, kinds)) if kinds.contains(&l.of_type.as_str()) => {}
            Some((_, kinds)) => kinds.push(&l.of_type),
            None => creators.push((aux, vec![&l.of_type])),
        }
    }
    for (aux, kinds) in creators {
        let also = at
            .as_ref()
            .map(|(name, _)| format!(" --at {name}"))
            .unwrap_or_default();
        out.next(
            format!("pile show {}{also}", aux.handle()),
            format!(
                "the aux pile that creates {}: what a link of it must say, and must not",
                kinds.join(", ")
            ),
        );
    }

    let unread: Vec<String> = s
        .unread_in(&pile.path)
        .iter()
        .map(|f| piles::shown(f))
        .collect();
    if !unread.is_empty() {
        say_all(out, "unread", &unread);
    }
    result.insert("unread".into(), json!(unread));

    let name = piles::shown(&pile.intent_path());
    putln!(out);
    match &pile.intent {
        Some(text) => {
            put!(out, "{name}\n\n{text}");
            if !text.ends_with('\n') {
                putln!(out);
            }
        }
        None => putln!(
            out,
            "{name} is not there: nothing says why this pile exists"
        ),
    }
    result.insert("files".into(), json!([{"name": name, "text": pile.intent}]));
    out.result(Value::Object(result));
    Ok(())
}

/// What a pile gives in `[pile]`: its title, its description and each key
/// of its own, each with its values as text prints them, by key.
fn pile_values(rows: &Relations, pile: &Node) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for p in rows.prop.iter().filter(|p| &p.node == pile) {
        if [graph::ID, graph::NAME, graph::PATH, graph::HEADLINE].contains(&p.property.as_str()) {
            continue;
        }
        match out.iter_mut().find(|(name, _)| *name == p.property) {
            Some((_, said)) => said.push(p.value.to_string()),
            None => out.push((p.property.clone(), vec![p.value.to_string()])),
        }
    }
    out
}

/// A type an aux pile creates, as show prints it.
struct ShownType {
    name: String,
    edge: bool,
    source: Vec<String>,
    target: Vec<String>,
    title: Option<String>,
    description: Option<String>,
    do_: Option<String>,
    dont: Option<String>,
    at: Option<String>,
}

/// The types a pile creates, read off the rows of `type`, `type_prop` and
/// `allows`.
fn shown_types(rows: &Relations, pile: &Node) -> Vec<ShownType> {
    rows.types_of(pile)
        .into_iter()
        .map(|t| {
            let value = |p: &str| rows.type_value(&t.name, p).map(|v| v.to_string());
            let ends = |end| {
                rows.allowed(&t.name, end)
                    .into_iter()
                    .map(String::from)
                    .collect()
            };
            ShownType {
                name: t.name.clone(),
                edge: t.sort == graph::Sort::Edge,
                source: ends(graph::End::Source),
                target: ends(graph::End::Target),
                title: value("title"),
                description: value("description"),
                do_: value("do"),
                dont: value("dont"),
                at: rows
                    .type_at(&t.name)
                    .map(|a| format!("{}:{}:{}", a.file, a.at.line, a.at.column)),
            }
        })
        .collect()
}

/// A node at home in a pile, as show prints it and as its result holds it.
struct AtHome {
    shown: String,
    title: Option<String>,
    value: Value,
}

fn at_home(rows: &Relations, pile: &Node) -> Vec<AtHome> {
    rows.at_home(pile)
        .into_iter()
        .filter_map(|n| {
            let Node::Named { of_type, name } = &n.node else {
                return None;
            };
            let shown = format!("{of_type}:{name}");
            let title = rows.value(&n.node, "title").map(|v| v.to_string());
            let at = rows
                .at(&n.node)
                .map(|a| format!("{}:{}:{}", a.file, a.at.line, a.at.column));
            Some(AtHome {
                value: json!({
                    "reference": shown,
                    "type": of_type,
                    "name": name,
                    "title": title,
                    "at": at,
                }),
                shown,
                title,
            })
        })
        .collect()
}

/// The written line, and the same as data: the commit that last changed the
/// intent, at the path it has now, and how many commits have changed the
/// pile's own files since.
fn written_line(r: &Repository, s: &Structure, pile: &Pile) -> Result<(String, Value)> {
    let storage = r.storage();
    let last = storage.history(&Subtree::new(pile.intent_path()), None, Some(1))?;
    let Some(t) = last.revisions.first() else {
        return Ok((
            "not committed yet".to_string(),
            json!({"commit": null, "date": null, "since": 0}),
        ));
    };
    let info = &t.info;
    let since = storage
        .history(&s.own(&pile.path), Some(&info.revision), Some(0))?
        .total;
    let tail = if since > 0 {
        format!("{} to this pile since", plural(since, "commit"))
    } else {
        "nothing to this pile since".to_string()
    };
    Ok((
        format!("{} {}, {tail}", info.short, info.day()),
        json!({"commit": info.revision.as_str(), "date": info.day(), "since": since}),
    ))
}
