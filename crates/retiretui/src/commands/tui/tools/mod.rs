//! The Tools tab's tools: each panes of its own over a line of help, a
//! search on a thread of its own, and the highlighted option written as a
//! scenario over the document or taken into the draft. What a tool
//! searches and how it lays out its options are its own; the search's
//! life, the options table and the highlight are shared.

pub mod claims;
pub mod ladders;
pub mod markets;
mod options;
mod worker;
mod write;

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::event::EntityEvent;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, On, Query, Res, ResMut, Resource, With,
};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::FocusedInput;
use bevy_ui::{FlexDirection, Node};
use plurimus::core::UiWidget;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{KeyBinding, first_bound};
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use retiretui_engine::market::Progress;
use retiretui_engine::plan::{Issue, Plan};

pub use claims::Claims;
pub use ladders::Ladders;
pub(crate) use worker::Keyed;
pub use worker::Searches;
pub use write::OVERLAY_OVER;

use super::command;
use super::edit::Draft;
use super::layout::{self, Body, fixed, growing, placed};
use super::nav::{self, Page};
use super::pane::Framed;
use super::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<Searches>();
    app.add_plugins((
        ladders::plugin,
        claims::plugin,
        markets::plugin,
        options::plugin_said,
    ));
}

const NOTHING_SEARCHED_YET: &str = "nothing searched yet";

/// Which tool commands have nothing to act on yet, so the key row leaves
/// their hints out; the keys still run and say why they refuse.
#[derive(SystemParam)]
pub struct Idle<'w> {
    ladders: Res<'w, Ladders>,
    claims: Res<'w, Claims>,
}

impl Idle<'_> {
    /// Whether the command named `name` would refuse for want of a result.
    pub fn is_idle(&self, name: &str) -> bool {
        use super::command::{TAKE_CLAIMS, TAKE_LADDER, WRITE_CLAIMS, WRITE_LADDER};
        match name {
            WRITE_LADDER | TAKE_LADDER => self.ladders.highlighted_bracket().is_none(),
            WRITE_CLAIMS | TAKE_CLAIMS => self.claims.found().is_none(),
            _ => false,
        }
    }
}

/// What a tool's search found, as its options pane lists it: the plan's
/// own row, then an option per result, best first.
pub trait Found: Send + Sync + 'static {
    /// What the pane says before anything has been searched.
    const NOTHING_SEARCHED: &'static str;
    /// Whether the search counts its steps, its answer then speaking for
    /// itself with no time beside it.
    const IS_COUNTED: bool = false;
    /// Whether the cursor may rest on the plan's own row, as a row of its
    /// own to open, rather than going on to the best option.
    const IS_PLAN_ROW_CHOSEN: bool = false;

    /// The options pane's rows, over `plan` as it stands.
    fn laid(&self, plan: &Plan, nominal: bool) -> options::Laid;
}

/// The page a tool is drawn as: its panes, spawned into a row, over a line
/// of help the tool writes.
#[derive(Clone, Copy)]
pub(crate) struct ToolPage {
    surface: Page,
    panes: fn(&mut Commands, Entity),
}

/// The line of help under a page's panes, on the page's surface.
#[derive(Component)]
pub struct HelpLine(pub Page);

/// Spawns `surface`'s line of help, a row across the foot of `view`.
pub(crate) fn spawn_help(commands: &mut Commands, view: Entity, surface: Page) {
    commands.spawn((
        HelpLine(surface),
        UiWidget::default(),
        fixed(1.0),
        placed(),
        ChildOf(view),
    ));
}

/// Writes `text`, dimmed, into the help line under `surface`'s panes.
pub(crate) fn show_help(
    lines: &mut Query<(&mut UiWidget, &HelpLine)>,
    surface: Page,
    text: &str,
    theme: &Theme,
) {
    let on_page = lines.iter_mut().filter(|(_, help)| help.0 == surface);
    for (mut widget, _) in on_page {
        *widget = UiWidget::new(Paragraph::new(text.to_owned()).style(theme.dimmed()));
    }
}
const ENTER_KEYS: &[(KeyBinding, ())] = &[(KeyBinding::new(Key::Enter), ())];

struct Running<R> {
    /// What the search found over the plan it is keyed by.
    search: Keyed<Arc<Plan>, Result<R, Vec<Issue>>>,
    /// How many steps the search takes, where it counts them.
    total: Option<usize>,
}

/// A tool's state: the search under way, what the last one found, and the
/// result row highlighted.
#[derive(Resource)]
pub struct Tool<R: Found> {
    running: Option<Running<R>>,
    found: Option<(R, Duration)>,
    /// Why the last search found nothing, which the pane says in its place.
    refused: Option<String>,
    /// Zero is the plan's own row.
    highlighted: usize,
    /// A test's hold on the answer, so what shows while a search runs can
    /// be looked at however fast it answers.
    #[cfg(test)]
    is_held: bool,
}

impl<R: Found> Default for Tool<R> {
    fn default() -> Self {
        Self {
            running: None,
            found: None,
            refused: None,
            highlighted: 0,
            #[cfg(test)]
            is_held: false,
        }
    }
}

/// Whether a tool's search by itself is due over a valid draft: what it
/// would search differs from the last `searched`, as `is_same` compares
/// them.
fn is_due<K>(draft: &Draft, searched: Option<&K>, is_same: impl FnOnce(&K) -> bool) -> bool {
    draft.issues().is_empty() && !searched.is_some_and(is_same)
}

impl<R: Found> Tool<R> {
    /// Runs `work` over `plan` on a thread of its own, dropping what the
    /// last search found.
    fn start(
        &mut self,
        plan: Plan,
        work: impl FnOnce(&Plan) -> Result<R, Vec<Issue>> + Send + 'static,
    ) {
        self.spawn(plan, None, move |plan, _| work(plan));
        self.found = None;
        self.refused = None;
        self.highlighted = 0;
    }

    /// Starts `work` over `plan` by itself in place of any search under
    /// way; what the last one found stays on show until this one answers,
    /// and the note counts its `total` steps.
    fn restart(
        &mut self,
        plan: Plan,
        total: usize,
        work: impl FnOnce(&Plan, &Progress) -> Result<R, Vec<Issue>> + Send + 'static,
    ) {
        self.spawn(plan, Some(total), work);
    }

    fn spawn(
        &mut self,
        plan: Plan,
        total: Option<usize>,
        work: impl FnOnce(&Plan, &Progress) -> Result<R, Vec<Issue>> + Send + 'static,
    ) {
        let plan = Arc::new(plan);
        let searched = Arc::clone(&plan);
        let search = Keyed::spawn(plan, move |progress| work(&searched, progress));
        self.running = Some(Running { search, total });
    }

    fn has_answered(&self) -> bool {
        #[cfg(test)]
        if self.is_held {
            return false;
        }
        let running = self.running.as_ref();
        running.is_some_and(|running| running.search.is_finished())
    }

    /// Takes the answer: one describing a plan the draft has since left
    /// behind is dropped, and the engine's refusal is kept for the pane.
    fn receive(&mut self, draft: &Draft) {
        let Some(running) = self.running.take() else {
            return;
        };
        let (plan, found, took) = running.search.join();
        let Some(found) = found else {
            return;
        };
        if *plan != draft.plan {
            return;
        }
        match found {
            Ok(found) => self.found = Some((found, took)),
            Err(issues) => self.refused = issues.first().map(|issue| issue.message.clone()),
        }
    }

    pub fn found(&self) -> Option<&R> {
        self.found.as_ref().map(|(found, _)| found)
    }

    /// The highlighted result's place among them, where a result row is.
    fn highlighted(&self) -> Option<usize> {
        self.highlighted.checked_sub(1)
    }

    /// What the result pane's title says: how long the search has run, or
    /// took.
    fn note(&self) -> String {
        match (&self.running, &self.found) {
            (Some(running), _) => match running.total {
                Some(total) => running_text(running.search.progress().done(), total),
                None => format!("searching… {}s", running.search.elapsed().as_secs()),
            },
            (None, Some(_)) if R::IS_COUNTED => String::new(),
            (None, Some((_, took))) => format!("{:.1}s", took.as_secs_f32()),
            (None, None) => String::new(),
        }
    }

    /// What a pane says in place of results: why the last search found
    /// nothing, or that none has run.
    fn said(&self) -> String {
        self.refused
            .clone()
            .unwrap_or_else(|| R::NOTHING_SEARCHED.to_owned())
    }

    fn is_running(&self) -> bool {
        self.running.is_some()
    }
}

/// A count as a person reads it: `1,000`.
pub(crate) fn count_text(count: usize) -> String {
    crate::commands::table::grouped(u64::try_from(count).unwrap_or(u64::MAX))
}

/// How far a counted search has got: "running 340 of 1,000".
pub(crate) fn running_text(done: usize, total: usize) -> String {
    format!("running {} of {}", count_text(done), count_text(total))
}

/// The pane a tool's results fill.
#[derive(Component)]
pub struct ResultPane<R>(PhantomData<R>);

impl<R> Default for ResultPane<R> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Installs a tool: its state, its page, and the systems that keep the
/// page saying what the search is up to.
fn install<R: Found>(app: &mut App, page: &'static ToolPage) {
    app.init_resource::<Tool<R>>();
    let spawn = move |bodies: Query<Entity, With<Body>>, mut commands: Commands| {
        if let Ok(body) = bodies.single() {
            spawn_tool(&mut commands, body, page);
        }
    };
    app.add_systems(Startup, spawn.after(layout::spawn_frame));
    app.add_systems(Update, poll_search::<R>.before(Repainted));
}

/// The tool's page: its panes in a row, over its line of help.
fn spawn_tool(commands: &mut Commands, body: Entity, page: &'static ToolPage) {
    let view = nav::spawn_surface(commands, body, Some(page.surface));
    let row = Node {
        flex_direction: FlexDirection::Row,
        ..growing()
    };
    let row = commands.spawn((row, ChildOf(view))).id();
    (page.panes)(commands, row);
    spawn_help(commands, view, page.surface);
}

/// The command ⏎ runs on the pane that carries it.
#[derive(Component, Clone, Copy)]
pub struct EnterRuns(pub &'static str);

/// ⏎ on a pane runs the command it carries.
pub fn handle_enter(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    runs: Query<&EnterRuns>,
    mut commands: Commands,
) {
    let Ok(&EnterRuns(name)) = runs.get(input.event_target()) else {
        return;
    };
    if first_bound(ENTER_KEYS, &input.input, held.get()).is_none() {
        return;
    }
    input.propagate(false);
    command::defer_named(&mut commands, name);
}

/// Takes a search's answer as it arrives, and keeps the pane's title
/// saying how long it has run.
fn poll_search<R: Found>(
    mut tool: ResMut<Tool<R>>,
    draft: Res<Draft>,
    mut panes: Query<&mut Framed, With<ResultPane<R>>>,
) {
    if tool.has_answered() {
        tool.receive(&draft);
    }
    if tool.running.is_none() && !tool.is_changed() {
        return;
    }
    let note = tool.note();
    for mut pane in &mut panes {
        Framed::renote(&mut pane, &note);
    }
}

/// Ticks until every tool's search has answered.
#[cfg(test)]
pub fn settle_all(app: &mut bevy_app::App) {
    settle::<retiretui_engine::optimize::ClaimSearch>(app);
    settle::<ladders::Swept>(app);
    settle::<retiretui_engine::market::MonteCarlo>(app);
    settle::<retiretui_engine::market::Runs>(app);
    settle_until_idle(app, |app| {
        let successes = app.world().resource::<super::success::Successes>();
        successes.is_running()
    });
    settle_until_idle(app, |app| {
        app.world()
            .resource::<super::overview::Better>()
            .is_running()
    });
}

/// Holds a tool's answer back until released, or releases it.
#[cfg(test)]
pub fn hold<R: Found>(app: &mut bevy_app::App, is_held: bool) {
    app.world_mut().resource_mut::<Tool<R>>().is_held = is_held;
}

/// Ticks until `R`'s search has answered, then draws the answer as having
/// taken no time: how long it took is the machine's, not the answer's.
#[cfg(test)]
fn settle<R: Found>(app: &mut bevy_app::App) {
    settle_until_idle(app, |app| app.world().resource::<Tool<R>>().is_running());
    if let Some((_, took)) = &mut app.world_mut().resource_mut::<Tool<R>>().found {
        *took = Duration::ZERO;
    }
    app.update();
}

/// Ticks until nothing `is_running` and the tick after starts nothing,
/// or gives up.
#[cfg(test)]
fn settle_until_idle(app: &mut bevy_app::App, is_running: impl Fn(&bevy_app::App) -> bool) {
    for _ in 0..500 {
        app.update();
        if !is_running(app) {
            app.update();
            if !is_running(app) {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the search never answered");
}
