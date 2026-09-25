use serde::{Deserialize, Serialize};

use super::Dollars;
use super::escalation::ColaSpec;
use super::triggers::Trigger;

/// A MAGI-triggered cost: while the window is active, a year whose MAGI
/// exceeds `magi_over` spends `cost` that same year. The ACA subsidy
/// cliff's shape, declared with the user's own numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cliff {
    /// Unique id, which scenarios and references match the cliff by; a
    /// plan without one is refused by validation.
    #[serde(default)]
    pub id: String,
    /// Display name; defaults to the id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// MAGI threshold in today's dollars; strictly above it incurs the
    /// cost.
    pub magi_over: Dollars,
    /// Annual cost in today's dollars when crossed.
    pub cost: Dollars,
    /// First active year; absent means from plan start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<Trigger>,
    /// Last active year; absent means through the year before the
    /// youngest person turns 65.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<Trigger>,
    /// How the threshold and cost escalate: plan inflation (`true`,
    /// default), frozen nominal, or a fixed rate of their own.
    #[serde(default)]
    pub cola: ColaSpec,
}
