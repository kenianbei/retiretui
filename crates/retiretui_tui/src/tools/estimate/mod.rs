//! The Spending Estimate tool: a bank's exports read for the session, each
//! category's spending proposed as the amount of the expense that is for
//! it, beside the highlighted proposal read out, and the proposals taken
//! into the draft or written as a scenario.

mod edits;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, IntoScheduleConfigs, Query, Res, ResMut, Resource, With,
};
use bevy_input_focus::InputFocus;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::Style;
use plurimus::ui::ScrollArea;
use plurimus::widgets::ActiveDescendant;
use retiretui_client::estimate::{ABOUT, COLUMNS, Estimator, NOTHING_READ, listed, summary};
use retiretui_client::present::counted;
use retiretui_engine::estimate::Patterns;

pub use edits::{
    add_export, add_picker, adopt, edit_row, remove_export, write_overlay, write_picker,
};

use super::options::say_instead;
use super::{EnterRuns, HelpLine, ToolPage, show_help, writes_it};
use crate::command::{EDIT_PROPOSAL, Keymap, WRITE_ESTIMATE};
use crate::edit::{self, Draft, table_bundle};
use crate::hints::Hints;
use crate::layout::{self, Body, filling, placed};
use crate::nav::{self, FocusStop, Page, ShownSurface};
use crate::pane::Pane;
use crate::present::MoneyForm;
use crate::tabulate;
use crate::theme::{Repainted, Theme};

const FILES_TITLE: &str = "Files";
const PROPOSALS_TITLE: &str = "Proposals";
const READ_OUT_TITLE: &str = "Proposal";
const SETTINGS_TITLE: &str = "Settings";
/// The right column's width, borders included.
const SIDE_COLS: f32 = 48.0;
/// Borders, and the exports a session is likely to hold.
const FILES_ROWS: f32 = 6.0;
/// Borders, the read-out's header, and the lean, the large share and the
/// bucket.
const SETTINGS_ROWS: f32 = 6.0;
const GAP: u16 = 1;
const ON_PROPOSALS: &str = "⏎ edits the row; t takes the proposals into the plan, after asking";
const NOTHING_HIGHLIGHTED: &str = "nothing highlighted";
const NO_FILES: &str = "none yet";

pub fn plugin(app: &mut App) {
    app.init_resource::<Held>();
    app.init_resource::<Cursor>();
    app.init_resource::<PatternsFile>();
    app.add_systems(Startup, spawn_page.after(layout::spawn_frame));
    app.add_systems(
        Update,
        (
            estimate_by_itself,
            list_files,
            table_proposals,
            follow_cursor,
            show_read_out,
            say_help,
        )
            .chain()
            .run_if(nav::shows(Page::SpendingEstimate))
            .before(Repainted),
    );
}

/// The exports the session holds and what they say the draft spends,
/// named by the launch's patterns.
#[derive(Resource)]
pub struct Held(pub Estimator);

impl Default for Held {
    fn default() -> Self {
        Self(Estimator::new(Patterns::embedded()))
    }
}

/// The user's patterns file a payee named on the page is remembered in;
/// none where there is no machine to keep one.
#[derive(Resource, Default)]
pub struct PatternsFile(pub Option<PathBuf>);

/// The place of the highlighted row in the table.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cursor(usize);

#[derive(Component)]
pub struct FileList;

#[derive(Component, Clone, Copy)]
pub struct FileRow(usize);

#[derive(Component)]
struct ProposalsTable;

#[derive(Component, Clone, Copy)]
struct ProposalRow(usize);

#[derive(Component)]
struct ReadOut;

static PAGE: ToolPage = ToolPage {
    surface: Page::SpendingEstimate,
    panes: spawn_panes,
};

fn spawn_page(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    if let Ok(body) = bodies.single() {
        super::spawn_tool(&mut commands, body, &PAGE);
    }
}

/// The proposals, beside the files over the highlighted row read out.
fn spawn_panes(commands: &mut Commands, row: Entity) {
    let proposals = Pane::new(PROPOSALS_TITLE).spawn(commands, row);
    commands
        .spawn((
            table_bundle(),
            ProposalsTable,
            EnterRuns(EDIT_PROPOSAL),
            FocusStop,
            Hints(&[("↑↓", "row"), ("⏎", "edit")]),
            layout::Rests,
            filling(),
            placed(),
            ChildOf(proposals),
        ))
        .observe(super::handle_enter);
    spawn_column(commands, row);
}

/// The files over the highlighted row read out, over the plan's
/// `[estimate]` table.
fn spawn_column(commands: &mut Commands, row: Entity) {
    let column = Node {
        flex_direction: FlexDirection::Column,
        width: Val::Px(SIDE_COLS),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..Node::default()
    };
    let column = commands.spawn((column, ChildOf(row))).id();
    let files = Pane::new(FILES_TITLE)
        .tall(FILES_ROWS)
        .spawn(commands, column);
    let list = layout::spawn_scrolled_list(commands, files, Hints(&[("↑↓", "file")]));
    commands.entity(list).insert(FileList);
    let read_out = Pane::new(READ_OUT_TITLE)
        .sharing(1.0)
        .spawn(commands, column);
    commands.spawn((
        table_bundle(),
        ReadOut,
        FocusStop,
        Hints(&[("↑↓", "line")]),
        layout::Rests,
        filling(),
        placed(),
        ChildOf(read_out),
    ));
    let settings = Pane::new(SETTINGS_TITLE)
        .tall(SETTINGS_ROWS)
        .spawn(commands, column);
    edit::spawn_details(commands, settings, edits::SETTINGS_OPS);
}

/// Estimates again whenever the plan, the exports or the patterns move;
/// the form's answers, held beside the plan, move nothing.
fn estimate_by_itself((draft, shown): (Res<Draft>, ShownSurface), mut held: ResMut<Held>) {
    if !(draft.is_changed() || held.is_changed() || shown.is_changed()) {
        return;
    }
    if held.bypass_change_detection().0.refresh(&draft.plan) {
        held.set_changed();
    }
}

fn list_files(
    (held, shown, theme): (Res<Held>, ShownSurface, Res<Theme>),
    lists: Query<(Entity, &ActiveDescendant), With<FileList>>,
    rows: Query<&FileRow>,
    mut commands: Commands,
) {
    if !(held.is_changed() || shown.is_changed() || theme.is_changed()) {
        return;
    }
    let exports = held.0.exports();
    let files = exports.listed().enumerate().map(|(at, (name, count))| {
        let said = format!("{name}  {}", counted(count, "row", "rows"));
        (at, said, Style::default())
    });
    let lines: Vec<_> = if exports.is_empty() {
        vec![(0, NO_FILES.to_owned(), theme.dimmed())]
    } else {
        files.collect()
    };
    for (list, on) in &lists {
        let kept =
            on.0.and_then(|row| rows.get(row).ok())
                .map_or(0, |row| row.0);
        let mut spawned = Vec::new();
        layout::fill_lines(&mut commands, list, lines.clone(), |at, row| {
            if !exports.is_empty() {
                row.insert(FileRow(at));
                spawned.push(row.id());
            }
        });
        let at = kept.min(spawned.len().saturating_sub(1));
        commands
            .entity(list)
            .insert(ActiveDescendant(spawned.get(at).copied()));
    }
}

fn table_proposals(
    (held, cursor, shown): (Res<Held>, Res<Cursor>, ShownSurface),
    mut tables: Query<(Entity, &mut ScrollArea), With<ProposalsTable>>,
    mut commands: Commands,
) {
    if !(held.is_changed() || shown.is_changed()) {
        return;
    }
    let found = held.0.estimate();
    let rows: Vec<Vec<String>> = listed(found)
        .map(|row| row.cells(found.months, MoneyForm::Full).to_vec())
        .collect();
    let header = COLUMNS.map(str::to_owned);
    for (table, mut scroll) in &mut tables {
        if rows.is_empty() {
            commands.entity(table).despawn_related::<Children>();
            say_instead(&mut commands, table, NOTHING_READ.to_owned());
            continue;
        }
        commands
            .entity(table)
            .insert(tabulate::columns((&header, &rows), GAP));
        let spawned = tabulate::refill(&mut commands, (table, &mut scroll), (&header, &rows), &[0]);
        for (at, &row) in spawned.iter().enumerate() {
            commands.entity(row).insert(ProposalRow(at));
        }
        let at = cursor.0.min(spawned.len().saturating_sub(1));
        commands
            .entity(table)
            .insert(ActiveDescendant(spawned.get(at).copied()));
    }
}

fn follow_cursor(
    tables: Query<&ActiveDescendant, (With<ProposalsTable>, Changed<ActiveDescendant>)>,
    rows: Query<&ProposalRow>,
    mut cursor: ResMut<Cursor>,
) {
    for on in &tables {
        if let Some(&ProposalRow(at)) = on.0.and_then(|row| rows.get(row).ok()) {
            cursor.set_if_neq(Cursor(at));
        }
    }
}

fn show_read_out(
    (held, cursor, draft, shown): (Res<Held>, Res<Cursor>, Res<Draft>, ShownSurface),
    mut outs: Query<(Entity, &mut ScrollArea), With<ReadOut>>,
    mut commands: Commands,
) {
    if !(held.is_changed() || cursor.is_changed() || shown.is_changed()) {
        return;
    }
    let header: [String; 0] = [];
    let months = held.0.estimate().months;
    for (table, mut scroll) in &mut outs {
        commands.entity(table).despawn_related::<Children>();
        let Some(row) = held.0.row(cursor.0) else {
            say_instead(&mut commands, table, NOTHING_HIGHLIGHTED.to_owned());
            continue;
        };
        let rows: Vec<Vec<String>> = row
            .read_out(&draft.plan, months, MoneyForm::Full)
            .into_iter()
            .map(|(label, text)| vec![label, text])
            .collect();
        commands
            .entity(table)
            .insert(tabulate::labelled(&rows, GAP));
        tabulate::refill(
            &mut commands,
            (table, &mut scroll),
            (&header, &rows),
            &[0, 1],
        );
    }
}

fn say_help(
    (held, focus, shown): (Res<Held>, Res<InputFocus>, ShownSurface),
    (theme, keymap): (Res<Theme>, Res<Keymap>),
    tables: Query<(), With<ProposalsTable>>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    if !(held.is_changed() || focus.is_changed() || shown.is_changed() || theme.is_changed()) {
        return;
    }
    let found = held.0.estimate();
    let is_on_rows = focus.get().is_some_and(|holder| tables.contains(holder));
    let text = if is_on_rows && !found.proposals.is_empty() {
        format!("{ON_PROPOSALS}{}", writes_it(&keymap, WRITE_ESTIMATE))
    } else if found.through.is_some() {
        summary(found, MoneyForm::Full)
    } else {
        ABOUT.to_owned()
    };
    show_help(&mut lines, Page::SpendingEstimate, &text, &theme);
}
