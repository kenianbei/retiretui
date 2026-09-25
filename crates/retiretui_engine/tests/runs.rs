//! A plan run through many markets: what each run keeps, what the search
//! singles out, and how it stops.

use retiretui_engine::market::{
    History, Progress, RunError, RunName, draw, historical, monte_carlo, replay,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{deflate, project_on};

fn plan(market: &str) -> Plan {
    let plan = Plan::from_toml_str(&format!(
        r#"
schema = 1

[plan]
start_year = 2026
horizon_age = 95
inflation = 0.025

[household]
filing = "single"

[[household.people]]
id = "me"
birth = 1961-01-01

[[accounts]]
id = "cash"
kind = "cash"
owner = "me"
balance = 0

[[accounts]]
id = "ira"
kind = "ira"
owner = "me"
balance = 1200000
allocation = {{ stocks = 0.6, bonds = 0.4 }}

[[expenses]]
id = "living"
amount = 60000

[market.monte_carlo]
trials = 200
{market}
"#
    ))
    .unwrap();
    assert!(plan.validate().is_empty(), "{:?}", plan.validate());
    plan
}

const STILL: &str = "[market.stocks]\nvolatility = 0.0\n[market.bonds]\nvolatility = 0.0\n\
                     [market.cash]\nvolatility = 0.0\n[market.inflation]\nvolatility = 0.0\n";

#[test]
fn without_spread_every_market_is_the_plans_own() {
    let plan = plan(STILL);
    let found = monte_carlo(
        &plan,
        &TaxTables::embedded(),
        History::embedded(),
        &Progress::default(),
    )
    .unwrap();
    let runs = &found.runs;
    assert_eq!(runs.runs.len(), 200);
    assert!(
        runs.runs
            .iter()
            .all(|run| run.net_worth == runs.planned.net_worth)
    );
    let rate = if runs.planned.is_success { 1.0 } else { 0.0 };
    assert!((runs.success_rate() - rate).abs() < f64::EPSILON);
}

#[test]
fn inflation_alone_still_moves_the_runs() {
    let plan = plan(
        "[market.stocks]\nvolatility = 0.0\n[market.bonds]\nvolatility = 0.0\n\
         [market.cash]\nvolatility = 0.0\n",
    );
    let found = monte_carlo(
        &plan,
        &TaxTables::embedded(),
        History::embedded(),
        &Progress::default(),
    )
    .unwrap();
    let first = &found.runs.runs[0].net_worth;
    assert!(found.runs.runs.iter().any(|run| &run.net_worth != first));
}

#[test]
fn each_trial_is_its_own_market_however_the_threads_split_them() {
    let plan = plan("");
    let history = History::embedded();
    let found = monte_carlo(&plan, &TaxTables::embedded(), history, &Progress::default()).unwrap();
    for trial in [0_u32, 57, 199] {
        let projection = project_on(&plan, &TaxTables::embedded(), &draw(&plan, history, trial));
        let worths: Vec<i64> = projection
            .years
            .iter()
            .map(|row| deflate(row.net_worth, row.deflator))
            .collect();
        let run = &found.runs.runs[usize::try_from(trial).unwrap()];
        assert_eq!(run.name, RunName::Trial(trial));
        assert_eq!(run.net_worth, worths);
    }
}

#[test]
fn the_singled_out_markets_run_from_best_to_worst() {
    let plan = plan("");
    let found = monte_carlo(
        &plan,
        &TaxTables::embedded(),
        History::embedded(),
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(found.singled_out.len(), 6);
    let key = |at: usize| {
        let run = &found.runs.runs[at];
        (run.unfunded, std::cmp::Reverse(run.ending))
    };
    assert!(
        found
            .singled_out
            .windows(2)
            .all(|pair| key(pair[0]) <= key(pair[1]))
    );
    assert_eq!(found.singled_out.last(), found.runs.worst_first().first());
    assert!(
        found.runs.successes > 0 && found.runs.successes < 200,
        "a plan on the edge"
    );
}

#[test]
fn a_floor_above_every_ending_fails_every_market() {
    let plan = plan(&format!("{STILL}[market]\nleave_at_least = 900000000\n"));
    let found = monte_carlo(
        &plan,
        &TaxTables::embedded(),
        History::embedded(),
        &Progress::default(),
    )
    .unwrap();
    assert_eq!(found.runs.successes, 0);
}

#[test]
fn a_cancelled_search_answers_nothing() {
    let plan = plan("");
    let progress = Progress::default();
    progress.cancel();
    let answer = monte_carlo(
        &plan,
        &TaxTables::embedded(),
        History::embedded(),
        &progress,
    );
    assert_eq!(answer, Err(RunError::Cancelled));
    assert_eq!(progress.done(), 0);
}

#[test]
fn history_runs_every_start_year_and_lists_them_worst_first() {
    let plan = plan("");
    let history = History::embedded();
    let found = historical(&plan, &TaxTables::embedded(), history, &Progress::default()).unwrap();
    assert_eq!(
        found.runs.len(),
        155,
        "wrapped, every year from 1871 to 2025"
    );
    let order = found.worst_first();
    let key = |at: usize| {
        let run = &found.runs[at];
        (run.unfunded, std::cmp::Reverse(run.ending))
    };
    assert!(order.windows(2).all(|pair| key(pair[0]) >= key(pair[1])));
    let worst = &found.runs[order[0]];
    let replayed = replay(&plan, &TaxTables::embedded(), history, worst.name).unwrap();
    let worths: Vec<i64> = replayed
        .years
        .iter()
        .map(|row| deflate(row.net_worth, row.deflator))
        .collect();
    assert_eq!(worths, worst.net_worth, "a replay is the run it names");
}

#[test]
fn history_refuses_years_it_does_not_hold_or_cannot_run_whole() {
    let history = History::embedded();
    let outside = plan("[market.historical]\nfrom = 1800\n");
    let Err(RunError::Refused(issues)) = historical(
        &outside,
        &TaxTables::embedded(),
        history,
        &Progress::default(),
    ) else {
        panic!("1800 is before the record");
    };
    assert_eq!(issues[0].path, "market.historical.from");
    let unwrapped = plan("[market.historical]\nfrom = 2000\nwrap = false\n");
    let Err(RunError::Refused(issues)) = historical(
        &unwrapped,
        &TaxTables::embedded(),
        history,
        &Progress::default(),
    ) else {
        panic!("no year from 2000 has 35 years of history");
    };
    assert_eq!(issues[0].path, "market.historical.wrap");
}
