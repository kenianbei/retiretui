//! The Roth conversion ladders the planner searches: the constraints a
//! person holds them to, and the sweep or single bracket searched under them.

use retiretui_engine::market::{Progress, RunError};
use retiretui_engine::optimize::{
    BracketSweep, LadderStep, OptimizeOptions, SweptBracket, is_ladder, optimize_conversions,
    sweep_brackets,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Dollars, Plan};
use retiretui_engine::project::Summary;
use serde::Deserialize;

use crate::codec::from_table;
use crate::draft::Draft;
use crate::forms::offers::{RefSource, Vocabulary, ref_offers};
use crate::forms::{FieldSpec, ToolAnswers};
use crate::ladder::LadderConstraints;
use crate::present::{self, MoneyForm};

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
        .blank("Every bracket")
        .help("Fill this tax bracket, as a percent such as 22. Blank tries every bracket."),
    FieldSpec::whole("start_year", "First year")
        .blank("The plan's start")
        .help("The first year to convert in. Blank means the start of the plan."),
    FieldSpec::whole("end_year", "Last year")
        .blank("The year before RMDs")
        .help("The last year to convert in. Blank means the year before the owner's RMDs begin."),
    FieldSpec::money("annual_max", "Annual cap")
        .blank("No cap")
        .help("The most to convert in any one year. Blank sets no cap."),
    FieldSpec::money("total_max", "Total cap")
        .blank("No cap")
        .help("The most to convert over the whole ladder. Blank sets no cap."),
    FieldSpec::money("headroom", "Headroom")
        .blank("None")
        .help("Dollars left below the top of the bracket each year, so an estimate that runs high does not spill into the next bracket."),
    FieldSpec::whole("irmaa_tier", "IRMAA tier")
        .blank("Not held to one")
        .help("IRMAA is what Medicare adds to the Part B and D premiums once income passes a tier. Stay under this one; 0 stays under them all. Blank ignores it."),
    FieldSpec::money("max_magi", "MAGI cap")
        .blank("No cap")
        .help("Keep every year's MAGI (modified adjusted gross income) under this."),
    FieldSpec::choice("gains_rate", "Gains rate", Vocabulary::GainsRate)
        .blank("Not held")
        .help("Keep realized capital gains at this rate or under: 0% keeps them untaxed, 15% out of 20%. A year that realizes none is not held."),
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

/// What the tool is for, in a line.
pub const ABOUT: &str = "A ladder converts pre-tax savings to Roth yearly up to a bracket's top: tax now at that rate, not later.";

/// What names an option's columns before its [`FIGURES`](super::FIGURES).
pub const OPTION_COLUMNS: [&str; 3] = ["Bracket", super::AGAINST_PLAN, "Converted"];
/// The columns a ladder's conversions are tabled under, year by year.
pub const CONVERSION_COLUMNS: [&str; 4] = ["Year", "From", "Amount", "Taxable"];

/// What is said while no Roth account is named to convert to.
pub const PICK_DESTINATION: &str = "Pick the Roth account to convert to under Constraints, and every bracket's ladder is searched.";
/// What is said where the constraints leave no bracket to fill.
pub const NO_BRACKET: &str = "no bracket can be filled";
/// What is said in place of a ladder that converts nothing.
pub const CONVERTS_NOTHING: &str = "This ladder converts nothing under these constraints.";

/// A ladder's cells under [`OPTION_COLUMNS`] past the bracket, then its
/// [`FIGURES`](super::FIGURES), in `form`: what `own` ends with against
/// `plan`, what it converts over its life, and what it is chosen by.
#[must_use]
pub fn option_cells(own: &Summary, plan: Option<&Summary>, form: MoneyForm) -> Vec<String> {
    let mut cells = super::option_cells(own, plan, form);
    cells.insert(1, form.money(own.lifetime_conversions));
    cells
}

/// The ordinary income taxed in `year` with `bracket`'s ladder, nominal,
/// and the year's deflator.
#[must_use]
pub fn taxed_in(bracket: &SweptBracket, year: i16) -> (Dollars, f64) {
    let row = bracket.optimized.row(year);
    (
        row.map_or(0, |row| row.taxes.ordinary_taxable),
        row.map_or(1.0, |row| row.deflator),
    )
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

/// The options the Roth Conversions tool searches under with the `held`
/// answers into `destination`, and the one bracket rate or `None` to sweep.
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

/// The plan's one Roth account, where the answers name no destination and
/// there is exactly one to name.
#[must_use]
pub fn only_roth(draft: &Draft) -> Option<String> {
    if draft.answers::<Constraints>().contains_key(DESTINATION) {
        return None;
    }
    let offered = ref_offers(&draft.plan, RefSource::RothAccount);
    let [only] = offered.as_slice() else {
        return None;
    };
    Some(only.value.clone())
}

/// What is asked before `bracket`'s ladder is taken into `plan`.
#[must_use]
pub fn take_question(bracket: &SweptBracket, plan: &Plan) -> String {
    let replacing = if plan.conversions.iter().any(is_ladder) {
        ", in place of the ladder taken before"
    } else {
        ""
    };
    format!(
        "Take the {} ladder? {}{replacing}.",
        rate_label(bracket.rate),
        said(&bracket.steps)
    )
}

/// What is said once `steps` are taken into the plan.
#[must_use]
pub fn taken(steps: &[LadderStep]) -> String {
    format!("took {} into the plan", conversions(steps))
}

/// How many conversions `steps` makes: `1 conversion`, `9 conversions`.
fn conversions(steps: &[LadderStep]) -> String {
    present::counted(steps.len(), "conversion", "conversions")
}

/// A ladder as a sentence says it: `9 conversions, 2027–2035`.
fn said(steps: &[LadderStep]) -> String {
    match (steps.first(), steps.last()) {
        (Some(first), Some(last)) => {
            format!("{}, {}–{}", conversions(steps), first.year, last.year)
        }
        _ => "It converts nothing".to_owned(),
    }
}

/// The constraints the draft holds, and what the engine is to be asked
/// under them.
///
/// # Errors
///
/// Where the answers do not read, or name no destination.
pub fn held(draft: &Draft) -> Result<(OptimizeOptions, Option<f64>), String> {
    constraints_in(draft.answers::<Constraints>())
}

/// What the engine is to be asked under `answers`, the form's table.
///
/// # Errors
///
/// Where the answers do not read, or name no destination.
pub fn constraints_in(answers: toml::Table) -> Result<(OptimizeOptions, Option<f64>), String> {
    from_table::<Constraints>(answers).and_then(Constraints::options)
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
/// the Roth Conversions tool searches them; none where the answers do not
/// make a search.
pub(crate) fn sweep_into(
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
    use retiretui_engine::optimize::GainsRate;

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
    fn every_gains_rate_the_form_offers_holds_the_search_to_it() {
        let offers = Vocabulary::GainsRate.offers();
        let labels: Vec<&str> = offers.iter().map(|offer| offer.label.as_str()).collect();
        assert_eq!(labels, ["0%", "15%"]);
        for (offer, rate) in offers.iter().zip(GainsRate::ALL) {
            let mut held = toml::Table::new();
            held.insert("gains_rate".to_owned(), offer.value.clone().into());
            let (options, _) = options_into(&held, "roth-ira-morgan").expect("the pick reads");
            assert_eq!(options.gains_rate, Some(*rate));
        }
        let (options, _) = options_into(&toml::Table::new(), "roth-ira-morgan").expect("blank");
        assert_eq!(options.gains_rate, None);
    }

    #[test]
    fn only_a_lone_roth_account_is_aimed_at() {
        let mut draft = Draft::new(early_retiree(), false);
        let roth = ref_offers(&draft.plan, RefSource::RothAccount);
        assert_eq!(roth.len(), 1, "the example holds one Roth account");
        assert_eq!(only_roth(&draft).as_deref(), Some(roth[0].value.as_str()));
        aim_at(&mut draft, "elsewhere");
        assert_eq!(only_roth(&draft), None, "a named destination is kept");
    }

    #[test]
    fn the_take_question_says_the_ladder_and_what_it_replaces() {
        let mut plan = early_retiree();
        let step = |year| LadderStep {
            year,
            source: "401k-morgan".to_owned(),
            amount: 1000,
        };
        let options = LadderConstraints::default().options(&[], "roth-ira-morgan");
        let swept = search(
            &plan,
            &TaxTables::embedded(),
            &options,
            Some(0.22),
            &Progress::default(),
        )
        .expect("a ladder");
        let bracket = SweptBracket {
            steps: vec![step(2027), step(2035)],
            ..swept.brackets[0].clone()
        };
        assert_eq!(
            take_question(&bracket, &plan),
            "Take the 22% ladder? 2 conversions, 2027–2035."
        );
        retiretui_engine::optimize::apply_ladder(&mut plan, &options, &bracket.steps);
        assert!(take_question(&bracket, &plan).ends_with(", in place of the ladder taken before."));
        assert_eq!(taken(&bracket.steps), "took 2 conversions into the plan");
        let none = SweptBracket {
            steps: Vec::new(),
            ..bracket
        };
        assert!(take_question(&none, &plan).contains("It converts nothing"));
    }

    #[test]
    fn a_rate_reads_as_a_whole_percent() {
        assert_eq!(rate_label(0.22), "22%");
        assert_eq!(rate_label(0.1), "10%");
    }
}
