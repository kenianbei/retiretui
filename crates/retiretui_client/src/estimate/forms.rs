//! The forms the estimate is edited in: an expense the plan holds and one
//! to add, a category moved to another expense, a payee named, and the
//! plan's `[estimate]` table.

use std::fmt::Write;

use retiretui_engine::estimate::{Category, Patterns, Phases, Proposal};
use retiretui_engine::plan::{Assign, Dollars, EstimateSettings, Lean, Plan};
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::{Applied, Listed, UNKNOWN};
use crate::codec::{from_table, to_table};
use crate::draft::Draft;
use crate::forms::offers::{RefSource, Vocabulary, held_word};
use crate::forms::{FieldSpec, Form, Target, ToolAnswers};

/// What a category's expense field reads as while the estimate places it.
const PLACED: &str = "Where the estimate places it";
/// What the bucket field reads as while the plan names none.
const DETECTED: &str = "Detected by its name";
const LEAN: &str = "lean";
const ESTIMATE: &str = "estimate";

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
        .help("The window's total on its last date, in place of a yearly amount."),
];

const MOVE_FIELDS: &[FieldSpec] = &[FieldSpec::refers("expense", "Goes to", RefSource::Expense)
    .blank(PLACED)
    .help("The expense its spending goes to, kept in the plan.")];

const PAYEE_FIELDS: &[FieldSpec] =
    &[
        FieldSpec::choice("category", "Category", Vocabulary::Category)
            .blank(UNKNOWN)
            .help("What the payee's rows were for; every row of it is sorted so."),
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
    category: Option<Category>,
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
        Listed::Unnamed { .. } => return None,
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
            answers
                .category
                .map_or(Applied::Held, |category| Applied::Named {
                    payee: unknown.payee.clone(),
                    category,
                })
        }
        Listed::Expense(_) | Listed::Unnamed { .. } => Applied::Held,
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
    Patterns::default()
        .add(&file)
        .map_err(|error| format!("the patterns file does not read: {error}"))?;
    Ok(file)
}
