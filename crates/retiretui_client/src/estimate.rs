//! The spending estimate as every surface says it: the exports a session
//! holds, each proposal and unknown payee as a row and read out, the forms
//! a proposal and a payee are edited in, what is asked and said when the
//! proposals are taken, and the shape they are replied in.

use retiretui_engine::estimate::{
    BankError, Category, Estimate, Patterns, Phases, Proposal, Row, Unknown, apply, estimate,
    merge, read,
};
use retiretui_engine::plan::{self, Dollars, Item, Plan, Scenario};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toml::Table;

use crate::codec::{from_table, to_table};
use crate::forms::offers::{RefSource, Vocabulary, held_word};
use crate::forms::{FieldSpec, ToolAnswers};
use crate::present::{MoneyForm, counted};

/// What the tool is for, in a line.
pub const ABOUT: &str = "The plan's expenses from what a bank's exports say the household spent.";
/// What the table says before any export is read.
pub const NOTHING_READ: &str =
    "Add a bank's CSV, OFX or QFX export and the plan's spending is estimated from it.";
/// The columns the proposals are tabled under.
pub const COLUMNS: [&str; 4] = ["Category", "Expense", "Months", "Per year"];
/// The extensions an export may have.
pub const EXTENSIONS: [&str; 3] = ["csv", "ofx", "qfx"];
/// What a proposal's expense reads as where the plan has none for it.
pub const NEW_EXPENSE: &str = "New expense";
/// What a payee's category reads as while nothing names it.
pub const UNKNOWN: &str = "Unknown";
const NOTHING: &str = "-";

/// The exports a session holds, each by its name, read once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exports {
    files: Vec<(String, Vec<Row>)>,
}

impl Exports {
    /// Reads `text` as the export `name`, in place of one of the same name.
    ///
    /// # Errors
    ///
    /// As the file fails to read, by name.
    pub fn add(&mut self, name: &str, text: &str) -> Result<usize, BankError> {
        let rows = read(name, text)?;
        let count = rows.len();
        self.remove(name);
        self.files.push((name.to_owned(), rows));
        Ok(count)
    }

    /// Drops the export `name`.
    pub fn remove(&mut self, name: &str) {
        self.files.retain(|(held, _)| held != name);
    }

    /// Each export's name and how many rows it holds, in the order added.
    pub fn listed(&self) -> impl Iterator<Item = (&str, usize)> {
        self.files
            .iter()
            .map(|(name, rows)| (name.as_str(), rows.len()))
    }

    /// Whether no export is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The exports' rows, each file's merged over the ones before it.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for (_, file) in &self.files {
            merge(&mut rows, file.clone());
        }
        rows
    }

    /// What the exports say `plan` spends.
    #[must_use]
    pub fn estimate(&self, plan: &Plan, patterns: &Patterns) -> Estimate {
        estimate(plan, &self.rows(), patterns)
    }
}

/// One row of the table: a category proposed, or a payee nothing named.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Listed<'a> {
    /// A category's spending as an expense.
    Proposal(&'a Proposal),
    /// A payee the patterns did not name.
    Unknown(&'a Unknown),
}

/// The table's rows: the proposals, then the unknown payees.
#[must_use]
pub fn listed(estimate: &Estimate) -> Vec<Listed<'_>> {
    let proposals = estimate.proposals.iter().map(Listed::Proposal);
    proposals
        .chain(estimate.unknown.iter().map(Listed::Unknown))
        .collect()
}

impl Listed<'_> {
    /// The row under [`COLUMNS`].
    #[must_use]
    pub fn cells(&self, months: u8, form: MoneyForm) -> [String; 4] {
        match self {
            Self::Proposal(proposal) => [
                proposal.category.title(),
                proposal
                    .expense
                    .as_ref()
                    .map_or(NEW_EXPENSE.to_owned(), |_| proposal.name.clone()),
                months.to_string(),
                form.money(proposal.amount),
            ],
            Self::Unknown(unknown) => [
                UNKNOWN.to_owned(),
                unknown.payee.clone(),
                months.to_string(),
                form.money(unknown.amount),
            ],
        }
    }

    /// The row read out beside the table.
    #[must_use]
    pub fn read_out(&self, plan: &Plan, months: u8, form: MoneyForm) -> Vec<(String, String)> {
        let own = |label: &str, text: String| (label.to_owned(), text);
        match self {
            Self::Proposal(proposal) => {
                let stated = proposal
                    .expense
                    .as_deref()
                    .and_then(|id| plan.expenses.iter().find(|e| e.id == id));
                let mut rows = vec![
                    own("Category", proposal.category.title()),
                    own(
                        "Expense",
                        stated.map_or(NEW_EXPENSE.to_owned(), |e| e.display_name().to_owned()),
                    ),
                    own(
                        "Now",
                        stated.map_or(NOTHING.to_owned(), |e| form.money(e.amount)),
                    ),
                    own(
                        "Proposed",
                        format!("{} a year", form.money(proposal.amount)),
                    ),
                    own(
                        "From",
                        format!(
                            "{} over {}, {}",
                            counted(proposal.rows, "row", "rows"),
                            counted(months.into(), "month", "months"),
                            form.money(proposal.total)
                        ),
                    ),
                ];
                for (payee, amount) in &proposal.payees {
                    rows.push((format!("  {payee}"), form.money(*amount)));
                }
                rows
            }
            Self::Unknown(unknown) => vec![
                own("Payee", unknown.payee.clone()),
                own("Category", UNKNOWN.to_owned()),
                own("Spent", format!("{} a year", form.money(unknown.amount))),
                own("From", counted(unknown.rows, "row", "rows")),
            ],
        }
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
        through.0.strftime("%b %-Y"),
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

/// The overlay over `base` that makes `plan` into itself with the
/// proposals applied.
///
/// # Errors
///
/// Where the plan does not serialize.
pub fn overlay(base: &str, plan: &Plan, estimate: &Estimate) -> Result<String, String> {
    let mut applied = plan.clone();
    apply(&mut applied, estimate);
    let beneath = plan::to_table(plan).map_err(|e| e.to_string())?;
    let draft = plan::to_table(&applied).map_err(|e| e.to_string())?;
    let scenario = Scenario::over(base, &beneath, &draft, None).map_err(|issues| {
        issues
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    scenario.to_toml_string().map_err(|e| e.to_string())
}

const EXPENSE: &str = "expense";

fn is_new(table: &Table) -> bool {
    held_word(table.get(EXPENSE)).is_none()
}

fn is_set(table: &Table) -> bool {
    !is_new(table)
}

/// The proposal form's fields.
pub const PROPOSAL_FIELDS: &[FieldSpec] = &[
    FieldSpec::refers(EXPENSE, "Expense", RefSource::Expense)
        .blank(NEW_EXPENSE)
        .help("The expense the amount sets, or a new one for it."),
    FieldSpec::text("name", "Name")
        .shown_when(is_new)
        .help("What the new expense is called."),
    FieldSpec::money("amount", "Amount").help("A year, in today's dollars."),
    FieldSpec::flag("one_time", "Once")
        .shown_when(is_new)
        .help("The window's total on its last date, in place of a yearly amount."),
    FieldSpec::choice("phases", "Phases", Vocabulary::Phases)
        .shown_when(is_set)
        .help("Which of the expense's amounts are set, where it states phases."),
];

/// The unknown payee form's field.
pub const UNKNOWN_FIELDS: &[FieldSpec] =
    &[
        FieldSpec::choice("category", "Category", Vocabulary::Category)
            .blank(UNKNOWN)
            .help("What the payee's rows were for; every row of it is sorted so."),
    ];

/// The proposal form's answers, held beside the plan while it is edited.
#[derive(Serialize, Deserialize)]
pub struct ProposalAnswers {
    #[serde(skip_serializing_if = "Option::is_none")]
    expense: Option<String>,
    name: String,
    amount: Dollars,
    one_time: bool,
    phases: Phases,
}

impl ToolAnswers for ProposalAnswers {
    const SLOT: &'static str = "estimate-proposal";
}

/// The payee form's answer, held the same way.
#[derive(Serialize, Deserialize)]
pub struct UnknownAnswers {
    category: Option<Category>,
}

impl ToolAnswers for UnknownAnswers {
    const SLOT: &'static str = "estimate-payee";
}

/// The proposal as its form holds it.
#[must_use]
pub fn proposal_table(proposal: &Proposal) -> Table {
    to_table(&ProposalAnswers {
        expense: proposal.expense.clone(),
        name: proposal.name.clone(),
        amount: proposal.amount,
        one_time: proposal.one_time,
        phases: proposal.phases,
    })
}

/// Writes the form's `table` into `proposal`.
///
/// # Errors
///
/// The schema's complaint about a value.
pub fn proposal_from(table: Table, proposal: &mut Proposal) -> Result<(), String> {
    let answers: ProposalAnswers = from_table(table)?;
    proposal.expense = answers.expense.filter(|id| !id.is_empty());
    proposal.name = answers.name;
    proposal.amount = answers.amount;
    proposal.one_time = answers.one_time && proposal.expense.is_none();
    proposal.phases = answers.phases;
    Ok(())
}

/// The category the form's `table` assigns, if any.
///
/// # Errors
///
/// The schema's complaint about a value.
pub fn category_from(table: Table) -> Result<Option<Category>, String> {
    let answers: UnknownAnswers = from_table(table)?;
    Ok(answers.category)
}

/// The estimate as `estimate` and `estimate_spending` reply.
#[derive(Serialize, JsonSchema)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EstimateReply {
    /// The whole months the window covers, twelve at most.
    pub months: u8,
    /// The latest row's date, `YYYY-MM-DD`; none where no row was read.
    pub through: Option<String>,
    /// What came in over the window, annualized in the plan's dollars.
    pub income: Dollars,
    /// What moved without being spent, net, annualized.
    pub transfers: Dollars,
    /// One per spending category seen.
    pub proposals: Vec<Proposal>,
    /// Each payee nothing named, largest first.
    pub unknown: Vec<Unknown>,
    /// The scenario written with the proposals applied, where one was.
    pub written: Option<String>,
}

impl EstimateReply {
    /// The reply for `estimate`, written to `written` where it was.
    #[must_use]
    pub fn new(estimate: Estimate, written: Option<String>) -> Self {
        Self {
            months: estimate.months,
            through: estimate.through.map(|on| on.0.to_string()),
            income: estimate.income,
            transfers: estimate.transfers,
            proposals: estimate.proposals,
            unknown: estimate.unknown,
            written,
        }
    }
}

#[cfg(test)]
mod tests {
    use toml::Value;

    use super::*;
    use crate::setup::examples::named;

    const CSV: &str = "Date,Description,Amount\n2026-10-01,ROCKET MORTGAGE,-500.00\n2026-10-02,NETFLIX,-20.00\n2026-10-03,SOMEONE,-30.00\n";

    fn retired() -> Plan {
        let (_, _, text) = named("retired-couple.toml").expect("the example");
        Plan::from_toml_str(text).expect("the example parses")
    }

    #[test]
    fn the_exports_are_tabled_read_out_and_summed_up() {
        let plan = retired();
        let mut exports = Exports::default();
        assert!(exports.add("x.csv", CSV).is_ok());
        assert!(exports.add("x.csv", CSV).is_ok());
        assert_eq!(exports.listed().collect::<Vec<_>>(), [("x.csv", 3)]);
        let found = exports.estimate(&plan, &Patterns::embedded());
        let rows = listed(&found);
        let cells: Vec<_> = rows
            .iter()
            .map(|row| row.cells(found.months, MoneyForm::Full))
            .collect();
        assert_eq!(cells[0], ["Housing", "Housing", "1", "$6,000"]);
        assert_eq!(cells[1], ["Subscriptions", NEW_EXPENSE, "1", "$240"]);
        assert_eq!(cells[2], [UNKNOWN, "SOMEONE", "1", "$360"]);
        let read_out = rows[0].read_out(&plan, found.months, MoneyForm::Full);
        assert_eq!(
            read_out[3],
            ("Proposed".to_owned(), "$6,000 a year".to_owned())
        );
        assert_eq!(
            read_out[5],
            ("  ROCKET MORTGAGE".to_owned(), "$500".to_owned())
        );
        assert_eq!(
            summary(&found, MoneyForm::Full),
            "1 month through Oct 2026: $0 of income, $0 in transfers a year; 1 payee unnamed."
        );
        assert_eq!(
            take_question(&found),
            "Set 1 expense and add 1 from 1 month of spending?"
        );
        assert_eq!(
            taken(&found),
            "1 expense set and 1 added from 1 month of spending"
        );
        exports.remove("x.csv");
        assert!(exports.is_empty());
        assert_eq!(summary(&Estimate::default(), MoneyForm::Full), NOTHING_READ);
    }

    #[test]
    fn the_forms_edit_a_proposal_and_name_a_payee() {
        let plan = retired();
        let mut exports = Exports::default();
        exports.add("x.csv", CSV).unwrap();
        let mut found = exports.estimate(&plan, &Patterns::embedded());
        let mut table = proposal_table(&found.proposals[1]);
        assert!(is_new(&table));
        table.insert("name".to_owned(), Value::String("Streaming".to_owned()));
        table.insert("one_time".to_owned(), Value::Boolean(true));
        proposal_from(table, &mut found.proposals[1]).unwrap();
        assert_eq!(found.proposals[1].name, "Streaming");
        assert!(found.proposals[1].one_time);
        let mut table = proposal_table(&found.proposals[0]);
        assert!(is_set(&table));
        table.insert("phases".to_owned(), Value::String("all".to_owned()));
        proposal_from(table, &mut found.proposals[0]).unwrap();
        assert_eq!(found.proposals[0].phases, Phases::All);
        let mut answers = Table::new();
        answers.insert("category".to_owned(), Value::String("pets".to_owned()));
        assert_eq!(category_from(answers), Ok(Some(Category::Pets)));
        assert_eq!(category_from(Table::new()), Ok(None));
        let text = overlay("base.toml", &plan, &found).unwrap();
        assert!(text.contains("base = \"base.toml\""), "{text}");
        assert!(text.contains("id = \"subscriptions\""), "{text}");
        let reply = EstimateReply::new(found, None);
        assert_eq!(reply.through.as_deref(), Some("2026-10-03"));
    }
}
