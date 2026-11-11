//! The only writes `pile` makes: the `.pile/` files `init` and `set` write,
//! and the drawing `diagram render` writes where it is told, on disk.
//! Nothing here goes through the storage interface, which only reads.
//!
//! A new file is created only where there is none, so a config that is
//! already there is never overwritten. A file that is rewritten goes to a
//! temporary file beside it first, then is renamed over it, so an
//! interrupted write leaves the file it found.

use std::io::{ErrorKind, Write as _};
use std::path::Path;

use crate::exit::{Code, Refusal};

/// What creating a file came to.
pub enum Created {
    Written,
    /// There was a file there already, and it is left as it was.
    There,
}

/// Creates a file holding content, and the directories above it, unless a
/// file is there already.
pub fn create(path: &Path, content: &str) -> Result<Created, Refusal> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| failed(dir, &e))?;
    }
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => return Ok(Created::There),
        Err(e) => return Err(failed(path, &e)),
    };
    file.write_all(content.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| failed(path, &e))?;
    Ok(Created::Written)
}

/// Replaces a file's content, through a temporary file beside it and a
/// rename.
pub fn replace(path: &Path, content: impl AsRef<[u8]>) -> Result<(), Refusal> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    let temp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut f| {
            f.write_all(content.as_ref())?;
            f.sync_all()
        })
        .and_then(|()| std::fs::rename(&temp, path));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(failed(path, &e));
    }
    Ok(())
}

fn failed(path: &Path, e: &std::io::Error) -> Refusal {
    Refusal::new(
        Code::WriteFailed,
        format!(
            "could not write {}: {}",
            path.display(),
            crate::cli::os_says(e)
        ),
    )
}
