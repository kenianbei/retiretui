//! The Roth Conversions tool: the constraints a conversion ladder is
//! searched under, beside every fillable bracket's ladder, best first, and
//! the highlighted one's conversions year by year.

mod guide;
mod panes;
#[cfg(test)]
pub(crate) mod tests;

use std::path::PathBuf;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{Commands, In, IntoScheduleConfigs, Local, Res, ResMut, World};
use retiretui_engine::optimize::{
    BracketSweep, LadderStep, OptimizeOptions, SweptBracket, apply_ladder, is_ladder,
    ladder_overlay, optimize_conversions, rank_key, sweep_brackets,
};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::{Issue, Plan, TreatmentClass};
use retiretui_engine::project::Projection;
use serde::Deserialize;

use super::options::{CURRENT_PLAN, FIGURES, Laid, figures};
use super::{Found, NOTHING_SEARCHED_YET, Tool, ToolPage, write};
use crate::commands::optimize::LadderConstraints;
use crate::commands::tui::command::Outcome;
use crate::commands::tui::confirm::{Answer, Confirm};
use crate::commands::tui::documents::{Browsing, Pickers};
use crate::commands::tui::edit::{self, Draft, DraftEditor, FieldSpec, FormButton, Ops, RefSource};
use crate::commands::tui::journal;
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::present::compact_dollars;
use crate::commands::tui::session::Session;

pub type Ladders = Tool<Swept>;

pub fn plugin(app: &mut App) {
    super::install::<Swept>(app, &PAGE);
    super::options::plugin::<Swept>(app);
    app.add_systems(
        Update,
        (aim_at_only_roth, search_by_itself)
            .chain()
            .before(super::poll_search::<Swept>),
    );
    panes::plugin(app);
    guide::plugin(app);
}

/// The constraints as the form holds them: the CLI's flags, blank where
/// its are optional; `bracket` is a percent, blank sweeping every one.
#[derive(Deserialize)]
struct Constraints {
    from: Option<String>,
    to: Option<String>,
    bracket: Option<u8>,
    #[serde(flatten)]
    held: LadderConstraints,
}

const FIELDS: &[FieldSpec] = &[
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

impl edit::ToolAnswers for Constraints {
    const SLOT: &'static str = "optimizer";
}

const OPS: Ops = Ops::tool::<Constraints>(Some(Page::RothConversions), "Constraints", FIELDS)
    .acting(["Discard", "Apply"], act);
const _: () = assert!(
    edit::help_fits(OPS),
    "a field's help is missing or too long"
);
const PAGE: ToolPage = ToolPage {
    surface: Page::RothConversions,
    panes: panes::spawn_panes,
};
const NO_DESTINATION: &str = "no destination account";
const NO_BRACKET: &str = "no bracket can be filled";

impl Constraints {
    /// The engine's options, and the one bracket rate or `None` to sweep.
    /// Blank sources are every deferred account of the destination's
    /// owner.
    fn options(self, plan: &Plan) -> Result<(OptimizeOptions, Option<f64>), String> {
        let destination = self.to.ok_or_else(|| NO_DESTINATION.to_owned())?;
        let sources = if let Some(source) = self.from {
            vec![source]
        } else {
            let owner = plan.account(&destination).map(|account| &account.owner);
            plan.accounts
                .iter()
                .filter(|account| {
                    account.treatment() == TreatmentClass::Deferred && Some(&account.owner) == owner
                })
                .map(|account| account.id.clone())
                .collect()
        };
        let options = self.held.options(&sources, &destination);
        let bracket = self.bracket.map(|percent| f64::from(percent) / 100.0);
        Ok((options, bracket))
    }
}

const DESTINATION: &str = "to";

/// The answers the draft holds but the destination, which each Roth
/// owner's search names for itself.
pub(crate) fn held_answers(draft: &Draft) -> toml::Table {
    let mut answers = draft.answers::<Constraints>();
    answers.remove(DESTINATION);
    answers
}

/// The options the page searches under with the `held` answers into
/// `destination`, and the one bracket rate or `None` to sweep.
pub(crate) fn options_into(
    plan: &Plan,
    held: &toml::Table,
    destination: &str,
) -> Option<(OptimizeOptions, Option<f64>)> {
    let mut answers = held.clone();
    answers.insert(DESTINATION.to_owned(), destination.into());
    let constraints = edit::from_table::<Constraints>(answers).ok()?;
    constraints.options(plan).ok()
}

/// Sets the form to search into `destination`, keeping the rest of what
/// it holds.
pub(crate) fn aim_at(draft: &mut Draft, destination: &str) {
    let mut answers = draft.answers::<Constraints>();
    answers.insert(DESTINATION.to_owned(), destination.into());
    draft.tools.insert(
        <Constraints as edit::ToolAnswers>::SLOT.to_owned(),
        toml::Value::Table(answers),
    );
}

/// The constraints the draft holds, and what the engine is to be asked
/// under them.
fn held(draft: &Draft) -> Result<(OptimizeOptions, Option<f64>), String> {
    edit::from_table::<Constraints>(draft.answers::<Constraints>())
        .and_then(|constraints| constraints.options(&draft.plan))
}

/// What a search found, and what it was searched under.
pub struct Swept {
    sweep: BracketSweep,
    options: OptimizeOptions,
}

impl Found for Swept {
    const NOTHING_SEARCHED: &'static str = "Ranked here once Convert to names a Roth account.";

    /// The plan's own row, then a row per bracket: its rate, what the plan
    /// converts over its life with that bracket's ladder, and the figures.
    fn laid(&self, _plan: &Plan, nominal: bool) -> Laid {
        let deflated = !nominal;
        let header = ["Bracket", "converted"]
            .into_iter()
            .chain(FIGURES)
            .map(str::to_owned)
            .collect();
        let row = |label: String, projection: &Projection| {
            let summary = projection.summary(deflated);
            let converted = compact_dollars(summary.lifetime_conversions);
            [label, converted]
                .into_iter()
                .chain(figures(&summary))
                .collect()
        };
        let options = self
            .sweep
            .brackets
            .iter()
            .map(|bracket| row(rate_label(bracket.rate), &bracket.optimized));
        Laid {
            header,
            current: row(CURRENT_PLAN.to_owned(), &self.sweep.baseline),
            options: options.collect(),
        }
    }
}

impl Tool<Swept> {
    /// The highlighted bracket, or the best while the plan's own row is
    /// highlighted.
    pub(super) fn highlighted_bracket(&self) -> Option<&SweptBracket> {
        let brackets = &self.found()?.sweep.brackets;
        let highlighted = self.highlighted().and_then(|at| brackets.get(at));
        highlighted.or_else(|| brackets.first())
    }
}

pub(crate) fn rate_label(rate: f64) -> String {
    format!("{:.0}%", rate * 100.0)
}

/// Applying holds the answers beside the draft without marking it
/// changed, so the tool is marked instead, for the page to search again.
fn act(which: FormButton, commands: &mut Commands) {
    if which == FormButton::Apply {
        commands.run_system_cached(mark_applied);
    }
}

fn mark_applied(mut ladders: ResMut<Ladders>) {
    ladders.set_changed();
}

/// Names the plan's one Roth account as the destination while the page is
/// on show and the form names none, so the first look is already ranked.
fn aim_at_only_roth(active: Res<ActivePage>, mut draft: ResMut<Draft>) {
    let is_moved = draft.is_changed() || active.is_changed();
    if !is_moved || active.0 != Page::RothConversions {
        return;
    }
    if draft.answers::<Constraints>().contains_key(DESTINATION) {
        return;
    }
    let offered = edit::ref_offers(&draft.plan, RefSource::RothAccount);
    let [only] = offered.as_slice() else {
        return;
    };
    aim_at(&mut draft, &only.value);
}

/// Searches again whenever the page is on show over a valid draft whose
/// plan or constraints differ from the last it searched, once they name a
/// destination, so the ranking is never asked for.
fn search_by_itself(
    (draft, session): (Res<Draft>, Res<Session>),
    active: Res<ActivePage>,
    mut searched: Local<Option<(Plan, toml::Table)>>,
    mut ladders: ResMut<Ladders>,
) {
    let is_moved = draft.is_changed() || active.is_changed() || ladders.is_changed();
    if !is_moved || active.0 != Page::RothConversions || ladders.is_running() {
        return;
    }
    let answers = draft.answers::<Constraints>();
    let is_same =
        |(plan, searched): &(Plan, toml::Table)| *plan == draft.plan && *searched == answers;
    if !super::is_due(&draft, searched.as_ref(), is_same) {
        return;
    }
    // Constraints that hold nothing are never the key searched, so
    // returning to the last ones searched does not search them again.
    let Ok((options, rate)) = held(&draft) else {
        return;
    };
    *searched = Some((draft.plan.clone(), answers));
    let tables = session.tables.clone();
    ladders.start(draft.plan.clone(), move |plan| {
        search(plan, &tables, &options, rate).map(|sweep| Swept { sweep, options })
    });
}

/// The `rate` bracket's ladder, or every bracket's with none, best first
/// as the claim search ranks; the sort is stable, so a tie keeps the
/// lower rate first.
pub(crate) fn search(
    plan: &Plan,
    tables: &TaxTables,
    options: &OptimizeOptions,
    rate: Option<f64>,
) -> Result<BracketSweep, Vec<Issue>> {
    let mut sweep = match rate {
        Some(rate) => {
            optimize_conversions(plan, tables, options, rate).map(|ladder| BracketSweep {
                baseline: ladder.baseline,
                brackets: vec![ladder.ladder],
            })
        }
        None => sweep_brackets(plan, tables, options),
    }?;
    sweep
        .brackets
        .sort_by_cached_key(|bracket| rank_key(&bracket.optimized));
    Ok(sweep)
}

/// The `take-ladder` command: asks before taking the highlighted ladder -
/// or the best while the plan's own row is highlighted - into the draft,
/// in place of any ladder taken before. The answer carries the ladder
/// asked about, so a highlight that moves under the question changes
/// nothing.
pub fn adopt(ladders: Res<Ladders>, draft: Res<Draft>, mut confirm: ResMut<Confirm>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return refusal;
    }
    let Some(swept) = ladders.found() else {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    };
    let Some(bracket) = ladders.highlighted_bracket() else {
        return Outcome::Refused(NO_BRACKET.to_owned());
    };
    let replacing = if draft.plan.conversions.iter().any(is_ladder) {
        ", in place of the ladder taken before"
    } else {
        ""
    };
    let question = format!(
        "Take the {} ladder? {}{replacing}.",
        rate_label(bracket.rate),
        said(&bracket.steps)
    );
    let asked = (swept.options.clone(), bracket.steps.clone());
    let answers = vec![
        Answer::closing("Cancel"),
        Answer::running("Take", move |commands| {
            commands.run_system_cached_with(take, asked);
        })
        .primary(),
    ];
    confirm.ask_among(question, answers);
    Outcome::Done
}

/// Takes the ladder into the draft, as one step of history.
fn take(In((options, steps)): In<(OptimizeOptions, Vec<LadderStep>)>, mut editor: DraftEditor) {
    apply_ladder(&mut editor.draft.plan, &options, &steps);
    editor.commit();
    journal::say(format!("took {} conversion(s) into the plan", steps.len()));
}

/// A ladder as a sentence says it: `9 conversion(s), 2027–2035`.
fn said(steps: &[LadderStep]) -> String {
    match (steps.first(), steps.last()) {
        (Some(first), Some(last)) => format!(
            "{} conversion(s), {}–{}",
            steps.len(),
            first.year,
            last.year
        ),
        _ => "It converts nothing".to_owned(),
    }
}

/// The `write-ladder` command: asks where to write the highlighted
/// bracket's ladder.
pub fn write_picker(
    pickers: Res<Pickers>,
    ladders: Res<Ladders>,
    draft: Res<Draft>,
    mut browsing: ResMut<Browsing>,
) -> Outcome {
    if ladders.found().is_none() {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    }
    if ladders.highlighted_bracket().is_none() {
        return Outcome::Refused(NO_BRACKET.to_owned());
    }
    write::open_picker(pickers.ladder, &draft, &mut browsing)
}

/// Writes the highlighted ladder at `path` as a scenario over the
/// document, and compares the file written.
pub fn write_overlay(In(path): In<PathBuf>, world: &mut World) {
    let text = write::base_of(world, &path).and_then(|base| {
        let ladders = world.resource::<Ladders>();
        let over = &world.resource::<Draft>().plan;
        let (swept, bracket) = ladders
            .found()
            .zip(ladders.highlighted_bracket())
            .ok_or_else(|| NOTHING_SEARCHED_YET.to_owned())?;
        ladder_overlay(&base, over, &swept.options, &bracket.steps)
            .map_err(|error| format!("not written: {error}"))
    });
    write::write(world, path, text);
}
