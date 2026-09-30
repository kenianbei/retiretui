//! The Tax Tables page: the tables the projection applies in the shared
//! year, for the plan's filing status and the state it lives in, or those
//! picked - read out, searching nothing.

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Query, Res, Resource, With,
};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::ui::{ScrollArea, UiStyle};
use retiretui_client::tax_tables::{TablesView, YearTables, year_tables};
use retiretui_engine::plan::{US_STATES, place_name};

use super::{HelpLine, show_help};
use crate::edit::{Draft, table_bundle};
use crate::hints::Hints;
use crate::layout::{self, Body, filling, placed};
use crate::nav::{self, FocusStop, Page};
use crate::pane::{Framed, Pane};
use crate::session::{Session, Shown};
use crate::tabulate;
use crate::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<TaxPicks>();
    app.add_systems(Startup, spawn_page.after(layout::spawn_frame));
    app.add_systems(Update, refresh_tables.before(Repainted));
}

/// The filing status and the state picked in place of the plan's own,
/// held for the session.
#[derive(Resource, Default, Debug)]
pub struct TaxPicks {
    pub status: Option<String>,
    pub state: Option<String>,
}

#[derive(Component)]
struct TaxTable;

/// The cells between one column and the next.
const GAP: u16 = 2;
/// The most columns a section has.
const WIDEST: usize = 3;

fn spawn_page(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    let view = nav::spawn_surface(&mut commands, body, Some(Page::TaxTables));
    let pane = Pane::new(Page::TaxTables.title()).spawn(&mut commands, view);
    commands.spawn((
        table_bundle(),
        TaxTable,
        FocusStop,
        Hints(&[("↑↓", "line")]),
        layout::Rests,
        filling(),
        placed(),
        ChildOf(pane),
    ));
    super::spawn_help(&mut commands, view, Page::TaxTables);
}

/// How a row of the page is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Title,
    Columns,
    Figures,
    Note,
}

/// `cells`, blank to [`WIDEST`].
fn padded(cells: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut row: Vec<String> = cells.into_iter().collect();
    row.resize(WIDEST, String::new());
    row
}

/// The sections as rows of [`WIDEST`] cells: each title, its column
/// headers, its rows or its note, and a blank line between sections.
fn rows(tables: &YearTables) -> Vec<(Kind, Vec<String>)> {
    let mut rows = Vec::new();
    for section in &tables.sections {
        if !rows.is_empty() {
            rows.push((Kind::Figures, padded([])));
        }
        rows.push((Kind::Title, padded([section.title.clone()])));
        if let Some(note) = &section.note {
            rows.push((Kind::Note, padded([note.clone()])));
            continue;
        }
        let columns = section.columns.iter().map(|&column| column.to_owned());
        rows.push((Kind::Columns, padded(columns)));
        let figures = section
            .rows
            .iter()
            .map(|row| (Kind::Figures, padded(row.iter().cloned())));
        rows.extend(figures);
    }
    rows
}

/// "Tax Tables · 2027 · Married filing jointly · Oregon".
fn title(tables: &YearTables) -> String {
    let status = (tables.statuses.iter())
        .find(|offer| offer.value == tables.status)
        .map_or(tables.status.as_str(), |offer| offer.label.as_str());
    let mut title = format!("{} · {} · {status}", Page::TaxTables.title(), tables.year);
    if let Some(code) = &tables.state {
        title = format!("{title} · {}", place_name(US_STATES, code).unwrap_or(code));
    }
    title
}

fn refresh_tables(
    (draft, session, picks, shown, theme): (
        Res<Draft>,
        Res<Session>,
        Res<TaxPicks>,
        Shown,
        Res<Theme>,
    ),
    mut tables: Query<(Entity, &mut ScrollArea, &ChildOf), With<TaxTable>>,
    mut panes: Query<&mut Framed>,
    mut help: Query<(&mut UiWidget, &HelpLine)>,
    mut commands: Commands,
) {
    let is_moved = draft.is_changed() || picks.is_changed() || shown.is_changed();
    if !is_moved && !theme.is_changed() {
        return;
    }
    let view = TablesView {
        year: shown.year(),
        status: picks.status.clone(),
        state: picks.state.clone(),
    };
    let said = year_tables(&draft.plan, &session.tables, &view);
    show_help(&mut help, Page::TaxTables, said.about, &theme);
    let body = rows(&said);
    // The sections carry their own headers.
    let header = padded([]);
    let cells: Vec<Vec<String>> = body.iter().map(|(_, cells)| cells.clone()).collect();
    let widths = tabulate::columns((&header, &cells), GAP);
    for (table, mut scroll, &ChildOf(pane)) in &mut tables {
        commands.entity(table).insert(widths.clone());
        let spawned =
            tabulate::refill(&mut commands, (table, &mut scroll), (&header, &cells), &[0]);
        for (row, (kind, _)) in spawned.into_iter().zip(&body) {
            if let Some(style) = style_of(*kind, &theme) {
                commands.entity(row).insert(UiStyle(style));
            }
        }
        if let Ok(mut framed) = panes.get_mut(pane) {
            Framed::retitle(&mut framed, &title(&said));
        }
    }
}

fn style_of(kind: Kind, theme: &Theme) -> Option<Style> {
    match kind {
        Kind::Title => Some(Style::new().add_modifier(Modifier::BOLD)),
        Kind::Columns | Kind::Note => Some(theme.dimmed()),
        Kind::Figures => None,
    }
}
