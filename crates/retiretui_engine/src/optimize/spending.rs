//! The spending ceiling: the most a plan's flexible spending can be and
//! still meet a measure, found by scaling every flexible expense alike and
//! judging the plan whole at each step.

use serde::{Deserialize, Serialize};

use super::measure::{Judged, Measure, judge};
use crate::market::History;
use crate::params::TaxTables;
use crate::plan::{Dollars, Plan, PlanError, SCHEMA_VERSION};
use crate::search::{Progress, RunError};

/// The most plans a search judges.
pub const CEILING_STEPS: usize = 16;
/// The most a search multiplies flexible spending by.
pub const MAX_FACTOR: f64 = 8.0;
/// A search stops once what it has left to settle is under this much
/// flexible spending a year.
const SETTLED_WITHIN: f64 = 100.0;

/// One flexible expense at the ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScaledExpense {
    /// The expense's id.
    pub id: String,
    /// Its annual amount at the ceiling, in today's dollars.
    pub amount: Dollars,
}

/// The most flexible spending a measure allows.
#[derive(Debug, Clone, PartialEq)]
pub struct SpendingCeiling {
    /// The plan as stated, judged.
    pub baseline: Judged,
    /// What every flexible amount is multiplied by; 1.0 is the plan.
    pub factor: f64,
    /// Whether the search stopped at [`MAX_FACTOR`] with the measure still
    /// met, so the ceiling is at least this.
    pub is_capped: bool,
    /// Each flexible expense at the ceiling, in the plan's order.
    pub expenses: Vec<ScaledExpense>,
    /// The plan at the ceiling, judged.
    pub judged: Judged,
}

/// One search, and how many plans it has judged.
struct Search<'a> {
    plan: &'a Plan,
    tables: &'a TaxTables,
    history: &'a History,
    measure: Measure,
    progress: &'a Progress,
    judged: usize,
}

impl Search<'_> {
    fn judge_at(&mut self, factor: f64) -> Result<Judged, RunError> {
        let mut candidate = self.plan.clone();
        apply_spending(&mut candidate, &scaled(self.plan, factor));
        self.judged += 1;
        judge(
            &candidate,
            self.tables,
            self.history,
            self.measure,
            self.progress,
        )
    }

    /// The highest factor found to meet the measure, and the lowest found
    /// to fail it; nothing failed where the plan meets it at [`MAX_FACTOR`].
    fn bracket(&mut self, baseline: &Judged) -> Result<(Met, Option<f64>), RunError> {
        if !baseline.is_met {
            let nothing = self.judge_at(0.0)?;
            if !nothing.is_met {
                return Err(refused(match self.measure {
                    Measure::Planned => "the plan runs short even with no flexible spending",
                    Measure::Success(_) => {
                        "the plan lasts in fewer markets than the target even with no flexible \
                         spending"
                    }
                }));
            }
            return Ok(((0.0, nothing), Some(1.0)));
        }
        let mut met = (1.0, baseline.clone());
        while met.0 < MAX_FACTOR {
            let doubled = met.0 * 2.0;
            let judged = self.judge_at(doubled)?;
            if !judged.is_met {
                return Ok((met, Some(doubled)));
            }
            met = (doubled, judged);
        }
        Ok((met, None))
    }

    /// Halves what lies between `met` and `failed` until it is under
    /// [`SETTLED_WITHIN`] of `flexible` spending or the steps run out.
    fn bisect(
        &mut self,
        mut met: Met,
        mut failed: f64,
        flexible: Dollars,
    ) -> Result<Met, RunError> {
        while self.judged < CEILING_STEPS && (failed - met.0) * flexible as f64 >= SETTLED_WITHIN {
            let middle = f64::midpoint(met.0, failed);
            let judged = self.judge_at(middle)?;
            if judged.is_met {
                met = (middle, judged);
            } else {
                failed = middle;
            }
        }
        Ok(met)
    }
}

/// A factor that met the measure, and the plan judged at it.
type Met = (f64, Judged);

fn refused(message: &str) -> RunError {
    RunError::refused("expenses", message)
}

/// Each flexible expense of `plan` at `factor` times its amount, floored to
/// the dollar.
fn scaled(plan: &Plan, factor: f64) -> Vec<ScaledExpense> {
    let flexible = plan.expenses.iter().filter(|expense| expense.is_flexible());
    flexible
        .map(|expense| ScaledExpense {
            id: expense.id.clone(),
            amount: (expense.amount as f64 * factor).floor() as Dollars,
        })
        .collect()
}

/// Searches the most `plan`'s flexible spending can be multiplied by and
/// still meet `measure`: every recurring expense not marked `essential`
/// scaled alike, from the plan as stated up to [`MAX_FACTOR`] or down to
/// nothing, in at most [`CEILING_STEPS`] plans judged. A Monte Carlo
/// measure walks every candidate through the same markets.
///
/// # Errors
///
/// [`RunError::Refused`] when the plan has no flexible spending, when it
/// fails the measure with none, or for a target share outside 0 to 1;
/// [`RunError::Cancelled`] when `progress` was cancelled first.
pub fn spending_ceiling(
    plan: &Plan,
    tables: &TaxTables,
    history: &History,
    measure: Measure,
    progress: &Progress,
) -> Result<SpendingCeiling, RunError> {
    measure.check()?;
    let flexible = plan.flexible_spending();
    if flexible == 0 {
        return Err(refused(
            "there is no flexible spending to scale: every expense is essential, one-time or zero",
        ));
    }
    let mut search = Search {
        plan,
        tables,
        history,
        measure,
        progress,
        judged: 0,
    };
    let baseline = search.judge_at(1.0)?;
    let (met, failed) = search.bracket(&baseline)?;
    let (factor, judged) = match failed {
        Some(failed) => search.bisect(met, failed, flexible)?,
        None => met,
    };
    Ok(SpendingCeiling {
        baseline,
        factor,
        is_capped: failed.is_none(),
        expenses: scaled(plan, factor),
        judged,
    })
}

/// What a ceiling does to a plan: each scaled expense's `amount` restated.
pub fn apply_spending(plan: &mut Plan, expenses: &[ScaledExpense]) {
    for scaled in expenses {
        let stated = plan.expenses.iter_mut().find(|it| it.id == scaled.id);
        if let Some(expense) = stated {
            expense.amount = scaled.amount;
        }
    }
}

/// The scenario overlay for a ceiling, as canonical TOML: `base` plus one
/// `[[expenses]]` fragment per scaled expense, its `id` and `amount`.
///
/// # Errors
///
/// Returns [`PlanError::Serialize`] when serialization fails.
pub fn spending_overlay(base: &str, expenses: &[ScaledExpense]) -> Result<String, PlanError> {
    #[derive(Serialize)]
    struct OverlayDocument<'a> {
        schema: u32,
        base: &'a str,
        expenses: &'a [ScaledExpense],
    }
    Ok(toml::to_string_pretty(&OverlayDocument {
        schema: SCHEMA_VERSION,
        base,
        expenses,
    })?)
}
