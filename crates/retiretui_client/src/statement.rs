//! A Social Security statement's earnings, recorded on a person of a plan.

use retiretui_engine::plan::{Plan, Scenario};
use retiretui_engine::statement::{self, Statement};

/// The statement `xml` holds, once its earnings are recorded on `person`
/// of `plan`.
///
/// # Errors
///
/// Where the statement does not parse, or `person` is not the plan's.
pub fn record(plan: &mut Plan, person: &str, xml: &str) -> Result<Statement, String> {
    let statement = statement::parse(xml).map_err(|error| error.to_string())?;
    plan.adopt_earnings(person, &statement)
        .map_err(|issue| issue.message)?;
    Ok(statement)
}

/// The plan `text` holds with a statement's earnings recorded on `person`,
/// and the statement's note on any years it spread. A scenario is refused:
/// a resolved plan cannot be written back into an overlay, so the record
/// belongs in its base plan.
///
/// # Errors
///
/// Where the text is a scenario or does not read, the statement does not
/// parse, or `person` is not the plan's.
pub fn adopt_statement(
    text: &str,
    person: &str,
    xml: &str,
) -> Result<(Plan, Option<String>), String> {
    let is_scenario = Scenario::from_toml_str(text)
        .map_err(|error| error.to_string())?
        .is_some();
    if is_scenario {
        return Err("a scenario; import the record into its base plan".to_owned());
    }
    let mut plan = Plan::from_toml_str(text).map_err(|error| error.to_string())?;
    let statement = record(&mut plan, person, xml)?;
    Ok((plan, statement.spread_note()))
}
