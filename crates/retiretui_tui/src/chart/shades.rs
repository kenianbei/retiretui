//! Bands a chart shades under its lines: between a low and a high value
//! each year, a cell shaded in every column the band covers.

use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::ratatui_core::style::{Color, Style};

use super::SeriesChart;

/// A band: each year's low and high, drawn in `symbol`.
#[derive(Clone)]
pub struct Shade {
    pub points: Vec<(f64, f64, f64)>,
    pub symbol: &'static str,
    pub color: Color,
}

/// The band's low and high at `year`, between the two points around it.
fn at(points: &[(f64, f64, f64)], year: f64) -> Option<(f64, f64)> {
    let after = points.iter().position(|&(at, _, _)| at >= year)?;
    let (to, low_to, high_to) = points[after];
    let Some(&(from, low_from, high_from)) = after.checked_sub(1).map(|before| &points[before])
    else {
        return Some((low_to, high_to));
    };
    let along = if to > from {
        (year - from) / (to - from)
    } else {
        0.0
    };
    let between = |from: f64, to: f64| from + along * (to - from);
    Some((between(low_from, low_to), between(high_from, high_to)))
}

/// Drawn before the chart, whose lines then cross the bands; a later band
/// shades over an earlier one, and a band where it is empty shades
/// nothing. Every column is shaded, between the years either side of it.
pub(super) fn draw_shades(chart: &SeriesChart, area: Rect, buf: &mut Buffer) {
    let Some(graph) = chart
        .graph(area)
        .filter(|graph| graph.height > 0 && graph.width > 1)
    else {
        return;
    };
    let [floor, top] = chart.y_bounds;
    let row_of = |value: f64| {
        let along = ((value - floor) / (top - floor)).clamp(0.0, 1.0);
        graph.bottom() - 1 - (along * f64::from(graph.height - 1)).round() as u16
    };
    for shade in &chart.shades {
        let style = Style::new().fg(shade.color);
        for column in graph.x..graph.right() {
            let Some((low, high)) =
                at(&shade.points, chart.year_along(graph, column)).filter(|(low, high)| high > low)
            else {
                continue;
            };
            for row in row_of(high)..=row_of(low) {
                if let Some(cell) = buf.cell_mut((column, row)) {
                    cell.set_symbol(shade.symbol).set_style(style);
                }
            }
        }
    }
}
