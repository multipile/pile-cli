use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;

use piles::config::{self, AUX, KEY_KIND, KIND};
use piles::{CONFIG_FILE, INTENT_FILE, PILE_DIR, intent};

use super::set::{check_key, name_free};
use super::{Out, Result, open};
use crate::exit::{Code, Refusal};
use crate::write::{Created, create};

pub(super) fn command() -> Command {
    Command::new("init")
        .about("Declare the directory you are in a pile")
        .long_about(
            "Declare the directory you are in a pile: mint an id, and write its\n\
             .pile/config and .pile/intent.md. The .pile/ directory is what makes a\n\
             subtree a pile.\n\n\
             init takes no path. It declares the directory it runs in and no other,\n\
             so cd to the directory first. A directory inside another pile is not\n\
             refused: the new pile is that pile's child. A directory that is a pile\n\
             already is refused.\n\n\
             The config is written whole. The intent is written as a question: why\n\
             this pile exists. Only a person can answer it, so init does not try, and\n\
             pile guide write-an-intent says how it is answered. An intent that is\n\
             already there is left as it is.\n\n\
             Declaring the top of the repository and declaring a directory inside it\n\
             are one act. A path is in the nearest pile at or above it, so the new\n\
             pile takes the paths under it from the pile it sits in. init moves no\n\
             file and changes no history: git sees a new directory, to be committed\n\
             like any other. Committing it is what carries the pile to a clone.\n\n\
             --set name=lib gives the new pile its name, as pile set would, and is\n\
             refused for the same reasons. Without it, init writes no name, and\n\
             offers pile set name with the directory's own name when that is a name\n\
             no pile has.\n\n\
             --set kind=auxiliary-gitpile declares an aux pile: the one kind of pile\n\
             that may create types (pile guide aux). Without it, the kind is gitpile,\n\
             an ordinary pile.",
        )
        .override_usage("pile init [--set name=<name>] [--set kind=auxiliary-gitpile]")
        .after_help(
            "Examples:\n  \
             pile init                            the directory you are in\n  \
             cd packages/lib && pile init         a directory inside it, the same act\n  \
             pile init --set name=lib             with the name it is typed by\n  \
             pile init --set name=words --set kind=auxiliary-gitpile\n                                       \
             an aux pile, which may create types\n\n\
             Refusals:\n  \
             usage                    2  init was given a path: cd to the directory instead\n  \
             malformed-set            2  --set key=value\n  \
             key-given-twice          2  one value for each key\n  \
             unknown-key              2  the keys init sets are name and kind\n  \
             not-settable             2  init writes version and id itself\n  \
             invalid-value            2  the message says what a name or a kind may be\n  \
             already-a-pile           1  pile set changes what its config says\n  \
             name-taken               1  another pile claims the name: choose another\n  \
             write-failed             1  the message says what the system said\n  \
             unreadable-config        1  a pile's config is wrong: the message names the line",
        )
        .arg(super::no_positional())
        .arg(
            Arg::new("set")
                .long("set")
                .value_name("key=value")
                .action(ArgAction::Append)
                .help("a key of the new config, written key=value: name=lib, kind=auxiliary-gitpile"),
        )
}

/// What the text says committing the new `.pile/` is for.
const WHY_COMMIT: &str =
    "it is what says this subtree is a pile, and git is what carries it to a clone";

/// The result is the new pile, `name` (or `null`), `path`, `id` and `kind`; `inside`,
/// the path of the pile it was in before, or `null`; and `to_commit`, each
/// path to commit, as git spells it, with `why`. `next` offers
/// `pile set name` when no name was given.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    if let Some(first) = m.get_many::<String>("args").and_then(|mut v| v.next()) {
        return Err(Refusal::new(
            Code::Usage,
            format!(
                "pile init declares the directory you are in, and takes no path, and was given \
                 {first:?}. cd to the directory first"
            ),
        ));
    }
    let mut name = String::new();
    let mut kind = KIND;
    let mut given: Vec<String> = Vec::new();
    for set in m.get_many::<String>("set").into_iter().flatten() {
        let Some((key, value)) = set.split_once('=') else {
            return Err(Refusal::new(
                Code::MalformedSet,
                format!("--set takes key=value, and was given {set:?}"),
            ));
        };
        let (key, value) = (key.trim(), value.trim());
        if key != KEY_KIND {
            check_key(key)?;
        }
        if given.iter().any(|k| k == key) {
            return Err(Refusal::new(
                Code::KeyGivenTwice,
                format!("--set gives {key} twice, and a pile has one {key}"),
            ));
        }
        given.push(key.to_string());
        if key == KEY_KIND {
            kind = [KIND, AUX]
                .into_iter()
                .find(|k| *k == value)
                .ok_or_else(|| {
                    Refusal::new(
                        Code::InvalidValue,
                        format!(
                            "a new pile's kind is {KIND}, or {AUX} for an aux pile, and was \
                             given {value:?}"
                        ),
                    )
                })?;
            continue;
        }
        config::check_name(value).map_err(|e| Refusal::new(Code::InvalidValue, e))?;
        name = value.to_string();
    }

    let r = open()?;
    let s = r.piles()?;
    let top = r.top()?;
    let path = r.here().to_string();
    let dir = top.join(path.trim_start_matches('/'));
    let at = piles::shown(&path);
    let already = || {
        Refusal::new(Code::AlreadyAPile, format!("{at} is a pile already"))
            .next(format!("pile show {at}"), "prints what its config says")
    };
    if s.find(&path).is_some() {
        return Err(already());
    }
    if !name.is_empty() {
        name_free(&s, &name, None)?;
    }
    let inside = s.owner(&path);

    let id = config::new_id().map_err(|e| {
        Refusal::new(
            Code::Failed,
            format!("could not read random bytes for an id: {e}"),
        )
    })?;
    let pile_dir = dir.join(PILE_DIR);
    let rendered = config::render_kind(kind, &id, &name);
    if let Created::There = create(&pile_dir.join(CONFIG_FILE), &rendered)? {
        return Err(already());
    }
    create(&pile_dir.join(INTENT_FILE), intent::template())?;

    putln!(out, "pile      {at}");
    if kind == AUX {
        putln!(out, "kind      {kind}, an aux pile, which may create types");
    }
    putln!(out, "id        {id}");
    if !name.is_empty() {
        putln!(out, "name      {}", piles::typed(&name));
    }
    if let Some(p) = inside {
        putln!(
            out,
            "inside    {}, whose paths under {at} are now this pile's",
            piles::shown(&p.path)
        );
    }
    let git_dir = piles::shown(&piles::join(&path, &[PILE_DIR]));
    putln!(
        out,
        "\n{git_dir} is new, and should be committed: it is what says this\n\
         subtree is a pile, and git is what carries it to a clone."
    );
    putln!(
        out,
        "\n{git_dir}/{INTENT_FILE} is a question so far. Answer it: say why this pile\n\
         exists. pile doctor reports the pile until you do.\n\
         pile guide write-an-intent says how to answer it."
    );
    if name.is_empty() {
        let word = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let fits = config::check_name(&word);
        let free = fits.is_ok() && s.by_name(&word).is_empty();
        let (line, why) = if free {
            (
                format!("pile set name {word} {at}"),
                "gives it a name to be typed and shown by",
            )
        } else {
            (
                format!("pile set name <name> {at}"),
                "gives it a name, which you choose",
            )
        };
        putln!(out, "\nit has no name yet:\n  {line}   {why}");
        out.next(line, why);
    }
    out.next(
        "pile guide write-an-intent",
        "says how to answer the question the intent asks",
    );
    out.result(json!({
        "name": (!name.is_empty()).then_some(&name),
        "path": at,
        "id": id,
        "kind": kind,
        "inside": inside.map(|p| piles::shown(&p.path)),
        "to_commit": [{"path": git_dir, "why": WHY_COMMIT}],
    }));
    Ok(())
}
