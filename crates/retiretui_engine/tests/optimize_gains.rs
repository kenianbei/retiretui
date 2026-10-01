//! The long-term gains ceiling on a conversion ladder, over a zero-inflation
//! plan so every figure hand-checks.

mod common;

use retiretui_engine::optimize::{GainsRate, OptimizeOptions, SweptBracket};
use retiretui_engine::project::YearRow;

use common::{ladder_options, params_2026, plan_from, searched_ladder};

/// Lives off a brokerage, five sixths of every dollar drawn from it gain.
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

fn searched(text: &str, gains_rate: Option<GainsRate>, rate: f64) -> SweptBracket {
    let options = OptimizeOptions {
        gains_rate,
        ..ladder_options()
    };
    searched_ladder(&plan_from(text), &options, rate)
}

/// Where the year's gains stack ends: ordinary taxable income plus gains.
fn stack_end(row: &YearRow) -> i64 {
    row.taxes.ordinary_taxable + row.taxes.gains
}

#[test]
fn a_ladder_held_at_zero_converts_only_while_gains_stay_untaxed() {
    let plain = searched(LIVING_OFF_GAINS, None, 0.12);
    assert!(plain.optimized.years.iter().any(|row| row.taxes.ltcg > 0));

    let held = searched(LIVING_OFF_GAINS, Some(GainsRate::Zero), 0.12);
    assert_eq!(
        held.steps[0].amount, 30_939,
        "taxable 14,839 under gains of 34,611 on 41,533 drawn ends at 49,450"
    );
    assert!(held.converted(false) < plain.converted(false));
    for step in &held.steps {
        let row = held.optimized.row(step.year).unwrap();
        assert!(row.taxes.gains > 0, "{}", step.year);
        assert_eq!(row.taxes.ltcg, 0, "{}", step.year);
    }
}

#[test]
fn a_year_that_realizes_no_gain_is_not_held() {
    let off_cash = LIVING_OFF_GAINS
        .replace("kind = \"brokerage\"", "kind = \"cash\"")
        .replace("basis = 100000\n", "");
    let plain = searched(&off_cash, None, 0.12);
    let held = searched(&off_cash, Some(GainsRate::Zero), 0.12);
    assert_eq!(held.steps, plain.steps);
    let first = plain.optimized.row(2026).unwrap();
    assert!(
        first.taxes.ordinary_taxable > params_2026().ltcg.zero_until.single,
        "the 12% bracket's top is past the 0% top, so holding it would bind"
    );
}

#[test]
fn a_year_whose_gains_already_pass_the_top_converts_nothing() {
    let spending_more = LIVING_OFF_GAINS.replace("amount = 40000", "amount = 80000");
    let held = searched(&spending_more, Some(GainsRate::Zero), 0.12);
    assert_eq!(held.steps, []);
    let first = held.optimized.row(2026).unwrap();
    assert!(first.taxes.gains > params_2026().ltcg.zero_until.single);
}

/// One year of a 35% fill, whose tax is drawn from the brokerage.
fn top_fill(gains_rate: Option<GainsRate>) -> SweptBracket {
    let options = OptimizeOptions {
        gains_rate,
        end_year: Some(2026),
        ..ladder_options()
    };
    searched_ladder(&plan_from(LIVING_OFF_GAINS), &options, 0.35)
}

#[test]
fn a_ladder_held_at_fifteen_keeps_gains_out_of_the_top_rate() {
    let fifteen_top = params_2026().ltcg.fifteen_until.single;
    let plain = top_fill(None);
    assert!(stack_end(plain.optimized.row(2026).unwrap()) > fifteen_top);

    let held = top_fill(Some(GainsRate::Fifteen));
    let row = held.optimized.row(2026).unwrap();
    assert!(row.taxes.gains > 0 && row.conversions > 0);
    assert_eq!(stack_end(row), fifteen_top, "fills to the 15% top");
}
