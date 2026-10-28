//! F36: parsing isolado. The decoders (`image`, `kamadak-exif`, `imagesize`) read bytes
//! from files nobody vetted; a malformed one may make them panic. Each parse runs inside
//! `contained`, so a panic costs that one file, never the app or the indexing run.
//! Needs `panic = "unwind"` in the release profile (Cargo.toml): with `abort` nothing
//! can be caught.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// `f()`, or `None` when it panicked. The parsers share no state with the caller, so a
/// half-finished parse leaves nothing inconsistent behind.
pub fn contained<T>(f: impl FnOnce() -> T) -> Option<T> {
    catch_unwind(AssertUnwindSafe(f)).ok()
}

#[cfg(test)]
mod tests {
    use super::contained;

    #[test]
    fn sec01_a_panicking_parser_costs_only_its_file() {
        assert_eq!(contained(|| 7), Some(7));
        let parsed: Option<u32> = contained(|| panic!("arquivo malformado"));
        assert_eq!(parsed, None);
        // The thread goes on and can parse the next file.
        assert_eq!(contained(|| "próximo"), Some("próximo"));
    }
}
