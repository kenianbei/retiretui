//! The Spending Estimate tool: a bank's exports read for the session, each
//! category's spending proposed as the amount of the expense that is for
//! it, beside the highlighted proposal read out, and the proposals taken
//! into the draft or written as a scenario.

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, In, IntoScheduleConfigs, Local, Query, Res, ResMut,
    Resource, With, World,
};
use bevy_input_focus::InputFocus;
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::ScrollArea;
use plurimus::widgets::{ActiveDescendant, list_item};
use retiretui_client::estimate::{
    self, ABOUT, COLUMNS, Exports, NOTHING_READ, PROPOSAL_FIELDS, ProposalAnswers, UNKNOWN_FIELDS,
    UnknownAnswers, listed, summary, take_question, taken,
};
use retiretui_client::present::counted;
use retiretui_engine::estimate::{Estimate, Patterns, apply};
use retiretui_engine::plan::Plan;

use super::options::say_instead;
use super::{EnterRuns, HelpLine, ToolPage, show_help, write, writes_it};
use crate::command::{EDIT_PROPOSAL, Keymap, Outcome, WRITE_ESTIMATE};
use crate::confirm::{Answer, Confirm};
use crate::documents::{Browsing, Pickers};
use crate::edit::{self, Draft, DraftEditor, FormButton, Ops, Row, Slot, table_bundle};
use crate::hints::Hints;
use crate::journal;
use crate::layout::{self, Body, filling, placed};
use crate::nav::{self, FocusStop, Page, ShownSurface};
use crate::pane::Pane;
use crate::present::MoneyForm;
use crate::session::{self, Session};
use crate::tabulate;
use crate::theme::{Repainted, Theme};

const FILES_TITLE: &str = "Files";
const PROPOSALS_TITLE: &str = "Proposals";
const READ_OUT_TITLE: &str = "Proposal";
/// The right column's width, borders included.
const SIDE_COLS: f32 = 48.0;
/// Borders, and the exports a session is likely to hold.
const FILES_ROWS: f32 = 6.0;
const GAP: u16 = 1;
const ON_PROPOSALS: &str = "⏎ edits the row; t takes the proposals into the plan, after asking";
const NOTHING_HIGHLIGHTED: &str = "nothing highlighted";
const NO_EXPORTS: &str = "no export to drop";
const NO_FILES: &str = "none yet";
const NO_PROPOSALS: &str = "nothing proposed yet";

pub fn plugin(app: &mut App) {
    app.init_resource::<Held>();
    app.init_resource::<HeldPatterns>();
    app.init_resource::<Estimated>();
    app.init_resource::<Cursor>();
    app.add_systems(Startup, spawn_page.after(layout::spawn_frame));
    app.add_systems(Update, estimate_by_itself.before(Repainted));
    app.add_systems(
        Update,
        (
            list_files,
            table_proposals,
            follow_cursor,
            show_read_out,
            say_help,
        )
            .chain()
            .after(estimate_by_itself)
            .run_if(nav::shows(Page::SpendingEstimate))
            .before(Repainted),
    );
}

/// The exports the session holds.
#[derive(Resource, Default)]
pub struct Held(pub Exports);

/// The patterns in force: the embedded table under the user's own, as the
/// launch supplies them, and under what the session names a payee for.
#[derive(Resource)]
pub struct HeldPatterns(pub Patterns);

impl Default for HeldPatterns {
    fn default() -> Self {
        Self(Patterns::embedded())
    }
}

/// What the exports say the draft spends.
#[derive(Resource, Default)]
pub struct Estimated(pub Estimate);

/// The place of the highlighted row among the proposals, then the unknown
/// payees.
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

const PROPOSAL_OPS: Ops = Ops::tool::<ProposalAnswers>(
    Some(Page::SpendingEstimate),
    READ_OUT_TITLE,
    PROPOSAL_FIELDS,
)
.acting(ACTIONS, after_proposal);
const UNKNOWN_OPS: Ops =
    Ops::tool::<UnknownAnswers>(Some(Page::SpendingEstimate), "Payee", UNKNOWN_FIELDS)
        .acting(ACTIONS, after_payee);
const ACTIONS: [&str; 2] = ["Discard", "Apply"];
const _: () = assert!(
    edit::help_fits(PROPOSAL_OPS.form.fields) && edit::help_fits(UNKNOWN_OPS.form.fields),
    "a field's help is missing or too long"
);

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
    let column = Node {
        flex_direction: FlexDirection::Column,
        width: Val::Px(SIDE_COLS),
        height: Val::Percent(100.0),
        ..Node::default()
    };
    let column = commands.spawn((column, ChildOf(row))).id();
    let files = Pane::new(FILES_TITLE)
        .tall(FILES_ROWS)
        .spawn(commands, column);
    let list = layout::spawn_scrolled_list(commands, files, Hints(&[("↑↓", "file")]));
    commands.entity(list).insert(FileList);
    let beneath = Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_height: Val::Px(0.0),
        ..Node::default()
    };
    let beneath = commands.spawn((beneath, ChildOf(column))).id();
    let read_out = Pane::new(READ_OUT_TITLE).spawn(commands, beneath);
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
}

/// Estimates again whenever the plan, the exports or the patterns move;
/// the form's answers, held beside the plan, move nothing.
fn estimate_by_itself(
    (draft, held, patterns): (Res<Draft>, Res<Held>, Res<HeldPatterns>),
    mut estimated_over: Local<Option<Plan>>,
    mut estimated: ResMut<Estimated>,
) {
    let is_plan_moved = draft.is_changed() && estimated_over.as_ref() != Some(&draft.plan);
    if !(is_plan_moved || held.is_changed() || patterns.is_changed()) {
        return;
    }
    *estimated_over = Some(draft.plan.clone());
    estimated.0 = held.0.estimate(&draft.plan, &patterns.0);
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
    for (list, on) in &lists {
        let kept =
            on.0.and_then(|row| rows.get(row).ok())
                .map_or(0, |row| row.0);
        commands.entity(list).despawn_related::<Children>();
        let spawned: Vec<Entity> = held
            .0
            .listed()
            .enumerate()
            .map(|(at, (name, count))| {
                let said = format!("{name}  {}", counted(count, "row", "rows"));
                let item = list_item(Line::from(said));
                commands.spawn((item, FileRow(at), ChildOf(list))).id()
            })
            .collect();
        if spawned.is_empty() {
            let none = list_item(Line::styled(NO_FILES, theme.dimmed()));
            commands.spawn((none, ChildOf(list)));
        }
        let at = kept.min(spawned.len().saturating_sub(1));
        commands
            .entity(list)
            .insert(ActiveDescendant(spawned.get(at).copied()));
    }
}

fn table_proposals(
    (estimated, cursor, shown): (Res<Estimated>, Res<Cursor>, ShownSurface),
    mut tables: Query<(Entity, &mut ScrollArea), With<ProposalsTable>>,
    mut commands: Commands,
) {
    if !(estimated.is_changed() || shown.is_changed()) {
        return;
    }
    let months = estimated.0.months;
    let rows: Vec<Vec<String>> = listed(&estimated.0)
        .iter()
        .map(|row| row.cells(months, MoneyForm::Full).to_vec())
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
        let spawned = tabulate::refill(
            &mut commands,
            (table, &mut scroll),
            (&header, &rows),
            &[0, 1],
        );
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
    (estimated, cursor, draft, shown): (Res<Estimated>, Res<Cursor>, Res<Draft>, ShownSurface),
    mut outs: Query<(Entity, &mut ScrollArea), With<ReadOut>>,
    mut commands: Commands,
) {
    if !(estimated.is_changed() || cursor.is_changed() || shown.is_changed()) {
        return;
    }
    let rows_of = listed(&estimated.0);
    let header: [String; 0] = [];
    for (table, mut scroll) in &mut outs {
        commands.entity(table).despawn_related::<Children>();
        let Some(row) = rows_of.get(cursor.0) else {
            say_instead(&mut commands, table, NOTHING_HIGHLIGHTED.to_owned());
            continue;
        };
        let read = row.read_out(&draft.plan, estimated.0.months, MoneyForm::Full);
        let rows: Vec<Vec<String>> = read
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
    (estimated, focus, shown): (Res<Estimated>, Res<InputFocus>, ShownSurface),
    (theme, keymap): (Res<Theme>, Res<Keymap>),
    tables: Query<(), With<ProposalsTable>>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    if !(estimated.is_changed() || focus.is_changed() || shown.is_changed() || theme.is_changed()) {
        return;
    }
    let is_on_rows = focus.get().is_some_and(|holder| tables.contains(holder));
    let text = if is_on_rows && !estimated.0.proposals.is_empty() {
        format!("{ON_PROPOSALS}{}", writes_it(&keymap, WRITE_ESTIMATE))
    } else if estimated.0.through.is_some() {
        summary(&estimated.0, MoneyForm::Full)
    } else {
        ABOUT.to_owned()
    };
    show_help(&mut lines, Page::SpendingEstimate, &text, &theme);
}

/// The `add-export` command: asks which export to read.
pub fn add_picker(pickers: Res<Pickers>, mut browsing: ResMut<Browsing>) -> Outcome {
    browsing.open(pickers.export);
    Outcome::Done
}

/// Reads the export at `path` into the session, in place of one of the
/// same name.
pub fn add_export(In(path): In<PathBuf>, session: Res<Session>, mut held: ResMut<Held>) {
    let name = session::file_name(&path).into_owned();
    let read = session
        .store
        .read(&path)
        .map_err(|error| format!("{}: {error}", path.display()))
        .and_then(|text| held.0.add(&name, &text).map_err(|error| error.to_string()));
    match read {
        Ok(count) => journal::say(format!(
            "read {} from {name}",
            counted(count, "row", "rows")
        )),
        Err(reason) => journal::warn(format!("not read: {reason}")),
    }
}

/// The `remove-export` command: drops the export under the files' cursor.
pub fn remove_export(
    lists: Query<&ActiveDescendant, With<FileList>>,
    rows: Query<&FileRow>,
    mut held: ResMut<Held>,
) -> Outcome {
    let at = lists
        .iter()
        .find_map(|on| on.0.and_then(|row| rows.get(row).ok()))
        .map(|row| row.0);
    let name = at
        .and_then(|at| held.0.listed().nth(at))
        .map(|(name, _)| name.to_owned());
    let Some(name) = name else {
        return Outcome::Refused(NO_EXPORTS.to_owned());
    };
    held.0.remove(&name);
    journal::say(format!("dropped {name}"));
    Outcome::Done
}

/// The `edit-proposal` command: opens the highlighted proposal's form, or
/// the payee's, over the page.
pub fn edit_row(
    (estimated, cursor): (Res<Estimated>, Res<Cursor>),
    mut draft: ResMut<Draft>,
    mut commands: Commands,
) -> Outcome {
    let count = estimated.0.proposals.len();
    let ops = if let Some(proposal) = estimated.0.proposals.get(cursor.0) {
        draft.set_answers::<ProposalAnswers>(estimate::proposal_table(proposal));
        PROPOSAL_OPS
    } else if cursor.0 < count + estimated.0.unknown.len() {
        draft.set_answers::<UnknownAnswers>(toml::Table::new());
        UNKNOWN_OPS
    } else {
        return Outcome::Refused(NOTHING_HIGHLIGHTED.to_owned());
    };
    commands.run_system_cached_with(edit::open_item, (ops, None, Slot::At(Row(0))));
    Outcome::Done
}

fn after_proposal(button: FormButton, commands: &mut Commands) {
    if matches!(button, FormButton::Apply) {
        commands.run_system_cached(apply_proposal);
    }
}

fn after_payee(button: FormButton, commands: &mut Commands) {
    if matches!(button, FormButton::Apply) {
        commands.run_system_cached(apply_payee);
    }
}

/// Writes the form's answers into the highlighted proposal.
fn apply_proposal(draft: Res<Draft>, cursor: Res<Cursor>, mut estimated: ResMut<Estimated>) {
    let Some(proposal) = estimated.0.proposals.get_mut(cursor.0) else {
        return;
    };
    if let Err(reason) = estimate::proposal_from(draft.answers::<ProposalAnswers>(), proposal) {
        journal::warn(reason);
    }
}

/// Sorts the highlighted payee's rows under the category the form named,
/// for the session, which estimates again.
fn apply_payee(
    (draft, cursor, estimated): (Res<Draft>, Res<Cursor>, Res<Estimated>),
    mut patterns: ResMut<HeldPatterns>,
) {
    let at = cursor.0.saturating_sub(estimated.0.proposals.len());
    let Some(unknown) = estimated.0.unknown.get(at) else {
        return;
    };
    match estimate::category_from(draft.answers::<UnknownAnswers>()) {
        Ok(Some(category)) => patterns.0.assign(&unknown.payee, category),
        Ok(None) => {}
        Err(reason) => journal::warn(reason),
    }
}

/// The `take-estimate` command: asks before the proposals set the plan's
/// expenses. The answer carries the proposals asked about, so an estimate
/// that moves under the question changes nothing.
pub fn adopt(
    estimated: Res<Estimated>,
    draft: Res<Draft>,
    mut confirm: ResMut<Confirm>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    if estimated.0.proposals.is_empty() {
        return Outcome::Refused(NO_PROPOSALS.to_owned());
    }
    let asked = estimated.0.clone();
    let answers = vec![
        Answer::closing("Cancel"),
        Answer::running("Take", move |commands| {
            commands.run_system_cached_with(take, asked.clone());
        })
        .primary(),
    ];
    confirm.ask_among(take_question(&estimated.0), answers);
    Outcome::Done
}

fn take(In(estimate): In<Estimate>, mut editor: DraftEditor) {
    apply(&mut editor.draft.plan, &estimate);
    editor.commit();
    journal::say(taken(&estimate));
}

/// The `write-estimate` command: asks where to write the proposals applied.
pub fn write_picker(
    pickers: Res<Pickers>,
    estimated: Res<Estimated>,
    draft: Res<Draft>,
    mut browsing: ResMut<Browsing>,
) -> Outcome {
    if estimated.0.proposals.is_empty() {
        return Outcome::Refused(NO_PROPOSALS.to_owned());
    }
    write::open_picker(pickers.estimate, &draft, &mut browsing)
}

/// Writes the proposals applied at `path` as a scenario over the document,
/// and compares the file written.
pub fn write_overlay(In(path): In<PathBuf>, world: &mut World) {
    let text = write::base_of(world, &path).and_then(|base| {
        let estimated = &world.resource::<Estimated>().0;
        let plan = &world.resource::<Draft>().plan;
        estimate::overlay(&base, plan, estimated).map_err(|error| format!("not written: {error}"))
    });
    write::write(world, path, text);
}
