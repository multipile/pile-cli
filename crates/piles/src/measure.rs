//! How long a file is: in lines and bytes, so a reader can say what loading
//! it will cost before loading it.

use storage::FileKind;

/// How far into a file a NUL makes it binary. It is how far git looks, so
/// the two agree about which files have lines.
pub const BINARY_PROBE: usize = 8000;

/// What is known of a file's length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// A text file: its lines, and its bytes.
    Text { lines: usize, bytes: u64 },
    /// A file with a NUL in its first [`BINARY_PROBE`] bytes. Its newlines
    /// are not lines anybody wrote, so only its bytes count.
    Binary { bytes: u64 },
    /// A symbolic link: as long as the path it points to.
    Link { bytes: u64 },
    /// Another repository inside this one. It is not measured.
    Submodule,
    /// A file that is there and could not be read.
    Unreadable,
}

impl Length {
    /// The lines, where they were counted.
    pub fn lines(&self) -> Option<usize> {
        match self {
            Length::Text { lines, .. } => Some(*lines),
            _ => None,
        }
    }

    /// The bytes, where they are known.
    pub fn bytes(&self) -> Option<u64> {
        match self {
            Length::Text { bytes, .. } | Length::Binary { bytes } | Length::Link { bytes } => {
                Some(*bytes)
            }
            Length::Submodule | Length::Unreadable => None,
        }
    }
}

/// A file's length, from its kind and what it holds. Lines are what `wc -l`
/// counts, and a last line with no newline counts too.
pub fn measure(kind: FileKind, content: &[u8]) -> Length {
    let bytes = content.len() as u64;
    match kind {
        FileKind::Submodule => Length::Submodule,
        FileKind::Symlink => Length::Link { bytes },
        _ if content[..content.len().min(BINARY_PROBE)].contains(&0) => Length::Binary { bytes },
        _ => {
            let newlines = content.iter().filter(|&&b| b == b'\n').count();
            let unended = !content.is_empty() && !content.ends_with(b"\n");
            Length::Text {
                lines: newlines + usize::from(unended),
                bytes,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_count_a_last_line_without_its_newline() {
        assert_eq!(
            measure(FileKind::File, b""),
            Length::Text { lines: 0, bytes: 0 }
        );
        assert_eq!(
            measure(FileKind::File, b"a\nb\n"),
            Length::Text { lines: 2, bytes: 4 }
        );
        assert_eq!(
            measure(FileKind::Executable, b"a\nb"),
            Length::Text { lines: 2, bytes: 3 }
        );
    }

    #[test]
    fn a_nul_early_on_makes_a_file_binary_and_one_later_does_not() {
        assert_eq!(
            measure(FileKind::File, b"\x89PNG\0\n\n"),
            Length::Binary { bytes: 7 }
        );
        let mut late = vec![b'a'; BINARY_PROBE];
        late.extend_from_slice(b"\0\n");
        assert_eq!(measure(FileKind::File, &late).lines(), Some(1));
    }

    #[test]
    fn a_link_is_as_long_as_where_it_points_and_a_submodule_is_not_measured() {
        assert_eq!(
            measure(FileKind::Symlink, b"../x"),
            Length::Link { bytes: 4 }
        );
        assert_eq!(measure(FileKind::Submodule, b""), Length::Submodule);
        assert_eq!(Length::Submodule.bytes(), None);
    }
}
