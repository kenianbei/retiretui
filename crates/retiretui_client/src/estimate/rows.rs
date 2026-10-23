//! The estimate as rows: each expense a proposal sets or adds, the
//! categories it gathers beneath it, what the bucket takes of the payees
//! nothing names, and then those payees; each said in a row and read out.

use retiretui_engine::estimate::{Category, Estimate, Proposal, Unknown};
use retiretui_engine::plan::{Dollars, Plan};

use super::{NOTHING_READ, UNKNOWN};
use crate::present::{MoneyForm, counted, month};

const NOTHING: &str = "-";
const INDENT: &str = "  ";
const NEW_MARK: &str = " (new)";
const BUCKET_MARK: &str = " (bucket)";
const NEW_BUCKET_MARK: &str = " (new bucket)";
/// What the bucket's row for the payees nothing names says.
const UNNAMED: &str = "payees nothing names";

/// One row of the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Listed<'a> {
    /// An expense the proposals set or add.
    Expense(&'a Proposal),
    /// A category gathered into the expense above it.
    Category {
        /// The category.
        category: Category,
        /// What it adds a year.
        amount: Dollars,
        /// The expense it goes to.
        into: &'a Proposal,
    },
    /// What the payees nothing names add to the bucket above.
    Unnamed {
        /// What they add a year.
        amount: Dollars,
        /// The bucket.
        into: &'a Proposal,
    },
    /// A payee the patterns did not name.
    Payee(&'a Unknown),
}

/// The table's rows: each expense with what it gathers beneath it, then
/// the payees nothing names.
#[must_use]
pub fn listed(estimate: &Estimate) -> Vec<Listed<'_>> {
    let mut rows = Vec::new();
    for proposal in &estimate.proposals {
        rows.push(Listed::Expense(proposal));
        let gathered = proposal
            .categories
            .iter()
            .map(|&(category, amount)| Listed::Category {
                category,
                amount,
                into: proposal,
            });
        rows.extend(gathered);
        if proposal.unknown > 0 {
            rows.push(Listed::Unnamed {
                amount: proposal.unknown,
                into: proposal,
            });
        }
    }
    rows.extend(estimate.unknown.iter().map(Listed::Payee));
    rows
}

/// The expense's name, marked where it is new or the bucket.
fn marked(proposal: &Proposal) -> String {
    let mark = match (proposal.expense.is_some(), proposal.is_bucket) {
        (true, true) => BUCKET_MARK,
        (true, false) => "",
        (false, true) => NEW_BUCKET_MARK,
        (false, false) => NEW_MARK,
    };
    format!("{}{mark}", proposal.name)
}

impl Listed<'_> {
    /// The row under [`super::COLUMNS`].
    #[must_use]
    pub fn cells(&self, months: u8, form: MoneyForm) -> [String; 3] {
        match self {
            Self::Expense(proposal) => [
                marked(proposal),
                months.to_string(),
                form.money(proposal.amount),
            ],
            Self::Category {
                category, amount, ..
            } => [
                format!("{INDENT}{}", category.title()),
                String::new(),
                form.money(*amount),
            ],
            Self::Unnamed { amount, .. } => [
                format!("{INDENT}{UNNAMED}"),
                String::new(),
                form.money(*amount),
            ],
            Self::Payee(unknown) => [
                unknown.payee.clone(),
                String::new(),
                form.money(unknown.amount),
            ],
        }
    }

    /// Whether the row is one of those an expense gathers.
    #[must_use]
    pub const fn is_nested(&self) -> bool {
        matches!(self, Self::Category { .. } | Self::Unnamed { .. })
    }

    /// The row read out beside the table.
    #[must_use]
    pub fn read_out(&self, plan: &Plan, months: u8, form: MoneyForm) -> Vec<(String, String)> {
        let own = |label: &str, text: String| (label.to_owned(), text);
        match *self {
            Self::Expense(proposal) => expense_read(plan, proposal, months, form),
            Self::Category {
                category,
                amount,
                into,
            } => vec![
                own("Category", category.title()),
                own("Goes to", marked(into)),
                own("Because", reason(plan, category, into).to_owned()),
                own("Spent", format!("{} a year", form.money(amount))),
            ],
            Self::Unnamed { amount, into } => vec![
                own("Payees", UNNAMED.to_owned()),
                own("Goes to", marked(into)),
                own("Spent", format!("{} a year", form.money(amount))),
            ],
            Self::Payee(unknown) => vec![
                own("Payee", unknown.payee.clone()),
                own("Category", UNKNOWN.to_owned()),
                own("Spent", format!("{} a year", form.money(unknown.amount))),
                own("From", counted(unknown.rows, "row", "rows")),
            ],
        }
    }
}

fn expense_read(
    plan: &Plan,
    proposal: &Proposal,
    months: u8,
    form: MoneyForm,
) -> Vec<(String, String)> {
    let stated = proposal
        .expense
        .as_deref()
        .and_then(|id| plan.expenses.iter().find(|e| e.id == id));
    let mut rows = vec![
        ("Expense".to_owned(), marked(proposal)),
        (
            "Now".to_owned(),
            stated.map_or(NOTHING.to_owned(), |e| form.money(e.amount)),
        ),
        (
            "Proposed".to_owned(),
            format!("{} a year", form.money(proposal.amount)),
        ),
        (
            "From".to_owned(),
            format!(
                "{} over {}, {}",
                counted(proposal.rows, "row", "rows"),
                counted(months.into(), "month", "months"),
                form.money(proposal.total)
            ),
        ),
    ];
    for (payee, amount) in &proposal.payees {
        rows.push((format!("{INDENT}{payee}"), form.money(*amount)));
    }
    rows
}

/// Why `category` landed in `into`.
fn reason(plan: &Plan, category: Category, into: &Proposal) -> &'static str {
    let assigns = plan.estimate.iter().flat_map(|it| &it.assign);
    let stated = into
        .expense
        .as_deref()
        .and_then(|id| plan.expenses.iter().find(|e| e.id == id));
    if assigns.into_iter().any(|it| it.category == category) {
        "assigned in the plan"
    } else if stated.is_some_and(|e| category.matches(e)) {
        "the expense is named for it"
    } else if into.is_bucket {
        "nothing else fits it"
    } else if stated.is_some() {
        "the expense is of its family"
    } else {
        "it is large enough for its own"
    }
}

/// What the estimate rests on, in a line: the window, what came in and
/// what moved.
#[must_use]
pub fn summary(estimate: &Estimate, form: MoneyForm) -> String {
    let Some(through) = estimate.through else {
        return NOTHING_READ.to_owned();
    };
    format!(
        "{} through {}: {} of income, {} in transfers a year; {} unnamed.",
        counted(estimate.months.into(), "month", "months"),
        month(through),
        form.money(estimate.income),
        form.money(estimate.transfers),
        counted(estimate.unknown.len(), "payee", "payees"),
    )
}

fn counts(estimate: &Estimate) -> (usize, usize) {
    let set = estimate
        .proposals
        .iter()
        .filter(|p| p.expense.is_some())
        .count();
    (set, estimate.proposals.len() - set)
}

/// What is asked before the proposals are taken into the plan.
#[must_use]
pub fn take_question(estimate: &Estimate) -> String {
    let (set, added) = counts(estimate);
    format!(
        "Set {} and add {} from {} of spending?",
        counted(set, "expense", "expenses"),
        added,
        counted(estimate.months.into(), "month", "months"),
    )
}

/// What is said once they are.
#[must_use]
pub fn taken(estimate: &Estimate) -> String {
    let (set, added) = counts(estimate);
    format!(
        "{} set and {} added from {} of spending",
        counted(set, "expense", "expenses"),
        added,
        counted(estimate.months.into(), "month", "months"),
    )
}
