//! Contributions into accounts: each names the account it pays into, who
//! pays, and one amount, over the window every other flow has.

use serde::{Deserialize, Serialize};

use super::escalation::ColaSpec;
use super::triggers::Trigger;
use super::{Dollars, Issue, Plan, push_issue, validate};
use crate::tax;

/// Who pays a contribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Payer {
    /// The account's owner, out of cash flow; deducted where the kind
    /// defers tax.
    #[default]
    Employee,
    /// An employer, landing in the account directly.
    Employer,
    /// The owner's after-tax money into a workplace plan, kept as basis
    /// and filled last into the plan's yearly cap.
    AfterTax,
}

impl Payer {
    /// Every payer, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[Self::Employee, Self::Employer, Self::AfterTax];

    /// The payer as a plan file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Employee => "employee",
            Self::Employer => "employer",
            Self::AfterTax => "after-tax",
        }
    }
}

/// An employer's match on what the employee pays into the same account.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Match {
    /// The share of the employee's contribution matched.
    pub rate: f64,
    /// The share of the income's gross up to which it is matched.
    pub up_to: f64,
}

/// A yearly rise in a contribution's rate, up to a ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// Added to the rate each year after the first.
    pub add: f64,
    /// The rate it stops rising at.
    pub up_to: f64,
}

/// A contribution into an account. Its amount is stated one way: dollars
/// per year, a share of a named income's gross for the year, the year's
/// legal maximum, or an employer's match on the employee's own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    /// Unique id, which scenarios and references match the contribution by; a
    /// plan without one is refused by validation.
    #[serde(default)]
    pub id: String,
    /// Display name; defaults to the id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The account paid into.
    pub to: String,
    /// Who pays.
    #[serde(default)]
    pub by: Payer,
    /// Dollars per year, in today's dollars.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Dollars>,
    /// A share of the gross of the income `of` names, each year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    /// The income id a `rate` or a match's `up_to` is a share of.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    /// A yearly rise in the rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<Step>,
    /// The year's employee limit for the account's kind and the owner's age.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub max: bool,
    /// An employer's match on the employee's contributions to the account.
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub matching: Option<Match>,
    /// First year paid; absent means from plan start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<Trigger>,
    /// Last year paid; absent means through the horizon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<Trigger>,
    /// A single payment in the trigger's year; excludes `start`/`end`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<Trigger>,
    /// How a dollar amount escalates: plan inflation (`true`, default),
    /// frozen nominal (`false`), or a fixed annual rate of its own.
    #[serde(default)]
    pub cola: ColaSpec,
}

impl Contribution {
    /// The rate the item pays in a year `years` after its first, to the
    /// basis point.
    #[must_use]
    pub fn rate_after(&self, years: i32) -> Option<f64> {
        let rate = self.rate?;
        let stepped = match self.step {
            Some(step) => (rate + step.add * f64::from(years)).min(step.up_to),
            None => rate,
        };
        Some((stepped * BASIS_POINTS).round() / BASIS_POINTS)
    }
}

const BASIS_POINTS: f64 = 10_000.0;

const ONE_FORM: &str = "states exactly one of `amount`, `rate`, `max` and `match`";
const A_SHARE: &str = "must be a share between 0 and 1";

fn is_share(rate: f64) -> bool {
    rate.is_finite() && (0.0..=1.0).contains(&rate)
}

/// One form of amount, stated by the payer it belongs to.
fn check_form(path: &str, contribution: &Contribution, issues: &mut Vec<Issue>) {
    let forms = [
        contribution.amount.is_some(),
        contribution.rate.is_some(),
        contribution.max,
        contribution.matching.is_some(),
    ];
    if forms.iter().filter(|&&stated| stated).count() != 1 {
        push_issue(issues, path, ONE_FORM);
    }
    if contribution.max && contribution.by != Payer::Employee {
        push_issue(
            issues,
            format!("{path}.max"),
            "only an employee amount has a maximum",
        );
    }
    if contribution.matching.is_some() && contribution.by != Payer::Employer {
        push_issue(issues, format!("{path}.match"), "only an employer matches");
    }
    if contribution.amount.is_some_and(|amount| amount < 0) {
        push_issue(issues, format!("{path}.amount"), "must not be negative");
    }
    if let Some(matching) = contribution.matching {
        if !is_share(matching.rate) {
            push_issue(issues, format!("{path}.match.rate"), A_SHARE);
        }
        if !is_share(matching.up_to) {
            push_issue(issues, format!("{path}.match.up_to"), A_SHARE);
        }
    }
}

/// The income a share or a match is of, and the step a share rises by.
fn check_share(path: &str, contribution: &Contribution, plan: &Plan, issues: &mut Vec<Issue>) {
    let wants_income = contribution.rate.is_some() || contribution.matching.is_some();
    if wants_income != contribution.of.is_some() {
        push_issue(
            issues,
            format!("{path}.of"),
            "`of` goes with `rate` and `match`",
        );
    }
    if let Some(of) = &contribution.of
        && plan.income_source(of).is_none()
    {
        push_issue(
            issues,
            format!("{path}.of"),
            format!("unknown income `{of}`"),
        );
    }
    if contribution.rate.is_some_and(|rate| !is_share(rate)) {
        push_issue(issues, format!("{path}.rate"), A_SHARE);
    }
    let Some(step) = contribution.step else {
        return;
    };
    let Some(rate) = contribution.rate else {
        push_issue(issues, format!("{path}.step"), "`step` goes with `rate`");
        return;
    };
    if !step.add.is_finite() || step.add < 0.0 {
        push_issue(issues, format!("{path}.step.add"), "must not be negative");
    }
    if !is_share(step.up_to) || step.up_to < rate {
        push_issue(
            issues,
            format!("{path}.step.up_to"),
            "must be a share at or above the rate",
        );
    }
}

pub(super) fn check_contributions(plan: &Plan, issues: &mut Vec<Issue>) {
    for (i, contribution) in plan.contributions.iter().enumerate() {
        let path = format!("contributions[{i}]");
        check_form(&path, contribution, issues);
        check_share(&path, contribution, plan, issues);
        validate::check_span(issues, &path, contribution.span(), contribution.cola);
        let Some(account) = plan.account(&contribution.to) else {
            let unknown = format!("unknown account `{}`", contribution.to);
            push_issue(issues, format!("{path}.to"), unknown);
            continue;
        };
        let is_taken = match contribution.by {
            Payer::Employee => account.kind.accepts_employee(),
            Payer::Employer => account.kind.accepts_employer(),
            Payer::AfterTax => account.kind.accepts_after_tax(),
        };
        if contribution.max && tax::limit_pool(account.kind).is_none() {
            push_issue(
                issues,
                format!("{path}.max"),
                "this account kind has no employee limit",
            );
        }
        if !is_taken {
            let refused = format!(
                "this account kind takes no {} contributions",
                contribution.by.as_str()
            );
            push_issue(issues, format!("{path}.by"), refused);
        }
    }
}
