//! The Tax Tables page: the tables the projection applies in the shared
//! year, for the plan's filing status and the state it lives in, or those
//! picked - their sections listed beside the highlighted one's table, read
//! out, searching nothing.

mod pick;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, IntoScheduleConfigs, Query, Res, ResMut, Resource, With,
};
use plurimus::core::ratatui_core::text::Line;
use plurimus::widgets::{ActiveDescendant, list_item};
use retiretui_client::tax_tables::{TablesView, TaxSection, YearTables, year_tables};

pub use pick::{Pick, opens};

use super::{Help, ToolPage};
use crate::edit::Draft;
use crate::grid::{self, Grid};
use crate::hints::Hints;
use crate::layout::{self, Body};
use crate::nav::{self, FocusStop, Page};
use crate::pane::{Framed, Pane};
use crate::session::{Session, Shown};
use crate::tabulate;
use crate::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<TaxPicks>();
    app.init_resource::<Tabled>();
    app.init_resource::<SectionCursor>();
    app.init_resource::<pick::Held>();
    app.add_systems(
        Startup,
        (spawn_page.after(layout::spawn_frame), pick::register),
    );
    app.add_systems(
        Update,
        (list_sections, follow_cursor, show_section)
            .chain()
            .run_if(nav::shows(Page::TaxTables))
            .before(Repainted),
    );
}

/// The filing status and the state picked in place of the plan's own,
/// held for the session.
#[derive(Resource, Default, Clone, PartialEq, Eq, Debug)]
pub struct TaxPicks {
    pub status: Option<String>,
    pub state: Option<String>,
}

/// The tables last shown, which the list and the pickers are drawn from.
#[derive(Resource, Default)]
struct Tabled(Option<YearTables>);

/// The place of the section highlighted, kept across a year or a pick.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
struct SectionCursor(usize);

#[derive(Component)]
struct SectionList;

/// The place of the section a row of the list names.
#[derive(Component, Clone, Copy)]
struct SectionRow(usize);

#[derive(Component)]
struct SectionTable;

/// The cells between one column and the next.
const GAP: u16 = 2;
/// The list's width, borders included: the longest title beside the
/// cursor.
const LIST_COLS: f32 = 35.0;

static PAGE: ToolPage = ToolPage {
    surface: Page::TaxTables,
    panes: spawn_panes,
};

fn spawn_page(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    if let Ok(body) = bodies.single() {
        super::spawn_tool(&mut commands, body, &PAGE);
    }
}

fn spawn_panes(commands: &mut Commands, row: Entity) {
    let sections = Pane::new("Sections").wide(LIST_COLS).spawn(commands, row);
    let list = layout::spawn_scrolled_list(commands, sections, Hints(&[("↑↓", "section")]));
    commands.entity(list).insert(SectionList);
    let pane = Pane::new(Page::TaxTables.title()).spawn(commands, row);
    let table = grid::spawn(commands, pane);
    commands.entity(table).insert((
        SectionTable,
        FocusStop,
        Hints(&[("↑↓", "line")]),
        layout::Rests,
    ));
}

/// Works the tables out again whenever what they are for moves, and lists
/// their sections, the cursor kept in its place.
fn list_sections(
    (draft, session, picks, shown): (Res<Draft>, Res<Session>, Res<TaxPicks>, Shown),
    (cursor, theme): (Res<SectionCursor>, Res<Theme>),
    lists: Query<Entity, With<SectionList>>,
    mut help: Help,
    (mut last, mut commands): (ResMut<Tabled>, Commands),
) {
    let is_year_moved = shown.is_year_changed() || shown.projected.is_changed();
    if !(draft.is_changed() || picks.is_changed() || is_year_moved || theme.is_changed()) {
        return;
    }
    let view = TablesView {
        year: shown.year(),
        status: picks.status.clone(),
        state: picks.state.clone(),
    };
    let said = year_tables(&draft.plan, &session.tables, &view);
    help.show(Page::TaxTables, said.about);
    let kept = cursor.0.min(said.sections.len().saturating_sub(1));
    for list in &lists {
        commands.entity(list).despawn_related::<Children>();
        let titles = said.sections.iter().enumerate();
        let rows: Vec<Entity> = titles
            .map(|(at, section)| {
                let item = list_item(Line::from(section.title.clone()));
                commands.spawn((item, SectionRow(at), ChildOf(list))).id()
            })
            .collect();
        commands
            .entity(list)
            .insert(ActiveDescendant(rows.get(kept).copied()));
    }
    last.0 = Some(said);
}

/// The section the list's cursor rests on is the one tabled beside it.
fn follow_cursor(
    lists: Query<&ActiveDescendant, (With<SectionList>, Changed<ActiveDescendant>)>,
    rows: Query<&SectionRow>,
    mut cursor: ResMut<SectionCursor>,
) {
    for on in &lists {
        if let Some(&SectionRow(at)) = on.0.and_then(|row| rows.get(row).ok()) {
            cursor.set_if_neq(SectionCursor(at));
        }
    }
}

/// Tables the highlighted section: its columns over its rows, or its note.
fn show_section(
    (tabled, cursor): (Res<Tabled>, Res<SectionCursor>),
    sections: Query<(&Grid, &ChildOf), With<SectionTable>>,
    mut panes: Query<&mut Framed>,
    mut commands: Commands,
) {
    if !tabled.is_changed() && !cursor.is_changed() {
        return;
    }
    let Some(shown) = &tabled.0 else {
        return;
    };
    let Some(section) = shown.sections.get(cursor.0) else {
        return;
    };
    for (grid, &ChildOf(pane)) in &sections {
        fill_section(&mut commands, grid, section);
        if let Ok(mut framed) = panes.get_mut(pane) {
            Framed::retitle(&mut framed, &title(shown, section));
        }
    }
}

fn fill_section(commands: &mut Commands, grid: &Grid, section: &TaxSection) {
    if let Some(note) = &section.note {
        grid::say_instead(commands, grid, note.clone());
        return;
    }
    let header: Vec<String> = (section.columns.iter())
        .map(|&column| column.to_owned())
        .collect();
    let widths = tabulate::columns((&header, &section.rows), GAP);
    grid::refill(commands, grid, widths, (&header, &section.rows), &[0]);
}

/// "Income tax brackets · 2027 · Married filing jointly · Oregon".
fn title(tables: &YearTables, section: &TaxSection) -> String {
    let said = format!(
        "{} · {} · {}",
        section.title, tables.year, tables.status_name
    );
    match &tables.state_name {
        Some(state) => format!("{said} · {state}"),
        None => said,
    }
}

#[cfg(test)]
mod tests;
