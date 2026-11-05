//! The revisions a base is chosen from, as git holds them now.
//!
//! Every answer is read from refs and config already in the repository.
//! Nothing is fetched, so a remote-tracking branch is as old as the last
//! `git fetch`.

use super::{FORMAT, Git, parse};
use crate::{Base, BaseRole, Bases, Error, Result, RevisionInfo, Storage as _};

pub(super) fn read(git: &Git, recent: usize) -> Result<Bases> {
    let Some(head) = git.head()? else {
        return Ok(Bases::default());
    };
    let branch = git
        .git(&["symbolic-ref", "--quiet", "--short", "HEAD"])
        .ok()
        .map(|out| String::from_utf8_lossy(&out).trim().to_string())
        .filter(|b| !b.is_empty());

    // First parent only: a merge brought in revisions from another branch,
    // and the revisions near the head are the ones its own branch made.
    let recent = if recent == 0 {
        Vec::new()
    } else {
        let n = format!("--max-count={recent}");
        let out = git.git(&[
            "log",
            "--first-parent",
            &n,
            "--abbrev=8",
            FORMAT,
            "--end-of-options",
            head.as_str(),
        ])?;
        infos(&out)?
    };

    let remote = branch
        .as_deref()
        .and_then(|b| config(git, &format!("branch.{b}.remote")))
        .filter(|r| r != ".")
        .unwrap_or_else(|| "origin".to_string());

    let mut named = Vec::new();
    let main = main(git, &remote)?;
    let upstream = match git.git(&[
        "rev-parse",
        "--abbrev-ref",
        "--symbolic-full-name",
        "@{upstream}",
    ]) {
        Ok(out) => base(
            git,
            String::from_utf8_lossy(&out).trim(),
            BaseRole::Upstream,
        )?,
        Err(_) => None,
    };
    let fork = match (&main, &branch) {
        (Some(main), Some(branch)) if !same_branch(&main.name, branch, &remote) => {
            fork_point(git, main)?
        }
        _ => None,
    };
    named.extend(main);
    named.extend(upstream);
    named.extend(fork);

    Ok(Bases {
        head: Some(info(git, head.as_str())?),
        branch,
        recent,
        named,
    })
}

/// The branch the repository's work goes into: the remote's HEAD where a
/// clone or `git remote set-head` recorded it, and otherwise main or master,
/// here or on the remote.
fn main(git: &Git, remote: &str) -> Result<Option<Base>> {
    let head = format!("refs/remotes/{remote}/HEAD");
    if let Ok(out) = git.git(&["symbolic-ref", "--quiet", "--short", &head])
        && let Some(b) = base(git, String::from_utf8_lossy(&out).trim(), BaseRole::Main)?
    {
        return Ok(Some(b));
    }
    for name in ["main", "master"] {
        for candidate in [name.to_string(), format!("{remote}/{name}")] {
            if let Some(b) = base(git, &candidate, BaseRole::Main)? {
                return Ok(Some(b));
            }
        }
    }
    Ok(None)
}

/// Whether the main branch, by a name such as `origin/main`, is the branch
/// the working copy is on.
fn same_branch(main: &str, branch: &str, remote: &str) -> bool {
    main.strip_prefix(&format!("{remote}/")).unwrap_or(main) == branch
}

/// Where the head's history left the main branch's: their merge base,
/// named after the main branch. None when the two share no revision.
fn fork_point(git: &Git, main: &Base) -> Result<Option<Base>> {
    let Ok(out) = git.git(&[
        "merge-base",
        "--end-of-options",
        main.info.revision.as_str(),
        "HEAD",
    ]) else {
        return Ok(None);
    };
    let id = String::from_utf8_lossy(&out).trim().to_string();
    Ok(Some(Base {
        name: main.name.clone(),
        role: BaseRole::ForkPoint,
        info: info(git, &id)?,
    }))
}

/// A ref by the name it is asked as, where it names a commit.
fn base(git: &Git, name: &str, role: BaseRole) -> Result<Option<Base>> {
    if name.is_empty() {
        return Ok(None);
    }
    let Some(revision) = git.commit(name)? else {
        return Ok(None);
    };
    Ok(Some(Base {
        name: name.to_string(),
        role,
        info: info(git, revision.as_str())?,
    }))
}

/// One revision, by its whole id.
fn info(git: &Git, id: &str) -> Result<RevisionInfo> {
    let out = git.git(&[
        "log",
        "--no-walk",
        "--abbrev=8",
        FORMAT,
        "--end-of-options",
        id,
    ])?;
    infos(&out)?
        .into_iter()
        .next()
        .ok_or_else(|| Error::Failed(format!("git log printed nothing for {id}")))
}

fn infos(out: &[u8]) -> Result<Vec<RevisionInfo>> {
    Ok(parse::log(out)?.into_iter().map(|t| t.info).collect())
}

/// One value of the repository's config, as git reads it from every file it
/// reads. Read, never written.
fn config(git: &Git, key: &str) -> Option<String> {
    let out = git.git(&["config", "--get", key]).ok()?;
    let value = String::from_utf8_lossy(&out).trim().to_string();
    (!value.is_empty()).then_some(value)
}
