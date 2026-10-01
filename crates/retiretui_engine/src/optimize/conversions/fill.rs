use crate::params::TaxTables;
use crate::plan::{Dollars, Plan};
use crate::project::{Projection, project};
use crate::search::Progress;

use super::ladder::ladder_conversion;
use super::targets::{YearCeilings, conversion_window, year_ceilings};
use super::{LadderStep, OptimizeOptions, UNBOUNDED};

/// Settles the window years front to back against `baseline`, returning the
/// steps and the final optimized projection; once `progress` is cancelled,
/// the steps settled so far.
pub(super) fn search_ladder(
    plan: &Plan,
    tables: &TaxTables,
    (options, bracket_rate): (&OptimizeOptions, f64),
    baseline: &Projection,
    progress: &Progress,
) -> (Vec<LadderStep>, Projection) {
    let mut working = plan.clone();
    let mut current = baseline.clone();
    let mut steps: Vec<LadderStep> = Vec::new();
    let mut total = 0;
    for year in conversion_window(plan, options) {
        let Some(ceilings) = year_ceilings(plan, tables, year, bracket_rate, options) else {
            continue;
        };
        let mut annual_left = options.annual_max.unwrap_or(UNBOUNDED);
        for source in &options.sources {
            if progress.is_cancelled() {
                return (steps, current);
            }
            let total_left = options.total_max.map_or(UNBOUNDED, |max| max - total);
            let cap = annual_left.min(total_left);
            if cap <= 0 {
                break;
            }
            let fill = FillYear {
                year,
                source,
                destination: &options.destination,
                ceilings,
                cap,
            };
            let amount = fill_year(&mut working, tables, &fill, &current);
            if amount <= 0 {
                continue;
            }
            working.conversions.push(ladder_conversion(
                source,
                &options.destination,
                year,
                amount,
            ));
            current = project(&working, tables);
            steps.push(LadderStep {
                year,
                source: source.clone(),
                amount,
            });
            total += amount;
            annual_left -= amount;
        }
    }
    (steps, current)
}

struct FillYear<'a> {
    year: i16,
    source: &'a str,
    destination: &'a str,
    ceilings: YearCeilings,
    cap: Dollars,
}

impl FillYear<'_> {
    /// Whether a probed year has passed one of its ceilings. A gain is past
    /// its top when the stack it sits on ends above it.
    fn is_over(&self, metrics: &YearFill) -> bool {
        let ceilings = self.ceilings;
        metrics.taxable > ceilings.target
            || ceilings.magi.is_some_and(|ceiling| metrics.magi > ceiling)
            || ceilings
                .gains
                .is_some_and(|top| metrics.gains > 0 && metrics.taxable + metrics.gains > top)
    }

    /// Whether a year has no room left under one of its ceilings.
    fn is_at_limit(&self, metrics: &YearFill) -> bool {
        let ceilings = self.ceilings;
        metrics.taxable >= ceilings.target
            || ceilings.magi.is_some_and(|ceiling| metrics.magi >= ceiling)
            || ceilings
                .gains
                .is_some_and(|top| metrics.gains > 0 && metrics.taxable + metrics.gains >= top)
    }
}

/// The stated amount converting from one source in one year so that the
/// year's projected ordinary taxable income reaches the target within a
/// dollar - or everything the source can give under the cap, when that
/// still falls short. `current` must be `working`'s projection; candidate
/// conversions are pushed and popped on `working` per probe.
fn fill_year(
    working: &mut Plan,
    tables: &TaxTables,
    fill: &FillYear<'_>,
    current: &Projection,
) -> Dollars {
    let base = year_metrics(current, fill.year);
    if fill.is_at_limit(&base) {
        return 0;
    }
    let poured = probe(working, tables, fill, fill.cap);
    let achievable = (poured.converted - base.converted).min(fill.cap);
    if !fill.is_over(&poured) {
        return achievable;
    }
    let (mut lo, mut hi) = (0, achievable);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if fill.is_over(&probe(working, tables, fill, mid)) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    lo
}

/// Projects `working` plus one candidate conversion and reports the year's
/// ordinary taxable income and total conversions.
fn probe(working: &mut Plan, tables: &TaxTables, fill: &FillYear<'_>, amount: Dollars) -> YearFill {
    working.conversions.push(ladder_conversion(
        fill.source,
        fill.destination,
        fill.year,
        amount,
    ));
    let metrics = year_metrics(&project(working, tables), fill.year);
    working.conversions.pop();
    metrics
}

/// One probed year's fill-relevant figures.
#[derive(Default)]
struct YearFill {
    taxable: Dollars,
    magi: Dollars,
    gains: Dollars,
    converted: Dollars,
}

fn year_metrics(projection: &Projection, year: i16) -> YearFill {
    projection
        .row(year)
        .map(|row| YearFill {
            taxable: row.taxes.ordinary_taxable,
            magi: row.taxes.magi,
            gains: row.taxes.gains,
            converted: row.conversions,
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::conversions::with_default_sources;

    fn settled(progress: &Progress) -> Vec<LadderStep> {
        let plan = Plan::from_toml_str(include_str!("../../../tests/fixtures/full.toml")).unwrap();
        let tables = TaxTables::embedded();
        let options = with_default_sources(
            &plan,
            &OptimizeOptions {
                sources: Vec::new(),
                destination: "roth-ira".to_owned(),
                start_year: None,
                end_year: None,
                annual_max: None,
                total_max: None,
                headroom: 0,
                irmaa_tier: None,
                max_magi: None,
                gains_rate: None,
            },
        );
        let baseline = project(&plan, &tables);
        search_ladder(&plan, &tables, (&options, 0.22), &baseline, progress).0
    }

    #[test]
    fn a_cancelled_search_settles_no_step() {
        assert!(!settled(&Progress::default()).is_empty());
        let cancelled = Progress::default();
        cancelled.cancel();
        assert_eq!(settled(&cancelled), []);
    }
}
