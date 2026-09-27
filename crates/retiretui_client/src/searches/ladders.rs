//! The Roth conversion ladders the planner searches: the constraints a
//! person holds them to, and the sweep or single bracket searched under them.

use retiretui_engine::market::{Progress, RunError};
use retiretui_engine::optimize::{
    BracketSweep, OptimizeOptions, SweptBracket, optimize_conversions, sweep_brackets,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use serde::Deserialize;

use crate::codec::from_table;
use crate::draft::Draft;
use crate::forms::offers::RefSource;
use crate::forms::{FieldSpec, ToolAnswers};
use crate::ladder::LadderConstraints;

/// The constraints as the form holds them: the CLI's flags, blank where
/// its are optional; `bracket` is a percent, blank sweeping every one.
#[derive(Deserialize)]
pub struct Constraints {
    from: Option<String>,
    to: Option<String>,
    bracket: Option<u8>,
    #[serde(flatten)]
    held: LadderConstraints,
}

/// The Roth Conversions form's fields.
pub const FIELDS: &[FieldSpec] = &[
    FieldSpec::refers("from", "Convert from", RefSource::DeferredAccount)
        .blank("Every deferred account")
        .help("The tax-deferred account to convert out of."),
    FieldSpec::refers("to", "Convert to", RefSource::RothAccount)
        .help("The Roth account the conversions land in."),
    FieldSpec::whole("bracket", "Fill bracket")
        .help("Fill this tax bracket, as a percent such as 22. Blank tries every bracket."),
    FieldSpec::whole("start_year", "First year")
        .help("The first year to convert in. Blank means the start of the plan."),
    FieldSpec::whole("end_year", "Last year")
        .help("The last year to convert in. Blank means the end of the plan."),
    FieldSpec::money("annual_max", "Annual cap")
        .help("The most to convert in any one year. Blank sets no cap."),
    FieldSpec::money("total_max", "Total cap")
        .help("The most to convert over the whole ladder. Blank sets no cap."),
    FieldSpec::money("headroom", "Headroom")
        .help("Dollars to stay below the top of the bracket, as a margin for error."),
    FieldSpec::whole("irmaa_tier", "IRMAA tier")
        .help("Stay under this Medicare surcharge tier; 0 avoids them all. Blank ignores it."),
    FieldSpec::money("max_magi", "MAGI cap")
        .help("Keep every year's MAGI (modified adjusted gross income) under this."),
];

impl ToolAnswers for Constraints {
    const SLOT: &'static str = "optimizer";
}

const NO_DESTINATION: &str = "no destination account";

impl Constraints {
    /// The engine's options, and the one bracket rate or `None` to sweep.
    fn options(self) -> Result<(OptimizeOptions, Option<f64>), String> {
        let destination = self.to.ok_or_else(|| NO_DESTINATION.to_owned())?;
        let sources: Vec<String> = self.from.into_iter().collect();
        let options = self.held.options(&sources, &destination);
        let bracket = self.bracket.map(|percent| f64::from(percent) / 100.0);
        Ok((options, bracket))
    }
}

/// The key the destination account is under.
pub const DESTINATION: &str = "to";

/// The answers the draft holds but the destination, which each Roth
/// owner's search names for itself.
#[must_use]
pub fn held_answers(draft: &Draft) -> toml::Table {
    let mut answers = draft.answers::<Constraints>();
    answers.remove(DESTINATION);
    answers
}

/// The options the page searches under with the `held` answers into
/// `destination`, and the one bracket rate or `None` to sweep.
#[must_use]
pub fn options_into(
    held: &toml::Table,
    destination: &str,
) -> Option<(OptimizeOptions, Option<f64>)> {
    let mut answers = held.clone();
    answers.insert(DESTINATION.to_owned(), destination.into());
    let constraints = from_table::<Constraints>(answers).ok()?;
    constraints.options().ok()
}

/// Sets the form to search into `destination`, keeping the rest of what
/// it holds.
pub fn aim_at(draft: &mut Draft, destination: &str) {
    let mut answers = draft.answers::<Constraints>();
    answers.insert(DESTINATION.to_owned(), destination.into());
    draft.set_answers::<Constraints>(answers);
}

/// The constraints the draft holds, and what the engine is to be asked
/// under them.
///
/// # Errors
///
/// Where the answers do not read, or name no destination.
pub fn held(draft: &Draft) -> Result<(OptimizeOptions, Option<f64>), String> {
    from_table::<Constraints>(draft.answers::<Constraints>()).and_then(Constraints::options)
}

/// What a search found, and what it was searched under.
#[derive(Clone)]
pub struct Swept {
    /// The ladders, best first, beside the plan without one.
    pub sweep: BracketSweep,
    /// What they were searched under.
    pub options: OptimizeOptions,
}

impl Swept {
    /// The best bracket's ladder, where any bracket was searched.
    #[must_use]
    pub fn best(&self) -> Option<&SweptBracket> {
        self.sweep.brackets.first()
    }
}

/// The ladders into `destination` under the `held` answers, searched as
/// the page searches them; none where the answers do not make a search.
pub fn sweep_into(
    plan: &Plan,
    tables: &TaxTables,
    held: &toml::Table,
    destination: &str,
    progress: &Progress,
) -> Option<Swept> {
    let (options, rate) = options_into(held, destination)?;
    let sweep = search(plan, tables, &options, rate, progress).ok()?;
    Some(Swept { sweep, options })
}

/// A bracket's rate as a percent.
#[must_use]
pub fn rate_label(rate: f64) -> String {
    format!("{:.0}%", rate * 100.0)
}

/// The `rate` bracket's ladder, or every bracket's best first with none.
///
/// # Errors
///
/// Where the search is cancelled, or the plan refuses it.
pub fn search(
    plan: &Plan,
    tables: &TaxTables,
    options: &OptimizeOptions,
    rate: Option<f64>,
    progress: &Progress,
) -> Result<BracketSweep, RunError> {
    match rate {
        Some(rate) => optimize_conversions(plan, tables, options, rate)
            .map(|ladder| BracketSweep {
                baseline: ladder.baseline,
                brackets: vec![ladder.ladder],
            })
            .map_err(RunError::Refused),
        None => sweep_brackets(plan, tables, options, progress),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::examples::named;

    fn early_retiree() -> Plan {
        let (_, _, text) = named("early-retiree.toml").expect("the example");
        Plan::from_toml_str(text).expect("the example parses")
    }

    #[test]
    fn a_held_bracket_searches_that_one_ladder_into_the_destination() {
        let plan = early_retiree();
        let tables = TaxTables::embedded();
        let held: toml::Table = "bracket = 22\nfrom = \"401k-morgan\"".parse().unwrap();
        let swept = sweep_into(
            &plan,
            &tables,
            &held,
            "roth-ira-morgan",
            &Progress::default(),
        )
        .expect("a search");
        assert_eq!(swept.options.destination, "roth-ira-morgan");
        assert_eq!(swept.options.sources, ["401k-morgan"]);
        let rates: Vec<f64> = swept.sweep.brackets.iter().map(|b| b.rate).collect();
        assert_eq!(rates, [0.22]);
    }

    #[test]
    fn answers_that_do_not_read_make_no_search() {
        let held: toml::Table = "bracket = \"high\"".parse().unwrap();
        assert!(options_into(&held, "roth-ira-morgan").is_none());
        let cancelled = Progress::default();
        cancelled.cancel();
        let swept = sweep_into(
            &early_retiree(),
            &TaxTables::embedded(),
            &toml::Table::new(),
            "roth-ira-morgan",
            &cancelled,
        );
        assert!(swept.is_none(), "a cancelled sweep answers nothing");
    }

    #[test]
    fn a_rate_reads_as_a_whole_percent() {
        assert_eq!(rate_label(0.22), "22%");
        assert_eq!(rate_label(0.1), "10%");
    }
}
