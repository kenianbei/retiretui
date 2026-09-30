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

/// What recording `statement` on the person called `name` did: the years
/// recorded, and the statement's note on any it spread.
#[must_use]
pub fn recorded(name: &str, statement: &Statement) -> String {
    let years = crate::present::counted(statement.earnings.len(), "year", "years");
    let said = format!("recorded {years} of earnings for {name}");
    match statement.spread_note() {
        Some(note) => format!("{said}; {note}"),
        None => said,
    }
}

/// The plan `text` holds with a statement's earnings recorded on `person`,
/// and the statement's note on any years it spread. A scenario is refused:
/// the plan is written back whole, which would flatten it, so the record
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

#[cfg(test)]
mod tests {
    use retiretui_engine::plan::PlanDate;

    use super::*;

    #[test]
    fn what_was_recorded_names_the_years_and_any_spread() {
        let mut statement = Statement {
            birth: PlanDate(jiff::civil::date(1975, 6, 14)),
            earnings: [(2023, 1), (2024, 2)].into(),
            grouped: Vec::new(),
        };
        assert_eq!(
            recorded("Jordan", &statement),
            "recorded 2 years of earnings for Jordan"
        );
        statement.grouped.push((2023, 2024));
        assert!(recorded("Jordan", &statement).ends_with(
            "; earnings stated as one sum for 2023-2024 were spread evenly over those years"
        ));
    }
}
