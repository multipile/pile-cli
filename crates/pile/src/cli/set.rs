use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::json;

use piles::config::{self, KEY_ID, KEY_KIND, KEY_NAME, KEY_VERSION};
use piles::{Pile, Structure};
use storage::State;

use super::{Out, Result, name_of, open, say, switch, target};
use crate::exit::{Code, Refusal};
use crate::write::replace;

pub(super) fn command() -> Command {
    Command::new("set")
        .about("Set the name in a pile's config")
        .long_about(
            "Set a key in a pile's config. The one key set writes is name: the word\n\
             the pile is typed and shown by, as @lib or lib. Setting it replaces what\n\
             the line said, and --forget takes it away, so the pile is typed and shown\n\
             by its path.\n\n\
             The other keys are not set:\n  \
             - kind and version are what this binary writes\n  \
             - id is minted once, and is what says a pile is the same pile after a\n    \
             move\n\n\
             A name is lowercase letters, digits and hyphens, and begins with a\n\
             letter. So it is never a number, never reads as a path, and has no\n\
             colon. No other pile may claim it already: a name two piles claim\n\
             names neither.\n\n\
             set rewrites the one line, and leaves the rest of the file exactly as it\n\
             was, comments and order included. It writes nothing, and says so, when\n\
             the line already says what was asked. Commit the changed config: the\n\
             name reaches a clone in the same file the id does.\n\n\
             The pile is the one named, or the one you are in.",
        )
        .override_usage("pile set name <name> [pile]\n       pile set --forget name [pile]")
        .after_help(format!(
            "Examples:\n  \
             pile set name lib :/packages/lib   name a pile\n  \
             pile set name lib                the pile you are in\n  \
             pile set --forget name @lib      take the name away\n\n\
             Refusals:\n  \
             unknown-key              2  the one key set writes is name\n  \
             not-settable             2  kind, version and id are not set\n  \
             invalid-value            2  the message says what a name may be\n  \
             name-taken               1  another pile claims the name: choose another\n  \
             write-failed             1  the message says what the system said\n{}",
            target::REFUSALS
        ))
        .arg(
            Arg::new("args")
                .num_args(0..)
                .action(ArgAction::Append)
                .hide(true),
        )
        .arg(switch("forget", "take the name away instead"))
}

/// Refuses a key set and `init --set` do not write.
pub(super) fn check_key(key: &str) -> Result {
    if key == KEY_NAME {
        return Ok(());
    }
    if [KEY_KIND, KEY_VERSION, KEY_ID].contains(&key) {
        return Err(Refusal::new(
            Code::NotSettable,
            format!(
                "{key} is written by pile init and never changed. The one key set writes is {KEY_NAME}"
            ),
        ));
    }
    Err(Refusal::new(
        Code::UnknownKey,
        format!("{key:?} is not a key set writes. The one key set writes is {KEY_NAME}"),
    ))
}

/// Refuses a name a pile other than `me` claims already.
pub(super) fn name_free(s: &Structure, name: &str, me: Option<&str>) -> Result {
    if let Some(taken) = s
        .by_name(name)
        .into_iter()
        .find(|p| Some(p.path.as_str()) != me)
    {
        return Err(Refusal::new(
            Code::NameTaken,
            format!(
                "{} is claimed by {} already, and a name names one pile",
                piles::typed(name),
                piles::shown(&taken.path)
            ),
        ));
    }
    Ok(())
}

/// The result is the pile as it is now, `name` (or `null`), `path` and
/// `id`; `key`, which is `name`; `value`, what it says now, or `null` after
/// `--forget`; `was`, what it said before, or `null`; `written`, `false`
/// when the line already said it; and `to_commit`, the config's path as git
/// spells it, with `why`, or empty when nothing was written.
pub(super) fn run(m: &ArgMatches, out: &mut Out) -> Result {
    let forget = m.get_flag("forget");
    let args: Vec<String> = m
        .get_many::<String>("args")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let usage = if forget {
        "name [pile]"
    } else {
        "name <name> [pile]"
    };
    let Some(key) = args.first() else {
        return Err(Refusal::new(
            Code::Usage,
            format!("pile set takes {usage}: say which key, and what it is to say"),
        ));
    };
    check_key(key)?;
    let (value, rest) = if forget {
        (String::new(), &args[1..])
    } else {
        let Some(value) = args.get(1) else {
            return Err(Refusal::new(
                Code::Usage,
                format!("pile set takes {usage}: say what the name is to be"),
            ));
        };
        config::check_name(value).map_err(|e| Refusal::new(Code::InvalidValue, e))?;
        (value.clone(), &args[2..])
    };
    if rest.len() > 1 {
        return Err(Refusal::new(
            Code::Usage,
            format!(
                "pile set takes {usage}, and was given {} arguments",
                args.len()
            ),
        ));
    }

    let r = open()?;
    let s = target::declared(&r)?;
    let pile = target::chosen(&r, &s, rest.first().map(String::as_str))?;
    if !forget {
        name_free(&s, &value, Some(&pile.path))?;
    }
    let file = pile.config_path();
    let content = r
        .storage()
        .read(&State::Working, &[file.as_str()])?
        .pop()
        .flatten()
        .ok_or_else(|| Refusal::new(Code::Failed, format!("{file} is not there to rewrite")))?;
    let content = String::from_utf8_lossy(&content);
    let git_file = file.trim_start_matches('/').to_string();
    let config = piles::shown(&file);
    let changed = match config::set_line(&content, KEY_NAME, &value, &[KEY_ID]) {
        Some(new) => {
            replace(&r.top()?.join(&git_file), &new)?;
            true
        }
        None => false,
    };
    report(out, pile, &value, forget, changed, &config);
    out.result(json!({
        "name": name_of(&value),
        "path": piles::shown(&pile.path),
        "id": pile.id,
        "key": KEY_NAME,
        "value": name_of(&value),
        "was": name_of(&pile.name),
        "written": changed,
        "to_commit": if changed {
            json!([{"path": config, "why": "the name reaches a clone in the same file the id does"}])
        } else {
            json!([])
        },
    }));
    Ok(())
}

fn report(out: &mut Out, pile: &Pile, value: &str, forget: bool, changed: bool, file: &str) {
    if !changed {
        if forget {
            putln!(
                out,
                "{} has no name, and is typed and shown by its path",
                piles::shown(&pile.path)
            );
        } else {
            putln!(
                out,
                "{} is {} already, and nothing was written",
                piles::shown(&pile.path),
                piles::typed(value)
            );
        }
        return;
    }
    putln!(
        out,
        "{}",
        piles::heading(value, &pile.path, pile.short_id())
    );
    if !pile.name.is_empty() {
        say(out, "was", &piles::typed(&pile.name));
    }
    if forget {
        putln!(
            out,
            "\nit is typed and shown by its path now, which is {}.",
            piles::shown(&pile.path)
        );
    }
    putln!(
        out,
        "\n{file} is changed, and should be committed: the name\n\
         reaches a clone in the same file the id does."
    );
}
