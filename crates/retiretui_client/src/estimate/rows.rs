//! The estimate as rows: each expense a proposal sets or adds, the
//! categories it gathers beneath it, beneath the bucket the payees nothing
//! names, and last what the user's rules ignore; each said in a row, read
//! out, and with the records behind it.

use retiretui_engine::estimate::{
    CENTS_PER_DOLLAR, Cents, Estimate, Gathered, Of, Patterns, Proposal, Row, Unknown, Why, records,
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

const NEW_MARK: &str = " (new)";
const BUCKET_MARK: &str = " (bucket)";
const NEW_BUCKET_MARK: &str = " (new bucket)";
/// What the bucket's row for the payees nothing names says.
const UNNAMED: &str = "Unnamed payees";
/// The columns an expense's payees are tabled under.
const PAYEE_COLUMNS: [&str; 2] = ["Payee", "Per year"];
/// The columns a row's transactions are tabled under.
const RECORD_COLUMNS: [&str; 3] = ["Date", "Payee", "Amount"];
const IGNORED: &str = "Ignored";
/// What choosing a record does, where it is counted and where ignored.
const IGNORE_RECORD: &str = "Ignore";
const COUNT_RECORD: &str = "Count";

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
    /// The records the user's rules ignore.
    Ignored {
        /// What they spent a year.
        amount: Dollars,
        /// How many there are.
        rows: usize,
    },
}

/// The table's rows: each expense with what it gathers beneath it, the
/// bucket's last the payees nothing names - where no expense takes them,
/// those payees after the expenses - then what the user's rules ignore.
pub fn listed(estimate: &Estimate) -> impl Iterator<Item = Listed<'_>> {
    let mut rows = Vec::new();
    let payees = estimate.unknown.iter().map(Listed::Payee);
    let mut has_payees = false;
    for proposal in &estimate.proposals {
        rows.push(Listed::Expense(proposal));
        rows.extend(proposal.categories.iter().map(|gathered| Listed::Category {
            gathered,
            into: proposal,
        }));
        if proposal.is_bucket && !estimate.unknown.is_empty() {
            rows.push(Listed::Unnamed {
                amount: proposal.unknown,
                into: proposal,
            });
            rows.extend(payees.clone());
            has_payees = true;
        }
    }
    if !has_payees {
        rows.extend(payees);
    }
    if estimate.ignored_rows > 0 {
        rows.push(Listed::Ignored {
            amount: estimate.ignored,
            rows: estimate.ignored_rows,
        });
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
            Self::Ignored { amount, .. } => figure(IGNORED.to_owned(), *amount, ""),
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
        !matches!(self, Self::Expense(_) | Self::Ignored { .. })
    }

    /// Whether a form edits the row.
    #[must_use]
    pub const fn is_editable(&self) -> bool {
        !matches!(self, Self::Unnamed { .. } | Self::Ignored { .. })
    }

    /// What the row is, as its read-out is headed.
    #[must_use]
    pub const fn title(&self) -> &'static str {
        match self {
            Self::Expense(_) => "Expense",
            Self::Category { .. } => "Category",
            Self::Unnamed { .. } => "Payees",
            Self::Payee(_) => "Payee",
            Self::Ignored { .. } => IGNORED,
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
            Self::Ignored { amount, rows } => vec![
                own(IGNORED, counted(rows, "record", "records")),
                own("Spent", format!("{} a year", form.money(amount))),
            ],
            Self::Payee(unknown) => {
                let refund = if unknown.amount < 0 {
                    ", a refund, not counted"
                } else {
                    ""
                };
                let spent = format!("{} a year{refund}", form.money(unknown.amount));
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
    /// What choosing one of the records does - ignores it or counts it
    /// again - where they are records.
    pub toggle: Option<&'static str>,
}

impl Listed<'_> {
    /// Whose records lie behind the row, and what they are called; none
    /// for an expense, behind which lie its payees.
    #[must_use]
    pub fn of(&self) -> Option<(Of, String)> {
        Some(match self {
            Self::Expense(_) => return None,
            Self::Category { gathered, .. } => {
                (Of::Category(gathered.category), gathered.category.title())
            }
            Self::Unnamed { .. } => (Of::Unnamed, UNNAMED.to_owned()),
            Self::Payee(unknown) => (Of::Payee(unknown.payee.clone()), unknown.payee.clone()),
            Self::Ignored { .. } => (Of::Ignored, IGNORED.to_lowercase()),
        })
    }

    /// What lies behind the row, among `rows` as `patterns` sort them.
    #[must_use]
    pub fn behind(&self, rows: &[Row], patterns: &Patterns, form: MoneyForm) -> Behind {
        let Some((of, what)) = self.of() else {
            let Self::Expense(proposal) = self else {
                return Behind::default();
            };
            return payees_behind(proposal, form);
        };
        let toggle = if of == Of::Ignored {
            COUNT_RECORD
        } else {
            IGNORE_RECORD
        };
        Behind {
            toggle: Some(toggle),
            ..records_behind(&what, &records(rows, patterns, &of))
        }
    }
}

/// An expense's payees, each with what it adds a year.
fn payees_behind(proposal: &Proposal, form: MoneyForm) -> Behind {
    Behind {
        title: format!("Payees: {}", proposal.name),
        columns: PAYEE_COLUMNS.map(str::to_owned).to_vec(),
        rows: proposal
            .payees
            .iter()
            .map(|(payee, amount)| vec![payee.clone(), form.money(*amount)])
            .collect(),
        toggle: None,
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
        toggle: None,
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
    let ignored = if estimate.ignored_rows > 0 {
        format!(
            "; {} ignored, {} a year",
            counted(estimate.ignored_rows, "record", "records"),
            form.money(estimate.ignored)
        )
    } else {
        String::new()
    };
    format!(
        "{} through {}: {} spent, {} of income, {} in transfers a year; {} unnamed, {} a year{ignored}.",
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
