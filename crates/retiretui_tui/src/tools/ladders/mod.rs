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
    LadderStep, OptimizeOptions, SweptBracket, apply_ladder, ladder_overlay,
};
use retiretui_engine::plan::Plan;
use retiretui_engine::project::{Projection, Summary};

use super::options::{CURRENT_PLAN, FIGURES, Laid};
use super::{Found, NOTHING_SEARCHED_YET, Tool, ToolPage, write};
use crate::command::Outcome;
use crate::confirm::{Answer, Confirm};
use crate::documents::{Browsing, Pickers};
use crate::edit::{self, Draft, DraftEditor, FormButton, Ops};
use crate::journal;
use crate::nav::{self, Page, ShownSurface};
use crate::overview::Better;
use crate::present::MoneyForm;
use crate::session::Session;
pub use retiretui_client::searches::ladders::{
    CONVERSION_COLUMNS, CONVERTS_NOTHING, Constraints, DESTINATION, FIELDS, NO_BRACKET,
    OPTION_COLUMNS, PICK_DESTINATION, Swept, aim_at, held, held_answers, only_roth, option_cells,
    rate_label, search, take_question, taken, taxed_in,
};

pub type Ladders = Tool<Swept>;

pub fn plugin(app: &mut App) {
    super::install::<Swept>(app, &PAGE);
    super::options::plugin::<Swept>(app);
    app.add_systems(
        Update,
        (aim_at_only_roth, search_by_itself)
            .chain()
            .run_if(nav::shows(Page::RothConversions))
            .before(super::poll_search::<Swept>),
    );
    panes::plugin(app);
    guide::plugin(app);
}

const OPS: Ops = Ops::tool::<Constraints>(Some(Page::RothConversions), "Constraints", FIELDS)
    .acting(["Discard", "Apply"], act);
const _: () = assert!(
    edit::help_fits(OPS.form.fields),
    "a field's help is missing or too long"
);
const PAGE: ToolPage = ToolPage {
    surface: Page::RothConversions,
    panes: panes::spawn_panes,
};
impl Found for Swept {
    const NOTHING_SEARCHED: &'static str = "Ranked here once Convert to names a Roth account.";

    /// The plan's own row, then a row per bracket: its rate, what it ends
    /// with against the plan, what the plan converts over its life with
    /// that bracket's ladder, and the figures.
    fn laid(&self, _plan: &Plan, nominal: bool) -> Laid {
        let deflated = !nominal;
        let header = OPTION_COLUMNS
            .into_iter()
            .chain(FIGURES)
            .map(str::to_owned)
            .collect();
        let plan = self.sweep.baseline.summary(deflated);
        let row = |label: String, projection: &Projection, against: Option<&Summary>| {
            let summary = projection.summary(deflated);
            std::iter::once(label)
                .chain(option_cells(&summary, against, MoneyForm::Compact))
                .collect()
        };
        let options = self
            .sweep
            .brackets
            .iter()
            .map(|bracket| row(rate_label(bracket.rate), &bracket.optimized, Some(&plan)));
        Laid {
            header,
            current: row(CURRENT_PLAN.to_owned(), &self.sweep.baseline, None),
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
fn aim_at_only_roth(shown: ShownSurface, mut draft: ResMut<Draft>) {
    let is_moved = draft.is_changed() || shown.is_changed();
    if !is_moved {
        return;
    }
    if let Some(only) = only_roth(&draft) {
        aim_at(&mut draft, &only);
    }
}

/// Searches again whenever the page is on show over a valid draft whose
/// plan or constraints differ from the last it searched, once they name a
/// destination, so the ranking is never asked for; what the Overview
/// already found over them is taken instead.
fn search_by_itself(
    (draft, session, better): (Res<Draft>, Res<Session>, Res<Better>),
    shown: ShownSurface,
    mut searched: Local<Option<(Plan, toml::Table)>>,
    mut ladders: ResMut<Ladders>,
) {
    let is_moved = draft.is_changed() || shown.is_changed() || ladders.is_changed();
    if !is_moved || ladders.is_running() {
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
    let mut rest = answers.clone();
    let destination = rest.remove(DESTINATION);
    let taken = destination
        .as_ref()
        .and_then(toml::Value::as_str)
        .and_then(|to| better.ladders(&draft.plan, &rest, to));
    *searched = Some((draft.plan.clone(), answers));
    if let Some(swept) = taken {
        ladders.take(swept.clone());
        return;
    }
    let tables = session.tables.clone();
    ladders.start(draft.plan.clone(), move |plan, progress| {
        search(plan, &tables, &options, rate, progress).map(|sweep| Swept { sweep, options })
    });
}

/// The `take-ladder` command: asks before taking the highlighted ladder -
/// or the best while the plan's own row is highlighted - into the draft,
/// in place of any ladder taken before. The answer carries the ladder
/// asked about, so a highlight that moves under the question changes
/// nothing.
pub fn adopt(ladders: Res<Ladders>, draft: Res<Draft>, mut confirm: ResMut<Confirm>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(swept) = ladders.found() else {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    };
    let Some(bracket) = ladders.highlighted_bracket() else {
        return Outcome::Refused(NO_BRACKET.to_owned());
    };
    let question = take_question(bracket, &draft.plan);
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
    journal::say(taken(&steps));
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
