//! What runs over a resolved plan's text alone, so that a worker can run
//! it: the gate and the example plans.

use retiretui_client::issues::issue_listing;
use retiretui_client::setup::EXAMPLES;
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

fn parse(text: &str) -> Result<Plan, String> {
    Plan::from_toml_str(text).map_err(|error| error.to_string())
}

pub(crate) fn gated(text: &str) -> Result<Plan, String> {
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

    #[test]
    fn every_example_passes_the_gate() {
        for example in examples() {
            assert_eq!(validate(example.text), Ok(Vec::new()), "{}", example.file);
        }
        assert!(validate("schema = [").is_err());
    }

    #[test]
    fn a_plan_the_gate_refuses_is_not_searched() {
        let mut plan = parse(starter()).expect("parses");
        plan.plan.inflation = 5.0;
        let text = plan.to_toml_string().expect("serializes");
        assert!(gated(&text).is_err());
    }
}
