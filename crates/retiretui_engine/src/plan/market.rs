//! What a plan assumes the market does, and how its market tools run: the
//! `[market]` table, every field optional and each one unstated taking the
//! engine's built-in default.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::allocation::AssetClass;
use super::validate::push_issue;
use super::{Dollars, Issue};

const DEFAULTS: &str = include_str!("../../market/defaults.toml");

/// The most runs a Monte Carlo search may ask for.
const MAX_TRIALS: u32 = 100_000;

/// The longest run of historical years a bootstrap may draw together.
const MAX_BLOCK_YEARS: u8 = 30;

/// The variables a market draws together, in the order of a correlation
/// matrix: the three asset classes, by their index, then inflation.
pub(crate) const VARIABLES: usize = 4;
pub(crate) const INFLATION: usize = 3;

/// A correlation matrix over the asset classes and inflation.
pub type Correlation = [[f64; VARIABLES]; VARIABLES];

/// Where a Monte Carlo search draws its markets from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Draw {
    /// Random years from the stated means, volatilities and correlations.
    Assumptions,
    /// Random historical years, drawn in blocks.
    History,
}

impl Draw {
    /// Every draw, in the order the schema declares them.
    pub const ALL: &'static [Self] = &[Self::Assumptions, Self::History];

    /// The draw as a plan file spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Assumptions => "assumptions",
            Self::History => "history",
        }
    }
}

/// One asset class's yearly return.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassAssumption {
    /// The compound yearly return: the median year's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean: Option<f64>,
    /// The yearly return's standard deviation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volatility: Option<f64>,
}

/// How inflation strays from the plan's own rate.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InflationAssumption {
    /// The yearly shock's standard deviation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volatility: Option<f64>,
    /// The share of a year's deviation carried into the next.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence: Option<f64>,
}

/// How the asset classes and inflation move together, pair by pair.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(missing_docs, reason = "each field is the pair its name spells")]
pub struct Correlations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stocks_bonds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stocks_cash: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stocks_inflation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bonds_cash: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bonds_inflation: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cash_inflation: Option<f64>,
}

impl Correlations {
    /// Each pair with where it sits in the matrix and its key.
    fn pairs(&self) -> [((usize, usize), &'static str, Option<f64>); 6] {
        let (stocks, bonds, cash) = (
            AssetClass::Stocks.index(),
            AssetClass::Bonds.index(),
            AssetClass::Cash.index(),
        );
        [
            ((stocks, bonds), "stocks_bonds", self.stocks_bonds),
            ((stocks, cash), "stocks_cash", self.stocks_cash),
            (
                (stocks, INFLATION),
                "stocks_inflation",
                self.stocks_inflation,
            ),
            ((bonds, cash), "bonds_cash", self.bonds_cash),
            ((bonds, INFLATION), "bonds_inflation", self.bonds_inflation),
            ((cash, INFLATION), "cash_inflation", self.cash_inflation),
        ]
    }
}

/// How the Monte Carlo tool runs.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonteCarloSettings {
    /// Where the markets are drawn from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draw: Option<Draw>,
    /// How many markets are run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trials: Option<u32>,
    /// What the markets are drawn from, so the same seed draws the same.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// How many consecutive historical years a draw from history takes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_years: Option<u8>,
}

/// How the Historical tool runs.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalSettings {
    /// The first start year tried.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<i16>,
    /// The last start year tried.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<i16>,
    /// Whether a history reaching past the data goes on from its start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<bool>,
}

/// The `[market]` table. Read it through the accessors, which answer with
/// the built-in default wherever the plan states nothing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Market {
    /// What a run must end with, in today's dollars, besides never falling
    /// short, to count as a success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leave_at_least: Option<Dollars>,
    /// Stocks' return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stocks: Option<ClassAssumption>,
    /// Bonds' return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bonds: Option<ClassAssumption>,
    /// Cash's return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cash: Option<ClassAssumption>,
    /// How inflation strays.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inflation: Option<InflationAssumption>,
    /// How the classes and inflation move together.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation: Option<Correlations>,
    /// The Monte Carlo tool's settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monte_carlo: Option<MonteCarloSettings>,
    /// The Historical tool's settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub historical: Option<HistoricalSettings>,
}

/// The built-in `[market]`, every field stated.
fn defaults() -> &'static Market {
    static DEFAULT: OnceLock<Market> = OnceLock::new();
    DEFAULT.get_or_init(|| toml::from_str(DEFAULTS).expect("the embedded market defaults parse"))
}

/// Answers `field` from `market`, else from the built-in defaults.
fn settle<T>(market: &Market, field: impl Fn(&Market) -> Option<T>) -> T {
    field(market)
        .or_else(|| field(defaults()))
        .expect("the embedded market defaults state every field")
}

impl Market {
    /// A `[market]` stating nothing.
    pub const NONE: Self = Self {
        leave_at_least: None,
        stocks: None,
        bonds: None,
        cash: None,
        inflation: None,
        correlation: None,
        monte_carlo: None,
        historical: None,
    };

    fn class(&self, class: AssetClass) -> Option<ClassAssumption> {
        match class {
            AssetClass::Stocks => self.stocks,
            AssetClass::Bonds => self.bonds,
            AssetClass::Cash => self.cash,
        }
    }

    /// `class`'s compound yearly return.
    #[must_use]
    pub fn mean(&self, class: AssetClass) -> f64 {
        settle(self, |market| market.class(class).and_then(|it| it.mean))
    }

    /// `class`'s yearly volatility.
    #[must_use]
    pub fn volatility(&self, class: AssetClass) -> f64 {
        settle(self, |market| {
            market.class(class).and_then(|it| it.volatility)
        })
    }

    /// Inflation's yearly shock.
    #[must_use]
    pub fn inflation_volatility(&self) -> f64 {
        settle(self, |market| market.inflation.and_then(|it| it.volatility))
    }

    /// The share of inflation's deviation carried into the next year.
    #[must_use]
    pub fn inflation_persistence(&self) -> f64 {
        settle(self, |market| {
            market.inflation.and_then(|it| it.persistence)
        })
    }

    /// The correlation matrix over stocks, bonds, cash and inflation.
    #[must_use]
    pub fn correlation(&self) -> Correlation {
        let mut matrix = [[0.0; VARIABLES]; VARIABLES];
        for (at, row) in matrix.iter_mut().enumerate() {
            row[at] = 1.0;
        }
        let stated = self.correlation.unwrap_or_default();
        let fallback = defaults().correlation.unwrap_or_default();
        for (((row, column), _, value), (_, _, default)) in
            stated.pairs().into_iter().zip(fallback.pairs())
        {
            let value = value.or(default).unwrap_or_default();
            matrix[row][column] = value;
            matrix[column][row] = value;
        }
        matrix
    }

    /// What a run must end with, in today's dollars, to count as a success.
    #[must_use]
    pub fn leave_at_least(&self) -> Option<Dollars> {
        self.leave_at_least
    }

    /// Where a Monte Carlo search draws from.
    #[must_use]
    pub fn draw(&self) -> Draw {
        settle(self, |market| market.monte_carlo.and_then(|it| it.draw))
    }

    /// How many markets a Monte Carlo search runs.
    #[must_use]
    pub fn trials(&self) -> u32 {
        settle(self, |market| market.monte_carlo.and_then(|it| it.trials))
    }

    /// The seed a Monte Carlo search draws from.
    #[must_use]
    pub fn seed(&self) -> u32 {
        settle(self, |market| market.monte_carlo.and_then(|it| it.seed))
    }

    /// How many consecutive years a draw from history takes.
    #[must_use]
    pub fn block_years(&self) -> u8 {
        settle(self, |market| {
            market.monte_carlo.and_then(|it| it.block_years)
        })
    }

    /// The first historical start year tried.
    #[must_use]
    pub fn from(&self) -> i16 {
        settle(self, |market| market.historical.and_then(|it| it.from))
    }

    /// The last historical start year tried.
    #[must_use]
    pub fn to(&self) -> i16 {
        settle(self, |market| market.historical.and_then(|it| it.to))
    }

    /// Whether a history reaching past the data goes on from its start.
    #[must_use]
    pub fn wrap(&self) -> bool {
        settle(self, |market| market.historical.and_then(|it| it.wrap))
    }
}

/// The lower-triangular `L` with `L * Lᵀ = matrix`, or none where the matrix
/// is not positive definite - where no market could move as it says.
#[must_use]
pub(crate) fn cholesky(matrix: &Correlation) -> Option<Correlation> {
    let mut lower = [[0.0; VARIABLES]; VARIABLES];
    for row in 0..VARIABLES {
        for column in 0..=row {
            let dot: f64 = (0..column).map(|k| lower[row][k] * lower[column][k]).sum();
            if row == column {
                let left = matrix[row][row] - dot;
                if left <= 0.0 {
                    return None;
                }
                lower[row][row] = left.sqrt();
            } else {
                lower[row][column] = (matrix[row][column] - dot) / lower[column][column];
            }
        }
    }
    Some(lower)
}

fn check_range(issues: &mut Vec<Issue>, path: &str, value: Option<f64>, range: (f64, f64)) {
    let Some(value) = value else {
        return;
    };
    let (low, high) = range;
    if !value.is_finite() || !(low..=high).contains(&value) {
        push_issue(issues, path, format!("must be between {low} and {high}"));
    }
}

/// Checks a plan's `[market]`.
pub(super) fn check_market(market: &Market, issues: &mut Vec<Issue>) {
    if market.leave_at_least.is_some_and(|floor| floor < 0) {
        push_issue(issues, "market.leave_at_least", "must not be negative");
    }
    for &class in AssetClass::ALL {
        let path = format!("market.{}", class.as_str());
        let stated = market.class(class).unwrap_or_default();
        check_range(issues, &format!("{path}.mean"), stated.mean, (-1.0, 1.0));
        check_range(
            issues,
            &format!("{path}.volatility"),
            stated.volatility,
            (0.0, 1.0),
        );
    }
    let inflation = market.inflation.unwrap_or_default();
    check_range(
        issues,
        "market.inflation.volatility",
        inflation.volatility,
        (0.0, 1.0),
    );
    check_range(
        issues,
        "market.inflation.persistence",
        inflation.persistence,
        (0.0, 0.99),
    );
    for (_, key, value) in market.correlation.unwrap_or_default().pairs() {
        check_range(
            issues,
            &format!("market.correlation.{key}"),
            value,
            (-1.0, 1.0),
        );
    }
    if cholesky(&market.correlation()).is_none() {
        push_issue(
            issues,
            "market.correlation",
            "these correlations cannot all hold at once",
        );
    }
    check_settings(market, issues);
}

fn check_settings(market: &Market, issues: &mut Vec<Issue>) {
    let monte_carlo = market.monte_carlo.unwrap_or_default();
    if monte_carlo
        .trials
        .is_some_and(|trials| !(1..=MAX_TRIALS).contains(&trials))
    {
        push_issue(
            issues,
            "market.monte_carlo.trials",
            format!("must be between 1 and {MAX_TRIALS}"),
        );
    }
    if monte_carlo
        .block_years
        .is_some_and(|years| !(1..=MAX_BLOCK_YEARS).contains(&years))
    {
        push_issue(
            issues,
            "market.monte_carlo.block_years",
            format!("must be between 1 and {MAX_BLOCK_YEARS}"),
        );
    }
    if market.from() > market.to() {
        push_issue(
            issues,
            "market.historical.to",
            "must not be before the first start year",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_state_every_field_and_their_correlations_can_hold() {
        let market = Market::default();
        for &class in AssetClass::ALL {
            assert!(market.mean(class) > 0.0);
            assert!(market.volatility(class) > 0.0);
        }
        let _ = (
            market.draw(),
            market.trials(),
            market.seed(),
            market.block_years(),
        );
        let _ = (market.from(), market.to(), market.wrap());
        assert!(cholesky(&market.correlation()).is_some());
    }

    #[test]
    fn a_stated_field_wins_and_the_rest_of_its_table_defaults() {
        let market: Market = toml::from_str("[stocks]\nmean = 0.03\n").unwrap();
        assert!((market.mean(AssetClass::Stocks) - 0.03).abs() < 1e-12);
        assert!((market.volatility(AssetClass::Stocks) - 0.16).abs() < 1e-12);
    }

    #[test]
    fn cholesky_rebuilds_its_matrix_and_refuses_one_that_cannot_hold() {
        let matrix = Market::default().correlation();
        let lower = cholesky(&matrix).unwrap();
        for row in 0..VARIABLES {
            for column in 0..VARIABLES {
                let product: f64 = (0..VARIABLES)
                    .map(|k| lower[row][k] * lower[column][k])
                    .sum();
                assert!((product - matrix[row][column]).abs() < 1e-12);
            }
        }
        let mut impossible = matrix;
        for (a, b) in [(0, 1), (0, 2)] {
            impossible[a][b] = 0.9;
            impossible[b][a] = 0.9;
        }
        impossible[1][2] = -0.9;
        impossible[2][1] = -0.9;
        assert!(cholesky(&impossible).is_none());
    }
}
