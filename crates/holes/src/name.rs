//! Tenant (pigeon) names.
//!
//! In a slot fleet a tenant id never becomes a file name — it is hashed to one
//! of 16,384 slots and the slot names the file — so `autumn-web` accepts any
//! string as a routing key there. That makes validating the id entirely the
//! app's job. [`TenantName`] keeps the tenant-mode rules anyway, so the same
//! names would work on a tenant fleet unchanged.

use std::fmt;

/// The longest pigeon name.
pub const MAX_LEN: usize = 40;

/// Names Windows reserves for devices. A file called `con.db` cannot be
/// created there, and the fleet refuses them on every platform so that a
/// fleet directory can be copied between machines.
const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A validated tenant (pigeon) name: `[a-z][a-z0-9-]*`, at most
/// [`MAX_LEN`] bytes, no `--`, no trailing `-`, not a reserved device name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TenantName(String);

/// Why a string is not a pigeon name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TenantNameError {
    /// The name is empty.
    #[error("a pigeon needs a name")]
    Empty,
    /// The name is longer than [`MAX_LEN`].
    #[error("a pigeon name is at most {MAX_LEN} characters")]
    TooLong,
    /// The name does not start with a lower-case letter.
    #[error("a pigeon name starts with a lower-case letter")]
    BadStart,
    /// A character outside `a-z`, `0-9` and `-`.
    #[error("a pigeon name uses only a-z, 0-9 and '-' (found {0:?})")]
    BadChar(char),
    /// `--`, or a `-` at the end.
    #[error("a pigeon name has no '--' and does not end in '-'")]
    BadHyphen,
    /// A Windows device name such as `con`.
    #[error("{0:?} is reserved by Windows and cannot name a file")]
    Reserved(String),
}

impl TenantName {
    /// Check `raw` and wrap it.
    ///
    /// # Errors
    ///
    /// The first rule `raw` breaks, in the order the variants of
    /// [`TenantNameError`] are listed.
    pub fn parse(raw: &str) -> Result<Self, TenantNameError> {
        if raw.is_empty() {
            return Err(TenantNameError::Empty);
        }
        if raw.len() > MAX_LEN {
            return Err(TenantNameError::TooLong);
        }
        let mut chars = raw.chars();
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() => {}
            _ => return Err(TenantNameError::BadStart),
        }
        if let Some(bad) = raw
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-'))
        {
            return Err(TenantNameError::BadChar(bad));
        }
        if raw.contains("--") || raw.ends_with('-') {
            return Err(TenantNameError::BadHyphen);
        }
        if RESERVED.contains(&raw) {
            return Err(TenantNameError::Reserved(raw.to_owned()));
        }
        Ok(Self(raw.to_owned()))
    }

    /// The name, as the fleet's routing key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The `n`th pigeon of a flock: a pronounceable, deterministic name.
    #[must_use]
    pub fn flock(n: u32) -> Self {
        const ADJ: [&str; 16] = [
            "brass", "dusk", "slate", "copper", "ash", "moss", "tawny", "pale", "storm", "rust",
            "ink", "sable", "chalk", "amber", "frost", "umber",
        ];
        const NOUN: [&str; 16] = [
            "rock",
            "homer",
            "tumbler",
            "fantail",
            "pouter",
            "barb",
            "jacobin",
            "runt",
            "owl",
            "turbit",
            "trumpeter",
            "roller",
            "carrier",
            "dragoon",
            "helmet",
            "nun",
        ];
        let a = ADJ[(n % 16) as usize];
        let b = NOUN[((n / 16) % 16) as usize];
        Self(format!("{a}-{b}-{}", n / 256 + 1))
    }
}

impl fmt::Display for TenantName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for TenantName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn ordinary_names_parse() {
        for ok in ["acme", "globex", "initech-2", "a", "storm-0001", "z9"] {
            assert_eq!(
                TenantName::parse(ok).map(|n| n.0),
                Ok(ok.to_owned()),
                "{ok}"
            );
        }
    }

    #[test]
    fn each_rule_has_its_own_refusal() {
        assert_eq!(TenantName::parse(""), Err(TenantNameError::Empty));
        assert_eq!(
            TenantName::parse(&"a".repeat(MAX_LEN + 1)),
            Err(TenantNameError::TooLong)
        );
        assert_eq!(TenantName::parse("Acme"), Err(TenantNameError::BadStart));
        assert_eq!(TenantName::parse("9lives"), Err(TenantNameError::BadStart));
        assert_eq!(TenantName::parse("-a"), Err(TenantNameError::BadStart));
        assert_eq!(TenantName::parse(".."), Err(TenantNameError::BadStart));
        assert_eq!(TenantName::parse("a.b"), Err(TenantNameError::BadChar('.')));
        assert_eq!(TenantName::parse("a/b"), Err(TenantNameError::BadChar('/')));
        assert_eq!(TenantName::parse("a_b"), Err(TenantNameError::BadChar('_')));
        assert_eq!(TenantName::parse("a--b"), Err(TenantNameError::BadHyphen));
        assert_eq!(TenantName::parse("ab-"), Err(TenantNameError::BadHyphen));
        assert_eq!(
            TenantName::parse("con"),
            Err(TenantNameError::Reserved("con".into()))
        );
        assert_eq!(
            TenantName::parse("lpt9"),
            Err(TenantNameError::Reserved("lpt9".into()))
        );
    }

    #[test]
    fn the_longest_name_is_allowed() {
        assert!(TenantName::parse(&"a".repeat(MAX_LEN)).is_ok());
    }

    #[test]
    fn flock_names_are_valid_and_distinct() {
        let names: std::collections::BTreeSet<String> = (0..20_000)
            .map(|n| TenantName::flock(n).to_string())
            .collect();
        assert_eq!(names.len(), 20_000);
        for n in [0, 1, 255, 256, 19_999] {
            let s = TenantName::flock(n);
            assert_eq!(TenantName::parse(s.as_str()), Ok(s));
        }
        assert_eq!(TenantName::flock(0).as_str(), "brass-rock-1");
    }

    proptest! {
        /// Whatever is accepted is exactly what was given, and re-parses.
        #[test]
        fn accepted_names_are_stable(raw in "[a-z][a-z0-9-]{0,45}") {
            if let Ok(n) = TenantName::parse(&raw) {
                prop_assert_eq!(n.as_str(), raw.as_str());
                prop_assert_eq!(TenantName::parse(n.as_str()), Ok(n));
            }
        }

        /// Nothing that could escape the fleet root is ever accepted.
        #[test]
        fn no_accepted_name_can_walk_a_path(raw in "\\PC{0,48}") {
            if let Ok(n) = TenantName::parse(&raw) {
                let s = n.as_str();
                prop_assert!(!s.contains('/') && !s.contains('\\') && !s.contains('.'));
                prop_assert!(!s.contains(char::is_whitespace));
                prop_assert!(s.len() <= MAX_LEN);
            }
        }
    }
}
