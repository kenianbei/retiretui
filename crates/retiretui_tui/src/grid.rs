//! A table of text laid out as nodes, drawn by bui in a terminal and by
//! bevy in a window: the header held over the rows as they scroll, each
//! row one line of cells cut to columns ratatui's layout solves, the
//! cursor row barred, a bar beside the rows, and the keys, the wheel, a
//! click and the pointer answered as plurimus's table answers them.

pub(crate) mod bar;
mod click;
mod keys;

pub use click::{HeaderClick, RowClick};

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, Has, IntoScheduleConfigs, Query, Res};
use bevy_input_focus::InputFocus;
use bevy_ui::{
    BackgroundColor, ComputedNode, Display, FlexDirection, Node, Overflow, ScrollPosition, Val,
};
use plurimus::bui::bevy_ui_widgets::ScrollArea;
use plurimus::core::ratatui_core::layout::{Alignment, Constraint, Layout, Rect};
use plurimus::core::ratatui_core::style::{Color, Modifier, Style};
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::ui::{Hovered, UiStyle};
use plurimus::widgets::ActiveDescendant;

use crate::edit::table_keys;
use crate::layout::{self, CURSOR_COLS, GUTTER, Rests, fixed, growing};
use crate::tabulate;
use crate::theme::{self, Repainted, Theme, say, text};

pub fn plugin(app: &mut App) {
    app.add_observer(keys::handle_key);
    app.add_systems(
        Update,
        (lay_rows, reveal_cursor, bar::draw_bars)
            .chain()
            .in_set(Repainted),
    );
}

/// The cells between one column and the next.
const GAP: u16 = 1;

/// How a header is said, unless its grid says otherwise.
pub const HEADING: Style = Style::new().add_modifier(Modifier::BOLD);

/// A table of text: its header row, where it has one, its scrolled body
/// and the bar beside them.
#[derive(Component, Debug)]
pub struct Grid {
    /// The row over the body, shown while it has cells to say.
    pub header: Entity,
    /// The node the rows are children of, as wide as their lines.
    pub body: Entity,
    bar: Entity,
}

/// The grid's columns, solved against the body's width each time it
/// changes.
#[derive(Component, Debug)]
struct GridColumns(Vec<Constraint>);

/// A row's cells, each a line of its own alignment.
#[derive(Component, Debug)]
struct Cells(Vec<Line<'static>>);

/// The rows the body showed when the cursor was last revealed, so a
/// wheel that scrolls the cursor out of view is not undone.
#[derive(Component, Debug, Default)]
struct Revealed(u16);

/// How a row was last drawn, so a frame that moves nothing says nothing.
#[derive(Component, Debug, Default, PartialEq, Clone, Copy)]
struct Drawn {
    width: u16,
    gutter: &'static str,
    is_lit: bool,
    ground: Option<Color>,
}

/// Spawns a grid under `parent`: a header row that says nothing until it
/// is filled, and under it the body the rows scroll in beside the bar's
/// column. The root carries the cursor, takes the keys and answers a
/// click on a row.
pub fn spawn(commands: &mut Commands, parent: Entity) -> Entity {
    let root = Node {
        flex_direction: FlexDirection::Column,
        min_width: Val::ZERO,
        ..growing()
    };
    let root = commands
        .spawn((root, ChildOf(parent)))
        .observe(click::point_on_click)
        .id();
    let header = Node {
        display: Display::None,
        ..fixed(1.0)
    };
    let header = commands
        .spawn((
            text(),
            header,
            Cells(Vec::new()),
            UiStyle(HEADING),
            Drawn::default(),
            Hovered::default(),
            ChildOf(root),
        ))
        .id();
    let scrolled = Node {
        flex_direction: FlexDirection::Row,
        min_height: Val::ZERO,
        ..growing()
    };
    let scrolled = commands.spawn((scrolled, ChildOf(root))).id();
    let body = Node {
        flex_direction: FlexDirection::Column,
        overflow: Overflow::scroll_y(),
        min_width: Val::ZERO,
        min_height: Val::ZERO,
        ..growing()
    };
    let body = commands.spawn((body, ScrollArea, ChildOf(scrolled))).id();
    let bar = commands.spawn((bar::column(), ChildOf(scrolled))).id();
    commands.entity(root).insert((
        Grid { header, body, bar },
        GridColumns(Vec::new()),
        bar::Bar::default(),
        Revealed::default(),
        table_keys(),
        ActiveDescendant(None),
    ));
    root
}

/// Replaces `root`'s rows with `header`, where it has one, and `rows`
/// under `columns`, the cells at `text_columns` on the left and the rest
/// right, the cursor on the first row, answering each row's entity.
pub fn refill(
    commands: &mut Commands,
    (root, grid): (Entity, &Grid),
    columns: Vec<Constraint>,
    (header, rows): (&[String], &[Vec<String>]),
    text_columns: &[usize],
) -> Vec<Entity> {
    let rows = rows.iter().map(|row| tabulate::cells(row, text_columns));
    let header = tabulate::cells(header, text_columns);
    refill_lines(commands, (root, grid), columns, header, rows)
}

/// Replaces `root`'s rows with `header`, where it has one, and `rows`
/// under `columns`, each cell a line of its own alignment, the cursor on
/// the first row, answering each row's entity.
pub fn refill_lines(
    commands: &mut Commands,
    (root, grid): (Entity, &Grid),
    columns: Vec<Constraint>,
    header: Vec<Line<'static>>,
    rows: impl IntoIterator<Item = Vec<Line<'static>>>,
) -> Vec<Entity> {
    commands.entity(grid.body).despawn_related::<Children>();
    commands
        .entity(grid.header)
        .insert((Cells(header), UiStyle(HEADING)));
    let spawned: Vec<Entity> = rows
        .into_iter()
        .map(|cells| {
            commands
                .spawn((
                    text(),
                    fixed(1.0),
                    Cells(cells),
                    Drawn::default(),
                    Hovered::default(),
                    ChildOf(grid.body),
                ))
                .id()
        })
        .collect();
    commands.entity(root).insert((
        GridColumns(columns),
        ActiveDescendant(spawned.first().copied()),
    ));
    spawned
}

/// The cells a grid's columns are solved over in a body `body` wide: what
/// its width leaves past the cursor.
#[must_use]
pub fn across(body: &ComputedNode) -> u16 {
    layout::node_cols(body).saturating_sub(CURSOR_COLS)
}

/// The cells `columns` take laid side by side with the gap between each:
/// their fixed lengths, and nothing for a column that fills.
#[must_use]
pub fn span(columns: &[Constraint]) -> u16 {
    let held = columns.iter().map(|column| match column {
        Constraint::Length(cells) => *cells,
        _ => 0,
    });
    let between = u16::try_from(columns.len().saturating_sub(1)).unwrap_or(u16::MAX);
    held.fold(between.saturating_mul(GAP), u16::saturating_add)
}

/// The column rects across `width` cells past the gutter, solved as
/// ratatui solves a table's: the gutter split off first, then the
/// constraints with one cell between.
fn solved(columns: &[Constraint], width: u16) -> Vec<Rect> {
    let [_, across] = Layout::horizontal([Constraint::Length(CURSOR_COLS), Constraint::Fill(0)])
        .areas(Rect::new(0, 0, width, 1));
    Layout::horizontal(columns.iter().copied())
        .spacing(GAP)
        .split(across)
        .to_vec()
}

/// One line for a row: the gutter, then each cell in its column, padded
/// to its alignment and cut to its width, as ratatui draws a table's row;
/// runs of one style are one span.
fn line_of(gutter: &str, cells: &[Line<'static>], columns: &[Rect]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut push = |text: String, style: Style| match spans.last_mut() {
        Some(last) if last.style == style => last.content.to_mut().push_str(&text),
        _ => spans.push(Span::styled(text, style)),
    };
    push(gutter.to_owned(), Style::new());
    let mut at = CURSOR_COLS;
    for (cell, column) in cells.iter().zip(columns) {
        if column.width == 0 {
            continue;
        }
        if column.x > at {
            push(" ".repeat(usize::from(column.x - at)), Style::new());
        }
        let width = usize::from(column.width);
        let pad = width.saturating_sub(cell.width());
        let left = match cell.alignment {
            Some(Alignment::Right) => pad,
            Some(Alignment::Center) => pad / 2,
            _ => 0,
        };
        push(" ".repeat(left), Style::new());
        let (shown, taken) = clipped(cell, column.width.saturating_sub(left as u16));
        for span in shown {
            push(span.content.into_owned(), span.style);
        }
        push(" ".repeat(width - left - taken), Style::new());
        at = column.x.saturating_add(column.width);
    }
    Line::from(spans)
}

/// The leading spans of `line` that fit in `width` cells, each in its
/// own style, and the cells they take: fewer where a wide glyph does not
/// fit.
fn clipped(line: &Line<'static>, width: u16) -> (Vec<Span<'static>>, usize) {
    let mut left = width;
    let mut spans = Vec::new();
    for span in &line.spans {
        if left == 0 {
            break;
        }
        let taken: String = layout::fitting(span.content.chars(), left).collect();
        left -= layout::cells_of(&taken);
        spans.push(Span::styled(taken, span.style));
    }
    (spans, usize::from(width - left))
}

type RowItem<'a> = (
    Ref<'a, Cells>,
    Option<&'a UiStyle>,
    &'a Hovered,
    &'a mut Drawn,
    &'a mut Node,
);

/// Draws every row whose line, place or state moved. A grid laid out
/// nowhere is left as it was drawn.
fn lay_rows(
    (theme, focus): (Res<Theme>, Res<InputFocus>),
    grids: Query<(Entity, &Grid, &GridColumns, &ActiveDescendant, Has<Rests>)>,
    bodies: Query<(&ComputedNode, Option<&Children>)>,
    mut rows: Query<RowItem>,
    mut commands: Commands,
) {
    for (root, grid, columns, cursor, rests) in &grids {
        let Ok((body, children)) = bodies.get(grid.body) else {
            continue;
        };
        let width = layout::node_cols(body);
        if width == 0 {
            continue;
        }
        let is_held = focus.get() == Some(root);
        let mut laying = Laying {
            width,
            bar: layout::cursor_bar(is_held || !rests),
            is_held,
            cursor: cursor.0,
            columns: &columns.0,
            solved: None,
        };
        let header = std::iter::once((grid.header, 0));
        let body_rows = children
            .into_iter()
            .flat_map(|children| children.iter().copied())
            .enumerate()
            .map(|(at, row)| (row, at));
        for (row, at) in header.chain(body_rows) {
            let Ok((cells, style, hovered, mut drawn, mut node)) = rows.get_mut(row) else {
                continue;
            };
            if row == grid.header {
                layout::set_display(&mut node, !cells.0.is_empty());
            }
            let is_said = !theme.is_changed() && !cells.is_changed();
            let item = (
                cells.0.as_slice(),
                style.map(|style| style.0),
                hovered.0 || at % 2 == 1,
            );
            laying.lay(&mut commands, &theme, (row, &mut drawn, is_said), item);
        }
    }
}

/// One grid being laid: its body's width, the bar its cursor row shows,
/// whether it holds the keyboard, its cursor and its columns, solved on
/// the first row that needs them.
struct Laying<'a> {
    width: u16,
    bar: &'static str,
    is_held: bool,
    cursor: Option<Entity>,
    columns: &'a [Constraint],
    solved: Option<Vec<Rect>>,
}

impl Laying<'_> {
    /// Draws `row` where what it would be drawn as differs from `drawn`,
    /// or where it or the theme moved: the cursor row barred and, while
    /// the grid holds the keyboard, in the focused style; a row under the
    /// pointer or every second one banded in the stripe, under whatever
    /// style the row carries of its own. A row only regrounded keeps its
    /// text.
    fn lay(
        &mut self,
        commands: &mut Commands,
        theme: &Theme,
        (row, drawn, is_said): (Entity, &mut Drawn, bool),
        (cells, own, is_banded): (&[Line<'static>], Option<Style>, bool),
    ) {
        let is_cursor = self.cursor == Some(row);
        let now = Drawn {
            width: self.width,
            gutter: if is_cursor { self.bar } else { GUTTER },
            is_lit: is_cursor && self.is_held,
            ground: own
                .and_then(|own| own.bg)
                .or_else(|| theme.stripe.filter(|_| is_banded)),
        };
        if is_said && *drawn == now {
            return;
        }
        let is_regrounded = is_said
            && Drawn {
                ground: now.ground,
                ..*drawn
            } == now;
        *drawn = now;
        commands
            .entity(row)
            .insert(BackgroundColor(theme::paper(now.ground)));
        if is_regrounded {
            return;
        }
        let columns = self
            .solved
            .get_or_insert_with(|| solved(self.columns, self.width));
        let mut line = line_of(now.gutter, cells, columns);
        if now.is_lit {
            line = line.patch_style(theme.focused());
        }
        if let Some(own) = own {
            line = line.patch_style(own);
        }
        say(commands, row, line, theme);
    }
}

/// Scrolls the body by the least that shows the cursor row, whoever set
/// it, as the cursor, the rows or the rows the body holds move - not as
/// the wheel moves the body, which a tick is free to scroll the cursor
/// out of; rows are a rem tall.
fn reveal_cursor(
    mut grids: Query<(&Grid, Ref<ActiveDescendant>, &mut Revealed)>,
    bodies: Query<(&ComputedNode, Ref<Children>)>,
    mut scrolls: Query<&mut ScrollPosition>,
) {
    for (grid, cursor, mut revealed) in &mut grids {
        let Some(row) = cursor.0 else {
            continue;
        };
        let Ok((body, children)) = bodies.get(grid.body) else {
            continue;
        };
        let rows_shown = layout::node_rows(body);
        if !cursor.is_changed() && !children.is_changed() && revealed.0 == rows_shown {
            continue;
        }
        revealed.0 = rows_shown;
        let Some(at) = children.iter().position(|&child| child == row) else {
            continue;
        };
        let Ok(mut scroll) = scrolls.get_mut(grid.body) else {
            continue;
        };
        let rem = body.rem_size.0;
        let shown = f32::from(rows_shown);
        let top = at as f32;
        let offset = scroll.0.y / rem;
        if top < offset {
            scroll.0.y = top * rem;
        } else if top + 1.0 > offset + shown && shown > 0.0 {
            scroll.0.y = (top + 1.0 - shown) * rem;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_s_cells_are_cut_to_their_columns_and_padded_to_their_side() {
        let columns = solved(&[Constraint::Length(4), Constraint::Fill(1)], 14);
        assert_eq!(columns, [Rect::new(2, 0, 4, 1), Rect::new(7, 0, 7, 1)]);
        let cells = vec![
            Line::from("name that is long"),
            Line::from("$1,234").right_aligned(),
        ];
        let line = line_of("▌ ", &cells, &columns);
        assert_eq!(line.to_string(), "▌ name  $1,234");
        assert_eq!(line.spans.len(), 1, "one style is one span: {line:?}");
        let marks = vec![Line::from(vec![Span::raw("◆"), Span::raw("!")])];
        let one = solved(&[Constraint::Length(1)], 4);
        assert_eq!(line_of("  ", &marks, &one).to_string(), "  ◆");
        let wide = vec![Line::from("日本")];
        assert_eq!(
            line_of("  ", &wide, &one).to_string(),
            "   ",
            "cut by cells"
        );
    }

    #[test]
    fn columns_span_their_lengths_and_the_gaps_between() {
        let columns = [
            Constraint::Length(4),
            Constraint::Fill(1),
            Constraint::Length(2),
        ];
        assert_eq!(span(&columns), 4 + 2 + 2 * GAP);
        assert_eq!(span(&[]), 0);
    }
}
