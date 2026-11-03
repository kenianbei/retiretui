//! A tool's options pane: every option its search found under the plan as
//! it stands. The plan's own row heads the body, dimmed, and the cursor
//! never rests on it: landing there, by key or by pointer, sends it on to
//! the first option.

use std::marker::PhantomData;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::{
    Changed, Commands, Component, Entity, Has, IntoScheduleConfigs, Local, Query, Res, ResMut, With,
};
use bevy_ui::ComputedNode;
use plurimus::ui::UiStyle;
use plurimus::widgets::ActiveDescendant;

use super::{EnterRuns, Found, ResultPane, Tool, handle_enter};
use crate::edit::Draft;
use crate::grid::{self, Grid};
use crate::hints::Hints;
use crate::layout::{self, Rests};
use crate::session::Basis;
use crate::tabulate;
use crate::theme::{Repainted, Theme};

/// Keeps `R`'s options pane saying what its search found.
pub fn plugin<R: Found>(app: &mut App) {
    app.add_systems(
        Update,
        (refresh_options::<R>, follow_cursor::<R>)
            .chain()
            .before(Repainted)
            .after(super::poll_search::<R>),
    );
}

pub use retiretui_client::searches::{CURRENT_PLAN, FIGURES};

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
    let table = grid::spawn(commands, framed);
    commands
        .entity(table)
        .insert((
            OptionsTable::<R>(PhantomData),
            EnterRuns(enter),
            hints,
            Rests,
        ))
        .observe(handle_enter);
    table
}

/// Respawns the table's rows from what the search found, or a line
/// saying why there are none, whenever the search, the basis, the plan or
/// the table's width moves.
fn refresh_options<R: Found>(
    state: (Res<Tool<R>>, Res<Basis>, Res<Theme>, Res<Draft>),
    tables: Query<(Entity, &Grid), With<OptionsTable<R>>>,
    bodies: Query<&ComputedNode>,
    mut drawn: Local<u16>,
    mut commands: Commands,
) {
    let (tool, basis, theme, draft) = state;
    let is_moved =
        tool.is_changed() || basis.is_changed() || theme.is_changed() || draft.is_changed();
    // A search under way leaves the last options in view; its title says so.
    if tool.found().is_none() && tool.is_running() {
        return;
    }
    for (table, grid) in &tables {
        let Ok(body) = bodies.get(grid.body) else {
            continue;
        };
        let Some(found) = tool.found() else {
            if is_moved {
                grid::say_instead(&mut commands, table, tool.said());
            }
            continue;
        };
        let given = layout::node_cols(body);
        if given == 0 || (!is_moved && given == *drawn) {
            *drawn = given;
            continue;
        }
        *drawn = given;
        let laid = found.laid(&draft.plan, basis.nominal);
        let chosen = tool.highlighted().unwrap_or(R::LEADING);
        fill(&mut commands, (table, grid), (laid, given), chosen, &theme);
    }
}

/// The table's rows: the column names, the plan's own row, dimmed, and an
/// option per result, cut to the columns that fit in `given` cells, the
/// cursor on option `chosen`.
fn fill(
    commands: &mut Commands,
    (table, grid): (Entity, &Grid),
    (laid, given): (Laid, u16),
    chosen: usize,
    theme: &Theme,
) {
    let Laid {
        header,
        current,
        mut options,
    } = laid;
    options.insert(0, current);
    let spawned = tabulate::fill(commands, (table, grid), (&header, &options), (0, given));
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
/// opens it, and goes on to the first option where it does not.
pub fn follow_cursor<R: Found>(
    mut tables: Query<
        (&mut ActiveDescendant, &Grid),
        (With<OptionsTable<R>>, Changed<ActiveDescendant>),
    >,
    bodies: Query<&Children>,
    rows: Query<(Has<CurrentRow>, Option<&OptionRow>)>,
    mut tool: ResMut<Tool<R>>,
) {
    for (mut on, grid) in &mut tables {
        let Some(row) = on.0 else {
            continue;
        };
        match rows.get(row) {
            Ok((true, _)) if R::IS_PLAN_ROW_CHOSEN => {
                tool.bypass_change_detection().highlighted = 0;
            }
            Ok((true, _)) => {
                let best = bodies.get(grid.body).ok().and_then(|children| {
                    children
                        .iter()
                        .find(|&&child| rows.get(child).is_ok_and(|(_, option)| option.is_some()))
                        .copied()
                });
                on.0 = best;
            }
            Ok((false, Some(&OptionRow(index)))) => {
                tool.bypass_change_detection().highlighted = index + 1;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::hierarchy::Children;
    use bevy_math::Vec2;

    use super::*;
    use crate::nav::Page;
    use crate::support::{SIZE, click_entity, headless_app_at, said_by, show};
    use crate::tools::ladders::Swept;

    fn options_grid(app: &mut App) -> (Entity, Entity) {
        let mut tables = app
            .world_mut()
            .query_filtered::<(Entity, &Grid), With<OptionsTable<Swept>>>();
        let (root, grid) = tables.single(app.world()).unwrap();
        (root, grid.body)
    }

    #[test]
    fn the_options_are_said_as_rows_and_a_click_on_one_moves_the_cursor() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../retiretui_engine/tests/fixtures/full.toml"
        );
        let mut app = headless_app_at(fixture.into(), SIZE);
        app.update();
        show(&mut app, Page::RothConversions);
        crate::tools::settle_all(&mut app);
        let (root, body) = options_grid(&mut app);
        let rows: Vec<Entity> = app.world().get::<Children>(body).unwrap().to_vec();
        let texts: Vec<String> = rows.iter().map(|&row| said_by(app.world(), row)).collect();
        assert!(
            rows.len() > 1,
            "the plan's row and the ladders found: {texts:?}"
        );
        assert!(texts[0].contains("Current"), "{texts:?}");
        assert!(!texts[1].trim().is_empty(), "{texts:?}");
        assert!(app.world().get::<OptionRow>(rows[1]).is_some());
        click_entity(&mut app, rows[1], Vec2::ZERO);
        app.update();
        let cursor = app.world().get::<ActiveDescendant>(root).unwrap();
        assert_eq!(cursor.0, Some(rows[1]));
    }
}
