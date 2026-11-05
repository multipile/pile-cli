//! The repository a command reads: the storage that answers for it, and
//! where the command was run in it.
//!
//! A command that reads a repository calls [`Repository::open`] with the
//! directory it was run in, and asks every question of
//! [`Repository::storage`]. It never asks git itself. What the storage can
//! be asked is crates/storage/src/lib.rs.
//!
//! Each way a backend fails becomes a refusal here, with a code of its own,
//! and so does a config that cannot be read. So every command refuses one
//! failure in the same words.

use std::path::Path;

use piles::Structure;
use storage::{Error, State, Storage};

use crate::exit::{Code, Refusal};

/// A repository, opened for the directory a command was run in.
pub struct Repository {
    storage: Box<dyn Storage>,
    here: String,
}

impl Repository {
    /// The repository that encloses dir.
    pub fn open(dir: &Path) -> Result<Repository, Refusal> {
        let storage = storage::open(dir)?;
        let here = match storage.working_copy() {
            Some(top) => path_in(top, dir),
            None => "/".to_string(),
        };
        Ok(Repository { storage, here })
    }

    /// What every question about the repository is asked of.
    pub fn storage(&self) -> &dyn Storage {
        self.storage.as_ref()
    }

    /// Where the command was run, as a path from the top of the repository:
    /// `/` at the top, `/crates/storage` inside it.
    pub fn here(&self) -> &str {
        &self.here
    }

    /// Where the working copy is on disk: the directory `init` and `set`
    /// write `.pile/` files under.
    pub fn top(&self) -> Result<&Path, Refusal> {
        self.storage.working_copy().ok_or_else(|| {
            Refusal::new(
                Code::Failed,
                "this repository has no working copy on disk to write in",
            )
        })
    }

    /// Every pile the working copy declares.
    pub fn piles(&self) -> Result<Structure, Refusal> {
        Ok(piles::discover(self.storage(), &State::Working)?)
    }
}

/// Where dir is under top, as a path from the top. Both are resolved first,
/// so a directory reached through a symbolic link is still under the top it
/// leads to. A dir not under top is the top.
pub(crate) fn path_in(top: &Path, dir: &Path) -> String {
    let top = std::fs::canonicalize(top).unwrap_or_else(|_| top.to_path_buf());
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let Ok(rel) = dir.strip_prefix(&top) else {
        return "/".to_string();
    };
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    format!("/{}", parts.join("/"))
}

impl From<piles::Error> for Refusal {
    fn from(e: piles::Error) -> Refusal {
        match e {
            piles::Error::Storage(e) => e.into(),
            piles::Error::Config { .. } => Refusal::new(Code::UnreadableConfig, e.to_string()),
        }
    }
}

impl From<Error> for Refusal {
    fn from(e: Error) -> Refusal {
        match e {
            Error::NotARepository => Refusal::new(
                Code::NotARepository,
                "not inside a git repository, and pile reads only a git repository",
            )
            .next("git init", "makes this directory a git repository"),
            Error::NoGit => Refusal::new(
                Code::NoGit,
                "pile reads a repository with git, and there is no git on PATH",
            ),
            Error::UnknownRevision(name) => Refusal::new(
                Code::UnknownRevision,
                format!("{name:?} names no revision in this repository"),
            )
            .next("git log --oneline", "lists the revisions there are"),
            Error::Failed(why) => Refusal::new(Code::StorageFailed, why),
            // A new way to fail that has no code here yet still refuses, as
            // the code that asks to be given one.
            other => Refusal::new(Code::Failed, other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exit::{FAILURE, USAGE};

    #[test]
    fn each_way_a_backend_fails_has_its_own_code() {
        let cases = [
            (Error::NotARepository, "not-a-repository", FAILURE),
            (Error::NoGit, "no-git", FAILURE),
            (
                Error::UnknownRevision("nope".into()),
                "unknown-revision",
                USAGE,
            ),
            (
                Error::Failed("git log: bad".into()),
                "storage-failed",
                FAILURE,
            ),
        ];
        for (e, code, exit) in cases {
            let r = Refusal::from(e);
            assert_eq!((r.code.as_str(), r.exit()), (code, exit));
        }
    }

    #[test]
    fn outside_a_repository_it_refuses_and_says_how_to_make_one() {
        // Opened alone, so a git hook's GIT_DIR, set while this test runs
        // inside one, does not point it at the hook's repository.
        let dir = tempfile::tempdir().unwrap();
        let Err(e) = storage::git::Git::open_alone(dir.path()) else {
            panic!("a temporary directory is in no repository");
        };
        let r = Refusal::from(e);
        assert_eq!(r.code, Code::NotARepository);
        assert_eq!(
            r.text(),
            "pile: not inside a git repository, and pile reads only a git repository\n  \
             git init  makes this directory a git repository\n"
        );
    }

    #[test]
    fn here_is_where_the_command_was_run_from_the_top() {
        let top = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(top.path().join("a/b")).unwrap();
        assert_eq!(path_in(top.path(), top.path()), "/");
        assert_eq!(path_in(top.path(), &top.path().join("a/b")), "/a/b");
        let elsewhere = tempfile::tempdir().unwrap();
        assert_eq!(path_in(top.path(), elsewhere.path()), "/");
    }
}
