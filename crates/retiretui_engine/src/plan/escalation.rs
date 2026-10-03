use serde::{Deserialize, Serialize};

/// How an amount escalates from its value at its anchor, the plan's start
/// unless an income says otherwise: following plan inflation (`true`, the
/// default), frozen in nominal dollars (`false`), or at a fixed annual rate
/// of its own (`cola = 0.0125`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColaSpec {
    /// `true` follows plan inflation; `false` freezes the nominal amount.
    Follows(bool),
    /// A fixed annual rate independent of plan inflation.
    Rate(f64),
}

/// Where an income's escalation runs from, which is also what its amount
/// states: today's dollars, escalated from the plan's start (the default),
/// or what the income pays in its first year, escalated from that year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColaAnchor {
    /// From the plan's start.
    #[default]
    Plan,
    /// From the income's first year, or the plan's start where that is
    /// later.
    Start,
}

impl ColaAnchor {
    /// Every anchor, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[Self::Plan, Self::Start];

    /// The anchor as a plan file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Start => "start",
        }
    }
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
