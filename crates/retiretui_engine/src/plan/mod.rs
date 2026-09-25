//! The plan document: schema types, TOML load/save, and validation.

mod accounts;
mod allocation;
mod cliffs;
mod contributions;
mod dates;
mod diff;
mod escalation;
mod events;
mod expenses;
mod flows;
mod households;
mod incomes;
mod market;

pub(crate) use market::{INFLATION, VARIABLES, cholesky};
mod medicare;
mod places;
mod references;
mod residency;
mod scenario;
mod span;
mod triggers;
mod validate;

pub use accounts::{Account, AccountKind, TreatmentClass};
pub use allocation::{Allocation, AssetClass, ClassReturns, Mix, MixPhase};
pub use cliffs::Cliff;
pub use contributions::{Contribution, Match, Payer, Step};
pub use dates::PlanDate;
pub use diff::{Change, ChangeKind, diff};
pub use escalation::ColaSpec;
pub use events::Event;
pub use expenses::Expense;
pub use flows::{Conversion, Transfer};
pub use households::{FilingStatus, Household, Person, Residency};
pub use incomes::{Income, IncomeKind};
pub use market::{
    ClassAssumption, Correlation, Correlations, Draw, HistoricalSettings, InflationAssumption,
    Market, MonteCarloSettings,
};
pub use medicare::Medicare;
pub use places::{COUNTRIES, Place, US, US_STATES, place_name};
pub use scenario::{ID_KEY, Scenario, ScenarioError};
pub(crate) use span::{Node, Span};
pub use triggers::{Operand, Trigger, TriggerBasis, TriggerForm};
pub use validate::Issue;
pub(crate) use validate::push_issue;

use serde::{Deserialize, Serialize};

/// The plan file schema version this build reads and writes.
pub const SCHEMA_VERSION: u32 = 1;

/// Money in whole dollars.
pub type Dollars = i64;

/// Errors from reading or writing a plan document.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    /// The TOML failed to parse or did not match the schema.
    #[error(transparent)]
    Parse(#[from] toml::de::Error),
    /// The plan could not be serialized to TOML.
    #[error(transparent)]
    Serialize(#[from] toml::ser::Error),
}

/// Plan-wide settings under `[plan]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// First projected calendar year.
    pub start_year: i16,
    /// The projection runs until the oldest person reaches this age.
    pub horizon_age: u8,
    /// Annual inflation rate.
    pub inflation: f64,
    /// Annual growth of the national average wage past the last year the
    /// tax table's index carries, which a computed Social Security benefit
    /// is indexed over; the table's own assumption when unstated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wage_growth: Option<f64>,
    /// Treatment classes drained in order to cover shortfalls.
    #[serde(default = "default_withdrawal_order")]
    pub withdrawal_order: Vec<TreatmentClass>,
    /// Account receiving unspent income; defaults to the first cash account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surplus_to: Option<String>,
}

fn default_withdrawal_order() -> Vec<TreatmentClass> {
    vec![
        TreatmentClass::Taxable,
        TreatmentClass::Deferred,
        TreatmentClass::Roth,
        TreatmentClass::Hsa,
    ]
}

/// A complete plan document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// Schema version of the file.
    pub schema: u32,
    /// Plan-wide settings.
    pub plan: Settings,
    /// The household.
    pub household: Household,
    /// Opt-in Medicare surcharge modeling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medicare: Option<Medicare>,
    /// Named milestones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<Event>,
    /// Fund buckets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<Account>,
    /// Income sources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub income: Vec<Income>,
    /// Spending items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expenses: Vec<Expense>,
    /// MAGI-triggered costs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cliffs: Vec<Cliff>,
    /// One-time account-to-account moves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transfers: Vec<Transfer>,
    /// Roth conversion schedules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conversions: Vec<Conversion>,
    /// Contributions into accounts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributions: Vec<Contribution>,
    /// Residency periods; carried for later milestones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub residency: Vec<Residency>,
    /// What the market is assumed to do, and how the market tools run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<Market>,
}

impl Plan {
    /// The plan's `[market]`, empty where it states none, so every figure
    /// reads as the built-in default.
    #[must_use]
    pub fn market(&self) -> &Market {
        static NONE: Market = Market::NONE;
        self.market.as_ref().unwrap_or(&NONE)
    }

    /// Parses a plan from TOML text.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError::Parse`] when the text is not valid TOML or does
    /// not match the schema; semantic problems are reported by
    /// [`Plan::validate`] instead.
    pub fn from_toml_str(text: &str) -> Result<Self, PlanError> {
        Ok(toml::from_str(text)?)
    }

    /// Builds a plan from a parsed TOML table, such as a scenario merge
    /// result.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError`] when the table does not match the schema or
    /// cannot be serialized.
    pub fn from_toml_table(table: toml::Table) -> Result<Self, PlanError> {
        // `toml::Value`'s in-memory Deserializer stringifies datetimes, so a
        // direct `try_into` loses dates; the text round-trip keeps them.
        Self::from_toml_str(&toml::to_string(&table)?)
    }

    /// Serializes the plan to canonical TOML. The app owns the format:
    /// comments and layout of the source file are not preserved.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError::Serialize`] when serialization fails.
    pub fn to_toml_string(&self) -> Result<String, PlanError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Checks the plan's semantic rules and returns every problem found;
    /// an empty list means the plan is valid.
    #[must_use]
    pub fn validate(&self) -> Vec<Issue> {
        validate::validate(self)
    }

    /// The person with the given id, if any.
    #[must_use]
    pub fn person(&self, id: &str) -> Option<&Person> {
        self.household.people.iter().find(|person| person.id == id)
    }

    /// What the person with `id` is shown as: their name, or `id` itself.
    #[must_use]
    pub fn person_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.person(id).map_or(id, Person::display_name)
    }

    /// The account with the given id, if any.
    #[must_use]
    pub fn account(&self, id: &str) -> Option<&Account> {
        self.accounts.iter().find(|account| account.id == id)
    }

    /// The account unspent income is swept to: the one the plan names, or
    /// its first cash account.
    #[must_use]
    pub fn surplus_account(&self) -> Option<&str> {
        let swept = match &self.plan.surplus_to {
            Some(id) => self.account(id),
            None => self
                .accounts
                .iter()
                .find(|account| account.kind == AccountKind::Cash),
        };
        swept.map(|account| account.id.as_str())
    }

    /// The income source with the given id, if any.
    #[must_use]
    pub fn income_source(&self, id: &str) -> Option<&Income> {
        self.income.iter().find(|income| income.id == id)
    }
}
