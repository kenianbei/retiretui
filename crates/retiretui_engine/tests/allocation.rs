//! An account's allocation and the plan's `[market]`: what each accepts,
//! how the ledger grows a mix, and how a scenario overrides them.

mod common;

use retiretui_engine::plan::{Plan, Scenario};
use retiretui_engine::project::Projection;

use common::{assert_issue, issues, run};

fn plan(accounts: &str, market: &str) -> String {
    format!(
        r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 50
inflation = 0.0

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
{accounts}
{market}
"#
    )
}

const MARKET: &str = "[market.stocks]\nmean = 0.10\n\n[market.bonds]\nmean = 0.0\n";

fn growth(projection: &Projection, year: usize, account: &str) -> i64 {
    projection.years[year]
        .growth
        .get(account)
        .copied()
        .unwrap_or_default()
}

#[test]
fn an_allocation_is_refused_beside_an_expected_return_and_must_add_up() {
    let text = plan(
        r#"
[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 1
expected_return = 0.05
allocation = { stocks = 0.6, bonds = 0.3 }

[[accounts]]
id = "b"
kind = "brokerage"
owner = "me"
balance = 1
allocation = { stocks = 1.5 }
"#,
        "",
    );
    let found = issues(&text);
    assert_issue(&found, "accounts[1].expected_return", "earns its mix");
    assert_issue(&found, "accounts[1].allocation", "add up to 1");
    assert_issue(&found, "accounts[2].allocation", "between 0 and 1");
}

#[test]
fn a_glide_path_needs_a_step_and_its_triggers_must_resolve() {
    let text = plan(
        r#"
[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 1
allocation = []

[[accounts]]
id = "b"
kind = "brokerage"
owner = "me"
balance = 1
allocation = [{ from = { event = "nowhere" }, stocks = 1.0 }]
"#,
        "",
    );
    let found = issues(&text);
    assert_issue(&found, "accounts[1].allocation", "at least one step");
    assert_issue(&found, "accounts[2].allocation[0].from", "nowhere");
}

#[test]
fn market_figures_are_held_to_what_a_market_can_do() {
    let text = plan(
        "",
        r"
[market]
leave_at_least = -1

[market.stocks]
volatility = -0.1

[market.inflation]
persistence = 1.0

[market.correlation]
stocks_bonds = 0.9
stocks_cash = 0.9
bonds_cash = -0.9

[market.monte_carlo]
trials = 0
block_years = 0

[market.historical]
from = 2000
to = 1990
",
    );
    let found = issues(&text);
    assert_issue(&found, "market.leave_at_least", "negative");
    assert_issue(&found, "market.stocks.volatility", "between 0 and 1");
    assert_issue(&found, "market.inflation.persistence", "between 0 and 0.99");
    assert_issue(&found, "market.correlation", "cannot all hold");
    assert_issue(&found, "market.monte_carlo.trials", "between 1");
    assert_issue(&found, "market.monte_carlo.block_years", "between 1");
    assert_issue(&found, "market.historical.to", "before the first");
}

#[test]
fn a_mix_grows_at_its_classes_blended_means() {
    let text = plan(
        r#"
[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 100000
allocation = { stocks = 0.6, bonds = 0.4 }
"#,
        MARKET,
    );
    assert_eq!(growth(&run(&text), 0, "k"), 6_000);
}

#[test]
fn a_glide_path_holds_its_first_step_until_one_fires_then_steps() {
    let text = plan(
        r#"
[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 100000
allocation = [
  { from = { date = 2027-01-01 }, stocks = 1.0 },
  { from = { date = 2028-01-01 }, bonds = 1.0 },
]
"#,
        MARKET,
    );
    let projection = run(&text);
    assert_eq!(growth(&projection, 0, "k"), 10_000, "before any step fires");
    assert_eq!(growth(&projection, 1, "k"), 11_000, "the first step");
    assert_eq!(growth(&projection, 2, "k"), 0, "the second step");
}

#[test]
fn a_withdrawal_after_a_loss_takes_no_more_than_itself_untaxed() {
    let text = plan(
        r#"
[[accounts]]
id = "b"
kind = "brokerage"
owner = "me"
balance = 100000
allocation = { stocks = 1.0 }

[[income]]
id = "income-1"
kind = "pension"
owner = "me"
amount = 30000

[[expenses]]
id = "living"
amount = 60000
"#,
        "[market.stocks]\nmean = -0.5\n",
    );
    let projection = run(&text);
    let first = &projection.years[0];
    assert_eq!(growth(&projection, 0, "b"), -50_000);
    assert!(
        first
            .withdrawals
            .get("b")
            .is_some_and(|&taken| taken > 30_000)
    );
    assert_eq!(
        first.taxes.magi, 30_000,
        "the pension alone: a loss leaves no negative gain to offset it"
    );
}

#[test]
fn a_scenario_restating_one_market_table_keeps_the_rest() {
    let base = plan(
        r#"
[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 1
allocation = { stocks = 1.0 }
"#,
        "[market]\nleave_at_least = 5000\n\n[market.monte_carlo]\nseed = 7\n\n[market.stocks]\nmean = 0.08\nvolatility = 0.2\n",
    );
    let overlay = "schema = 1\nbase = \"base.toml\"\n\n[market.stocks]\nmean = 0.03\n\n[[accounts]]\nid = \"k\"\nallocation = { bonds = 1.0 }\n";
    let merged = Scenario::from_toml_str(overlay)
        .unwrap()
        .unwrap()
        .apply(toml::from_str(&base).unwrap())
        .unwrap();
    let plan = Plan::from_toml_table(merged).unwrap();
    let market = plan.market();
    assert_eq!(market.leave_at_least(), Some(5_000));
    assert_eq!(market.seed(), 7);
    let stocks = market.stocks.unwrap();
    assert_eq!(stocks.mean, Some(0.03));
    assert_eq!(
        stocks.volatility, None,
        "a stated table replaces the base's"
    );
    assert!(plan.validate().is_empty());
}

#[test]
fn every_market_field_and_a_glide_path_round_trip() {
    let text = plan(
        r#"
[[accounts]]
id = "k"
kind = "401k"
owner = "me"
balance = 1
allocation = [
  { from = { date = 2030-01-01 }, stocks = 0.9, bonds = 0.1 },
  { from = { age = 65, owner = "me" }, stocks = 0.5, bonds = 0.4, cash = 0.1 },
]

[[accounts]]
id = "b"
kind = "brokerage"
owner = "me"
balance = 1
allocation = { stocks = 0.7, bonds = 0.3 }
"#,
        r#"
[market]
leave_at_least = 1000
stocks = { mean = 0.05, volatility = 0.15 }
bonds = { mean = 0.03, volatility = 0.05 }
cash = { mean = 0.02, volatility = 0.01 }
inflation = { volatility = 0.01, persistence = 0.5 }
correlation = { stocks_bonds = 0.2, stocks_cash = 0.0, stocks_inflation = 0.0, bonds_cash = 0.1, bonds_inflation = -0.1, cash_inflation = 0.4 }
monte_carlo = { draw = "history", trials = 500, seed = 3, block_years = 5 }
historical = { from = 1900, to = 2000, wrap = false }
"#,
    );
    let parsed = Plan::from_toml_str(&text).unwrap();
    assert!(parsed.validate().is_empty(), "{:?}", parsed.validate());
    let written = parsed.to_toml_string().unwrap();
    assert_eq!(Plan::from_toml_str(&written).unwrap(), parsed, "{written}");
}
