use serde::{Deserialize, Serialize};

use super::Dollars;
use super::escalation::ColaSpec;
use super::triggers::Trigger;

/// A spending item, in annual today's dollars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expense {
    /// Unique id, which scenarios and references match the expense by; a
    /// plan without one is refused by validation.
    #[serde(default)]
    pub id: String,
    /// Display name; defaults to the id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Annual amount in today's dollars (the one-time amount with `on`).
    pub amount: Dollars,
    /// First year spent; absent means from plan start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<Trigger>,
    /// Last year spent; absent means through the horizon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<Trigger>,
    /// One-time expense in the trigger's year; excludes `start`/`end`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<Trigger>,
    /// How the amount escalates: plan inflation (`true`, default), frozen
    /// nominal (`false`), or a fixed annual rate of its own.
    #[serde(default)]
    pub cola: ColaSpec,
}
