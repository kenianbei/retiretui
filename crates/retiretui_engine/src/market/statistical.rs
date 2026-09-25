//! Markets drawn from stated assumptions: each year one correlated draw of
//! every class's return and inflation's shock. A class's return is
//! lognormal with its stated mean as the median year's return and its
//! stated volatility as the yearly standard deviation; inflation strays from
//! the plan's rate and carries a share of each year's deviation into the
//! next.

use crate::plan::{AssetClass, ClassReturns, Correlation, INFLATION, Plan, VARIABLES, cholesky};

use super::random::Random;

/// The lowest inflation a draw is allowed, so prices never fall to nothing.
const INFLATION_FLOOR: f64 = -0.99;

/// Everything a year's draw needs, fitted once per plan.
pub(crate) struct Assumptions {
    lower: Correlation,
    mean: ClassReturns,
    log_median: ClassReturns,
    log_spread: ClassReturns,
    inflation: f64,
    shock: f64,
    persistence: f64,
}

/// The lognormal's log-scale spread whose yearly return, with `mean` as
/// its median, has `volatility` as its standard deviation.
fn log_spread(mean: f64, volatility: f64) -> f64 {
    let relative = volatility / (1.0 + mean);
    let growth = f64::midpoint(1.0, (1.0 + 4.0 * relative * relative).sqrt());
    growth.ln().sqrt()
}

impl Assumptions {
    pub(crate) fn of(plan: &Plan) -> Self {
        let market = plan.market();
        let mut means = ClassReturns::default();
        let mut log_median = ClassReturns::default();
        let mut log_spread_of = ClassReturns::default();
        for &class in AssetClass::ALL {
            let mean = market.mean(class);
            means[class.index()] = mean;
            log_median[class.index()] = mean.ln_1p();
            log_spread_of[class.index()] = log_spread(mean, market.volatility(class));
        }
        Self {
            lower: cholesky(&market.correlation())
                .expect("validation holds the correlations to ones a market can have"),
            mean: means,
            log_median,
            log_spread: log_spread_of,
            inflation: plan.plan.inflation,
            shock: market.inflation_volatility(),
            persistence: market.inflation_persistence(),
        }
    }

    /// One year: each class's return, and inflation, `deviation` carrying
    /// inflation's stray from the plan's rate from year to year.
    pub(crate) fn year(&self, random: &mut Random, deviation: &mut f64) -> (ClassReturns, f64) {
        let independent: [f64; VARIABLES] = std::array::from_fn(|_| random.normal());
        let correlated: [f64; VARIABLES] =
            std::array::from_fn(|row| (0..=row).map(|k| self.lower[row][k] * independent[k]).sum());
        let returns = std::array::from_fn(|at| {
            if self.log_spread[at] == 0.0 {
                return self.mean[at];
            }
            (self.log_median[at] + self.log_spread[at] * correlated[at]).exp_m1()
        });
        *deviation = self.persistence * *deviation + self.shock * correlated[INFLATION];
        (returns, (self.inflation + *deviation).max(INFLATION_FLOOR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRAWS: usize = 100_000;

    fn plan() -> Plan {
        Plan::from_toml_str(
            "schema = 1\n[plan]\nstart_year = 2026\nhorizon_age = 90\ninflation = 0.025\n\
             [household]\nfiling = \"single\"\n[[household.people]]\nid = \"me\"\nbirth = 1970-01-01\n",
        )
        .unwrap()
    }

    fn correlation(a: &[f64], b: &[f64]) -> f64 {
        let mean = |xs: &[f64]| xs.iter().sum::<f64>() / xs.len() as f64;
        let (mean_a, mean_b) = (mean(a), mean(b));
        let covariance: f64 = a
            .iter()
            .zip(b)
            .map(|(x, y)| (x - mean_a) * (y - mean_b))
            .sum();
        let spread = |xs: &[f64], m: f64| xs.iter().map(|x| (x - m).powi(2)).sum::<f64>().sqrt();
        covariance / (spread(a, mean_a) * spread(b, mean_b))
    }

    #[test]
    fn draws_hold_the_stated_median_volatility_correlation_and_persistence() {
        let plan = plan();
        let assumptions = Assumptions::of(&plan);
        let mut random = Random::new(42, 0);
        let mut deviation = 0.0;
        let years: Vec<(ClassReturns, f64)> = (0..DRAWS)
            .map(|_| assumptions.year(&mut random, &mut deviation))
            .collect();
        let stocks: Vec<f64> = years.iter().map(|(returns, _)| returns[0]).collect();
        let mut sorted = stocks.clone();
        sorted.sort_by(f64::total_cmp);
        let median = sorted[DRAWS / 2];
        assert!((median - 0.06).abs() < 0.003, "median {median}");
        let mean = stocks.iter().sum::<f64>() / DRAWS as f64;
        let volatility =
            (stocks.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / DRAWS as f64).sqrt();
        assert!((volatility - 0.16).abs() < 0.003, "volatility {volatility}");
        let logs = |at: usize| -> Vec<f64> {
            years
                .iter()
                .map(|(returns, _)| returns[at].ln_1p())
                .collect()
        };
        let stocks_bonds = correlation(&logs(0), &logs(1));
        assert!(
            (stocks_bonds - 0.1).abs() < 0.02,
            "stocks-bonds {stocks_bonds}"
        );
        let inflation: Vec<f64> = years.iter().map(|&(_, inflation)| inflation).collect();
        let lagged = correlation(&inflation[..DRAWS - 1], &inflation[1..]);
        assert!((lagged - 0.6).abs() < 0.02, "persistence {lagged}");
        let cash_inflation = correlation(&logs(2)[1..], &inflation[1..]);
        assert!(
            cash_inflation > 0.2,
            "cash follows inflation: {cash_inflation}"
        );
    }
}
