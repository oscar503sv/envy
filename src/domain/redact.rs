//! Wrapper that keeps user data out of Debug output.

use std::fmt;

/// Wraps a value so its Debug output reveals no information about it.
///
/// `Redacted` does not implement `Display` on purpose: formatting it by
/// mistake (for example when writing a file or the clipboard) must fail to
/// compile instead of silently writing a placeholder. It only protects
/// formatting; it does not zero memory or compare in constant time.
///
/// ```
/// use envy::domain::redact::Redacted;
///
/// assert_eq!(format!("{:?}", Redacted::new("x")), "Redacted(<redacted>)");
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Redacted(<redacted>)")
    }
}

impl<T: Default> Default for Redacted<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {
    // `assert!(a == b)` instead of `assert_eq!`: a failing `assert_eq!` would
    // print the sentinel into the test output, which is itself a leak.
    use super::Redacted;
    use crate::test_support::SENTINEL;

    #[test]
    fn formatting_hides_strings_and_bytes() {
        let string = Redacted::new(String::from(SENTINEL));
        let bytes = Redacted::new(SENTINEL.as_bytes().to_vec());

        // Formatting flags must not change the output either.
        for output in [
            format!("{string:?}"),
            format!("{string:#?}"),
            format!("{string:>40?}"),
            format!("{bytes:?}"),
            format!("{bytes:x?}"),
        ] {
            assert!(!output.contains(SENTINEL));
            assert!(output == "Redacted(<redacted>)");
        }
    }

    #[test]
    fn nested_pretty_debug_hides_the_value() {
        #[derive(Debug)]
        struct Entry {
            value: Redacted<String>,
        }

        let entry = Entry {
            value: Redacted::new(String::from(SENTINEL)),
        };
        assert!(entry.value.expose() == SENTINEL);
        let output = format!("{entry:#?}");
        assert!(!output.contains(SENTINEL));
        assert!(output.contains("Redacted(<redacted>)"));
    }

    #[test]
    fn access_and_comparison_preserve_the_value() {
        let value = Redacted::new(String::from(SENTINEL));
        assert!(value.clone() == value);
        assert!(value != Redacted::default());
        assert!(value.expose() == SENTINEL);
        assert!(value.into_inner() == SENTINEL);
    }
}
