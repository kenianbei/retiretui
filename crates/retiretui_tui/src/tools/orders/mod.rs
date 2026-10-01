//! The Withdrawal Order tool: the classes the plan's order lists, tried in
//! every order they can be drained in and ranked by what the household
//! ends with.

#[cfg(test)]
pub(crate) mod tests;

use std::path::PathBuf;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{
    Commands, In, IntoScheduleConfigs, Local, Query, Res, ResMut, With, World,
};
use bevy_input_focus::InputFocus;
use plurimus::core::UiWidget;
use retiretui_client::searches::{option_cells, orders};
use retiretui_engine::optimize::{
    OrderCandidate, OrderSearch, apply_order, optimize_order, order_overlay,
};
use retiretui_engine::plan::{Plan, TreatmentClass};

use super::options::{CURRENT_PLAN, Laid, OptionsTable, spawn_table};
use super::{Found, HelpLine, NOTHING_SEARCHED_YET, Tool, ToolPage, show_help, write};
use crate::command::{Outcome, TAKE_ORDER};
use crate::confirm::{Answer, Confirm};
use crate::documents::{Browsing, Pickers};
use crate::edit::{Draft, DraftEditor};
use crate::hints::Hints;
use crate::journal;
use crate::nav::{self, FocusStop, Page, ShownSurface};
use crate::overview::Better;
use crate::pane::Pane;
use crate::present::MoneyForm;
use crate::session::Session;
use crate::theme::{Repainted, Theme};

pub type Orders = Tool<OrderSearch>;

pub const TITLE: &str = "Orders";

const ON_OPTIONS: &str =
    "⏎ takes the highlighted order into the plan, after asking; w writes it as a scenario.";

pub fn plugin(app: &mut App) {
    super::install::<OrderSearch>(app, &PAGE);
    app.add_systems(
        Update,
        (
            search_by_itself.before(super::poll_search::<OrderSearch>),
            say_help.before(Repainted),
        )
            .run_if(nav::shows(Page::WithdrawalOrder)),
    );
    super::options::plugin::<OrderSearch>(app);
}

const PAGE: ToolPage = ToolPage {
    surface: Page::WithdrawalOrder,
    panes: spawn_pane,
};

/// The page's one pane: every order the search tried, as the shared
/// options table lists them.
fn spawn_pane(commands: &mut Commands, row: Entity) {
    let framed = Pane::new(TITLE).sharing(1.0).spawn(commands, row);
    let hints = Hints(&[("↑↓", "order"), ("⏎", "take")]);
    let table = spawn_table::<OrderSearch>(commands, framed, TAKE_ORDER, hints);
    commands.entity(table).insert(FocusStop);
}

impl Found for OrderSearch {
    const NOTHING_SEARCHED: &'static str = orders::NOTHING_SEARCHED;

    /// The orders head the rows, the plan's own row named in their place,
    /// ahead of what each ends with against the plan and the figures.
    fn laid(&self, _: &Plan, is_nominal: bool) -> Laid {
        let deflated = !is_nominal;
        let header = orders::option_columns()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let baseline = self.baseline.summary(deflated);
        let current = std::iter::once(CURRENT_PLAN.to_owned())
            .chain(option_cells(&baseline, None, MoneyForm::Compact))
            .collect();
        let options = self
            .candidates
            .iter()
            .map(|candidate| {
                std::iter::once(orders::said(&candidate.order))
                    .chain(option_cells(
                        &candidate.projection.summary(deflated),
                        Some(&baseline),
                        MoneyForm::Compact,
                    ))
                    .collect()
            })
            .collect();
        Laid {
            header,
            current,
            options,
        }
    }
}

impl Tool<OrderSearch> {
    /// The candidate chosen of what was found: the highlighted one, or the
    /// best while the baseline is highlighted.
    fn chosen(&self) -> Option<&OrderCandidate> {
        let found = self.found()?;
        let highlighted = self.highlighted().and_then(|at| found.candidates.get(at));
        Some(highlighted.unwrap_or_else(|| found.best()))
    }
}

/// Searches again whenever the page is on show over a valid draft whose plan
/// differs from the last it searched, so the ranking is never asked for;
/// what the Overview already found over it is taken instead.
fn search_by_itself(
    draft: Res<Draft>,
    (session, better): (Res<Session>, Res<Better>),
    shown: ShownSurface,
    mut searched: Local<Option<Plan>>,
    mut orders: ResMut<Orders>,
) {
    let is_moved = draft.is_changed() || shown.is_changed() || orders.is_changed();
    let is_ready = is_moved && !orders.is_running();
    if !is_ready || !super::is_due(&draft, searched.as_ref(), |plan| *plan == draft.plan) {
        return;
    }
    *searched = Some(draft.plan.clone());
    if let Some(found) = better.order(&draft.plan) {
        orders.take(found.clone());
        return;
    }
    let tables = session.tables.clone();
    orders.start(draft.plan.clone(), move |plan, progress| {
        optimize_order(plan, &tables, progress)
    });
}

/// The line under the page: what ⏎ does while the keyboard is on the
/// orders, and what the tool is while it is not.
fn say_help(
    (focus, shown, theme): (Res<InputFocus>, ShownSurface, Res<Theme>),
    options: Query<(), With<OptionsTable<OrderSearch>>>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    if !(focus.is_changed() || shown.is_changed() || theme.is_changed()) {
        return;
    }
    let text = match focus.get() {
        Some(holder) if options.contains(holder) => ON_OPTIONS,
        _ => orders::ABOUT,
    };
    show_help(&mut lines, Page::WithdrawalOrder, text, &theme);
}

/// The `take-order` command: asks before restating the plan's withdrawal
/// order as the chosen candidate's - the highlighted one, or the best while
/// the baseline is highlighted. The answer carries the order asked about,
/// so a highlight that moves under the question changes nothing.
pub fn adopt(orders: Res<Orders>, draft: Res<Draft>, mut confirm: ResMut<Confirm>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(candidate) = orders.chosen() else {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    };
    let asked = candidate.order.clone();
    let answers = vec![
        Answer::closing("Cancel"),
        Answer::running("Take", move |commands| {
            commands.run_system_cached_with(take, asked);
        })
        .primary(),
    ];
    confirm.ask_among(orders::take_question(&candidate.order), answers);
    Outcome::Done
}

/// Takes `order` into the draft, as one step of history.
fn take(In(order): In<Vec<TreatmentClass>>, mut editor: DraftEditor) {
    apply_order(&mut editor.draft.plan, &order);
    editor.commit();
    journal::say(orders::taken(&order));
}

/// The `write-order` command: asks where to write the highlighted order.
pub fn write_picker(
    pickers: Res<Pickers>,
    orders: Res<Orders>,
    draft: Res<Draft>,
    mut browsing: ResMut<Browsing>,
) -> Outcome {
    if orders.found().is_none() {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    }
    write::open_picker(pickers.order, &draft, &mut browsing)
}

/// Writes the highlighted order at `path` as a scenario over the document,
/// and compares the file written.
pub fn write_overlay(In(path): In<PathBuf>, world: &mut World) {
    let text = write::base_of(world, &path).and_then(|base| {
        let candidate = world
            .resource::<Orders>()
            .chosen()
            .ok_or_else(|| NOTHING_SEARCHED_YET.to_owned())?;
        order_overlay(&base, &candidate.order).map_err(|error| format!("not written: {error}"))
    });
    write::write(world, path, text);
}
