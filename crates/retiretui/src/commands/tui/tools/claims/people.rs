//! The SSA Benefits page's People table: each person's earnings record,
//! their Social Security income and what it comes to claimed at 62, full
//! retirement age and 70. It holds the keyboard for the page, and its
//! cursor is the person the page's commands act on.

use std::collections::BTreeSet;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, IntoScheduleConfigs, Local, Query, Res, ResMut, Resource,
    With,
};
use plurimus::widgets::{ActiveDescendant, WidgetSystems};
use retiretui_engine::optimize::benefit_estimates;
use retiretui_engine::plan::{Dollars, Income, Person, Plan};

use super::super::{EnterRuns, handle_enter};
use super::guide;
use crate::commands::tui::edit::{Draft, table_bundle};
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{self, filling, placed};
use crate::commands::tui::nav::{ActivePage, FocusStop, Page};
use crate::commands::tui::pane::Pane;
use crate::commands::tui::present::compact_money;
use crate::commands::tui::session::Session;
use crate::commands::tui::tabulate;
use crate::commands::tui::theme::Repainted;

pub fn plugin(app: &mut App) {
    app.init_resource::<PersonCursor>()
        .init_resource::<Estimates>()
        .init_resource::<HeldClaims>()
        .add_systems(
            Update,
            (estimate, refresh_people, follow_cursor)
                .chain()
                .before(Repainted)
                .before(WidgetSystems::Layout),
        );
}

const TITLE: &str = "People";
/// The cells between columns, past the one the table leaves: the pane is
/// wide enough to space them out.
const GAP: u16 = 1;
const HEADER: [&str; 6] = ["Person", "Record", "Income", "62", "FRA", "70"];
pub(super) const NOBODY: &str = "no one in the household";

/// The person the page's commands act on, by place in the household.
#[derive(Resource, Default, PartialEq, Eq, Debug)]
pub struct PersonCursor(usize);

impl PersonCursor {
    /// The cursor's place, held inside a household that may have shrunk
    /// under it.
    pub(super) fn index(&self, plan: &Plan) -> Option<usize> {
        let people = plan.household.people.len();
        (people > 0).then(|| self.0.min(people - 1))
    }

    pub(super) fn person<'a>(&self, plan: &'a Plan) -> Option<&'a Person> {
        plan.household.people.get(self.index(plan)?)
    }
}

/// The people whose claims the search leaves as the plan states them.
#[derive(Resource, Default, Debug)]
pub struct HeldClaims(pub BTreeSet<String>);

/// Each person's estimates, by place in the household, from the last
/// valid draft the page was shown over.
#[derive(Resource, Default)]
struct Estimates(Vec<[Option<Dollars>; 3]>);

/// The table, which holds the keyboard for the page.
#[derive(Component)]
pub(super) struct PeopleTable;

/// A person's row, by place in the household.
#[derive(Component)]
struct PersonRow(usize);

/// The person's `social-security` income, where they have one.
pub(super) fn benefit<'a>(plan: &'a Plan, person: &str) -> Option<&'a Income> {
    plan.income
        .iter()
        .find(|income| income.is_benefit_of(person))
}

/// The pane, sharing the row with the claim options.
pub fn spawn_pane(commands: &mut Commands, row: Entity) {
    let pane = Pane::new(TITLE).sharing(1.0).spawn(commands, row);
    commands
        .spawn((
            table_bundle(),
            PeopleTable,
            FocusStop,
            EnterRuns(guide::PERSON_ACTIONS),
            Hints(&[("↑↓", "person"), ("⏎", "actions")]),
            layout::Rests,
            filling(),
            placed(),
            ChildOf(pane),
        ))
        .observe(handle_enter);
}

/// Re-estimates once the draft has moved to a plan not yet estimated and
/// the page is on show: three projections a person are too many for every
/// applied item elsewhere. A draft with issues keeps the last estimates.
fn estimate(
    draft: Res<Draft>,
    session: Res<Session>,
    active: Res<ActivePage>,
    mut last_plan: Local<Option<Plan>>,
    mut estimates: ResMut<Estimates>,
) {
    let is_moved = draft.is_changed() || active.is_changed();
    if !is_moved || active.0 != Page::SsaBenefits || !draft.issues().is_empty() {
        return;
    }
    if last_plan.as_ref() == Some(&draft.plan) {
        return;
    }
    *last_plan = Some(draft.plan.clone());
    let people = &draft.plan.household.people;
    estimates.0 = people
        .iter()
        .map(|person| benefit_estimates(&draft.plan, &session.tables, &person.id))
        .collect();
}

/// Respawns a row per person, the cursor kept on the person it was on.
fn refresh_people(
    state: (Res<Draft>, Res<Estimates>, Res<HeldClaims>),
    cursor: Res<PersonCursor>,
    tables: Query<Entity, With<PeopleTable>>,
    mut commands: Commands,
) {
    let (draft, estimates, held) = state;
    if !draft.is_changed() && !estimates.is_changed() && !held.is_changed() {
        return;
    }
    let header: Vec<String> = HEADER.map(str::to_owned).to_vec();
    let rows = people_rows(&draft.plan, &estimates, &held);
    let at = cursor.index(&draft.plan);
    for table in &tables {
        commands.entity(table).despawn_related::<Children>();
        let spawned = tabulate::fill(&mut commands, table, (&header, &rows), GAP);
        for (index, &row) in spawned.iter().enumerate() {
            commands.entity(row).insert(PersonRow(index));
        }
        let on = at.and_then(|at| spawned.get(at)).copied();
        commands.entity(table).insert(ActiveDescendant(on));
    }
}

fn people_rows(plan: &Plan, estimates: &Estimates, held: &HeldClaims) -> Vec<Vec<String>> {
    let people = plan.household.people.iter().enumerate();
    people
        .map(|(at, person)| {
            let income = if held.0.contains(&person.id) {
                "held"
            } else {
                income(plan, person)
            };
            let mut row = vec![
                person.display_name().to_owned(),
                record(person),
                income.to_owned(),
            ];
            let figures = estimates.0.get(at).copied().unwrap_or_default();
            row.extend(figures.map(|figure| figure.map_or_else(|| "-".to_owned(), compact_money)));
            row
        })
        .collect()
}

/// The person the table's cursor rests on is the one the commands act on.
fn follow_cursor(
    tables: Query<&ActiveDescendant, (With<PeopleTable>, Changed<ActiveDescendant>)>,
    rows: Query<&PersonRow>,
    mut cursor: ResMut<PersonCursor>,
) {
    for on in &tables {
        if let Some(&PersonRow(index)) = on.0.and_then(|row| rows.get(row).ok()) {
            cursor.set_if_neq(PersonCursor(index));
        }
    }
}

fn record(person: &Person) -> String {
    match person.earnings.len() {
        0 => "none".to_owned(),
        years => format!("{years}y"),
    }
}

/// The person's `social-security` income: computed, typed, or none.
fn income(plan: &Plan, person: &Person) -> &'static str {
    match benefit(plan, &person.id).map(|income| income.amount) {
        None => "none",
        Some(None) => "computed",
        Some(Some(_)) => "typed",
    }
}
