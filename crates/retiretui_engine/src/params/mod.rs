//! Tax parameters: per-year TOML tables, embedded for known years, extended
//! past the last known year by inflating indexed values. Values are data;
//! rule shapes live in [`crate::tax`].

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::plan::{Dollars, FilingStatus};

mod index;

pub use index::Inflation;
use index::inflate;
pub(crate) use index::scale;

/// The tax parameter file schema version this build reads.
const PARAMS_SCHEMA_VERSION: u32 = 1;

const EMBEDDED: &[&str] = &[include_str!("../../tax/2026.toml")];

/// Errors from loading tax parameter files.
#[derive(Debug, thiserror::Error)]
pub enum ParamsError {
    /// A parameter file or directory could not be read.
    #[error("failed to read {path}: {source}")]
    Io {
        /// The offending path.
        path: String,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A parameter file failed to parse.
    #[error("failed to parse {path}: {source}")]
    Parse {
        /// The offending path.
        path: String,
        /// The underlying error.
        source: Box<toml::de::Error>,
    },
    /// A parameter file declares an unsupported schema version.
    #[error(
        "{path}: unsupported schema version {found} (this build reads {PARAMS_SCHEMA_VERSION})"
    )]
    Schema {
        /// The offending path.
        path: String,
        /// The version the file declares.
        found: u32,
    },
}

/// A value per filing status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PerStatus<T> {
    /// Single filer value.
    pub single: T,
    /// Married-filing-jointly value.
    pub married_joint: T,
}

impl<T> PerStatus<T> {
    /// The value for a filing status, by reference.
    #[must_use]
    pub fn for_status(&self, status: FilingStatus) -> &T {
        match status {
            FilingStatus::Single => &self.single,
            FilingStatus::MarriedJoint => &self.married_joint,
        }
    }
}

impl<T: Copy> PerStatus<T> {
    /// The value for a filing status.
    #[must_use]
    pub fn get(&self, status: FilingStatus) -> T {
        *self.for_status(status)
    }
}

/// One ordinary-income bracket: `rate` applies above `over`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bracket {
    /// Taxable income where this rate starts.
    pub over: Dollars,
    /// Marginal rate.
    pub rate: f64,
    /// Set where the law fixes `over` in nominal dollars, so that it is not
    /// inflated past the last known year.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unindexed: bool,
}

/// Standard deductions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deductions {
    /// Standard deduction per filing status.
    pub standard: PerStatus<Dollars>,
}

/// Long-term capital gains thresholds and rates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Ltcg {
    /// Rate for gains stacked above the zero band.
    pub middle_rate: f64,
    /// Rate for gains above `fifteen_until`.
    pub top_rate: f64,
    /// Taxable income up to which gains are untaxed.
    pub zero_until: PerStatus<Dollars>,
    /// Taxable income up to which gains take `middle_rate`.
    pub fifteen_until: PerStatus<Dollars>,
}

/// Social Security parameters: the statutory, unindexed provisional-income
/// thresholds, and what the benefit formula reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SocialSecurityThresholds {
    /// Below this, no benefit is taxable.
    pub provisional_base: PerStatus<Dollars>,
    /// Above this, up to 85% is taxable.
    pub provisional_upper: PerStatus<Dollars>,
    /// What computing a benefit from an earnings record needs; absent in
    /// an override file that predates it, and a derived benefit is then
    /// refused by validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benefit: Option<BenefitParams>,
}

/// What a benefit is computed from: the published average wage index and
/// the growth assumed past it. The wage base and the bend points of any
/// year derive from the index by statute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct BenefitParams {
    /// Annual growth of the average wage past the last year the index
    /// carries.
    pub wage_growth: f64,
    /// The national average wage index by year, as published.
    pub wage_index: BTreeMap<i16, f64>,
}

/// The 1994 contribution and benefit base; later bases scale it by the
/// average wage two years earlier over 1992's, to the nearest $300.
const WAGE_BASE_1994: Dollars = 60_600;
const WAGE_BASE_YEAR: i16 = 1994;
const WAGE_BASE_STEP: Dollars = 300;
/// The 1979 bend points; a later eligibility year scales them by the
/// average wage two years earlier over 1977's, to the dollar.
const BEND_POINTS_1979: [Dollars; 2] = [180, 1_085];
const BEND_POINTS_YEAR: i16 = 1979;
/// Years between an amount's year and the average wage it is set from.
const INDEX_LAG_YEARS: i16 = 2;

impl BenefitParams {
    /// The average wage of `year`: as published, or the last published
    /// grown at `wage_growth` a year; none before the index begins.
    #[must_use]
    pub fn average_wage(&self, year: i16) -> Option<f64> {
        if let Some(&wage) = self.wage_index.get(&year) {
            return Some(wage);
        }
        let (&last_year, &last) = self.wage_index.last_key_value()?;
        (year > last_year)
            .then(|| last * (1.0 + self.wage_growth).powi(i32::from(year - last_year)))
    }

    /// The contribution and benefit base of `year`, above which covered
    /// earnings count for nothing.
    #[must_use]
    pub fn wage_base(&self, year: i16) -> Dollars {
        let scaled = WAGE_BASE_1994 as f64 * self.index_ratio(year, WAGE_BASE_YEAR);
        (scaled / WAGE_BASE_STEP as f64).round() as Dollars * WAGE_BASE_STEP
    }

    /// The two monthly amounts the PIA formula bends at for a benefit first
    /// payable in `eligibility_year`.
    #[must_use]
    pub fn bend_points(&self, eligibility_year: i16) -> [Dollars; 2] {
        let ratio = self.index_ratio(eligibility_year, BEND_POINTS_YEAR);
        BEND_POINTS_1979.map(|point| (point as f64 * ratio).round() as Dollars)
    }

    /// The average wage of `year` over that of `over`; none where the index
    /// carries neither.
    #[must_use]
    pub fn wage_ratio(&self, year: i16, over: i16) -> Option<f64> {
        Some(self.average_wage(year)? / self.average_wage(over)?)
    }

    /// The average wage set for `year` over the one set for `base_year`;
    /// one where the index carries neither.
    fn index_ratio(&self, year: i16, base_year: i16) -> f64 {
        self.wage_ratio(year - INDEX_LAG_YEARS, base_year - INDEX_LAG_YEARS)
            .unwrap_or(1.0)
    }
}

/// Early-withdrawal parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EarlyWithdrawal {
    /// Penalty rate on early distributions.
    pub penalty: f64,
}

/// One row of the RMD Uniform Lifetime Table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RmdDivisor {
    /// Age reached during the distribution year.
    pub age: u8,
    /// The divisor applied to the prior year-end balance.
    pub divisor: f64,
}

/// The RMD table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RmdTable {
    /// Rows in ascending age order.
    pub divisors: Vec<RmdDivisor>,
}

/// A MAGI band over which something phases out: whole below `from`, gone
/// at `to`, linear between.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PhaseOut {
    /// MAGI at or below which nothing is lost.
    pub from: Dollars,
    /// MAGI at or above which all is lost.
    pub to: Dollars,
}

impl PhaseOut {
    /// How far into the band `magi` is, from 0 at its foot to 1 at its top.
    #[must_use]
    pub fn position(self, magi: Dollars) -> f64 {
        if magi <= self.from {
            return 0.0;
        }
        if magi >= self.to || self.to <= self.from {
            return 1.0;
        }
        (magi - self.from) as f64 / (self.to - self.from) as f64
    }
}

/// Annual contribution limits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContributionLimits {
    /// Elective deferral for 401(k)/403(b)/457(b).
    pub employer_plan: Dollars,
    /// Additional deferral from age 50.
    pub employer_plan_catch_up_50: Dollars,
    /// Replacement catch-up at ages 60-63.
    pub employer_plan_catch_up_60: Dollars,
    /// Everything paid into one defined-contribution plan in a year, by
    /// employee and employer together (415(c)).
    pub overall_plan: Dollars,
    /// SIMPLE IRA deferral.
    pub simple: Dollars,
    /// SIMPLE additional deferral from age 50.
    pub simple_catch_up_50: Dollars,
    /// SIMPLE replacement catch-up at ages 60-63.
    pub simple_catch_up_60: Dollars,
    /// IRA contribution (traditional plus Roth combined).
    pub ira: Dollars,
    /// IRA additional contribution from age 50.
    pub ira_catch_up_50: Dollars,
    /// HSA limit with self-only coverage.
    pub hsa_self: Dollars,
    /// HSA limit with family coverage.
    pub hsa_family: Dollars,
    /// HSA additional contribution from age 55.
    pub hsa_catch_up_55: Dollars,
    /// The MAGI band over which a Roth IRA contribution is no longer
    /// allowed.
    pub roth_ira_phase_out: PerStatus<PhaseOut>,
    /// The MAGI band over which a traditional IRA contribution stops being
    /// deductible for a person a workplace plan covers.
    pub ira_deduction_phase_out: PerStatus<PhaseOut>,
}

/// One IRMAA tier: the MAGI threshold it starts above and the annual
/// surcharges per covered person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct IrmaaTier {
    /// The tier applies when the lookback MAGI exceeds this.
    pub magi_over: PerStatus<Dollars>,
    /// Annual Part B surcharge per covered person.
    pub part_b: Dollars,
    /// Annual Part D surcharge per covered person.
    pub part_d: Dollars,
}

/// Every tax parameter for one year.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct TaxParams {
    /// Parameter file schema version.
    pub schema: u32,
    /// The tax year these values apply to.
    pub year: i16,
    /// Deductions.
    pub deductions: Deductions,
    /// Ordinary-income brackets per filing status.
    pub brackets: PerStatus<Vec<Bracket>>,
    /// Long-term capital gains parameters.
    pub ltcg: Ltcg,
    /// Social Security taxation thresholds.
    pub social_security: SocialSecurityThresholds,
    /// Early-withdrawal parameters.
    pub early_withdrawal: EarlyWithdrawal,
    /// Contribution limits.
    pub limits: ContributionLimits,
    /// The RMD table.
    pub rmd: RmdTable,
    /// IRMAA surcharge tiers, ascending by threshold; empty disables
    /// surcharges (older override files carry none).
    #[serde(default)]
    pub irmaa: Vec<IrmaaTier>,
    /// State income tax tables by lowercase state code; a state absent here
    /// is not modeled.
    #[serde(default)]
    pub states: BTreeMap<String, StateParams>,
}

/// One state's income tax. A state with no income tax is an empty table,
/// which is not the same as a state with none: that one is not modeled.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct StateParams {
    /// Standard deduction by filing status.
    pub deduction: PerStatus<Dollars>,
    /// Brackets by filing status, walked as the federal ones are.
    pub brackets: PerStatus<Vec<Bracket>>,
    /// Whether the federally taxable share of Social Security is taxed.
    pub taxes_social_security: bool,
}

/// All known tax years, ready to answer any projection year.
#[derive(Debug, Clone, PartialEq)]
pub struct TaxTables {
    years: BTreeMap<i16, TaxParams>,
}

impl TaxTables {
    /// The tables compiled into the binary.
    ///
    /// # Panics
    ///
    /// Panics if an embedded parameter file does not parse, which a unit test
    /// prevents from shipping.
    #[must_use]
    pub fn embedded() -> Self {
        let mut tables = Self {
            years: BTreeMap::new(),
        };
        for text in EMBEDDED {
            tables
                .add_source(text, "embedded")
                .expect("embedded tax parameter files are valid");
        }
        tables
    }

    /// Parses one parameter file's text and stores it, replacing any table
    /// already held for that year.
    ///
    /// # Errors
    ///
    /// Returns [`ParamsError::Parse`] or [`ParamsError::Schema`]; `label`
    /// names the source in the error.
    pub fn add_source(&mut self, text: &str, label: &str) -> Result<(), ParamsError> {
        let params: TaxParams = toml::from_str(text).map_err(|source| ParamsError::Parse {
            path: label.to_owned(),
            source: Box::new(source),
        })?;
        if params.schema != PARAMS_SCHEMA_VERSION {
            return Err(ParamsError::Schema {
                path: label.to_owned(),
                found: params.schema,
            });
        }
        self.years.insert(params.year, params);
        Ok(())
    }

    /// Loads every `*.toml` file in a directory, overriding embedded years.
    /// A missing directory is fine; it simply adds nothing.
    ///
    /// # Errors
    ///
    /// Returns [`ParamsError::Io`] when a present directory or file cannot be
    /// read, and parse/schema errors from [`TaxTables::add_source`].
    pub fn add_dir(&mut self, dir: &Path) -> Result<(), ParamsError> {
        if !dir.is_dir() {
            return Ok(());
        }
        let entries = std::fs::read_dir(dir).map_err(|source| ParamsError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        for entry in entries {
            let path = entry
                .map_err(|source| ParamsError::Io {
                    path: dir.display().to_string(),
                    source,
                })?
                .path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                let text = std::fs::read_to_string(&path).map_err(|source| ParamsError::Io {
                    path: path.display().to_string(),
                    source,
                })?;
                self.add_source(&text, &path.display().to_string())?;
            }
        }
        Ok(())
    }

    /// The latest year with a real (non-extended) table.
    #[must_use]
    pub fn latest_known_year(&self) -> Option<i16> {
        self.years.keys().next_back().copied()
    }

    /// Parameters for a year. A year between known tables uses the nearest
    /// table at or below it; a year past the last known table is that table
    /// with its indexed dollar values carried to `year` by `inflation`.
    /// Statutory unindexed values (Social Security thresholds, RMD divisors,
    /// rates) are never scaled.
    ///
    /// # Panics
    ///
    /// Panics if no tables are loaded; [`TaxTables::embedded`] always holds
    /// at least one.
    #[must_use]
    pub fn params_for(&self, year: i16, inflation: &Inflation) -> TaxParams {
        let (&base_year, base) = self
            .years
            .range(..=year)
            .next_back()
            .or_else(|| self.years.iter().next())
            .expect("at least one tax table is loaded");
        if year <= base_year {
            let mut params = base.clone();
            params.year = year;
            return params;
        }
        inflate(base, year, inflation.factor(base_year, year))
    }
}
