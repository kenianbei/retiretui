use serde::Serialize;

use crate::plan::{
    ColaSpec, Conversion, Dollars, Plan, PlanDate, PlanError, SCHEMA_VERSION, Trigger,
};

use super::{LadderStep, OptimizeOptions};

/// The scenario overlay for a searched ladder, as canonical TOML: `base`
/// plus one `[[conversions]]` entry per step, ids `opt-<source>-<year>`,
/// nominal amounts frozen with `cola = false`, and a `remove = true`
/// marker for each ladder conversion of `over` - the plan `base` names -
/// that the new ladder does not restate. Written by the serializer plans
/// are, so a conversion reads here as it does in a plan file.
///
/// # Errors
///
/// Returns [`PlanError::Serialize`] when serialization fails.
pub fn ladder_overlay(
    base: &str,
    over: &Plan,
    options: &OptimizeOptions,
    steps: &[LadderStep],
) -> Result<String, PlanError> {
    #[derive(Serialize)]
    #[serde(untagged)]
    enum LadderFragment<'a> {
        Stated(Box<Conversion>),
        Removed { id: &'a str, remove: bool },
    }
    #[derive(Serialize)]
    struct OverlayDocument<'a> {
        schema: u32,
        base: &'a str,
        conversions: Vec<LadderFragment<'a>>,
    }
    let stated = ladder_conversions(options, steps);
    let removed: Vec<LadderFragment> = over
        .conversions
        .iter()
        .filter(|conversion| is_ladder(conversion))
        .map(|conversion| conversion.id.as_str())
        .filter(|id| !stated.iter().any(|new| new.id == *id))
        .map(|id| LadderFragment::Removed { id, remove: true })
        .collect();
    let conversions = stated
        .into_iter()
        .map(|conversion| LadderFragment::Stated(Box::new(conversion)))
        .chain(removed)
        .collect();
    Ok(toml::to_string_pretty(&OverlayDocument {
        schema: SCHEMA_VERSION,
        base,
        conversions,
    })?)
}

/// What every id a ladder gives out starts with, so a plan can tell the
/// optimizer's conversions from its own.
pub const LADDER_ID_PREFIX: &str = "opt-";

/// Whether `conversion` is one a ladder put in a plan: its id is exactly
/// `opt-<source>-<year>`, as a ladder names what it puts there.
#[must_use]
pub fn is_ladder(conversion: &Conversion) -> bool {
    let Some(year) = conversion
        .id
        .strip_prefix(LADDER_ID_PREFIX)
        .and_then(|rest| rest.strip_prefix(conversion.from.as_str()))
        .and_then(|rest| rest.strip_prefix('-'))
    else {
        return false;
    };
    year.parse::<i16>()
        .is_ok_and(|year| ladder_id(&conversion.from, year) == conversion.id)
}

fn ladder_id(source: &str, year: i16) -> String {
    format!("{LADDER_ID_PREFIX}{source}-{year}")
}

/// Takes the ladder `steps` into `plan` as conversions of its own, in
/// place of any ladder it held.
pub fn apply_ladder(plan: &mut Plan, options: &OptimizeOptions, steps: &[LadderStep]) {
    plan.conversions.retain(|conversion| !is_ladder(conversion));
    plan.conversions.extend(ladder_conversions(options, steps));
}

/// The ladder as plan items: one conversion per step into
/// `options.destination`, on January 1st of its year, its nominal amount
/// frozen with `cola = false`, under the id `opt-<source>-<year>`.
#[must_use]
fn ladder_conversions(options: &OptimizeOptions, steps: &[LadderStep]) -> Vec<Conversion> {
    steps
        .iter()
        .map(|step| ladder_conversion(&step.source, &options.destination, step.year, step.amount))
        .collect()
}

pub(super) fn ladder_conversion(
    source: &str,
    destination: &str,
    year: i16,
    amount: Dollars,
) -> Conversion {
    Conversion {
        id: ladder_id(source, year),
        name: None,
        from: source.to_owned(),
        to: destination.to_owned(),
        amount,
        start: None,
        end: None,
        on: Some(Trigger {
            date: Some(PlanDate(
                jiff::civil::Date::new(year, 1, 1).expect("January 1st of a validated year"),
            )),
            age: None,
            owner: None,
            event: None,
            income: None,
            offset: None,
        }),
        cola: ColaSpec::Follows(false),
    }
}
