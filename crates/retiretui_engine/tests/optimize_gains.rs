//! The long-term gains ceiling on a conversion ladder, over a zero-inflation
//! plan that lives off a brokerage so every year realizes gains.

mod common;

use retiretui_engine::optimize::{GainsRate, OptimizeOptions, SweptBracket, optimize_conversions};
use retiretui_engine::params::{Inflation, Ltcg, TaxTables};
use retiretui_engine::project::YearRow;

use common::plan_from;

/// Five sixths of every dollar drawn from the brokerage is gain.
const LIVING_OFF_GAINS: &str = r#"
schema = 1

[plan]
name = "opt-gains"
start_year = 2026
horizon_age = 75
inflation = 0.0

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1964-06-15

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "b"
kind = "brokerage"
owner = "me"
balance = 600000
basis = 100000

[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 5000000

[[accounts]]
id = "r"
kind = "ira"
roth = true
owner = "me"
balance = 0

[[expenses]]
id = "living"
amount = 40000
cola = false
"#;

/// The same plan living off cash, so no year realizes a gain.
fn living_off_cash() -> String {
    LIVING_OFF_GAINS
        .replace("kind = \"brokerage\"", "kind = \"cash\"")
        .replace("basis = 100000\n", "")
}

fn held_to(gains_rate: Option<GainsRate>) -> OptimizeOptions {
    OptimizeOptions {
        sources: vec!["k".to_owned()],
        destination: "r".to_owned(),
        start_year: None,
        end_year: None,
        annual_max: None,
        total_max: None,
        headroom: 0,
        irmaa_tier: None,
        max_magi: None,
        gains_rate,
    }
}

fn searched(text: &str, options: &OptimizeOptions, rate: f64) -> SweptBracket {
    optimize_conversions(&plan_from(text), &TaxTables::embedded(), options, rate)
        .unwrap()
        .ladder
}

fn ltcg() -> Ltcg {
    TaxTables::embedded()
        .params_for(2026, &Inflation::constant(0.0))
        .ltcg
}

/// Where the year's gains stack ends: ordinary taxable income plus gains.
fn stack_end(row: &YearRow) -> i64 {
    row.taxes.ordinary_taxable + row.taxes.gains
}

#[test]
fn a_ladder_held_at_zero_converts_only_while_gains_stay_untaxed() {
    let plain = searched(LIVING_OFF_GAINS, &held_to(None), 0.12);
    assert!(plain.optimized.years.iter().any(|row| row.taxes.ltcg > 0));

    let held = searched(LIVING_OFF_GAINS, &held_to(Some(GainsRate::Zero)), 0.12);
    assert_eq!(
        held.steps[0].amount, 30_939,
        "taxable 14,839 under gains of 34,611 on 41,533 drawn ends at 49,450"
    );
    assert!(held.converted(false) < plain.converted(false));
    let zero_top = ltcg().zero_until.single;
    for step in &held.steps {
        let row = held.optimized.row(step.year).unwrap();
        assert!(row.taxes.gains > 0, "{}", step.year);
        assert!(stack_end(row) <= zero_top, "{}", step.year);
        assert_eq!(row.taxes.ltcg, 0, "{}", step.year);
    }
}

#[test]
fn a_year_that_realizes_no_gain_is_not_held() {
    let plain = searched(&living_off_cash(), &held_to(None), 0.12);
    let held = searched(&living_off_cash(), &held_to(Some(GainsRate::Zero)), 0.12);
    assert_eq!(held.steps, plain.steps);
    let first = plain.optimized.row(2026).unwrap();
    assert!(
        first.taxes.ordinary_taxable > ltcg().zero_until.single,
        "the 12% bracket's top is past the 0% top, so holding it would bind"
    );
}

#[test]
fn a_year_whose_gains_already_pass_the_top_converts_nothing() {
    let spending_more = LIVING_OFF_GAINS.replace("amount = 40000", "amount = 80000");
    let held = searched(&spending_more, &held_to(Some(GainsRate::Zero)), 0.12);
    assert_eq!(held.steps, []);
    assert!(held.optimized.row(2026).unwrap().taxes.gains > ltcg().zero_until.single);
}

#[test]
fn a_ladder_held_at_fifteen_keeps_gains_out_of_the_top_rate() {
    let fifteen_top = ltcg().fifteen_until.single;
    let mut one_year = held_to(None);
    one_year.end_year = Some(2026);
    let plain = searched(LIVING_OFF_GAINS, &one_year, 0.35);
    assert!(stack_end(plain.optimized.row(2026).unwrap()) > fifteen_top);

    one_year.gains_rate = Some(GainsRate::Fifteen);
    let held = searched(LIVING_OFF_GAINS, &one_year, 0.35);
    let row = held.optimized.row(2026).unwrap();
    assert!(row.taxes.gains > 0 && row.conversions > 0);
    assert_eq!(stack_end(row), fifteen_top, "fills to the 15% top");
}

#[test]
fn a_gains_rate_is_spelled_as_its_percent() {
    for rate in GainsRate::ALL {
        assert_eq!(rate.as_str().parse(), Ok(*rate));
    }
    let refused = "5".parse::<GainsRate>().unwrap_err().to_string();
    assert!(refused.contains("expected `0` or `15`"), "{refused}");
}
