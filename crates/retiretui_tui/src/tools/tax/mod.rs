//! The Tax Tables page: the tables the projection applies in the shared
//! year, for the plan's filing status and the state it lives in, or those
//! picked - read out, searching nothing.

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, In, IntoScheduleConfigs, Query, Res, ResMut, Resource, With, World,
};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::ui::{ScrollArea, UiStyle};
use retiretui_client::forms::offers::Offer;
use retiretui_client::tax_tables::{STATE_PICK, STATUS_PICK, TablesView, YearTables, year_tables};

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
    app.init_resource::<Held>();
    app.add_systems(Startup, (spawn_page.after(layout::spawn_frame), register));
    app.add_systems(
        Update,
        refresh_tables
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

/// The tables last shown, which the pickers offer the choices of.
#[derive(Resource, Default)]
struct Tabled(Option<YearTables>);

#[derive(Component)]
struct TaxTable;

/// The cells between one column and the next.
const GAP: u16 = 2;

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

/// The sections as styled rows: each title, its column headers, its rows
/// or its note, and a blank line between sections.
fn rows(tables: &YearTables, theme: &Theme) -> Vec<(Option<Style>, Vec<String>)> {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let dimmed = theme.dimmed();
    let mut rows = Vec::new();
    for section in &tables.sections {
        if !rows.is_empty() {
            rows.push((None, Vec::new()));
        }
        rows.push((Some(bold), vec![section.title.clone()]));
        if let Some(note) = &section.note {
            rows.push((Some(dimmed), vec![note.clone()]));
            continue;
        }
        let columns = section.columns.iter().map(|&column| column.to_owned());
        rows.push((Some(dimmed), columns.collect()));
        rows.extend(section.rows.iter().map(|row| (None, row.clone())));
    }
    rows
}

/// "Tax Tables · 2027 · Married filing jointly · Oregon".
fn title(tables: &YearTables) -> String {
    let page = Page::TaxTables.title();
    let said = format!("{page} · {} · {}", tables.year, tables.status_name);
    match &tables.state_name {
        Some(state) => format!("{said} · {state}"),
        None => said,
    }
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
    show_help(&mut help, Page::TaxTables, said.about, &theme);
    let (mut styles, mut cells): (Vec<_>, Vec<_>) = rows(&said, &theme).into_iter().unzip();
    if cells.is_empty() {
        return;
    }
    // The first section's title heads the table, bold as every title is.
    styles.remove(0);
    let header = cells.remove(0);
    let widths = tabulate::columns((&header, &cells), GAP);
    for (table, mut scroll, &ChildOf(pane)) in &mut tables {
        commands.entity(table).insert(widths.clone());
        let spawned =
            tabulate::refill(&mut commands, (table, &mut scroll), (&header, &cells), &[0]);
        for (row, style) in spawned.into_iter().zip(&styles) {
            if let Some(style) = style {
                commands.entity(row).insert(UiStyle(*style));
            }
        }
        if let Ok(mut framed) = panes.get_mut(pane) {
            Framed::retitle(&mut framed, &title(&said));
        }
    }
    last.0 = Some(said);
}

/// Which of the view a picker picks.
#[derive(Clone, Copy, Debug)]
pub enum Pick {
    Status,
    State,
}

impl Pick {
    fn slot(self, picks: &mut TaxPicks) -> &mut Option<String> {
        match self {
            Self::Status => &mut picks.status,
            Self::State => &mut picks.state,
        }
    }

    /// What the picker offers after the plan's own row.
    fn offers(self, tables: &YearTables) -> (&str, &[Offer]) {
        match self {
            Self::Status => (&tables.own_status, &tables.statuses),
            Self::State => (&tables.own_state, &tables.states),
        }
    }

    /// The value the row `id` stands for: none for the plan's own row.
    fn value_at(self, tables: &YearTables, id: usize) -> Option<String> {
        let (_, offers) = self.offers(tables);
        let at = id.checked_sub(1)?;
        offers.get(at).map(|offer| offer.value.clone())
    }
}

/// The page's two pickers.
#[derive(Resource, Clone, Copy)]
pub struct TaxPickers {
    status: Picker,
    state: Picker,
}

/// The picks when a picker opened, put back where it closes with nothing
/// chosen.
#[derive(Resource, Default)]
pub struct Held(TaxPicks);

fn register(world: &mut World) {
    let status = picker(world, Pick::Status, (STATUS_PICK, "filing status"));
    let state = picker(world, Pick::State, (STATE_PICK, "state"));
    world.insert_resource(TaxPickers { status, state });
}

fn picker(world: &mut World, pick: Pick, named: (&'static str, &'static str)) -> Picker {
    let list = move |In(query): In<String>, tabled: Res<Tabled>| {
        let Some(tables) = &tabled.0 else {
            return Vec::new();
        };
        let (own, offers) = pick.offers(tables);
        let labels = std::iter::once(own).chain(offers.iter().map(|offer| offer.label.as_str()));
        ranked(
            &query,
            labels
                .enumerate()
                .map(|(id, label)| Offered::new(id, label)),
        )
    };
    let set = move |In(id): In<usize>, tabled: Res<Tabled>, picks: ResMut<TaxPicks>| {
        if let Some(tables) = &tabled.0 {
            let value = pick.value_at(tables, id);
            picks
                .map_unchanged(|picks| pick.slot(picks))
                .set_if_neq(value);
        }
    };
    let restore = |mut held: ResMut<Held>, mut picks: ResMut<TaxPicks>| {
        picks.set_if_neq(std::mem::take(&mut held.0));
    };
    Picker::new(world, named, list, set).trying(world, set, restore)
}

/// The command that opens `pick`'s picker.
pub fn opens(
    pick: Pick,
) -> impl FnMut(Res<TaxPickers>, Res<TaxPicks>, ResMut<Held>, ResMut<Picking>) -> Outcome {
    move |pickers, picks, mut held, mut picking| {
        held.0.clone_from(&picks);
        picking.open(match pick {
            Pick::Status => pickers.status,
            Pick::State => pickers.state,
        });
        Outcome::Done
    }
}

#[cfg(test)]
mod tests;
