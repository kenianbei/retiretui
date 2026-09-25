use serde::{Deserialize, Serialize};

/// How an amount escalates from its today's-dollar value at plan start:
/// following plan inflation (`true`, the default), frozen in nominal dollars
/// (`false`), or at a fixed annual rate of its own (`cola = 0.0125`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColaSpec {
    /// `true` follows plan inflation; `false` freezes the nominal amount.
    Follows(bool),
    /// A fixed annual rate independent of plan inflation.
    Rate(f64),
}

impl Default for ColaSpec {
    fn default() -> Self {
        Self::Follows(true)
    }
}

impl ColaSpec {
    /// The cumulative escalation factor after `years` years at this spec,
    /// given `deflator`, what prices grew by over those years.
    #[must_use]
    pub fn factor(self, deflator: f64, years: i32) -> f64 {
        match self {
            Self::Follows(true) => deflator,
            Self::Follows(false) => 1.0,
            Self::Rate(rate) => (1.0 + rate).powi(years),
        }
    }

    /// The explicit rate, when one was given.
    #[must_use]
    pub fn rate(self) -> Option<f64> {
        match self {
            Self::Rate(rate) => Some(rate),
            Self::Follows(_) => None,
        }
    }
}
