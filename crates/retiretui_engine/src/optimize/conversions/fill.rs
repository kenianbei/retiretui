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
    /// The least room a probed year has left under any of its ceilings,
    /// negative once past one. A gain is past its top when the stack it
    /// sits on ends above it, and a year that realizes none is not held.
    fn slack(&self, metrics: &YearFill) -> Dollars {
        let YearCeilings {
            target,
            magi,
            gains,
        } = self.ceilings;
        let under_magi = magi.map_or(Dollars::MAX, |ceiling| ceiling - metrics.magi);
        let under_gains = gains
            .filter(|_| metrics.gains > 0)
            .map_or(Dollars::MAX, |top| top - metrics.taxable - metrics.gains);
        (target - metrics.taxable).min(under_magi).min(under_gains)
    }
}

/// The most to convert from one source in one year that passes none of
/// the year's ceilings, to the dollar - or everything the source can give
/// under the cap, when that passes none either. `current` must be
/// `working`'s projection; candidate conversions are pushed and popped on
/// `working` per probe.
fn fill_year(
    working: &mut Plan,
    tables: &TaxTables,
    fill: &FillYear<'_>,
    current: &Projection,
) -> Dollars {
    let base = year_metrics(current, fill.year);
    if fill.slack(&base) <= 0 {
        return 0;
    }
    let poured = probe(working, tables, fill, fill.cap);
    let achievable = (poured.converted - base.converted).min(fill.cap);
    if fill.slack(&poured) >= 0 {
        return achievable;
    }
    let (mut lo, mut hi) = (0, achievable);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if fill.slack(&probe(working, tables, fill, mid)) < 0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    lo
}

/// Projects `working` plus one candidate conversion and reports the year's
/// fill-relevant figures.
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
        assert_ne!(settled(&Progress::default()), []);
        let cancelled = Progress::default();
        cancelled.cancel();
        assert_eq!(settled(&cancelled), []);
    }
}
