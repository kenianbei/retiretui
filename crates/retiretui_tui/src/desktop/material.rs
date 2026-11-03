//! The plot drawn through bevy's renderer: a material over a buffer of
//! the chart's runs - its bands and lines as points in plot space, and a
//! look saying how each is drawn - filled from the same chart the rows
//! are said from, and a fragment shader that paints each pixel from them.
//! The material and the plugin run only over bevy's assets and renderer,
//! so the window alone holds them.

use bevy_app::{App, Update};
use bevy_asset::{Asset, Assets, Handle, load_internal_asset, uuid_handle};
use bevy_color::{ColorToComponents, LinearRgba};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Query, Res, ResMut};
use bevy_math::{UVec4, Vec4};
use bevy_reflect::TypePath;
use bevy_render::render_resource::{AsBindGroup, ShaderType};
use bevy_render::storage::ShaderBuffer;
use bevy_shader::{Shader, ShaderRef};
use bevy_ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin};
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::ratatui_core::style::{Color, Style};

use super::charts::Charted;
use crate::chart::{SeriesChart, Shade};
use crate::theme::{Repainted, Theme};

const SHADER: Handle<Shader> = uuid_handle!("bd267fc8-4bce-4879-8704-aeb43acd3cc6");

pub fn plugin(app: &mut App) {
    load_internal_asset!(app, SHADER, "chart.wesl", Shader::from_wesl);
    app.add_plugins(UiMaterialPlugin::<ChartMaterial>::default());
    app.add_systems(Update, fill_plots.after(Repainted));
}

/// A plot's material: the points of its runs, and how they are drawn.
#[derive(AsBindGroup, Asset, TypePath, Clone)]
struct ChartMaterial {
    #[storage(0, read_only)]
    points: Handle<ShaderBuffer>,
    #[uniform(1)]
    look: ChartLook,
}

impl UiMaterial for ChartMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// Fills each plot's buffer and look again whenever its chart was said
/// again, and gives a plot not yet drawn its material.
fn fill_plots(
    theme: Res<Theme>,
    charts: Query<(&SeriesChart, Ref<Charted>)>,
    plots: Query<Option<&MaterialNode<ChartMaterial>>>,
    (mut materials, mut buffers): (ResMut<Assets<ChartMaterial>>, ResMut<Assets<ShaderBuffer>>),
    mut commands: Commands,
) {
    for (chart, charted) in &charts {
        let Ok(node) = plots.get(charted.plot) else {
            continue;
        };
        if !charted.is_changed() && node.is_some() {
            continue;
        }
        let (points, look) = runs(chart, charted.graph, &theme);
        let Some(node) = node else {
            let points = buffers.add(ShaderBuffer::from(points));
            let material = materials.add(ChartMaterial { points, look });
            commands.entity(charted.plot).insert(MaterialNode(material));
            continue;
        };
        let Some(mut material) = materials.get_mut(&node.0) else {
            continue;
        };
        material.look = look;
        if let Some(mut buffer) = buffers.get_mut(&material.points) {
            buffer.clear();
            buffer.extend_from_slice(&points);
        }
    }
}

/// The most runs and marks a plot draws.
const RUNS: usize = 16;
const RULES: usize = 8;
const BAND: u32 = 0;
const LINE: u32 = 1;
/// A line's width and a rule's dash, in pixels.
const LINE_WIDTH: f32 = 2.0;
const DASH: f32 = 4.0;

/// A point of a run in plot space: x along the years, a band's low and
/// high or a line's value twice, from the floor at 0 to the top at 1.
type Point = [f32; 4];

/// How a plot draws its runs: each run's start and length in the
/// points and its kind; its colour; the marks' rules and zero's; and
/// the widths. Laid out as the shader declares it.
#[derive(ShaderType, Clone, Copy, Debug, Default)]
struct ChartLook {
    runs: [UVec4; RUNS],
    colors: [Vec4; RUNS],
    rules: [Vec4; RULES],
    rule_colors: [Vec4; RULES],
    zero_color: Vec4,
    /// Runs, rules, whether zero is ruled, unused.
    counts: UVec4,
    /// Zero's y, the line width and the dash in pixels, unused.
    params: Vec4,
}

/// `chart`'s points and look, drawn in `graph`: a year's x is the middle
/// of the column `column_in` gives it, so the plot agrees with the
/// pointer and the rules rather than with ratatui's braille scale. A
/// chart with nothing to draw, or no graph to draw it in, is one point,
/// since a buffer of none is no buffer.
fn runs(chart: &SeriesChart, graph: Option<Rect>, theme: &Theme) -> (Vec<Point>, ChartLook) {
    let Some(graph) = graph else {
        return (vec![Point::default()], ChartLook::default());
    };
    let scale = Scale::of(chart, graph, theme);
    let mut runs = Runs::default();
    for shade in &chart.shades {
        let band =
            (shade.points.iter()).filter_map(|&(year, low, high)| scale.point(year, low, high));
        runs.add(BAND, scale.paint(shade.color, shade_alpha(shade)), band);
    }
    for series in &chart.series {
        let line =
            (series.points.iter()).filter_map(|&(year, value)| scale.point(year, value, value));
        runs.add(LINE, scale.paint(series.color, 1.0), line);
    }
    let Runs {
        mut points,
        mut look,
    } = runs;
    for (at, mark) in chart.marks.iter().take(RULES).enumerate() {
        let Some(x) = scale.x_of(f64::from(mark.year)) else {
            continue;
        };
        look.rules[at] = Vec4::new(x, 0.0, 0.0, 0.0);
        look.rule_colors[at] = scale.paint(mark.style.fg.unwrap_or(Color::Reset), 1.0);
        look.counts.y += 1;
    }
    let [floor, top] = chart.y_bounds;
    if floor < 0.0 && top > 0.0 {
        look.counts.z = 1;
        look.params.x = scale.y_of(0.0);
    }
    look.zero_color = scale.paint(chart.axis.fg.unwrap_or(Color::Reset), 1.0);
    look.params.y = LINE_WIDTH;
    look.params.z = DASH;
    if points.is_empty() {
        points.push(Point::default());
    }
    (points, look)
}

/// From a chart's years and values to plot space, and from its colours
/// to the theme's.
struct Scale<'a> {
    chart: &'a SeriesChart,
    graph: Rect,
    theme: &'a Theme,
    floor: f64,
    height: f64,
}

impl<'a> Scale<'a> {
    fn of(chart: &'a SeriesChart, graph: Rect, theme: &'a Theme) -> Self {
        let [floor, top] = chart.y_bounds;
        let height = if top > floor { top - floor } else { 1.0 };
        Self {
            chart,
            graph,
            theme,
            floor,
            height,
        }
    }

    fn x_of(&self, year: f64) -> Option<f32> {
        let column = self.chart.column_in(self.graph, year.round() as i16)?;
        let across = f64::from(column - self.graph.x) + 0.5;
        Some((across / f64::from(self.graph.width)) as f32)
    }

    fn y_of(&self, value: f64) -> f32 {
        ((value - self.floor) / self.height) as f32
    }

    fn point(&self, year: f64, low: f64, high: f64) -> Option<Point> {
        Some([self.x_of(year)?, self.y_of(low), self.y_of(high), 0.0])
    }

    fn paint(&self, color: Color, alpha: f32) -> Vec4 {
        let mut rgba: LinearRgba = self.theme.ink(Style::new().fg(color)).into();
        rgba.alpha = alpha;
        Vec4::from(rgba.to_f32_array())
    }
}

/// The runs gathered so far: their points end to end, and the look that
/// says where each starts.
#[derive(Default)]
struct Runs {
    points: Vec<Point>,
    look: ChartLook,
}

impl Runs {
    /// Adds a run of `points` where there are any and room for one more.
    fn add(&mut self, kind: u32, color: Vec4, points: impl Iterator<Item = Point>) {
        let run = self.look.counts.x as usize;
        if run == RUNS {
            return;
        }
        let start = self.points.len();
        self.points.extend(points);
        let len = self.points.len() - start;
        if len == 0 {
            return;
        }
        let (start, len) = (
            u32::try_from(start).unwrap_or(u32::MAX),
            u32::try_from(len).unwrap_or(u32::MAX),
        );
        self.look.runs[run] = UVec4::new(start, len, kind, 0);
        self.look.colors[run] = color;
        self.look.counts.x += 1;
    }
}

/// A band is as opaque as the glyph the terminal shades it with: the
/// lighter the glyph, the more the ground shows through.
fn shade_alpha(shade: &Shade) -> f32 {
    match shade.symbol {
        "░" => 0.35,
        "▒" => 0.6,
        "▓" => 0.8,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use naga::valid::{Capabilities, ValidationFlags, Validator};

    use super::*;
    use crate::support::{SIZE, headless_app, overview_chart};

    /// What bevy's vertex shader hands the fragment, as `bevy_ui_render`
    /// declares it; the shader imports it, which only bevy's loader resolves.
    const VERTEX_OUTPUT: &str = "struct UiVertexOutput {
        @location(0) uv: vec2<f32>,
        @location(1) border_widths: vec4<f32>,
        @location(2) border_radius_x: vec4<f32>,
        @location(3) border_radius_y: vec4<f32>,
        @location(4) @interpolate(flat) size: vec2<f32>,
        @builtin(position) position: vec4<f32>,
    };";

    #[test]
    fn the_plot_shader_is_valid_wgsl_over_bevy_s_vertex_output() {
        let source = include_str!("chart.wesl");
        let (import, body) = source.split_once(";\n").unwrap();
        assert_eq!(
            import, "import bevy_ui_render::ui_vertex_output::UiVertexOutput",
            "the one import, resolved here by hand"
        );
        let wgsl = format!("{VERTEX_OUTPUT}\n{body}");
        let module = naga::front::wgsl::parse_str(&wgsl)
            .unwrap_or_else(|failure| panic!("{}", failure.emit_to_string(&wgsl)));
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .expect("the shader validates");
    }

    #[test]
    fn a_year_s_x_is_the_middle_of_the_column_that_reads_back_as_it() {
        let mut app = headless_app(SIZE);
        app.update();
        let chart = overview_chart(&mut app);
        let drawn = app.world().get::<SeriesChart>(chart).unwrap().clone();
        let area = Rect::new(0, 0, 76, 19);
        let graph = drawn.graph(area).unwrap();
        let (points, look) = runs(&drawn, Some(graph), &Theme::default());
        let (bands, lines) = (drawn.shades.len(), drawn.series.len());
        assert!(bands >= 2 && lines == 1, "{bands} bands under net worth");
        assert_eq!(look.counts.x as usize, bands + lines);
        assert_eq!(
            (look.runs[0].z, look.runs[bands].z, look.counts.y),
            (BAND, LINE, 1),
            "the bands, the line, the salary's end"
        );
        let line = look.runs[bands];
        let along = &points[line.x as usize..(line.x + line.y) as usize];
        for (point, &(year, _)) in along.iter().zip(&drawn.series[0].points) {
            let column = graph.x + (point[0] * f32::from(graph.width)).floor() as u16;
            assert_eq!(Some(column), drawn.column_of(area, year as i16), "{year}");
            assert!((0.0..=1.0).contains(&point[1]), "{point:?}");
        }
        let (none, empty) = runs(&SeriesChart::default(), Some(graph), &Theme::default());
        assert_eq!((none, empty.counts), (vec![Point::default()], UVec4::ZERO));
        let (none, empty) = runs(&drawn, None, &Theme::default());
        assert_eq!((none, empty.counts), (vec![Point::default()], UVec4::ZERO));
    }
}
