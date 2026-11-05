//! What this binary is, stamped at build time.
//!
//! scripts/build and scripts/cross set PILE_VERSION, PILE_COMMIT and
//! PILE_DATE in the environment of the compile, and `option_env!` reads them
//! here. build.rs tells cargo to rebuild when one of them changes, so a build
//! cannot leave a stale stamp behind.
//!
//! A binary nobody stamped, such as one from a plain `cargo build`, has the
//! version "dev" and says "unknown" for the commit and the date. It does not
//! guess which commit it was built from.

/// The version a release names, and "dev" for any other build.
pub const VERSION: &str = match option_env!("PILE_VERSION") {
    Some(v) if !v.is_empty() => v,
    _ => "dev",
};

/// The short id of the commit the binary was built from, when the build said.
pub const COMMIT: Option<&str> = nonempty(option_env!("PILE_COMMIT"));

/// When the binary was built, when the build said.
pub const DATE: Option<&str> = nonempty(option_env!("PILE_DATE"));

/// The compiler that built it, as `rustc --version` prints it. build.rs
/// writes it.
pub const RUSTC: &str = env!("PILE_RUSTC");

/// The target triple it was built for.
pub const TARGET: &str = env!("PILE_TARGET");

/// The commit, or "unknown" for a binary nobody stamped.
pub fn commit() -> &'static str {
    COMMIT.unwrap_or("unknown")
}

/// The date, or "unknown" for a binary nobody stamped.
pub fn date() -> &'static str {
    DATE.unwrap_or("unknown")
}

/// The compiler's version and the target, without the word "rustc".
pub fn rustc() -> String {
    let version = RUSTC.strip_prefix("rustc ").unwrap_or(RUSTC);
    format!("{version} {TARGET}")
}

const fn nonempty(v: Option<&'static str>) -> Option<&'static str> {
    match v {
        Some(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unstamped_binary_says_unknown_rather_than_guessing() {
        assert!(!VERSION.is_empty());
        assert!(!commit().is_empty());
        assert!(!date().is_empty());
        assert!(RUSTC.starts_with("rustc "), "{RUSTC}");
        assert!(rustc().ends_with(TARGET));
    }
}
