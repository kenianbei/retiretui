//! The market a projection walks through: each year's return on each
//! asset class, and how prices moved.

use crate::params::Inflation;
use crate::plan::{AssetClass, ClassReturns, Plan};

use super::{horizon_len, plan_inflation};

/// One market history over a plan's projected years.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketPath {
    start_year: i16,
    /// Each projected year's return on each class.
    returns: Vec<ClassReturns>,
    inflation: Inflation,
    /// What a start-year price costs in each projected year.
    deflators: Vec<f64>,
}

impl MarketPath {
    /// The market the plan states: each class's mean return and the plan's
    /// inflation, every year.
    #[must_use]
    pub fn expected(plan: &Plan) -> Self {
        let market = plan.market();
        let mut means = ClassReturns::default();
        for &class in AssetClass::ALL {
            means[class.index()] = market.mean(class);
        }
        let returns = vec![means; horizon_len(plan)];
        Self::new(plan, returns, plan_inflation(plan))
    }

    /// A market of `returns`, one per projected year from the start, and
    /// `inflation`.
    pub(crate) fn new(plan: &Plan, returns: Vec<ClassReturns>, inflation: Inflation) -> Self {
        let start_year = plan.plan.start_year;
        let deflators = (start_year..)
            .take(horizon_len(plan))
            .map(|year| inflation.factor(start_year, year))
            .collect();
        Self {
            start_year,
            returns,
            inflation,
            deflators,
        }
    }

    /// `year`'s return on each class; nothing outside the projected years.
    #[must_use]
    pub fn returns(&self, year: i16) -> ClassReturns {
        usize::try_from(year - self.start_year)
            .ok()
            .and_then(|at| self.returns.get(at))
            .copied()
            .unwrap_or_default()
    }

    /// How prices moved, inside the projected years and outside them.
    #[must_use]
    pub fn inflation(&self) -> &Inflation {
        &self.inflation
    }

    /// What a start-year price costs in `year`, which may lie outside the
    /// projected years.
    #[must_use]
    pub fn deflator(&self, year: i16) -> f64 {
        usize::try_from(year - self.start_year)
            .ok()
            .and_then(|at| self.deflators.get(at))
            .copied()
            .unwrap_or_else(|| self.inflation.factor(self.start_year, year))
    }
}
