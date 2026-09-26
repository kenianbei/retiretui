//! A tool's options pane: every option its search found, best first,
//! under the plan as it stands. The plan's own row heads the body, dimmed,
//! and the cursor never rests on it: landing there, by key or by pointer,
//! sends it on to the best option.

use std::marker::PhantomData;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, Has, IntoScheduleConfigs, Query, Res, ResMut, SystemSet,
    With,
};
use plurimus::core::ratatui_core::layout::Constraint;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{ComputedWidgetArea, ScrollArea, UiStyle};
use plurimus::widgets::{ActiveDescendant, TableColumns, WidgetSystems, table_row};
use retiretui_engine::project::Summary;

use super::{EnterRuns, Found, ResultPane, Tool, handle_enter};
use crate::commands::tui::edit::{Draft, table_bundle};
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{self, filling, placed};
use crate::commands::tui::present::compact_dollars;
use crate::commands::tui::session::Basis;
use crate::commands::tui::tabulate::{self, Said};
use crate::commands::tui::theme::{Repainted, Theme};

/// Keeps `R`'s options pane saying what its search found.
pub fn plugin<R: Found>(app: &mut App) {
    app.add_systems(
        Update,
        (refresh_options::<R>, follow_cursor::<R>)
            .chain()
            .in_set(TablesFilled)
            .after(super::poll_search::<R>),
    );
}

/// Where the tools' tables are filled, or told what to say in place of
/// rows.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TablesFilled;

/// Wraps what each table says in place of rows, once filled.
pub(super) fn plugin_said(app: &mut App) {
    app.add_systems(
        Update,
        wrap_said
            .after(TablesFilled)
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
            say_instead(&mut commands, table, tool.said());
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

/// Dimmed lines in place of `table`'s rows, saying why there are none,
/// wrapped to the table's width: a table whose body has once gone empty
/// draws no rows it is given after.
pub fn say_instead(commands: &mut Commands, table: Entity, said: String) {
    commands.entity(table).insert((
        TableColumns(vec![Constraint::Fill(1)]),
        ActiveDescendant(None),
        Said {
            text: said,
            drawn: None,
        },
    ));
}

/// Rewraps each table's [`Said`] whenever it, the theme or the table's
/// width moves.
fn wrap_said(
    theme: Res<Theme>,
    mut tables: Query<(Entity, &mut Said, &mut ScrollArea, &ComputedWidgetArea)>,
    mut commands: Commands,
) {
    for (table, mut said, mut scroll, area) in &mut tables {
        // No row is the cursor's, so the lines start at the table's edge.
        let width = scroll.content_width(area.0.width);
        if said.drawn == Some(width) && !said.is_changed() && !theme.is_changed() {
            continue;
        }
        said.bypass_change_detection().drawn = Some(width);
        commands.entity(table).despawn_related::<Children>();
        let lines = layout::wrapped(&said.text, width, "");
        scroll.content_size.height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
        for line in lines {
            commands.spawn((
                table_row([Line::from(line)]),
                CurrentRow,
                UiStyle(theme.dimmed()),
                ChildOf(table),
            ));
        }
    }
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
/// cursor that lands on the plan's own row rests there where the tool
/// opens it, and goes on to the best option where it does not.
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
            Ok((true, _)) if R::IS_PLAN_ROW_CHOSEN => {
                tool.bypass_change_detection().highlighted = 0;
            }
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
