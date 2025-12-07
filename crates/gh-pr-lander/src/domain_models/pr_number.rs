//! PR Number model
//!
//! Type-safe wrapper for GitHub PR numbers.

use super::Pr;

/// Newtype wrapper for GitHub PR numbers, providing type safety.
/// Can only be constructed from a Pr to prevent confusion with array indices.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrNumber(usize);

#[allow(dead_code)]
impl PrNumber {
    /// Create a PrNumber from a PR reference
    pub fn from_pr(pr: &Pr) -> Self {
        PrNumber(pr.number)
    }

    /// Create a PrNumber from a raw value (use sparingly)
    pub fn from_raw(value: usize) -> Self {
        PrNumber(value)
    }

    /// Get the raw usize value (for API calls, display, serialization, etc.)
    pub fn value(&self) -> usize {
        self.0
    }
}

impl std::fmt::Display for PrNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
