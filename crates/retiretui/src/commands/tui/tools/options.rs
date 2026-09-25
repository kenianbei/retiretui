//! A tool's options pane: every option its search found, best first,
//! under the plan as it stands. The plan's own row heads the body, dimmed,
//! and the cursor never rests on it: landing there, by key or by pointer,
//! sends it on to the best option.

use std::marker::PhantomData;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, Has, IntoScheduleConfigs, Query, Res, ResMut, With,
};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{ScrollArea, UiStyle};
use plurimus::widgets::{ActiveDescendant, TableColumns, WidgetSystems, table_row};
use retiretui_engine::project::Summary;

use super::{EnterRuns, Found, ResultPane, Tool, handle_enter};
use crate::commands::tui::edit::{Draft, table_bundle};
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{self, filling, placed};
use crate::commands::tui::present::compact_dollars;
use crate::commands::tui::session::Basis;
use crate::commands::tui::tabulate;
use crate::commands::tui::theme::{Repainted, Theme};

/// Keeps `R`'s options pane saying what its search found.
pub fn plugin<R: Found>(app: &mut App) {
    app.add_systems(
        Update,
        (refresh_options::<R>, follow_cursor::<R>)
            .chain()
            .after(super::poll_search::<R>)
            .before(Repainted)
            .before(WidgetSystems::Layout),
    );
}

/// What the plan's own row says first.
pub const CURRENT_PLAN: &str = "Current";
/// The figures an option is chosen by, in the order the options are ranked
/// by and then what they cost; the rest of a summary is the Compare tab's.
pub const FIGURES: [&str; 4] = ["unfunded", "final net", "taxes", "medicare"];

/// A summary's [`FIGURES`].
pub fn figures(summary: &Summary) -> [String; 4] {
    [
        summary.lifetime_unfunded,
        summary.final_net_worth,
        summary.lifetime_taxes,
        summary.lifetime_medicare,
    ]
    .map(compact_dollars)
}

/// What the rows say: the column names, the plan's own row, and an option
/// per result, best first.
pub struct Laid {
    pub header: Vec<String>,
    pub current: Vec<String>,
    pub options: Vec<Vec<String>>,
}

/// The plan's own row, or the line saying why there is none: no option.
#[derive(Component)]
pub struct CurrentRow;

/// The table `R`'s options are rows of, which the keyboard walks to.
#[derive(Component)]
pub struct OptionsTable<R>(PhantomData<R>);

/// An option's row, by place among the results.
#[derive(Component)]
pub struct OptionRow(usize);

/// The table, inside the pane `framed`, whose title says what the search
/// is up to; ⏎ on it runs the command `enter`. Where it stands in the
/// keyboard's walk is the page's to say.
pub fn spawn_table<R: Found>(
    commands: &mut Commands,
    framed: Entity,
    enter: &'static str,
    hints: Hints,
) -> Entity {
    commands.entity(framed).insert(ResultPane::<R>::default());
    commands
        .spawn((
            table_bundle(),
            OptionsTable::<R>(PhantomData),
            EnterRuns(enter),
            hints,
            layout::Rests,
            filling(),
            placed(),
            ChildOf(framed),
        ))
        .observe(handle_enter)
        .id()
}

/// Respawns the table's rows from what the search found, or a line
/// saying why there are none.
fn refresh_options<R: Found>(
    state: (Res<Tool<R>>, Res<Basis>, Res<Theme>, Res<Draft>),
    mut tables: Query<(Entity, &mut ScrollArea), With<OptionsTable<R>>>,
    mut commands: Commands,
) {
    let (tool, basis, theme, draft) = state;
    let is_moved =
        tool.is_changed() || basis.is_changed() || theme.is_changed() || draft.is_changed();
    // A search under way leaves the last options in view; its title says so.
    if !is_moved || (tool.found().is_none() && tool.is_running()) {
        return;
    }
    for (table, mut scroll) in &mut tables {
        commands.entity(table).despawn_related::<Children>();
        let Some(found) = tool.found() else {
            say_instead(&mut commands, (table, &mut scroll), tool.said(), &theme);
            continue;
        };
        let laid = found.laid(&draft.plan, basis.nominal);
        // The widget measures the rows a frame behind a respawn, and keeps
        // that; the header, the plan's own row and the options are told.
        let rows = laid.options.len().saturating_add(2);
        scroll.content_size.height = u16::try_from(rows).unwrap_or(u16::MAX);
        let chosen = tool.highlighted().unwrap_or(0);
        fill(&mut commands, table, laid, chosen, &theme);
    }
}

/// A dimmed line in place of `table`'s rows, saying why there are none: a
/// table whose body has once gone empty draws no rows it is given after.
pub fn say_instead(
    commands: &mut Commands,
    (table, scroll): (Entity, &mut ScrollArea),
    said: String,
    theme: &Theme,
) {
    commands.entity(table).insert((
        TableColumns(vec![Constraint::Fill(1)]),
        ActiveDescendant(None),
    ));
    commands.spawn((
        table_row([Line::from(said)]),
        CurrentRow,
        UiStyle(theme.dimmed()),
        ChildOf(table),
    ));
    scroll.content_size.height = 1;
}

/// The table's rows: the column names, the plan's own row, dimmed, and an
/// option per result, the cursor on option `chosen`.
fn fill(commands: &mut Commands, table: Entity, laid: Laid, chosen: usize, theme: &Theme) {
    let Laid {
        header,
        current,
        mut options,
    } = laid;
    options.insert(0, current);
    let spawned = tabulate::fill(commands, table, (&header, &options), 0);
    let Some((&current, options)) = spawned.split_first() else {
        return;
    };
    commands
        .entity(current)
        .insert((CurrentRow, UiStyle(theme.dimmed())));
    for (index, &option) in options.iter().enumerate() {
        commands.entity(option).insert(OptionRow(index));
    }
    commands
        .entity(table)
        .insert(ActiveDescendant(options.get(chosen).copied()));
}

/// The option the table's cursor rests on is the one the tool's commands
/// take; moving it redraws nothing, so the tool is not marked changed. A
/// cursor that lands on the plan's own row goes on to the best option.
pub fn follow_cursor<R: Found>(
    mut tables: Query<
        (&mut ActiveDescendant, &Children),
        (With<OptionsTable<R>>, Changed<ActiveDescendant>),
    >,
    rows: Query<(Has<CurrentRow>, Option<&OptionRow>)>,
    mut tool: ResMut<Tool<R>>,
) {
    for (mut on, children) in &mut tables {
        let Some(row) = on.0 else {
            continue;
        };
        match rows.get(row) {
            Ok((true, _)) => {
                let best = children
                    .iter()
                    .find(|&&child| rows.get(child).is_ok_and(|(_, option)| option.is_some()))
                    .copied();
                on.0 = best;
            }
            Ok((false, Some(&OptionRow(index)))) => {
                tool.bypass_change_detection().highlighted = index + 1;
            }
            _ => {}
        }
    }
}
