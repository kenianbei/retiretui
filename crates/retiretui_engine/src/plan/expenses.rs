use serde::{Deserialize, Serialize};

use super::escalation::ColaSpec;
use super::triggers::Trigger;
use super::{Dollars, Plan};

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
    /// Health spending, which an HSA pays without tax.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub medical: bool,
    /// Spending the household cannot cut: a solver or a spending rule
    /// leaves it as stated.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub essential: bool,
    /// The amount restated from later triggers; excludes `on`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<Phase>,
}

/// An expense's amount from a trigger on: the year it fires is spent at
/// the old amount for the months before and the new from the trigger's
/// month, where it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    /// When the amount changes.
    pub from: Trigger,
    /// The annual amount from then on, in today's dollars.
    pub amount: Dollars,
}

impl Expense {
    /// Whether the household could spend more or less of it: recurring,
    /// and not `essential`.
    #[must_use]
    pub const fn is_flexible(&self) -> bool {
        !self.essential && self.on.is_none()
    }
}

impl Plan {
    /// What the plan spends a year on what it could cut, in today's
    /// dollars: every [flexible](Expense::is_flexible) expense as stated.
    #[must_use]
    pub fn flexible_spending(&self) -> Dollars {
        let flexible = self.expenses.iter().filter(|expense| expense.is_flexible());
        flexible.map(|expense| expense.amount).sum()
    }
}
