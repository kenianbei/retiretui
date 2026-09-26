//! Braille line charts of one figure per year, and the year the pointer
//! is over one.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Component, On, Query, Ref, Res, ResMut};
use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::{Position, Rect};
use plurimus::core::ratatui_core::style::{Color, Style};
use plurimus::core::ratatui_core::symbols::Marker;
use plurimus::core::ratatui_core::widgets::Widget;
use plurimus::core::{UiArea, UiWidget};
use plurimus::term::CursorCell;
use plurimus::ui::{ComputedWidgetArea, Hovered, PointerPress, PressFocusDisabled};
use plurimus::widgets::ratatui_widgets::chart::{Axis, Chart, Dataset, GraphType};
use retiretui_engine::plan::Dollars;
use retiretui_engine::project::{Projection, YearRow};

use crate::commands::table::basis_amount;

mod marks;
mod shades;

pub use marks::Mark;
pub use shades::Shade;

use super::layout::placed;
use super::pane::Framed;
use super::present::compact_dollars;
use super::session::YearCursor;

/// The rows ratatui keeps under the graph: the axis, and its labels.
const AXIS_ROWS: u16 = 2;

/// How far above the highest value the scale reaches.
const HEADROOM: f64 = 1.05;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (draw_charts, read_out));
    app.add_observer(handle_press);
}

/// A chart built from owned series, so the pointer can read them back;
/// the borrowing `Chart` is assembled per draw.
#[derive(Component, Clone, Default)]
#[require(UiWidget, UiArea = placed(), Hovered, PressFocusDisabled)]
pub struct SeriesChart {
    pub series: Vec<Series>,
    pub x_bounds: [f64; 2],
    pub y_bounds: [f64; 2],
    pub x_labels: Vec<String>,
    pub y_labels: Vec<String>,
    pub axis: Style,
    pub marks: Vec<Mark>,
    /// Bands shaded under the lines.
    pub shades: Vec<Shade>,
    /// Whether the lines go unnamed on the chart, something beside it
    /// naming them instead.
    pub is_legend_hidden: bool,
}

#[derive(Clone)]
pub struct Series {
    pub label: String,
    pub color: Color,
    pub points: Vec<(f64, f64)>,
}

impl Series {
    /// `value` of every projected year, on the basis asked for.
    pub fn of(
        label: impl Into<String>,
        color: Color,
        projection: &Projection,
        nominal: bool,
        value: impl Fn(&YearRow) -> Dollars,
    ) -> Self {
        let points = projection
            .years
            .iter()
            .map(|row| {
                let amount = basis_amount(value(row), row.deflator, nominal);
                (f64::from(row.year), amount as f64)
            })
            .collect();
        Self {
            label: label.into(),
            color,
            points,
        }
    }
}

impl SeriesChart {
    /// The series over the years they span, from zero - or just below
    /// their lowest, where they fall under it - to just above their peak.
    pub fn of(series: Vec<Series>, axis: Style) -> Self {
        let (mut first, mut last) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut low, mut peak) = (0.0_f64, 0.0_f64);
        for &(year, amount) in series.iter().flat_map(|series| &series.points) {
            first = first.min(year);
            last = last.max(year);
            low = low.min(amount);
            peak = peak.max(amount);
        }
        let (first, last) = if first.is_finite() {
            (first, last)
        } else {
            (0.0, 1.0)
        };
        let x_bounds = if last > first {
            [first, last]
        } else {
            [first - 0.5, first + 0.5]
        };
        let bottom = low * HEADROOM;
        let top = match (peak > 0.0, bottom < 0.0) {
            (true, _) => peak * HEADROOM,
            (false, true) => 0.0,
            (false, false) => 1.0,
        };
        let bottom_label = if bottom < 0.0 {
            compact_dollars(bottom as Dollars)
        } else {
            "0".to_owned()
        };
        Self {
            x_labels: vec![format!("{}", first as i64), format!("{}", last as i64)],
            y_labels: vec![bottom_label, compact_dollars(top as Dollars)],
            x_bounds,
            y_bounds: [bottom, top],
            series,
            axis,
            marks: Vec::new(),
            shades: Vec::new(),
            is_legend_hidden: false,
        }
    }

    /// The chart with `shades` under its lines, its scale raised to hold
    /// their highs.
    #[must_use]
    pub fn shaded(mut self, shades: Vec<Shade>) -> Self {
        let high = shades
            .iter()
            .flat_map(|shade| &shade.points)
            .fold(0.0_f64, |high, &(_, _, point)| high.max(point));
        if high * HEADROOM > self.y_bounds[1] {
            self.y_bounds[1] = high * HEADROOM;
            self.y_labels[1] = compact_dollars(self.y_bounds[1] as Dollars);
        }
        self.shades = shades;
        self
    }

    /// Where ratatui draws the graph: after the widest y label - or the first
    /// x label less a character, at most a third of the width - and the axis.
    fn graph(&self, area: Rect) -> Option<Rect> {
        let y_labels = self
            .y_labels
            .iter()
            .map(|label| label.chars().count())
            .max();
        let x_label = self.x_labels.first().map(|label| label.chars().count() - 1);
        let inset = (y_labels.unwrap_or(0).max(x_label.unwrap_or(0)) as u16).min(area.width / 3);
        let left = area.x + inset + 1;
        let width = area.right().checked_sub(left)?;
        let height = area.height.saturating_sub(AXIS_ROWS);
        (width >= 2).then_some(Rect::new(left, area.y, width, height))
    }

    /// The year drawn at `cell` of the chart laid out in `area`, whose
    /// columns run evenly from the first year to the last.
    pub fn year_at(&self, area: Rect, cell: Position) -> Option<i16> {
        let graph = self.graph(area)?;
        if !area.contains(cell) || cell.x < graph.x {
            return None;
        }
        Some(self.year_along(graph, cell.x).round() as i16)
    }

    /// The year, between whole ones, drawn at `column` of `graph`.
    fn year_along(&self, graph: Rect, column: u16) -> f64 {
        let [first, last] = self.x_bounds;
        let along = f64::from(column - graph.x) / f64::from(graph.width - 1);
        first + along * (last - first)
    }

    /// The column `year` is drawn in, which [`Self::year_at`] reads back.
    pub fn column_of(&self, area: Rect, year: i16) -> Option<u16> {
        self.column_in(self.graph(area)?, year)
    }

    fn column_in(&self, graph: Rect, year: i16) -> Option<u16> {
        let [first, last] = self.x_bounds;
        let along = (f64::from(year) - first) / (last - first);
        (0.0..=1.0)
            .contains(&along)
            .then(|| graph.x + (along * f64::from(graph.width - 1)).round() as u16)
    }

    /// What every series reads at `cell`'s year.
    pub fn read_at(&self, area: Rect, cell: Position) -> Option<String> {
        let year = self.year_at(area, cell)?;
        let values: Vec<String> = self
            .series
            .iter()
            .map(|series| {
                series
                    .points
                    .iter()
                    .find(|&&(at, _)| (at - f64::from(year)).abs() < 0.5)
                    .map_or_else(
                        || "-".to_owned(),
                        |&(_, amount)| compact_dollars(amount as Dollars),
                    )
            })
            .collect();
        Some(format!("{year} · {}", values.join(" / ")))
    }
}

impl Widget for &SeriesChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        shades::draw_shades(self, area, buf);
        marks::draw_zero(self, area, buf);
        marks::draw_rules(self, area, buf);
        let datasets = self
            .series
            .iter()
            .map(|series| {
                Dataset::default()
                    .name(series.label.as_str())
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::new().fg(series.color))
                    .data(&series.points)
            })
            .collect();
        let chart = Chart::new(datasets)
            .x_axis(
                Axis::default()
                    .style(self.axis)
                    .bounds(self.x_bounds)
                    .labels(self.x_labels.iter().map(String::as_str)),
            )
            .y_axis(
                Axis::default()
                    .style(self.axis)
                    .bounds(self.y_bounds)
                    .labels(self.y_labels.iter().map(String::as_str)),
            );
        if self.is_legend_hidden {
            chart.legend_position(None).render(area, buf);
        } else {
            chart.render(area, buf);
        }
        marks::draw_labels(self, area, buf);
    }
}

pub(crate) fn draw_charts(mut charts: Query<(Ref<SeriesChart>, &mut UiWidget)>) {
    for (chart, mut widget) in &mut charts {
        if chart.is_changed() {
            *widget = UiWidget::new(SeriesChart::clone(&chart));
        }
    }
}

/// The pane of the chart under the pointer notes the year and the values
/// there; every other pane's note is cleared.
fn read_out(
    cursor: Res<CursorCell>,
    charts: Query<(Ref<SeriesChart>, &ComputedWidgetArea, &ChildOf)>,
    mut frames: Query<&mut Framed>,
) {
    if !cursor.is_changed() && !charts.iter().any(|(chart, ..)| chart.is_changed()) {
        return;
    }
    for (chart, area, parent) in &charts {
        let note = cursor
            .0
            .and_then(|cell| chart.read_at(area.0, cell))
            .unwrap_or_default();
        if let Ok(mut framed) = frames.get_mut(parent.parent()) {
            Framed::renote(&mut framed, &note);
        }
    }
}

/// A press on a chart sets the year cursor to the column pressed.
fn handle_press(
    press: On<PointerPress>,
    charts: Query<(&SeriesChart, &ComputedWidgetArea)>,
    mut cursor: ResMut<YearCursor>,
) {
    if let Ok((chart, area)) = charts.get(press.entity)
        && let Some(year) = chart.year_at(area.0, press.position)
    {
        cursor.set_if_neq(YearCursor(Some(year)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::commands::tui::support::{
        SIZE, click, composed_frame, headless_app, hover, overview_chart,
    };

    fn flat(first: i16, last: i16) -> SeriesChart {
        let points = (first..=last).map(|year| (f64::from(year), 1.0)).collect();
        let series = Series {
            label: "flat".to_owned(),
            color: Color::Reset,
            points,
        };
        SeriesChart::of(vec![series], Style::new())
    }

    #[test]
    fn a_column_reads_as_a_year_between_the_ends() {
        let chart = flat(2026, 2050);
        // Labels "0" and "1", and "2026" less a character: three, then the axis.
        let area = Rect::new(1, 2, 38, 10);
        assert_eq!(chart.year_at(area, Position::new(5, 3)), Some(2026));
        assert_eq!(chart.year_at(area, Position::new(38, 3)), Some(2050));
        assert_eq!(chart.year_at(area, Position::new(21, 3)), Some(2038));
        assert_eq!(chart.year_at(area, Position::new(4, 3)), None, "the axis");
        assert_eq!(chart.year_at(area, Position::new(5, 12)), None, "below");
        assert_eq!(chart.year_at(area, Position::new(39, 3)), None, "beside");
        assert_eq!(
            chart.read_at(area, Position::new(5, 3)).as_deref(),
            Some("2026 · 1")
        );
        let narrow = Rect::new(1, 2, 3, 10);
        assert_eq!(
            chart.year_at(narrow, Position::new(3, 3)),
            None,
            "too narrow"
        );
    }

    #[test]
    fn hovering_a_chart_names_the_year_in_its_pane_and_a_press_sets_the_cursor() {
        let mut app = headless_app(SIZE);
        app.update();
        let chart = overview_chart(&mut app);
        let area = app.world().get::<ComputedWidgetArea>(chart).unwrap().0;
        let drawn = app.world().get::<SeriesChart>(chart).unwrap().clone();
        let column = |year| drawn.column_of(area, year).unwrap();
        let row = area.y + 1;
        hover(&mut app, column(2026), row);
        let frame = composed_frame(&app);
        assert!(frame.contains("dollars · 2026 · "), "{frame}");
        hover(&mut app, column(2050), row);
        let frame = composed_frame(&app);
        assert!(frame.contains("dollars · 2050 · "), "{frame}");
        hover(&mut app, 0, 0);
        let frame = composed_frame(&app);
        assert!(
            frame.contains("╭ Balances by tax treatment · today's dollars ─"),
            "{frame}"
        );
        click(&mut app, column(2038), row);
        assert_eq!(app.world().resource::<YearCursor>().0, Some(2038));
        app.update();
        let frame = composed_frame(&app);
        assert!(
            frame.contains("2038 · to do"),
            "the To do pane follows: {frame}"
        );
    }
}
