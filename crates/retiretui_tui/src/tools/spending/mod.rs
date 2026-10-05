//! The Spending Ceiling tool: the most the plan's flexible spending could
//! be, in its own market and in a target share of random markets, beside
//! that target and over the highlighted ceiling's expenses.

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, In, IntoScheduleConfigs, Local, Query, Res, ResMut, With, World,
};
use bevy_input_focus::InputFocus;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::UiWidget;
use plurimus::ui::ScrollArea;
use retiretui_client::searches::spending::{
    self, ABOUT, AT_TARGET, Answers, FIELDS, ITEM_COLUMNS, LEADING, Listed, search, taken, total,
};
use retiretui_engine::optimize::{ScaledExpense, apply_spending, spending_overlay};
use retiretui_engine::plan::Plan;

use super::markets::MarketHistory;
use super::options::{self, CURRENT_PLAN, Laid, OptionsTable, say_instead, spawn_table};
use super::{Found, HelpLine, NOTHING_SEARCHED_YET, Tool, ToolPage, show_help, write, writes_it};
use crate::command::{Keymap, Outcome, TAKE_SPENDING, WRITE_SPENDING};
use crate::confirm::{Answer, Confirm};
use crate::documents::{Browsing, Pickers};
use crate::edit::{self, Draft, DraftEditor, Ops, table_bundle};
use crate::hints::Hints;
use crate::journal;
use crate::layout::{self, filling, placed};
use crate::nav::{self, FocusStop, Page, ShownSurface};
use crate::pane::Pane;
use crate::present::MoneyForm;
use crate::session::Session;
use crate::tabulate;
use crate::theme::{Repainted, Theme};

/// What a search found: the ceiling in the plan's own market and the one
/// at the target.
pub use retiretui_client::searches::spending::Found as Ceilings;

pub type Spending = Tool<Ceilings>;

const OPTIONS_TITLE: &str = "Ceilings";
const EXPENSES_TITLE: &str = "Expenses";
/// Borders, the header, the plan's own row and the two ceilings.
const OPTIONS_ROWS: f32 = 6.0;
/// The Target pane's width, borders included.
const TARGET_COLS: f32 = 40.0;
/// The cells between columns, past the one the table leaves.
const GAP: u16 = 1;
const ON_OPTIONS: &str = "⏎ takes the highlighted ceiling into the plan, after asking";
const NOT_SEARCHED: &str = "The highlighted ceiling's expenses, now and at the ceiling.";

pub fn plugin(app: &mut App) {
    super::install::<Ceilings>(app, &PAGE);
    super::options::plugin::<Ceilings>(app);
    app.add_systems(
        Update,
        (
            search_by_itself.before(super::poll_search::<Ceilings>),
            say_help.before(Repainted),
        )
            .run_if(nav::shows(Page::SpendingCeiling)),
    );
    app.add_systems(
        Update,
        refresh_expenses
            .in_set(options::TablesFilled)
            .after(options::follow_cursor::<Ceilings>),
    );
}

const OPS: Ops = Ops::tool::<Answers>(Some(Page::SpendingCeiling), "Target", FIELDS);
const _: () = assert!(
    edit::help_fits(OPS.form.fields),
    "a field's help is missing or too long"
);
const PAGE: ToolPage = ToolPage {
    surface: Page::SpendingCeiling,
    panes: spawn_panes,
};

#[derive(Component)]
struct ExpensesTable;

/// The ceilings, over the highlighted one's expenses beside the target.
fn spawn_panes(commands: &mut Commands, row: Entity) {
    let stacked = |flex_direction| Node {
        flex_direction,
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        ..Node::default()
    };
    let column = Node {
        height: Val::Percent(100.0),
        ..stacked(FlexDirection::Column)
    };
    let column = commands.spawn((column, ChildOf(row))).id();
    let framed = Pane::new(OPTIONS_TITLE)
        .tall(OPTIONS_ROWS)
        .spawn(commands, column);
    let hints = Hints(&[("↑↓", "ceiling"), ("⏎", "take")]);
    let table = spawn_table::<Ceilings>(commands, framed, TAKE_SPENDING, hints);
    commands.entity(table).insert(FocusStop);
    let beneath = stacked(FlexDirection::Row);
    let beneath = commands.spawn((beneath, ChildOf(column))).id();
    let framed = Pane::new(EXPENSES_TITLE)
        .sharing(1.0)
        .spawn(commands, beneath);
    commands.spawn((
        table_bundle(),
        ExpensesTable,
        FocusStop,
        Hints(&[("↑↓", "expense")]),
        layout::Rests,
        filling(),
        placed(),
        ChildOf(framed),
    ));
    let form = Pane::new(OPS.title)
        .wide(TARGET_COLS)
        .spawn(commands, beneath);
    edit::spawn_details(commands, form, OPS);
}

impl Found for Ceilings {
    const NOTHING_SEARCHED: &'static str = spending::NOTHING_SEARCHED;
    const IS_COUNTED: bool = true;
    const LEADING: usize = LEADING;

    fn laid(&self, plan: &Plan, is_nominal: bool) -> Laid {
        let deflated = !is_nominal;
        let header = spending::option_columns();
        let row = |first: String, cells: Vec<String>| std::iter::once(first).chain(cells).collect();
        let current = self.plan_cells(plan, deflated, MoneyForm::Compact);
        let options = self.listed().into_iter().map(|listed| {
            let cells = listed.cells(plan, deflated, MoneyForm::Compact);
            row(listed.held_to, cells)
        });
        Laid {
            header: header.into_iter().map(str::to_owned).collect(),
            current: row(CURRENT_PLAN.to_owned(), current),
            options: options.collect(),
        }
    }
}

impl Tool<Ceilings> {
    /// The ceiling chosen of what was found: the highlighted one, or the
    /// one at the target while the plan's own row is highlighted.
    fn chosen(&self) -> Option<Listed<'_>> {
        let found = self.found()?;
        let at = self.highlighted().unwrap_or(LEADING);
        found.listed().into_iter().nth(at)
    }
}

/// Searches whenever the page shows a valid draft whose plan or target
/// differs from the last searched, stopping a search under way.
fn search_by_itself(
    (draft, session, history): (Res<Draft>, Res<Session>, Res<MarketHistory>),
    shown: ShownSurface,
    mut searched: Local<Option<(Plan, toml::Table)>>,
    mut tool: ResMut<Spending>,
) {
    if !(draft.is_changed() || shown.is_changed() || tool.is_changed()) {
        return;
    }
    let answers = draft.answers::<Answers>();

    let is_same = |(plan, held): &(Plan, toml::Table)| *plan == draft.plan && *held == answers;
    if !super::is_due(&draft, searched.as_ref(), is_same) {
        return;
    }
    let target = spending::target_in(answers.clone());
    *searched = Some((draft.plan.clone(), answers));
    let (tables, history) = (session.tables.clone(), history.0.clone());
    let total = total(&draft.plan);
    tool.restart(draft.plan.clone(), total, move |plan, progress| {
        search(plan, &tables, &history, target, progress)
    });
}

/// Respawns the expenses of the ceiling the cursor rests on as it or what
/// they were drawn from moves; a search under way leaves the last in view.
fn refresh_expenses(
    (tool, draft): (Res<Spending>, Res<Draft>),
    mut drawn: Local<Option<usize>>,
    mut tables: Query<(Entity, &mut ScrollArea), With<ExpensesTable>>,
    mut commands: Commands,
) {
    let is_moved = tool.is_changed() || draft.is_changed();
    if (!is_moved && *drawn == Some(tool.highlighted))
        || (tool.found().is_none() && tool.is_running())
    {
        return;
    }
    *drawn = Some(tool.highlighted);
    for (table, mut scroll) in &mut tables {
        let Some(listed) = tool.chosen() else {
            say_instead(&mut commands, table, NOT_SEARCHED.to_owned());
            continue;
        };
        let items = listed.items(&draft.plan, MoneyForm::Full);
        let rows: Vec<Vec<String>> = items.into_iter().map(Vec::from).collect();
        let header = ITEM_COLUMNS.map(str::to_owned);
        commands
            .entity(table)
            .insert(tabulate::columns((&header, &rows), GAP));
        tabulate::refill(&mut commands, (table, &mut scroll), (&header, &rows), &[0]);
    }
}

/// What the help line says with the keyboard on the ceilings: what ⏎
/// does, or the note on the ceiling in the plan's own market.
fn on_options(tool: &Spending, plan: &Plan, keymap: &Keymap) -> String {
    let is_at_target = tool.chosen().is_none_or(|listed| listed.key == AT_TARGET);
    match spending::note(plan) {
        Some(note) if !is_at_target => note.to_owned(),
        _ => format!("{ON_OPTIONS}{}", writes_it(keymap, WRITE_SPENDING)),
    }
}

fn say_help(
    (tool, draft, focus): (Res<Spending>, Res<Draft>, Res<InputFocus>),
    (shown, theme, keymap): (ShownSurface, Res<Theme>, Res<Keymap>),
    options: Query<(), With<OptionsTable<Ceilings>>>,
    mut said_of: Local<usize>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    let is_moved = tool.is_changed() || draft.is_changed() || focus.is_changed();
    let is_redrawn = shown.is_changed() || theme.is_changed();
    if !(is_moved || is_redrawn || *said_of != tool.highlighted) {
        return;
    }
    *said_of = tool.highlighted;
    let text = match focus.get() {
        Some(holder) if options.contains(holder) => on_options(&tool, &draft.plan, &keymap),
        _ => ABOUT.to_owned(),
    };
    show_help(&mut lines, Page::SpendingCeiling, &text, &theme);
}

/// The `take-spending` command: asks before restating the plan's flexible
/// expenses as the chosen ceiling's. The answer carries the amounts asked
/// about, so a highlight that moves under the question changes nothing.
pub fn adopt(tool: Res<Spending>, draft: Res<Draft>, mut confirm: ResMut<Confirm>) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    let Some(listed) = tool.chosen() else {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    };
    let asked = (listed.ceiling.expenses.clone(), taken(listed.flexible()));
    let answers = vec![
        Answer::closing("Cancel"),
        Answer::running("Take", move |commands| {
            commands.run_system_cached_with(take, asked);
        })
        .primary(),
    ];
    confirm.ask_among(listed.take_question(&draft.plan), answers);
    Outcome::Done
}

fn take(In((expenses, said)): In<(Vec<ScaledExpense>, String)>, mut editor: DraftEditor) {
    apply_spending(&mut editor.draft.plan, &expenses);
    editor.commit();
    journal::say(said);
}

/// The `write-spending` command: asks where to write the highlighted
/// ceiling.
pub fn write_picker(
    pickers: Res<Pickers>,
    tool: Res<Spending>,
    draft: Res<Draft>,
    mut browsing: ResMut<Browsing>,
) -> Outcome {
    if tool.found().is_none() {
        return Outcome::Refused(NOTHING_SEARCHED_YET.to_owned());
    }
    write::open_picker(pickers.spending, &draft, &mut browsing)
}

/// Writes the highlighted ceiling at `path` as a scenario over the
/// document, and compares the file written.
pub fn write_overlay(In(path): In<PathBuf>, world: &mut World) {
    let text = write::base_of(world, &path).and_then(|base| {
        let listed = world
            .resource::<Spending>()
            .chosen()
            .ok_or_else(|| NOTHING_SEARCHED_YET.to_owned())?;
        spending_overlay(&base, &listed.ceiling.expenses)
            .map_err(|error| format!("not written: {error}"))
    });
    write::write(world, path, text);
}
