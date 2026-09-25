use serde::{Deserialize, Serialize};

use super::Dollars;
use super::escalation::ColaSpec;
use super::triggers::Trigger;

/// A one-time move of money between accounts, e.g. a rollover when a linked
/// pension starts. Tax treatment is resolved by the engine: a same-treatment
/// move is not taxable; deferred to taxable is a taxable distribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transfer {
    /// Unique id, which scenarios and references match the transfer by; a
    /// plan without one is refused by validation.
    #[serde(default)]
    pub id: String,
    /// Display name; defaults to the id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Source account id.
    pub from: String,
    /// Destination account id.
    pub to: String,
    /// When the transfer executes.
    pub on: Trigger,
    /// Dollars to move in today's dollars; absent moves the whole balance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Dollars>,
}

/// A Roth conversion schedule: deferred money converted to Roth and taxed as
/// ordinary income in the year converted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conversion {
    /// Unique id, which scenarios and references match the conversion by; a
    /// plan without one is refused by validation.
    #[serde(default)]
    pub id: String,
    /// Display name; defaults to the id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Source deferred account id.
    pub from: String,
    /// Destination Roth account id.
    pub to: String,
    /// Amount converted per year, in today's dollars.
    pub amount: Dollars,
    /// First conversion year; absent means from plan start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<Trigger>,
    /// Last conversion year; absent means through the horizon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<Trigger>,
    /// Single conversion in the trigger's year; excludes `start`/`end`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<Trigger>,
    /// How the amount escalates: plan inflation (`true`, default), frozen
    /// nominal (`false`), or a fixed annual rate of its own.
    #[serde(default)]
    pub cola: ColaSpec,
}
