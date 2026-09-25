//! The markets a plan is projected through: drawn from its assumptions or
//! history, or replayed from a historical start year.

use retiretui_engine::market::{History, draw, start_in};
use retiretui_engine::plan::Plan;

fn plan(market: &str) -> Plan {
    Plan::from_toml_str(&format!(
        r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 90
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1980-01-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0
{market}
"#
    ))
    .unwrap()
}

const YEARS: i16 = 20;

fn bits(returns: [f64; 3]) -> [u64; 3] {
    returns.map(f64::to_bits)
}

fn returns(path: &retiretui_engine::project::MarketPath) -> Vec<[u64; 3]> {
    (2026..2026 + YEARS)
        .map(|year| bits(path.returns(year)))
        .collect()
}

#[test]
fn a_seed_draws_the_same_market_each_time_and_each_trial_its_own() {
    let plan = plan("");
    let history = History::embedded();
    assert_eq!(draw(&plan, history, 3), draw(&plan, history, 3));
    assert_ne!(
        returns(&draw(&plan, history, 3)),
        returns(&draw(&plan, history, 4))
    );
}

#[test]
fn a_market_without_spread_is_the_plans_own() {
    let still = plan(
        "[market]\nstocks = { volatility = 0.0 }\nbonds = { volatility = 0.0 }\n\
         cash = { volatility = 0.0 }\ninflation = { volatility = 0.0 }\n",
    );
    let drawn = draw(&still, History::embedded(), 9);
    let expected = retiretui_engine::project::MarketPath::expected(&still);
    for year in 2026..2026 + YEARS {
        assert_eq!(bits(drawn.returns(year)), bits(expected.returns(year)));
        assert_eq!(
            drawn.deflator(year).to_bits(),
            expected.deflator(year).to_bits()
        );
    }
}

#[test]
fn a_replay_walks_history_from_its_start_year() {
    let plan = plan("");
    let history = History::embedded();
    let path = start_in(&plan, history, 1966, true).unwrap();
    let recorded = |year: i16| history.years()[usize::try_from(year - 1871).unwrap()];
    assert_eq!(bits(path.returns(2026)), bits(recorded(1966).returns()));
    assert_eq!(bits(path.returns(2027)), bits(recorded(1967).returns()));
    let into_2027 = path.deflator(2027);
    assert!((into_2027 - (1.0 + recorded(1966).inflation)).abs() < 1e-12);
    assert!(
        start_in(&plan, history, 2000, false).is_none(),
        "2000 + 34 years passes 2025"
    );
    let wrapped = start_in(&plan, history, 2020, true).unwrap();
    assert_eq!(
        bits(wrapped.returns(2032)),
        bits(recorded(1871).returns()),
        "the seventh year from 2020 wraps to 1871"
    );
}

#[test]
fn a_draw_from_history_takes_its_blocks_of_consecutive_years() {
    let plan = plan("[market.monte_carlo]\ndraw = \"history\"\nblock_years = 5\n");
    let history = History::embedded();
    let path = draw(&plan, history, 1);
    let drawn = returns(&path);
    for block in drawn.chunks(5) {
        let start = history
            .years()
            .iter()
            .position(|year| bits(year.returns()) == block[0])
            .expect("every drawn year is a recorded one");
        for (k, returns) in block.iter().enumerate() {
            assert_eq!(*returns, bits(history.years()[start + k].returns()));
        }
    }
}
