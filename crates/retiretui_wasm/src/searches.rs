//! What runs over a resolved plan's text alone, so that a worker can run
//! it: the gate, the searches, the markets, and the example plans.

use retiretui_client::issues::issue_listing;
use retiretui_client::ladder::LadderConstraints;
use retiretui_client::replies::{ClaimsReply, HistoricalReply, MonteCarloReply, SweepReply};
use retiretui_client::searches::run_refusal;
use retiretui_client::setup::EXAMPLES;
use retiretui_engine::market::{self, History, Progress};
use retiretui_engine::optimize::{optimize_claims, sweep_brackets};
use retiretui_engine::plan::{Issue, Plan};
use retiretui_engine::project::validate_plan;
use serde::Serialize;

use crate::tables;

/// An example plan, as a first plan is chosen from them.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Example {
    /// The file it is kept as.
    pub file: &'static str,
    /// The words it is offered under.
    pub words: &'static str,
    /// The plan.
    pub text: &'static str,
}

/// Every example plan.
#[must_use]
pub fn examples() -> Vec<Example> {
    EXAMPLES
        .iter()
        .map(|&(file, words, text)| Example { file, words, text })
        .collect()
}

/// What the full gate finds wrong with `text`.
///
/// # Errors
///
/// Where `text` is not a plan.
pub fn validate(text: &str) -> Result<Vec<Issue>, String> {
    Ok(validate_plan(&parse(text)?, tables()))
}

/// Every bracket's conversion ladder into `destination`, best first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or the search refuses it.
pub fn sweep(text: &str, destination: &str, deflated: bool) -> Result<SweepReply, String> {
    let options = LadderConstraints::default().options(&[], destination);
    let sweep = sweep_brackets(&gated(text)?, tables(), &options, &Progress::default())
        .map_err(run_refusal)?;
    Ok(SweepReply::new(&sweep, deflated))
}

/// Every claim age for the household's computed benefits, best first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or the search refuses it.
pub fn claims(text: &str, deflated: bool) -> Result<ClaimsReply, String> {
    let search = optimize_claims(&gated(text)?, tables(), &[], &[], &Progress::default())
        .map_err(run_refusal)?;
    Ok(ClaimsReply::new(&search, deflated))
}

/// The plan through the random markets its settings draw.
///
/// # Errors
///
/// Where the plan does not pass the gate, or cannot be run.
pub fn monte_carlo(text: &str) -> Result<MonteCarloReply, String> {
    let plan = gated(text)?;
    let found = market::monte_carlo(&plan, tables(), History::embedded(), &Progress::default())
        .map_err(run_refusal)?;
    Ok(MonteCarloReply::new(&plan, &found))
}

/// The plan from every historical start year.
///
/// # Errors
///
/// Where the plan does not pass the gate, or cannot be run.
pub fn historical(text: &str) -> Result<HistoricalReply, String> {
    let plan = gated(text)?;
    let runs = market::historical(&plan, tables(), History::embedded(), &Progress::default())
        .map_err(run_refusal)?;
    Ok(HistoricalReply::new(&plan, &runs))
}

fn parse(text: &str) -> Result<Plan, String> {
    Plan::from_toml_str(text).map_err(|error| error.to_string())
}

fn gated(text: &str) -> Result<Plan, String> {
    let plan = parse(text)?;
    let issues = validate_plan(&plan, tables());
    if issues.is_empty() {
        Ok(plan)
    } else {
        Err(issue_listing(&issues))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starter() -> &'static str {
        EXAMPLES[0].2
    }

    /// The starter's Roth IRA.
    const ROTH: &str = "roth-ira-sam";

    #[test]
    fn every_example_passes_the_gate() {
        for example in examples() {
            assert_eq!(validate(example.text), Ok(Vec::new()), "{}", example.file);
        }
        assert!(validate("schema = [").is_err());
    }

    #[test]
    fn the_searches_answer_over_an_example() {
        let text = starter();
        let swept = sweep(text, ROTH, true).expect("sweeps");
        assert!(!swept.brackets.is_empty());
        assert!(!claims(text, true).expect("searches").candidates.is_empty());
        let historical = historical(text).expect("runs");
        assert!(!historical.start_years.is_empty());
        assert!((0.0..=1.0).contains(&historical.success_rate));
        assert!(monte_carlo(text).expect("runs").runs > 0);
    }

    #[test]
    fn a_plan_the_gate_refuses_is_not_searched() {
        let mut plan = parse(starter()).expect("parses");
        plan.plan.inflation = 5.0;
        let text = plan.to_toml_string().expect("serializes");
        assert!(historical(&text).is_err());
        assert!(sweep(starter(), "no-such-account", true).is_err());
    }
}
