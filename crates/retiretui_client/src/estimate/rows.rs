//! The estimate as rows: each expense a proposal sets or adds, the
//! categories it gathers beneath it, and beneath the bucket the payees
//! nothing names; each said in a row, read out, and with the records
//! behind it.

use retiretui_engine::estimate::{
    Cents, Estimate, Gathered, Of, Patterns, Proposal, Row, Unknown, Why, records,
};
use retiretui_engine::plan::Dollars;
use serde::Serialize;

use super::{NOTHING_READ, UNKNOWN};
use crate::present::{MoneyForm, counted, money, month};

const NOTHING: &str = "-";
const INDENT: &str = "  ";
const PAYEE_INDENT: &str = "    ";
/// What a payee's row says where money came in from it, uncounted.
const REFUND: &str = "refund";
const CENTS_PER_DOLLAR: i64 = 100;
const MONTHS_PER_YEAR: Dollars = 12;
const NEW_MARK: &str = " (new)";
const BUCKET_MARK: &str = " (bucket)";
const NEW_BUCKET_MARK: &str = " (new bucket)";
/// What the bucket's row for the payees nothing names says.
const UNNAMED: &str = "Unnamed payees";
/// The columns an expense's payees are tabled under.
const PAYEE_COLUMNS: [&str; 2] = ["Payee", "Per year"];
/// The columns a row's transactions are tabled under.
const RECORD_COLUMNS: [&str; 3] = ["Date", "Payee", "Amount"];

/// One row of the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Listed<'a> {
    /// An expense the proposals set or add.
    Expense(&'a Proposal),
    /// A category gathered into the expense above it.
    Category {
        /// The category, what it adds, and why it landed here.
        gathered: &'a Gathered,
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

/// The table's rows: each expense with what it gathers beneath it, the
/// bucket's last the payees nothing names; where no expense takes them,
/// those payees come last.
pub fn listed(estimate: &Estimate) -> impl Iterator<Item = Listed<'_>> {
    let mut rows = Vec::new();
    let payees = estimate.unknown.iter().map(Listed::Payee);
    for proposal in &estimate.proposals {
        rows.push(Listed::Expense(proposal));
        rows.extend(proposal.categories.iter().map(|gathered| Listed::Category {
            gathered,
            into: proposal,
        }));
        if proposal.is_bucket && proposal.unknown > 0 {
            rows.push(Listed::Unnamed {
                amount: proposal.unknown,
                into: proposal,
            });
            rows.extend(payees.clone());
        }
    }
    if !rows.iter().any(|row| matches!(row, Listed::Unnamed { .. })) {
        rows.extend(payees);
    }
    rows.into_iter()
}

/// The expense's name, marked where it is new or the bucket.
fn marked(proposal: &Proposal) -> String {
    let mark = match (proposal.is_new, proposal.is_bucket) {
        (false, true) => BUCKET_MARK,
        (false, false) => "",
        (true, true) => NEW_BUCKET_MARK,
        (true, false) => NEW_MARK,
    };
    format!("{}{mark}", proposal.name)
}

/// The amount `proposal` changes its expense by: the whole of it for a
/// new one.
fn change(proposal: &Proposal) -> Dollars {
    proposal.amount - proposal.now.unwrap_or_default()
}

impl Listed<'_> {
    /// The row under [`super::COLUMNS`].
    #[must_use]
    pub fn cells(&self, form: MoneyForm) -> [String; 5] {
        let figure = |name: String, amount: Dollars, why: &str| {
            [
                name,
                String::new(),
                form.money(amount),
                String::new(),
                why.to_owned(),
            ]
        };
        match self {
            Self::Expense(proposal) => [
                marked(proposal),
                proposal
                    .now
                    .map_or(NOTHING.to_owned(), |now| form.money(now)),
                form.money(proposal.amount),
                form.signed(change(proposal)),
                String::new(),
            ],
            Self::Category { gathered, .. } => figure(
                format!("{INDENT}{}", gathered.category.title()),
                gathered.amount,
                why_word(gathered.why),
            ),
            Self::Unnamed { amount, .. } => figure(format!("{INDENT}{UNNAMED}"), *amount, ""),
            Self::Payee(unknown) => figure(
                format!("{PAYEE_INDENT}{}", unknown.payee),
                unknown.amount,
                if unknown.amount < 0 { REFUND } else { "" },
            ),
        }
    }

    /// Whether the row is one of those an expense gathers.
    #[must_use]
    pub const fn is_nested(&self) -> bool {
        !matches!(self, Self::Expense(_))
    }

    /// Whether a form edits the row.
    #[must_use]
    pub const fn is_editable(&self) -> bool {
        !matches!(self, Self::Unnamed { .. })
    }

    /// What the row is, as its read-out is headed.
    #[must_use]
    pub const fn title(&self) -> &'static str {
        match self {
            Self::Expense(_) => "Expense",
            Self::Category { .. } => "Category",
            Self::Unnamed { .. } => "Payees",
            Self::Payee(_) => "Payee",
        }
    }

    /// The row read out beside the table.
    #[must_use]
    pub fn read_out(&self, months: u8, form: MoneyForm) -> Vec<(String, String)> {
        let own = |label: &str, text: String| (label.to_owned(), text);
        match *self {
            Self::Expense(proposal) => expense_read(proposal, months, form),
            Self::Category { gathered, into } => vec![
                own("Category", gathered.category.title()),
                own("Goes to", marked(into)),
                own("Because", because(gathered.why).to_owned()),
                own("Spent", format!("{} a year", form.money(gathered.amount))),
            ],
            Self::Unnamed { amount, into } => vec![
                own("Payees", UNNAMED.to_owned()),
                own("Goes to", marked(into)),
                own("Spent", format!("{} a year", form.money(amount))),
            ],
            Self::Payee(unknown) => {
                let spent = if unknown.amount < 0 {
                    format!(
                        "{} a year, a refund, not counted",
                        form.money(unknown.amount)
                    )
                } else {
                    format!("{} a year", form.money(unknown.amount))
                };
                vec![
                    own("Payee", unknown.payee.clone()),
                    own("Category", UNKNOWN.to_owned()),
                    own("Spent", spent),
                    own("From", counted(unknown.rows, "row", "rows")),
                ]
            }
        }
    }
}

fn expense_read(proposal: &Proposal, months: u8, form: MoneyForm) -> Vec<(String, String)> {
    vec![
        ("Expense".to_owned(), marked(proposal)),
        (
            "Now".to_owned(),
            proposal
                .now
                .map_or(NOTHING.to_owned(), |now| form.money(now)),
        ),
        (
            "Proposed".to_owned(),
            format!("{} a year", form.money(proposal.amount)),
        ),
        ("Change".to_owned(), form.signed(change(proposal))),
        (
            "From".to_owned(),
            format!(
                "{} over {}, {}",
                counted(proposal.rows, "row", "rows"),
                counted(months.into(), "month", "months"),
                form.money(proposal.total)
            ),
        ),
    ]
}

/// What lies behind a row: an expense's payees with what each adds a
/// year, or the transactions of a category, a payee or the unnamed payees.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Behind {
    /// What the records are, as their pane is headed.
    pub title: String,
    /// The columns they are tabled under.
    pub columns: Vec<String>,
    /// Each record's cells.
    pub rows: Vec<Vec<String>>,
}

impl Listed<'_> {
    /// What lies behind the row, among `rows` as `patterns` sort them,
    /// over a window of `months`.
    #[must_use]
    pub fn behind(
        &self,
        (rows, patterns): (&[Row], &Patterns),
        months: u8,
        form: MoneyForm,
    ) -> Behind {
        let of = |what: &str, of: Of| records_behind(what, &records(rows, patterns, &of));
        match self {
            Self::Expense(proposal) => payees_behind(proposal, months, form),
            Self::Category { gathered, .. } => {
                of(&gathered.category.title(), Of::Category(gathered.category))
            }
            Self::Unnamed { .. } => of(UNNAMED, Of::Unnamed),
            Self::Payee(unknown) => of(&unknown.payee, Of::Payee(unknown.payee.clone())),
        }
    }
}

/// An expense's payees, each with what it adds a year over a window of
/// `months`.
fn payees_behind(proposal: &Proposal, months: u8, form: MoneyForm) -> Behind {
    let annual = |total: Dollars| total * MONTHS_PER_YEAR / Dollars::from(months);
    Behind {
        title: format!("Payees: {}", proposal.name),
        columns: PAYEE_COLUMNS.map(str::to_owned).to_vec(),
        rows: proposal
            .payees
            .iter()
            .map(|(payee, total)| vec![payee.clone(), form.money(annual(*total))])
            .collect(),
    }
}

/// `records` of `what`, each its date, its payee as the bank wrote it and
/// what it spent, a credit negative.
fn records_behind(what: &str, records: &[&Row]) -> Behind {
    Behind {
        title: format!("Records: {what}"),
        columns: RECORD_COLUMNS.map(str::to_owned).to_vec(),
        rows: records
            .iter()
            .map(|row| {
                vec![
                    row.on.0.to_string(),
                    row.payee.clone(),
                    spent_cents(row.amount),
                ]
            })
            .collect(),
    }
}

/// What a row spent, in dollars and cents: `$42.10`, `-$5.00` for money in.
fn spent_cents(amount: Cents) -> String {
    let spent = -amount.0;
    let sign = if spent < 0 { "-" } else { "" };
    let whole = spent.abs();
    format!(
        "{sign}{}.{:02}",
        money(whole / CENTS_PER_DOLLAR),
        whole % CENTS_PER_DOLLAR
    )
}

/// Why a category landed where it did, in a word.
const fn why_word(why: Why) -> &'static str {
    match why {
        Why::Assigned => "assigned",
        Why::Named => "named",
        Why::Family => "family",
        Why::Bucket => "bucket",
        Why::Large => "large",
        Why::Always => "lean",
    }
}

/// Why a category landed where it did, in words.
const fn because(why: Why) -> &'static str {
    match why {
        Why::Assigned => "assigned in the plan",
        Why::Named => "the expense is named for it",
        Why::Family => "the expense is of its family",
        Why::Bucket => "nothing else fits it",
        Why::Large => "it is large enough for its own",
        Why::Always => "the plan leans to a new one for each",
    }
}

/// What the estimate rests on, in a line: the window, what came in and
/// what moved.
#[must_use]
pub fn summary(estimate: &Estimate, form: MoneyForm) -> String {
    let Some(through) = estimate.through else {
        return NOTHING_READ.to_owned();
    };
    let spent = estimate.proposals.iter().map(|p| p.amount).sum();
    let unnamed = estimate.proposals.iter().map(|p| p.unknown).sum();
    format!(
        "{} through {}: {} spent, {} of income, {} in transfers a year; {} unnamed, {} a year.",
        counted(estimate.months.into(), "month", "months"),
        month(through),
        form.money(spent),
        form.money(estimate.income),
        form.money(estimate.transfers),
        counted(estimate.unknown.len(), "payee", "payees"),
        form.money(unnamed),
    )
}

fn counts(estimate: &Estimate) -> (usize, usize) {
    let set = estimate.proposals.iter().filter(|p| !p.is_new).count();
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
