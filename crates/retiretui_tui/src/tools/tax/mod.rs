//! The Tax Tables page: the tables the projection applies in the shared
//! year, for the plan's filing status and the state it lives in, or those
//! picked - read out, searching nothing.

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, In, IntoScheduleConfigs, Query, Res, ResMut, Resource, With, World,
};
use bevy_ecs::system::SystemParam;
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::ui::{ScrollArea, UiStyle};
use retiretui_client::tax_tables::{STATE_PICK, STATUS_PICK, TablesView, YearTables, year_tables};
use retiretui_engine::plan::{US_STATES, place_name};

use super::{HelpLine, show_help};
use crate::command::Outcome;
use crate::edit::{Draft, table_bundle};
use crate::hints::Hints;
use crate::layout::{self, Body, filling, placed};
use crate::nav::{self, FocusStop, Page};
use crate::pane::{Framed, Pane};
use crate::picker::{Offered, Picker, Picking, ranked};
use crate::session::{Session, Shown};
use crate::tabulate;
use crate::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<TaxPicks>();
    app.init_resource::<Tabled>();
    app.add_systems(Startup, (spawn_page.after(layout::spawn_frame), register));
    app.add_systems(Update, refresh_tables.before(Repainted));
}

/// The filing status and the state picked in place of the plan's own,
/// held for the session.
#[derive(Resource, Default, Debug)]
pub struct TaxPicks {
    pub status: Option<String>,
    pub state: Option<String>,
}

/// The tables last shown, which the pickers offer the choices of.
#[derive(Resource, Default)]
struct Tabled(Option<YearTables>);

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
    (mut last, mut commands): (ResMut<Tabled>, Commands),
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
    last.0 = Some(said);
}

fn style_of(kind: Kind, theme: &Theme) -> Option<Style> {
    match kind {
        Kind::Title => Some(Style::new().add_modifier(Modifier::BOLD)),
        Kind::Columns | Kind::Note => Some(theme.dimmed()),
        Kind::Figures => None,
    }
}

/// Which of the view a picker picks.
#[derive(Clone, Copy, Debug)]
enum Pick {
    Status,
    State,
}

impl Pick {
    fn held(self, picks: &TaxPicks) -> Option<&String> {
        match self {
            Self::Status => picks.status.as_ref(),
            Self::State => picks.state.as_ref(),
        }
    }

    /// Puts `value` in the pick's place, leaving the picks unchanged
    /// where it is there already.
    fn put(self, picks: &mut ResMut<TaxPicks>, value: Option<String>) {
        if self.held(picks) == value.as_ref() {
            return;
        }
        match self {
            Self::Status => picks.status = value,
            Self::State => picks.state = value,
        }
    }

    /// What the picker offers, by id: the plan's own, then each choice.
    fn choices(self, tables: &YearTables) -> Vec<(Option<String>, String)> {
        let (own, offers) = match self {
            Self::Status => (&tables.own_status, &tables.statuses),
            Self::State => (&tables.own_state, &tables.states),
        };
        let each = (offers.iter()).map(|offer| (Some(offer.value.clone()), offer.label.clone()));
        std::iter::once((None, own.clone())).chain(each).collect()
    }
}

/// The page's two pickers.
#[derive(Resource, Clone, Copy)]
pub struct TaxPickers {
    status: Picker,
    state: Picker,
}

/// What was picked when a picker opened, put back where it closes with
/// nothing chosen.
#[derive(Resource, Default)]
pub struct Held(Option<String>);

fn register(world: &mut World) {
    world.init_resource::<Held>();
    let status = picker(world, Pick::Status, (STATUS_PICK, "filing status"));
    let state = picker(world, Pick::State, (STATE_PICK, "state"));
    world.insert_resource(TaxPickers { status, state });
}

fn picker(world: &mut World, pick: Pick, named: (&'static str, &'static str)) -> Picker {
    let list = move |In(query): In<String>, tabled: Res<Tabled>| {
        let choices = tabled.0.as_ref().map(|tables| pick.choices(tables));
        let offered = (choices.into_iter().flatten().enumerate())
            .map(|(id, (_, label))| Offered::new(id, label));
        ranked(&query, offered)
    };
    let set = move |In(id): In<usize>, tabled: Res<Tabled>, mut picks: ResMut<TaxPicks>| {
        let chosen =
            (tabled.0.as_ref()).and_then(|tables| pick.choices(tables).into_iter().nth(id));
        if let Some((value, _)) = chosen {
            pick.put(&mut picks, value);
        }
    };
    let restore = move |mut held: ResMut<Held>, mut picks: ResMut<TaxPicks>| {
        pick.put(&mut picks, held.0.take());
    };
    Picker::new(world, named, list, set).trying(world, set, restore)
}

/// What opening a picker asks of the world.
#[derive(SystemParam)]
pub struct Opening<'w> {
    pickers: Res<'w, TaxPickers>,
    picks: Res<'w, TaxPicks>,
    held: ResMut<'w, Held>,
    picking: ResMut<'w, Picking>,
}

impl Opening<'_> {
    fn open(&mut self, pick: Pick) -> Outcome {
        self.held.0 = pick.held(&self.picks).cloned();
        self.picking.open(match pick {
            Pick::Status => self.pickers.status,
            Pick::State => self.pickers.state,
        });
        Outcome::Done
    }
}

/// The `tax-status` command: picks the filing status the tables are for.
pub fn pick_status(mut opening: Opening) -> Outcome {
    opening.open(Pick::Status)
}

/// The `tax-state` command: picks the state the tables are for.
pub fn pick_state(mut opening: Opening) -> Outcome {
    opening.open(Pick::State)
}

#[cfg(test)]
mod tests;
