//! The forms the estimate is edited in: an expense the plan holds and one
//! to add, a category moved to another expense, a payee named, merged or
//! ignored, and the plan's `[estimate]` table.

use retiretui_engine::estimate::{Category, Phases, Proposal};
use retiretui_engine::plan::{Assign, Dollars, EstimateSettings, Lean, Plan};
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::{Applied, Change, Listed, To, UNKNOWN};
use crate::codec::{from_table, to_table};
use crate::draft::Draft;
use crate::forms::offers::{BUCKET_PAYEE, IGNORE_PAYEE, RefSource, Vocabulary, held_word};
use crate::forms::{FieldSpec, Form, Target, ToolAnswers};

/// What a category's expense field reads as while the estimate places it.
const PLACED: &str = "Where the estimate places it";
/// What the bucket field reads as while the plan names none.
const DETECTED: &str = "Detected by its name";
const LEAN: &str = "lean";
const ESTIMATE: &str = "estimate";
/// What a payee's name field reads as while it keeps its bank's text.
const AS_WRITTEN: &str = "As the bank writes it";

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
    /// The form, its answers held beside the plan while it is open.
    #[must_use]
    pub const fn form(self) -> Form {
        match self {
            Self::Set => Form::tool::<SetAnswers>(EXPENSE_TITLE, SET_FIELDS),
            Self::New => Form::tool::<NewAnswers>(EXPENSE_TITLE, NEW_FIELDS),
            Self::Move => Form::tool::<MoveAnswers>("Category", MOVE_FIELDS),
            Self::Payee => Form::tool::<PayeeAnswers>("Payee", PAYEE_FIELDS),
        }
    }
}

const EXPENSE_TITLE: &str = "Expense";

const SET_FIELDS: &[FieldSpec] = &[
    FieldSpec::money("amount", "Amount").help("A year, in today's dollars."),
    FieldSpec::choice("phases", "Phases", Vocabulary::Phases)
        .help("Which of the expense's amounts are set, where it states phases."),
];

const NEW_FIELDS: &[FieldSpec] = &[
    FieldSpec::text("name", "Name").help("What the new expense is called."),
    FieldSpec::money("amount", "Amount").help("A year, in today's dollars."),
    FieldSpec::flag("one_time", "Once")
        .help("The amount once, on the window's last date, in place of a year of it."),
];

const MOVE_FIELDS: &[FieldSpec] = &[FieldSpec::refers("expense", "Goes to", RefSource::Expense)
    .blank(PLACED)
    .help("The expense its spending goes to, kept in the plan.")];

const PAYEE_FIELDS: &[FieldSpec] = &[
    FieldSpec::text("name", "Name")
        .blank(AS_WRITTEN)
        .help("What the payee is listed as; payees given one name are one."),
    FieldSpec::choice("category", "Category", Vocabulary::PayeeOutcome)
        .blank(UNKNOWN)
        .help("What the payee's rows were for, or Ignored to count none of them."),
];

fn is_large(table: &Table) -> bool {
    held_word(table.get(LEAN)).is_none_or(|lean| lean == Lean::Large.slug())
}

const SETTINGS_FIELDS: &[FieldSpec] = &[
    FieldSpec::choice(LEAN, "Lean", Vocabulary::Lean)
        .help("How readily a category with no expense of its own gets one."),
    FieldSpec::share("large_share", "Large share")
        .shown_when(is_large)
        .defaults_to("10%")
        .help("The share of the year's spending a category needs for its own."),
    FieldSpec::refers("bucket", "Bucket", RefSource::Expense)
        .blank(DETECTED)
        .help("The expense what fits nowhere else goes to."),
];

/// The plan's `[estimate]` table: applying it is a step of the draft's
/// history, its assignments kept as they are.
pub const SETTINGS_FORM: Form = Form {
    domain: None,
    title: "Settings",
    target: Target::Draft,
    paths: &[ESTIMATE],
    fields: SETTINGS_FIELDS,
    item: settings_item,
    store: settings_store,
    typed: settings_typed,
    list: None,
};

fn settings_item(draft: &Draft, index: usize) -> Option<Table> {
    let mut settings = draft.plan.estimate.clone().unwrap_or_default();
    settings.assign.clear();
    (index == 0).then(|| to_table(&settings))
}

fn settings_store(draft: &mut Draft, _: usize, table: Table) -> Result<(), String> {
    let mut settings: EstimateSettings = from_table(table)?;
    settings.bucket = settings.bucket.filter(|id| !id.is_empty());
    settings.assign = draft
        .plan
        .estimate
        .take()
        .map(|it| it.assign)
        .unwrap_or_default();
    draft.plan.estimate = (settings != EstimateSettings::default()).then_some(settings);
    Ok(())
}

fn settings_typed(table: Table) -> Option<Table> {
    from_table::<EstimateSettings>(table)
        .ok()
        .map(|it| to_table(&it))
}

#[derive(Serialize, Deserialize)]
struct SetAnswers {
    amount: Dollars,
    phases: Phases,
}

impl ToolAnswers for SetAnswers {
    const SLOT: &'static str = "estimate-set";
}

#[derive(Serialize, Deserialize)]
struct NewAnswers {
    name: String,
    amount: Dollars,
    one_time: bool,
}

impl ToolAnswers for NewAnswers {
    const SLOT: &'static str = "estimate-new";
}

#[derive(Serialize, Deserialize)]
struct MoveAnswers {
    #[serde(skip_serializing_if = "Option::is_none")]
    expense: Option<String>,
}

impl ToolAnswers for MoveAnswers {
    const SLOT: &'static str = "estimate-move";
}

#[derive(Serialize, Deserialize)]
struct PayeeAnswers {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// A category's slug, or [`IGNORE_PAYEE`].
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
}

impl ToolAnswers for PayeeAnswers {
    const SLOT: &'static str = "estimate-payee";
}

/// The plan expense a proposal sets, none where it adds one.
fn stated(proposal: &Proposal) -> Option<String> {
    (!proposal.is_new).then(|| proposal.id.clone())
}

/// The form `row` is edited in, and the answers it opens on.
pub(super) fn opened(row: Listed<'_>) -> Option<(RowForm, Table)> {
    Some(match row {
        Listed::Expense(proposal) if !proposal.is_new => {
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
                expense: stated(into),
            };
            (RowForm::Move, to_table(&answers))
        }
        Listed::Payee(_) => (RowForm::Payee, Table::new()),
        Listed::Unnamed { .. } | Listed::Ignored { .. } => return None,
    })
}

/// What applying the form of `row`, other than an expense's, comes to with
/// its answers in `draft`.
pub(super) fn applied(row: Listed<'_>, draft: &Draft) -> Result<Applied, String> {
    Ok(match row {
        Listed::Category { gathered, into } => {
            let answers: MoveAnswers = from_table(draft.answers::<MoveAnswers>())?;
            let expense = answers.expense.filter(|id| !id.is_empty());
            if expense == stated(into) {
                Applied::Held
            } else {
                Applied::Moved {
                    category: gathered.category,
                    expense,
                }
            }
        }
        Listed::Payee(unknown) => {
            let answers: PayeeAnswers = from_table(draft.answers::<PayeeAnswers>())?;
            payee_changes(&unknown.payee, answers)?
        }
        Listed::Expense(_) | Listed::Unnamed { .. } | Listed::Ignored { .. } => Applied::Held,
    })
}

/// The changes a payee's answers make: merged under its name, then named
/// for a category or ignored, by the name it is then listed under.
fn payee_changes(payee: &str, answers: PayeeAnswers) -> Result<Applied, String> {
    let name = (answers.name.as_deref().map(str::trim)).filter(|name| !name.is_empty());
    let mut changes = Vec::new();
    if let Some(name) = name.filter(|name| *name != payee) {
        changes.push(Change::Merge {
            payee: payee.to_owned(),
            name: name.to_owned(),
        });
    }
    let listed = name.unwrap_or(payee).to_owned();
    match answers.category.as_deref() {
        None | Some("") => {}
        Some(IGNORE_PAYEE) => changes.push(Change::IgnorePayee { payee: listed }),
        Some(BUCKET_PAYEE) => changes.push(Change::Name {
            payee: listed,
            to: To::Bucket,
        }),
        Some(slug) => changes.push(Change::Name {
            payee: listed,
            to: To::Category(
                Value::String(slug.to_owned())
                    .try_into()
                    .map_err(|_| format!("`{slug}` is not a category"))?,
            ),
        }),
    }
    Ok(if changes.is_empty() {
        Applied::Held
    } else {
        Applied::Changed(changes)
    })
}

/// Writes the expense form's answers in `draft` into `proposal`.
pub(super) fn hold(proposal: &mut Proposal, draft: &Draft) -> Result<(), String> {
    if proposal.is_new {
        let answers: NewAnswers = from_table(draft.answers::<NewAnswers>())?;
        proposal.name = answers.name;
        proposal.amount = answers.amount;
        proposal.one_time = answers.one_time;
    } else {
        let answers: SetAnswers = from_table(draft.answers::<SetAnswers>())?;
        proposal.amount = answers.amount;
        proposal.phases = answers.phases;
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
