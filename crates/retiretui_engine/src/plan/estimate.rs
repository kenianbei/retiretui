//! How the Spending Estimate fits a bank's spending to the plan: the
//! `[estimate]` table, which the projection never reads.

use serde::{Deserialize, Serialize};

use super::validate::push_issue;
use super::{Issue, Plan};
use crate::estimate::Category;

/// The share of a year's spending a category reaches to earn an expense of
/// its own under [`Lean::Large`], where the plan states none.
pub const LARGE_SHARE: f64 = 0.10;

/// The `[estimate]` table.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstimateSettings {
    /// How readily a category with no expense of its own is proposed as a
    /// new one.
    #[serde(default)]
    pub lean: Lean,
    /// What [`Lean::Large`] holds a category to, a share of the year's
    /// spending; [`LARGE_SHARE`] where unstated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_share: Option<f64>,
    /// The expense what fits nowhere else goes to; detected by its name
    /// where unstated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    /// Categories sent to an expense outright.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assign: Vec<Assign>,
}

impl EstimateSettings {
    /// The share [`Lean::Large`] holds a category to.
    #[must_use]
    pub fn large_share(&self) -> f64 {
        self.large_share.unwrap_or(LARGE_SHARE)
    }
}

/// How readily a category is proposed as a new expense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lean {
    /// Never: it joins an expense of its family, else the bucket.
    Existing,
    /// Where it is at least the large share of the year's spending.
    #[default]
    Large,
    /// Always.
    New,
}

impl Lean {
    /// Every lean, in the order the schema declares them.
    pub const ALL: [Self; 3] = [Self::Existing, Self::Large, Self::New];

    /// The file's spelling.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Existing => "existing",
            Self::Large => "large",
            Self::New => "new",
        }
    }
}

/// One category sent to one expense.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assign {
    /// The spending category.
    pub category: Category,
    /// The expense it goes to.
    pub expense: String,
}

pub(super) fn check_estimate(plan: &Plan, settings: &EstimateSettings, issues: &mut Vec<Issue>) {
    let is_expense = |id: &str| plan.expenses.iter().any(|expense| expense.id == id);
    if let Some(bucket) = &settings.bucket
        && !is_expense(bucket)
    {
        push_issue(issues, "estimate.bucket", format!("no expense `{bucket}`"));
    }
    if settings
        .large_share
        .is_some_and(|share| !(share > 0.0 && share <= 1.0))
    {
        push_issue(
            issues,
            "estimate.large_share",
            "must be over 0 and at most 1",
        );
    }
    for (at, assign) in settings.assign.iter().enumerate() {
        let path = format!("estimate.assign[{at}]");
        if !is_expense(&assign.expense) {
            let message = format!("no expense `{}`", assign.expense);
            push_issue(issues, format!("{path}.expense"), message);
        }
        if !assign.category.is_spending() {
            push_issue(issues, format!("{path}.category"), "is not spending");
        }
        let earlier = &settings.assign[..at];
        if earlier.iter().any(|it| it.category == assign.category) {
            push_issue(issues, format!("{path}.category"), "is assigned already");
        }
    }
}
