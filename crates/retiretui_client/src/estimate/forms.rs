//! The forms the estimate is edited in: an expense the plan holds and one
//! to add, a category moved to another expense, a payee named, and the
//! plan's `[estimate]` table.

use std::fmt::Write;

use retiretui_engine::estimate::{self, Category, Phases, Proposal};
use retiretui_engine::plan::{Assign, Dollars, EstimateSettings, Item, Lean, Plan};
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::{Applied, Listed, UNKNOWN};
use crate::codec::{from_table, to_table};
use crate::draft::Draft;
use crate::forms::offers::{LEANS, RefSource, Vocabulary, held_word};
use crate::forms::{FieldSpec, ToolAnswers};
use crate::table::rate;

/// What a category's expense field reads as while the estimate places it.
const PLACED: &str = "Where the estimate places it";
/// What the bucket field reads as while the plan names none.
const DETECTED: &str = "Detected";
const LEAN: &str = "lean";
const NOTHING_PICKED: &str = "nothing picked";

/// The form a row is edited in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowForm {
    /// An expense the plan holds: its amount and phases.
    Set,
    /// An expense to add: its name, amount, and whether it is once.
    New,
    /// A category, to another expense.
    Move,
    /// A payee nothing names, to a category.
    Payee,
}

impl RowForm {
    /// The form's fields.
    #[must_use]
    pub const fn fields(self) -> &'static [FieldSpec] {
        match self {
            Self::Set => SET_FIELDS,
            Self::New => NEW_FIELDS,
            Self::Move => MOVE_FIELDS,
            Self::Payee => PAYEE_FIELDS,
        }
    }

    /// What the form is titled.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Set | Self::New => "Expense",
            Self::Move => "Category",
            Self::Payee => "Payee",
        }
    }
}

/// An expense the plan holds.
pub const SET_FIELDS: &[FieldSpec] = &[
    FieldSpec::money("amount", "Amount").help("A year, in today's dollars."),
    FieldSpec::choice("phases", "Phases", Vocabulary::Phases)
        .help("Which of the expense's amounts are set, where it states phases."),
];

/// An expense to add.
pub const NEW_FIELDS: &[FieldSpec] = &[
    FieldSpec::text("name", "Name").help("What the new expense is called."),
    FieldSpec::money("amount", "Amount").help("A year, in today's dollars."),
    FieldSpec::flag("one_time", "Once")
        .help("The window's total on its last date, in place of a yearly amount."),
];

const MOVE_FIELDS: &[FieldSpec] = &[FieldSpec::refers("expense", "Goes to", RefSource::Expense)
    .blank(PLACED)
    .help("The expense its spending goes to, kept in the plan.")];

/// A payee nothing names.
pub const PAYEE_FIELDS: &[FieldSpec] =
    &[
        FieldSpec::choice("category", "Category", Vocabulary::Category)
            .blank(UNKNOWN)
            .help("What the payee's rows were for; every row of it is sorted so."),
    ];

fn is_large(table: &Table) -> bool {
    held_word(table.get(LEAN)).is_none_or(|lean| lean == Lean::Large.slug())
}

/// The plan's `[estimate]` table.
pub const SETTINGS_FIELDS: &[FieldSpec] = &[
    FieldSpec::choice(LEAN, "Lean", Vocabulary::Lean)
        .help("How readily a category with no expense of its own gets one."),
    FieldSpec::share("large_share", "Large share")
        .shown_when(is_large)
        .help("The share of the year's spending a category needs for its own."),
    FieldSpec::refers("bucket", "Bucket", RefSource::Expense)
        .blank(DETECTED)
        .help("The expense what fits nowhere else goes to."),
];

/// An expense the plan holds, as its form holds it.
#[derive(Serialize, Deserialize)]
pub struct SetAnswers {
    amount: Dollars,
    phases: Phases,
}

impl ToolAnswers for SetAnswers {
    const SLOT: &'static str = "estimate-set";
}

/// An expense to add, as its form holds it.
#[derive(Serialize, Deserialize)]
pub struct NewAnswers {
    name: String,
    amount: Dollars,
    one_time: bool,
}

impl ToolAnswers for NewAnswers {
    const SLOT: &'static str = "estimate-new";
}

/// Where a category goes, as its form holds it.
#[derive(Serialize, Deserialize)]
pub struct MoveAnswers {
    #[serde(skip_serializing_if = "Option::is_none")]
    expense: Option<String>,
}

impl ToolAnswers for MoveAnswers {
    const SLOT: &'static str = "estimate-move";
}

/// What a payee is for, as its form holds it.
#[derive(Serialize, Deserialize)]
pub struct PayeeAnswers {
    category: Option<Category>,
}

impl ToolAnswers for PayeeAnswers {
    const SLOT: &'static str = "estimate-payee";
}

/// The plan's `[estimate]` table, as its form holds it.
#[derive(Serialize, Deserialize)]
pub struct SettingsAnswers {
    lean: Lean,
    #[serde(skip_serializing_if = "Option::is_none")]
    large_share: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bucket: Option<String>,
}

impl ToolAnswers for SettingsAnswers {
    const SLOT: &'static str = "estimate-settings";
}

/// The form `row` is edited in, and the answers it opens on.
pub(super) fn opened(row: Listed<'_>) -> Option<(RowForm, Table)> {
    Some(match row {
        Listed::Expense(proposal) if proposal.expense.is_some() => {
            let answers = SetAnswers {
                amount: proposal.amount,
                phases: proposal.phases,
            };
            (RowForm::Set, to_table(&answers))
        }
        Listed::Expense(proposal) => {
            let answers = NewAnswers {
                name: proposal.name.clone(),
                amount: proposal.amount,
                one_time: proposal.one_time,
            };
            (RowForm::New, to_table(&answers))
        }
        Listed::Category { into, .. } => {
            let answers = MoveAnswers {
                expense: into.expense.clone(),
            };
            (RowForm::Move, to_table(&answers))
        }
        Listed::Payee(_) => (RowForm::Payee, Table::new()),
        Listed::Unnamed { .. } => return None,
    })
}

/// What applying `row`'s form, its answers in `draft`, comes to.
pub(super) fn applied(row: Listed<'_>, draft: &Draft) -> Result<Applied, String> {
    match row {
        Listed::Expense(_) => Ok(Applied::Held),
        Listed::Category { category, into, .. } => {
            let answers: MoveAnswers = from_table(draft.answers::<MoveAnswers>())?;
            let expense = answers.expense.filter(|id| !id.is_empty());
            if expense == into.expense {
                return Ok(Applied::Held);
            }
            Ok(Applied::Moved { category, expense })
        }
        Listed::Payee(unknown) => {
            let answers: PayeeAnswers = from_table(draft.answers::<PayeeAnswers>())?;
            Ok(answers
                .category
                .map_or(Applied::Held, |category| Applied::Named {
                    payee: unknown.payee.clone(),
                    category,
                }))
        }
        Listed::Unnamed { .. } => Err(NOTHING_PICKED.to_owned()),
    }
}

/// Writes the expense form's answers in `draft` into `proposal`.
pub(super) fn hold(proposal: &mut Proposal, draft: &Draft) -> Result<(), String> {
    if proposal.expense.is_some() {
        let answers: SetAnswers = from_table(draft.answers::<SetAnswers>())?;
        proposal.amount = answers.amount;
        proposal.phases = answers.phases;
    } else {
        let answers: NewAnswers = from_table(draft.answers::<NewAnswers>())?;
        proposal.name = answers.name;
        proposal.amount = answers.amount;
        proposal.one_time = answers.one_time;
    }
    Ok(())
}

/// Sends `category` to `expense` in `plan`'s `[estimate]` table, or back
/// to where the estimate places it.
pub fn assign(plan: &mut Plan, category: Category, expense: Option<String>) {
    let mut settings = plan.estimate.take().unwrap_or_default();
    settings.assign.retain(|it| it.category != category);
    if let Some(expense) = expense {
        settings.assign.push(Assign { category, expense });
    }
    plan.estimate = (settings != EstimateSettings::default()).then_some(settings);
}

/// `plan`'s `[estimate]` table as its form holds it.
#[must_use]
pub fn settings_table(plan: &Plan) -> Table {
    let settings = plan.estimate.clone().unwrap_or_default();
    to_table(&SettingsAnswers {
        lean: settings.lean,
        large_share: settings.large_share,
        bucket: settings.bucket,
    })
}

/// Writes the settings form's `table` into `plan`'s `[estimate]`, keeping
/// its assignments.
///
/// # Errors
///
/// The schema's complaint about a value.
pub fn take_settings(plan: &mut Plan, table: Table) -> Result<(), String> {
    let answers: SettingsAnswers = from_table(table)?;
    let mut settings = plan.estimate.take().unwrap_or_default();
    settings.lean = answers.lean;
    settings.large_share = answers.large_share;
    settings.bucket = answers.bucket.filter(|id| !id.is_empty());
    plan.estimate = (settings != EstimateSettings::default()).then_some(settings);
    Ok(())
}

/// `plan`'s `[estimate]` table read out, each label beside what it says.
#[must_use]
pub fn settings_read(plan: &Plan) -> Vec<(String, String)> {
    let settings = plan.estimate.clone().unwrap_or_default();
    let lean = LEANS
        .iter()
        .find(|(slug, _)| *slug == settings.lean.slug())
        .map_or(settings.lean.slug(), |(_, words)| words);
    let mut rows = vec![("Lean".to_owned(), lean.to_owned())];
    if settings.lean == Lean::Large {
        rows.push(("Large share".to_owned(), rate(settings.large_share())));
    }
    let bucket = match (estimate::bucket(plan), &settings.bucket) {
        (Some(expense), Some(_)) => expense.display_name().to_owned(),
        (Some(expense), None) => format!("{}, detected", expense.display_name()),
        (None, _) => "a new Other expense".to_owned(),
    };
    rows.push(("Bucket".to_owned(), bucket));
    rows
}

/// The user's patterns file `text`, none where it does not exist yet, with
/// `payee` named for `category` after what it holds.
///
/// # Errors
///
/// Where the file does not read as patterns once it is added to.
pub fn with_payee(text: Option<&str>, payee: &str, category: Category) -> Result<String, String> {
    let mut file = text.unwrap_or_default().to_owned();
    if !file.is_empty() && !file.ends_with('\n') {
        file.push('\n');
    }
    let quoted = Value::String(payee.to_owned());
    let _ = write!(
        file,
        "\n[[patterns]]\nmatch = {quoted}\ncategory = \"{}\"\n",
        category.slug()
    );
    retiretui_engine::estimate::Patterns::default()
        .add(&file)
        .map_err(|error| format!("the patterns file does not read: {error}"))?;
    Ok(file)
}
