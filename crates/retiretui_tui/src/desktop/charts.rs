//! A chart in the window: the terminal's chart says its cells outside the
//! graph a row at a time, so its labels land where ratatui puts them, and
//! the graph is a node the plot is drawn into. The pointer reaches a chart
//! as it does in the terminal, through plurimus's mouse messages: the
//! cursor's moves, so a chart reads out under it, and a click on a plot
//! as the press and release it is, so the chart's own handlers answer.

use bevy_app::{App, PreUpdate, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Local, On, Query, Res, With, Without,
};
use bevy_math::Vec2;
use bevy_ui::{BorderColor, ComputedNode, Node, PositionType, UiRect, Val};
use bevy_window::CursorMoved;
use plurimus::bui::bevy_picking::events::PointerClick;
use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::{Position, Rect};
use plurimus::core::ratatui_core::widgets::Widget;
use plurimus::term::{KeyModifiers, MouseButton, MouseKind, MouseMessage};

use super::CELL;
use super::chrome::HAIRLINE;
use crate::chart::SeriesChart;
use crate::layout::{node_cols, node_rows};
use crate::theme::{Hung, Repainted, Theme, hang, row_line, say_lines};

pub fn plugin(app: &mut App) {
    // The window's plugin sends it; a harness without a window does not.
    app.add_message::<CursorMoved>();
    app.add_systems(PreUpdate, move_the_cursor);
    app.add_systems(
        Update,
        (frame_charts, paint_charts).chain().in_set(Repainted),
    );
}

/// A chart's plot node - over the cells ratatui draws the graph in and
/// the axis beside and under it, which the node's left and bottom borders
/// draw - and the graph's cells as last said, none while the chart has
/// no room for one.
#[derive(Component, Debug)]
pub(super) struct Charted {
    pub(super) plot: Entity,
    pub(super) graph: Option<Rect>,
}

/// Gives every chart not yet dressed its said rows and its plot.
fn frame_charts(
    charts: Query<Entity, (With<SeriesChart>, Without<Charted>)>,
    mut commands: Commands,
) {
    for chart in &charts {
        hang(&mut commands, chart);
        let plot = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    border: UiRect {
                        left: HAIRLINE,
                        bottom: HAIRLINE,
                        ..UiRect::ZERO
                    },
                    ..Node::default()
                },
                ChildOf(chart),
            ))
            .observe(press_the_chart)
            .id();
        commands.entity(chart).insert(Charted { plot, graph: None });
    }
}

/// Says each chart's cells again whenever it, its node or the theme
/// moves: the chart drawn into a buffer of its cells, each row said with
/// the graph and the axis blank, and the plot's node placed over them.
fn paint_charts(
    theme: Res<Theme>,
    mut charts: Query<(Ref<SeriesChart>, Ref<ComputedNode>, &Hung, &mut Charted)>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    for (drawn, computed, hung, mut charted) in &mut charts {
        let is_stale =
            drawn.is_changed() || computed.is_changed() || theme.is_changed() || charted.is_added();
        if !is_stale {
            continue;
        }
        let cells = Rect::new(0, 0, node_cols(&computed), node_rows(&computed));
        let mut buffer = Buffer::empty(cells);
        (&*drawn).render(cells, &mut buffer);
        let blank = drawn.plot(cells).unwrap_or(Rect::ZERO);
        charted.graph = drawn.graph(cells);
        let rows = (cells.top()..cells.bottom())
            .map(|y| row_line(&buffer, y, blank))
            .collect();
        say_lines(&mut commands, hung.0, rows, &theme);
        if let Ok(mut node) = nodes.get_mut(charted.plot) {
            node.left = Val::Em(f32::from(blank.x));
            node.top = Val::Rem(f32::from(blank.y));
            node.width = Val::Em(f32::from(blank.width));
            node.height = Val::Rem(f32::from(blank.height));
        }
        commands
            .entity(charted.plot)
            .insert(BorderColor::all(theme.ink(drawn.axis)));
    }
}

/// The cell under a point of the window.
fn cell_at(position: Vec2) -> Position {
    let cell = (position.max(Vec2::ZERO) / CELL).floor();
    Position::new(cell.x as u16, cell.y as u16)
}

/// bevy's cursor said to plurimus as the move it is, once for each cell it
/// reaches, so what the terminal's widgets answer to the pointer's place -
/// the chart's read-out among them - they answer in the window.
fn move_the_cursor(
    mut moved: MessageReader<CursorMoved>,
    mut last: Local<Option<Position>>,
    mut mouse: MessageWriter<MouseMessage>,
) {
    let Some(moved) = moved.read().last() else {
        return;
    };
    let position = cell_at(moved.position);
    if *last == Some(position) {
        return;
    }
    *last = Some(position);
    mouse.write(MouseMessage::new(
        MouseKind::Moved,
        position,
        KeyModifiers::default(),
    ));
}

/// A click on a plot said to plurimus as the press and release it is, at
/// its cell, so the chart's own handlers answer it - the year cursor,
/// the Ledger - as they answer a terminal's press.
fn press_the_chart(click: On<PointerClick>, mut mouse: MessageWriter<MouseMessage>) {
    let position = cell_at(click.pointer.position);
    for kind in [
        MouseKind::Down(MouseButton::Left),
        MouseKind::Up(MouseButton::Left),
    ] {
        mouse.write(MouseMessage::new(kind, position, KeyModifiers::default()));
    }
}

#[cfg(test)]
mod tests {
    use plurimus::term::CursorCell;
    use plurimus::ui::ComputedWidgetArea;

    use super::*;
    use crate::nav::Page;
    use crate::pane::Framed;
    use crate::session::YearCursor;
    use crate::support::{
        ROOMY, SIZE, active_page, click_entity, headless_app, overview_chart, said_by, show,
    };

    #[test]
    fn the_chart_says_its_labels_around_a_blank_graph_the_plot_covers() {
        let mut app = headless_app(SIZE);
        app.add_plugins(plugin);
        app.update();
        app.update();
        let chart = overview_chart(&mut app);
        let world = app.world();
        let drawn = world.get::<SeriesChart>(chart).unwrap();
        let computed = world.get::<ComputedNode>(chart).unwrap();
        let cells = Rect::new(0, 0, node_cols(computed), node_rows(computed));
        let blank = drawn.plot(cells).unwrap();
        let charted = world.get::<Charted>(chart).unwrap();
        assert_eq!(charted.graph, drawn.graph(cells));
        let said = said_by(world, world.get::<Hung>(chart).unwrap().0);
        let rows: Vec<&str> = said.split('\n').collect();
        assert_eq!(rows.len(), usize::from(cells.height));
        assert!(rows[0].starts_with(&drawn.y_labels[1]), "{:?}", rows[0]);
        let labels = rows.last().unwrap();
        for label in drawn
            .x_labels
            .iter()
            .chain(drawn.marks.iter().map(|mark| &mark.label))
        {
            assert!(labels.contains(label.as_str()), "{label} in {labels:?}");
        }
        for row in &rows[usize::from(blank.y)..usize::from(blank.bottom())] {
            let graph: String = row
                .chars()
                .skip(usize::from(blank.x))
                .take(usize::from(blank.width))
                .collect();
            assert!(graph.trim().is_empty(), "blank: {graph:?}");
        }
        let plot = world.get::<Node>(charted.plot).unwrap();
        assert_eq!(
            (plot.left, plot.top, plot.width, plot.height),
            (
                Val::Em(f32::from(blank.x)),
                Val::Rem(f32::from(blank.y)),
                Val::Em(f32::from(blank.width)),
                Val::Rem(f32::from(blank.height)),
            )
        );
    }

    #[test]
    fn the_cursor_s_move_reaches_the_chart_as_the_terminal_s_does() {
        let mut app = headless_app(SIZE);
        app.add_plugins(plugin);
        app.update();
        let chart = overview_chart(&mut app);
        let area = app.world().get::<ComputedWidgetArea>(chart).unwrap().0;
        let drawn = app.world().get::<SeriesChart>(chart).unwrap().clone();
        let column = drawn.column_of(area, 2038).unwrap();
        let row = area.y + 1;
        let position = Vec2::new(f32::from(column) + 0.5, f32::from(row) + 0.5) * CELL;
        app.world_mut().write_message(CursorMoved {
            window: Entity::PLACEHOLDER,
            position,
            delta: None,
        });
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<CursorCell>().0,
            Some(Position::new(column, row))
        );
        let pane = app.world().get::<ChildOf>(chart).unwrap().parent();
        let note = &app.world().get::<Framed>(pane).unwrap().note;
        assert_eq!(note, "2038");
    }

    #[test]
    fn a_click_on_the_plot_opens_the_ledger_at_its_year() {
        let mut app = headless_app(ROOMY);
        app.add_plugins(plugin);
        show(&mut app, Page::Overview);
        let chart = overview_chart(&mut app);
        app.update();
        let plot = app.world().get::<Charted>(chart).unwrap().plot;
        let area = app.world().get::<ComputedWidgetArea>(chart).unwrap().0;
        let drawn = app.world().get::<SeriesChart>(chart).unwrap().clone();
        let column = drawn.column_of(area, 2040).unwrap();
        let position = Vec2::new(f32::from(column) + 0.5, f32::from(area.y + 1) + 0.5) * CELL;
        click_entity(&mut app, plot, position);
        app.update();
        app.update();
        assert_eq!(active_page(&app), Page::Ledger, "the click turned to it");
        assert_eq!(
            app.world().resource::<YearCursor>(),
            &YearCursor(Some(2040))
        );
    }

    #[test]
    fn a_point_of_the_window_is_the_cell_it_falls_in() {
        assert_eq!(cell_at(Vec2::new(5.5, 3.2) * CELL), Position::new(5, 3));
        assert_eq!(cell_at(Vec2::new(-1.0, -1.0)), Position::new(0, 0));
    }
}
