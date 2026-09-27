//! A first plan from a household's basics, or from one of the examples:
//! the new-plan form's answers, its fields, and the plan they make.

pub mod examples;
pub mod generate;
#[cfg(test)]
mod generate_tests;

pub use examples::EXAMPLES;

use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, FilingStatus, Plan};
use serde::Deserialize;
use toml::Table;

use crate::codec::from_table;
use crate::forms::offers::Vocabulary;
use crate::forms::{FieldSpec, ToolAnswers};

/// Where the household is in life. It decides only what the generated
/// plan is pre-filled with; every question is asked of everyone.
#[derive(Clone, Copy, PartialEq, Eq, Default, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum LifeStage {
    /// Earning, and retiring at an age still ahead.
    #[default]
    Working,
    /// Retired already: no salary, and benefits from the first year.
    Retired,
}

impl LifeStage {
    /// Every stage, in the order the form offers them.
    pub const ALL: &'static [Self] = &[Self::Working, Self::Retired];

    /// The stage as the form spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Retired => "retired",
        }
    }

    /// The stage as the form shows it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Working => "Working",
            Self::Retired => "Retired",
        }
    }
}

/// The answers as the form holds them. Every one is optional because a
/// form is answered a field at a time, and the generator fills what was
/// left blank.
#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct SetupAnswers {
    example: Option<String>,
    filing: Option<FilingStatus>,
    stage: Option<LifeStage>,
    name: Option<String>,
    birth_year: Option<i16>,
    retirement_age: Option<u8>,
    working_since: Option<i16>,
    salary: Option<Dollars>,
    social_security: Option<Dollars>,
    claim_age: Option<u8>,
    partner_name: Option<String>,
    partner_birth_year: Option<i16>,
    partner_retirement_age: Option<u8>,
    partner_working_since: Option<i16>,
    partner_salary: Option<Dollars>,
    partner_social_security: Option<Dollars>,
    partner_claim_age: Option<u8>,
}

/// One person's answers, so the two are asked and read the same way.
struct Answered<'a> {
    name: Option<&'a str>,
    birth_year: Option<i16>,
    retirement_age: Option<u8>,
    working_since: Option<i16>,
    salary: Option<Dollars>,
    social_security: Option<Dollars>,
    claim_age: Option<u8>,
}

impl SetupAnswers {
    fn filing(&self) -> FilingStatus {
        self.filing.unwrap_or(FilingStatus::Single)
    }

    fn stage(&self) -> LifeStage {
        self.stage.unwrap_or_default()
    }

    fn first(&self) -> Answered<'_> {
        Answered {
            name: self.name.as_deref(),
            birth_year: self.birth_year,
            retirement_age: self.retirement_age,
            working_since: self.working_since,
            salary: self.salary,
            social_security: self.social_security,
            claim_age: self.claim_age,
        }
    }

    fn partner(&self) -> Answered<'_> {
        Answered {
            name: self.partner_name.as_deref(),
            birth_year: self.partner_birth_year,
            retirement_age: self.partner_retirement_age,
            working_since: self.partner_working_since,
            salary: self.partner_salary,
            social_security: self.partner_social_security,
            claim_age: self.partner_claim_age,
        }
    }
}

/// Whether the household is described by the answers rather than taken
/// from an example, which is when there is anything to answer.
fn is_answered(answers: &Table) -> bool {
    !answers.contains_key("example")
}

/// The new-plan form's fields.
pub const FIELDS: &[FieldSpec] = &[
    FieldSpec::choice("example", "Start from", Vocabulary::Example)
        .blank("My own answers")
        .help("An example household's plan to start from, or blank to answer for your own."),
    FieldSpec::choice("filing", "Filing status", Vocabulary::FilingStatus)
        .help("How you file your federal return. Married filing jointly adds a partner.")
        .shown_when(is_answered),
    FieldSpec::choice("stage", "Life stage", Vocabulary::LifeStage)
        .help("Whether you are still earning or already retired.")
        .shown_when(is_answered),
    FieldSpec::text("name", "Your name")
        .help("A first name is enough; it labels what is yours.")
        .shown_when(is_answered),
    FieldSpec::whole("birth_year", "Birth year")
        .help("The year you were born, such as 1975.")
        .shown_when(is_answered),
    FieldSpec::whole("retirement_age", "Retirement age")
        .help("The age you stop working, or stopped. Your salary ends that year.")
        .shown_when(is_answered),
    FieldSpec::whole("working_since", "Working since")
        .help("The year you started working. Blank means the year you turned 22.")
        .shown_when(is_answered),
    FieldSpec::money("salary", "Salary")
        .help("What you earn per year before tax, or last earned, in today's dollars.")
        .shown_when(is_answered),
    FieldSpec::money("social_security", "Social Security")
        .help("Your yearly benefit at the age you claim, from your SSA statement. Blank computes it from your salary.")
        .shown_when(is_answered),
    FieldSpec::whole("claim_age", "Claim age")
        .help("The age you start Social Security, from 62 to 70. Blank means 67.")
        .shown_when(is_answered),
    FieldSpec::text("partner_name", "Partner's name")
        .help("Your partner's first name.")
        .shown_when(is_answered),
    FieldSpec::whole("partner_birth_year", "Partner's birth year")
        .help("The year your partner was born.")
        .shown_when(is_answered),
    FieldSpec::whole("partner_retirement_age", "Partner's retirement age")
        .help("The age your partner stops working, or stopped. Blank means the same age as you.")
        .shown_when(is_answered),
    FieldSpec::whole("partner_working_since", "Partner's working since")
        .help("The year your partner started working. Blank means the year they turned 22.")
        .shown_when(is_answered),
    FieldSpec::money("partner_salary", "Partner's salary")
        .help("What your partner earns per year before tax, or last earned, in today's dollars.")
        .shown_when(is_answered),
    FieldSpec::money("partner_social_security", "Partner's Social Security")
        .help("Your partner's yearly benefit at the age they claim. Blank computes it from their salary.")
        .shown_when(is_answered),
    FieldSpec::whole("partner_claim_age", "Partner's claim age")
        .help("The age your partner starts Social Security. Blank means 67.")
        .shown_when(is_answered),
];

impl ToolAnswers for SetupAnswers {
    const SLOT: &'static str = "new-plan";
}

/// The plan the new-plan form's `answers` make, starting `start_year`: the
/// example they name, beside the name it is offered under, or one built
/// from the household they describe.
///
/// # Errors
///
/// Where the answers do not read, or the plan they make does not.
pub fn compose(
    answers: Table,
    start_year: i16,
    tables: &TaxTables,
) -> Result<(Plan, Option<&'static str>), String> {
    let answers = from_table::<SetupAnswers>(answers)?;
    match answers.example.as_deref().and_then(examples::named) {
        Some(&(file, _, text)) => Plan::from_toml_str(text)
            .map(|plan| (plan, Some(file)))
            .map_err(|error| format!("{file}: {error}")),
        None => generate::plan(&answers, start_year, tables).map(|plan| (plan, None)),
    }
}

/// The account a household starts with, which is also where unspent
/// income sweeps.
pub(crate) const CASH_ID: &str = "cash";

/// The age every starting plan runs to.
pub(crate) const HORIZON_AGE: u8 = 95;

/// The inflation every starting plan assumes.
pub(crate) const INFLATION: f64 = 0.025;

/// The smallest plan that validates: one person and the cash account
/// surplus lands in, starting `start_year`.
#[must_use]
pub(crate) fn blank_plan(start_year: i16) -> String {
    format!(
        r#"schema = 1

[plan]
start_year = {start_year}
horizon_age = {HORIZON_AGE}
inflation = {INFLATION}

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1970-01-01

[[accounts]]
id = "{CASH_ID}"
kind = "cash"
owner = "me"
balance = 0
"#
    )
}
