//! The Roth Conversions page's panes: the constraints beside the ladder
//! options, over the highlighted option's conversions year by year.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Local, Query, Res, With,
};
use bevy_ui::{FlexDirection, Node, Val};
use plurimus::ui::ScrollArea;
use retiretui_engine::optimize::SweptBracket;
use retiretui_engine::plan::Plan;

use super::super::options::{self, say_instead, spawn_table};
use super::{CONVERSION_COLUMNS, CONVERTS_NOTHING, Ladders, OPS, Swept, taxed_in};
use crate::edit::{self, Draft, table_bundle};
use crate::hints::Hints;
use crate::layout::{self, filling, placed};
use crate::nav::FocusStop;
use crate::pane::Pane;
use crate::present::{account_name, compact_dollars};
use crate::session::Basis;
use crate::table::basis_amount;
use crate::tabulate;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        refresh_conversions
            .in_set(options::TablesFilled)
            .after(options::follow_cursor::<Swept>),
    );
}

const OPTIONS_TITLE: &str = "Ladder Options";
const CONVERSIONS_TITLE: &str = "Conversions";
/// Borders, the header, the plan's own row and six brackets: every one a
/// filing status has below the top.
const OPTIONS_ROWS: f32 = 10.0;
/// The form's share of the row against the options': theirs has the more
/// columns to show, and at the narrowest frame needs a fifth more.
const FORM_SHARE: f32 = 1.0;
const OPTIONS_SHARE: f32 = 1.2;
/// The cells between columns, past the one the table leaves.
const GAP: u16 = 1;
const NOT_SEARCHED: &str = "The highlighted option's conversions, year by year.";

/// The table the highlighted option's conversions are rows of.
#[derive(Component)]
pub(super) struct ConversionsTable;

/// The constraints, beside the options over the conversions.
pub fn spawn_panes(commands: &mut Commands, row: Entity) {
    let form = Pane::new(OPS.title)
        .sharing(FORM_SHARE)
        .spawn(commands, row);
    edit::spawn_details(commands, form, OPS);
    let column = Node {
        flex_direction: FlexDirection::Column,
        flex_grow: OPTIONS_SHARE,
        flex_basis: Val::Px(0.0),
        height: Val::Percent(100.0),
        ..Node::default()
    };
    let column = commands.spawn((column, ChildOf(row))).id();
    let framed = Pane::new(OPTIONS_TITLE)
        .tall(OPTIONS_ROWS)
        .spawn(commands, column);
    let hints = Hints(&[("↑↓", "option"), ("⏎", "take")]);
    let table = spawn_table::<Swept>(commands, framed, "take-ladder", hints);
    commands.entity(table).insert(FocusStop);
    let framed = Pane::new(CONVERSIONS_TITLE)
        .sharing(1.0)
        .spawn(commands, column);
    commands.spawn((
        table_bundle(),
        ConversionsTable,
        FocusStop,
        Hints(&[("↑↓", "year")]),
        layout::Rests,
        filling(),
        placed(),
        ChildOf(framed),
    ));
}

/// Respawns the conversions of the option the cursor rests on, whenever
/// it moves or what they were drawn from changes. A search under way
/// leaves the last in view.
fn refresh_conversions(
    state: (Res<Ladders>, Res<Basis>, Res<Draft>),
    mut drawn: Local<Option<usize>>,
    mut tables: Query<(Entity, &mut ScrollArea), With<ConversionsTable>>,
    mut commands: Commands,
) {
    let (ladders, basis, draft) = state;
    let is_moved = ladders.is_changed() || basis.is_changed() || draft.is_changed();
    if (!is_moved && *drawn == Some(ladders.highlighted))
        || (ladders.found().is_none() && ladders.is_running())
    {
        return;
    }
    *drawn = Some(ladders.highlighted);
    for (table, mut scroll) in &mut tables {
        let bracket = ladders.highlighted_bracket();
        let said = match bracket {
            None => NOT_SEARCHED,
            Some(bracket) if bracket.steps.is_empty() => CONVERTS_NOTHING,
            Some(bracket) => {
                let rows = conversion_rows(bracket, &draft.plan, basis.nominal);
                let header = CONVERSION_COLUMNS.map(str::to_owned);
                commands
                    .entity(table)
                    .insert(tabulate::columns((&header, &rows), GAP));
                tabulate::refill(&mut commands, (table, &mut scroll), (&header, &rows), &[0]);
                continue;
            }
        };
        say_instead(&mut commands, table, said.to_owned());
    }
}

/// A row per conversion: its year, the account it draws from, what it
/// converts and the ordinary income taxed that year.
fn conversion_rows(bracket: &SweptBracket, plan: &Plan, is_nominal: bool) -> Vec<Vec<String>> {
    bracket
        .steps
        .iter()
        .map(|step| {
            let (taxable, deflator) = taxed_in(bracket, step.year);
            let shown = |amount| compact_dollars(basis_amount(amount, deflator, is_nominal));
            vec![
                step.year.to_string(),
                account_name(plan, &step.source).to_owned(),
                shown(step.amount),
                shown(taxable),
            ]
        })
        .collect()
}
