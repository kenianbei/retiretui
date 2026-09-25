use crate::params::TaxTables;
use crate::plan::{Dollars, Plan};
use crate::project::{Projection, project};

use super::ladder::ladder_conversion;
use super::targets::{conversion_window, year_targets};
use super::{LadderStep, OptimizeOptions, UNBOUNDED};

/// Settles the window years front to back against `baseline`, returning the
/// steps and the final optimized projection.
pub(super) fn search_ladder(
    plan: &Plan,
    tables: &TaxTables,
    options: &OptimizeOptions,
    bracket_rate: f64,
    baseline: &Projection,
) -> (Vec<LadderStep>, Projection) {
    let mut working = plan.clone();
    let mut current = baseline.clone();
    let mut steps: Vec<LadderStep> = Vec::new();
    let mut total = 0;
    for year in conversion_window(plan, options) {
        let Some((target, magi_ceiling)) = year_targets(plan, tables, year, bracket_rate, options)
        else {
            continue;
        };
        let mut annual_left = options.annual_max.unwrap_or(UNBOUNDED);
        for source in &options.sources {
            let total_left = options.total_max.map_or(UNBOUNDED, |max| max - total);
            let cap = annual_left.min(total_left);
            if cap <= 0 {
                break;
            }
            let fill = FillYear {
                year,
                source,
                destination: &options.destination,
                target,
                magi_ceiling,
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
    target: Dollars,
    magi_ceiling: Option<Dollars>,
    cap: Dollars,
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
    let over = |metrics: &YearFill| {
        metrics.taxable > fill.target
            || fill
                .magi_ceiling
                .is_some_and(|ceiling| metrics.magi > ceiling)
    };
    let at_limit = |metrics: &YearFill| {
        metrics.taxable >= fill.target
            || fill
                .magi_ceiling
                .is_some_and(|ceiling| metrics.magi >= ceiling)
    };
    let base = year_metrics(current, fill.year);
    if at_limit(&base) {
        return 0;
    }
    let poured = probe(working, tables, fill, fill.cap);
    let achievable = (poured.converted - base.converted).min(fill.cap);
    if !over(&poured) {
        return achievable;
    }
    let (mut lo, mut hi) = (0, achievable);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if over(&probe(working, tables, fill, mid)) {
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
    converted: Dollars,
}

fn year_metrics(projection: &Projection, year: i16) -> YearFill {
    projection
        .years
        .iter()
        .find(|row| row.year == year)
        .map(|row| YearFill {
            taxable: row.taxes.ordinary_taxable,
            magi: row.taxes.magi,
            converted: row.conversions,
        })
        .unwrap_or_default()
}
