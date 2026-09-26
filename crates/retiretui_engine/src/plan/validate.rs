use std::collections::BTreeSet;
use std::fmt;

use super::escalation::ColaSpec;
use super::incomes::IncomeKind;
use super::{AccountKind, TreatmentClass};
use super::{FilingStatus, Plan, SCHEMA_VERSION, Span};

const MIN_START_YEAR: i16 = 1900;
const MAX_START_YEAR: i16 = 2200;
const MIN_INFLATION: f64 = -0.10;
const MAX_INFLATION: f64 = 0.50;

fn is_plausible_rate(rate: f64) -> bool {
    rate.is_finite() && (MIN_INFLATION..=MAX_INFLATION).contains(&rate)
}

/// A semantic problem found in a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// TOML-style path of the offending item, e.g. `accounts[2].locked_until`.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

pub(crate) fn push_issue(
    issues: &mut Vec<Issue>,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(Issue {
        path: path.into(),
        message: message.into(),
    });
}

pub(crate) fn validate(plan: &Plan) -> Vec<Issue> {
    let mut checker = Checker {
        plan,
        issues: Vec::new(),
    };
    checker.check_schema();
    checker.check_settings();
    checker.check_household();
    checker.check_duplicate_ids();
    checker.check_accounts();
    if let Some(market) = &plan.market {
        super::market::check_market(market, &mut checker.issues);
    }
    checker.check_income();
    checker.check_expenses();
    checker.check_cliffs();
    checker.check_medicare();
    checker.check_transfers();
    checker.check_conversions();
    super::contributions::check_contributions(plan, &mut checker.issues);
    super::residency::check_residency(plan, &mut checker.issues);
    super::references::check_references(plan, &mut checker.issues);
    checker.issues
}

const MISSING_ID: &str = "is required: it is what scenarios and references match the item by, and `name` is only what it is shown as";

struct Checker<'a> {
    plan: &'a Plan,
    issues: Vec<Issue>,
}

impl<'a> Checker<'a> {
    fn push(&mut self, path: impl Into<String>, message: impl Into<String>) {
        push_issue(&mut self.issues, path, message);
    }

    fn check_schema(&mut self) {
        if self.plan.schema != SCHEMA_VERSION {
            self.push(
                "schema",
                format!(
                    "unsupported schema version {} (this build reads {SCHEMA_VERSION})",
                    self.plan.schema
                ),
            );
        }
    }

    fn check_settings(&mut self) {
        let settings = &self.plan.plan;
        if !(MIN_START_YEAR..=MAX_START_YEAR).contains(&settings.start_year) {
            self.push("plan.start_year", "not a plausible calendar year");
        }
        if !is_plausible_rate(settings.inflation) {
            self.push(
                "plan.inflation",
                format!("must be between {MIN_INFLATION} and {MAX_INFLATION}"),
            );
        }
        if settings
            .wage_growth
            .is_some_and(|rate| !is_plausible_rate(rate))
        {
            self.push(
                "plan.wage_growth",
                format!("must be between {MIN_INFLATION} and {MAX_INFLATION}"),
            );
        }
        if settings.withdrawal_order.is_empty() {
            self.push("plan.withdrawal_order", "must list at least one class");
        }
        let mut seen = BTreeSet::new();
        for (i, class) in settings.withdrawal_order.iter().enumerate() {
            if !seen.insert(class) {
                self.push(
                    format!("plan.withdrawal_order[{i}]"),
                    "classes must not repeat",
                );
            }
        }
        self.check_surplus_target();
    }

    fn check_surplus_target(&mut self) {
        let Some(id) = self.plan.plan.surplus_to.clone() else {
            let has_cash = self
                .plan
                .accounts
                .iter()
                .any(|account| account.kind == AccountKind::Cash);
            if !has_cash {
                self.push(
                    "plan.surplus_to",
                    "no cash account exists to receive surplus; set plan.surplus_to",
                );
            }
            return;
        };
        match self.plan.account(&id) {
            Some(account) if account.treatment() == TreatmentClass::Taxable => {}
            Some(_) => self.push(
                "plan.surplus_to",
                "surplus can only sweep into a taxable account",
            ),
            None => self.push("plan.surplus_to", format!("unknown account `{id}`")),
        }
    }

    fn check_household(&mut self) {
        let household = &self.plan.household;
        let expected = match household.filing {
            FilingStatus::Single => 1,
            FilingStatus::MarriedJoint => 2,
        };
        if household.people.len() != expected {
            self.push(
                "household.people",
                format!(
                    "filing status requires exactly {expected} person(s), found {}",
                    household.people.len()
                ),
            );
        }
        let start_year = self.plan.plan.start_year;
        let horizon = i16::from(self.plan.plan.horizon_age);
        for (i, person) in household.people.iter().enumerate() {
            if person.birth.year() >= start_year {
                self.push(
                    format!("household.people[{i}].birth"),
                    "must be before plan.start_year",
                );
            }
            for (year, &amount) in &person.earnings {
                if amount < 0 {
                    self.push(
                        format!("household.people[{i}].earnings.{year}"),
                        "must not be negative",
                    );
                }
            }
            if person.age_in_year(start_year) >= horizon {
                self.push(
                    format!("household.people[{i}]"),
                    "already past plan.horizon_age at plan start",
                );
            }
        }
    }

    fn check_duplicate_ids(&mut self) {
        let plan = self.plan;
        let groups = [
            ("household.people", keyed(&plan.household.people, |p| &p.id)),
            ("events", keyed(&plan.events, |e| &e.id)),
            ("accounts", keyed(&plan.accounts, |a| &a.id)),
            ("income", keyed(&plan.income, |i| &i.id)),
            ("expenses", keyed(&plan.expenses, |e| &e.id)),
            ("cliffs", keyed(&plan.cliffs, |c| &c.id)),
            ("transfers", keyed(&plan.transfers, |t| &t.id)),
            ("conversions", keyed(&plan.conversions, |c| &c.id)),
            ("contributions", keyed(&plan.contributions, |c| &c.id)),
        ];
        for (prefix, ids) in groups {
            self.check_unique(prefix, &ids);
        }
    }

    fn check_unique(&mut self, prefix: &str, ids: &[(usize, &str)]) {
        let mut seen = BTreeSet::new();
        for (i, id) in ids {
            if id.is_empty() {
                self.push(format!("{prefix}[{i}].id"), MISSING_ID);
            }
            if !seen.insert(*id) {
                self.push(format!("{prefix}[{i}].id"), "duplicate id");
            }
        }
    }

    fn check_accounts(&mut self) {
        for (i, account) in self.plan.accounts.iter().enumerate() {
            let path = format!("accounts[{i}]");
            if self.plan.person(&account.owner).is_none() {
                self.push(
                    format!("{path}.owner"),
                    format!("unknown person `{}`", account.owner),
                );
            }
            if account.roth && !account.kind.supports_roth() {
                self.push(format!("{path}.roth"), "this account kind cannot be Roth");
            }
            if account.balance < 0 {
                self.push(format!("{path}.balance"), "must not be negative");
            }
            self.check_basis(&path, account);
            super::allocation::check_returns(&path, account, &mut self.issues);
        }
    }

    fn check_basis(&mut self, path: &str, account: &super::Account) {
        let Some(basis) = account.basis else {
            return;
        };
        if !account.keeps_basis() {
            self.push(
                format!("{path}.basis"),
                "basis is tracked on brokerage and tax-deferred accounts",
            );
        }
        if basis < 0 {
            self.push(format!("{path}.basis"), "must not be negative");
        }
        if basis > account.balance {
            self.push(format!("{path}.basis"), "must not exceed the balance");
        }
    }

    fn check_income(&mut self) {
        for (i, income) in self.plan.income.iter().enumerate() {
            let path = format!("income[{i}]");
            if self.plan.person(&income.owner).is_none() {
                self.push(
                    format!("{path}.owner"),
                    format!("unknown person `{}`", income.owner),
                );
            }
            match income.amount {
                Some(amount) if amount < 0 => {
                    self.push(format!("{path}.amount"), "must not be negative");
                }
                None if income.kind != IncomeKind::SocialSecurity => self.push(
                    format!("{path}.amount"),
                    "required; only social security can be computed from an earnings record",
                ),
                _ => {}
            }
            let is_mixed = check_span(&mut self.issues, &path, income.span(), income.cola);
            if is_mixed {
                continue;
            }
            if income.kind == IncomeKind::Windfall && income.on.is_none() {
                self.push(path, "a windfall is one-time and requires `on`");
            } else if income.kind == IncomeKind::SocialSecurity && income.start.is_none() {
                self.push(path, "social security requires an explicit `start` (claim)");
            }
        }
    }

    fn check_expenses(&mut self) {
        for (i, expense) in self.plan.expenses.iter().enumerate() {
            let path = format!("expenses[{i}]");
            if expense.amount < 0 {
                self.push(format!("{path}.amount"), "must not be negative");
            }
            check_span(&mut self.issues, &path, expense.span(), expense.cola);
        }
    }

    fn check_cliffs(&mut self) {
        for (i, cliff) in self.plan.cliffs.iter().enumerate() {
            let path = format!("cliffs[{i}]");
            if cliff.magi_over < 0 {
                self.push(format!("{path}.magi_over"), "must not be negative");
            }
            if cliff.cost < 0 {
                self.push(format!("{path}.cost"), "must not be negative");
            }
            check_span(&mut self.issues, &path, cliff.span(), cliff.cola);
        }
    }

    fn check_medicare(&mut self) {
        let Some(medicare) = &self.plan.medicare else {
            return;
        };
        if medicare.prior_magi.len() > crate::tax::IRMAA_LOOKBACK_YEARS as usize {
            self.push(
                "medicare.prior_magi",
                "at most two pre-plan years are looked back to",
            );
        }
        for (i, &magi) in medicare.prior_magi.iter().enumerate() {
            if magi < 0 {
                self.push(format!("medicare.prior_magi[{i}]"), "must not be negative");
            }
        }
    }

    fn check_transfers(&mut self) {
        for (i, transfer) in self.plan.transfers.iter().enumerate() {
            let path = format!("transfers[{i}]");
            if transfer.from == transfer.to {
                self.push(path.clone(), "`from` and `to` must differ");
            }
            if let Some(amount) = transfer.amount
                && amount <= 0
            {
                self.push(format!("{path}.amount"), "must be positive when present");
            }
            let (Some(from), Some(to)) = (
                self.lookup_account(&path, "from", &transfer.from),
                self.lookup_account(&path, "to", &transfer.to),
            ) else {
                continue;
            };
            match (from.treatment(), to.treatment()) {
                (a, b) if a == b => {}
                (TreatmentClass::Deferred, TreatmentClass::Taxable) => {}
                (TreatmentClass::Deferred, TreatmentClass::Roth) => {
                    self.push(
                        path,
                        "deferred to Roth is a conversion; use [[conversions]]",
                    );
                }
                _ => self.push(path, "unsupported transfer between these tax treatments"),
            }
        }
    }

    fn check_conversions(&mut self) {
        for (i, conversion) in self.plan.conversions.iter().enumerate() {
            let path = format!("conversions[{i}]");
            if conversion.amount <= 0 {
                self.push(format!("{path}.amount"), "must be positive");
            }
            check_span(&mut self.issues, &path, conversion.span(), conversion.cola);
            let (Some(from), Some(to)) = (
                self.lookup_account(&path, "from", &conversion.from),
                self.lookup_account(&path, "to", &conversion.to),
            ) else {
                continue;
            };
            if from.treatment() != TreatmentClass::Deferred {
                self.push(format!("{path}.from"), "must be a tax-deferred account");
            }
            if to.treatment() != TreatmentClass::Roth {
                self.push(format!("{path}.to"), "must be a Roth account");
            }
            if from.owner != to.owner {
                self.push(path, "`from` and `to` must have the same owner");
            }
        }
    }

    fn lookup_account(&mut self, path: &str, field: &str, id: &str) -> Option<&'a super::Account> {
        let found = self.plan.account(id);
        if found.is_none() {
            self.push(format!("{path}.{field}"), format!("unknown account `{id}`"));
        }
        found
    }
}

/// Checks an item's window and escalation, and says whether its `on`
/// wrongly stands beside a window.
pub(super) fn check_span(
    issues: &mut Vec<Issue>,
    path: &str,
    span: Span<'_>,
    cola: ColaSpec,
) -> bool {
    check_cola(issues, path, cola);
    let is_mixed = span.on.is_some() && (span.start.is_some() || span.end.is_some());
    if is_mixed {
        push_issue(issues, path, "`on` excludes `start` and `end`");
    }
    is_mixed
}

fn check_cola(issues: &mut Vec<Issue>, path: &str, cola: ColaSpec) {
    if let Some(rate) = cola.rate()
        && !is_plausible_rate(rate)
    {
        push_issue(
            issues,
            format!("{path}.cola"),
            format!("must be a rate between {MIN_INFLATION} and {MAX_INFLATION}"),
        );
    }
}

fn keyed<'a, T>(items: &'a [T], key: impl Fn(&'a T) -> &'a str) -> Vec<(usize, &'a str)> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| (i, key(item)))
        .collect()
}
