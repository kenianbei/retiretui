import bevy_ui_render::ui_vertex_output::UiVertexOutput;

// The chart's runs: a run is `len` points from `start` of the storage
// buffer, each (x, low, high) in plot space, a band filled between its
// lows and highs or a line through its lows, in its own colour.
struct Look {
    runs: array<vec4<u32>, 16>,
    colors: array<vec4<f32>, 16>,
    rules: array<vec4<f32>, 8>,
    rule_colors: array<vec4<f32>, 8>,
    zero_color: vec4<f32>,
    // runs, rules, whether zero is ruled, unused
    counts: vec4<u32>,
    // zero's y, the line width and the dash in pixels, unused
    params: vec4<f32>,
}

@group(1) @binding(0) var<storage> points: array<vec4<f32>>;
@group(1) @binding(1) var<uniform> look: Look;

const BAND: u32 = 0u;
const LINE: u32 = 1u;
const FAR: f32 = 1e9;

fn over(under: vec4<f32>, paint: vec4<f32>) -> vec4<f32> {
    let alpha = paint.a + under.a * (1.0 - paint.a);
    if alpha <= 0.0 {
        return vec4(0.0);
    }
    let rgb = (paint.rgb * paint.a + under.rgb * under.a * (1.0 - paint.a)) / alpha;
    return vec4(rgb, alpha);
}

fn segment_distance(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let ab = b - a;
    let t = clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
    return length(p - (a + ab * t));
}

// The band's low and high at `x`, between the two points around it;
// nothing past its last point.
fn band_at(start: u32, len: u32, x: f32) -> vec2<f32> {
    var prev = points[start];
    if x <= prev.x || len == 1u {
        return vec2(prev.y, prev.z);
    }
    for (var i = 1u; i < len; i += 1u) {
        let next = points[start + i];
        if x <= next.x {
            let t = (x - prev.x) / max(next.x - prev.x, 1e-6);
            return mix(vec2(prev.y, prev.z), vec2(next.y, next.z), t);
        }
        prev = next;
    }
    return vec2(1.0, 0.0);
}

fn line_distance(start: u32, len: u32, p: vec2<f32>, px: vec2<f32>) -> f32 {
    if len == 1u {
        return length((p - points[start].xy) * px);
    }
    var nearest = FAR;
    for (var i = start + 1u; i < start + len; i += 1u) {
        let a = points[i - 1u].xy;
        let b = points[i].xy;
        if a.x > p.x + 0.05 {
            break;
        }
        if b.x < p.x - 0.05 {
            continue;
        }
        nearest = min(nearest, segment_distance(p * px, a * px, b * px));
    }
    return nearest;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // The borders are the axis; the plot is what is inside them.
    let b = in.border_widths;
    let span = vec2(max(1.0 - b.x - b.y, 1e-6), max(1.0 - b.z - b.w, 1e-6));
    let uv = vec2((in.uv.x - b.x) / span.x, (in.uv.y - b.z) / span.y);
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return vec4(0.0);
    }
    let px = in.size * span;
    let p = vec2(uv.x, 1.0 - uv.y);
    var color = vec4(0.0);
    for (var r = 0u; r < look.counts.x; r += 1u) {
        let run = look.runs[r];
        if run.z != BAND {
            continue;
        }
        let band = band_at(run.x, run.y, p.x);
        if p.y >= band.x && p.y <= band.y {
            color = over(color, look.colors[r]);
        }
    }
    let dash = look.params.z;
    if look.counts.z == 1u && abs(p.y - look.params.x) * px.y <= 0.5 && fract(p.x * px.x / dash) < 0.5 {
        color = over(color, look.zero_color);
    }
    for (var i = 0u; i < look.counts.y; i += 1u) {
        if abs(p.x - look.rules[i].x) * px.x <= 0.5 && fract(p.y * px.y / dash) < 0.5 {
            color = over(color, look.rule_colors[i]);
        }
    }
    let half = look.params.y / 2.0;
    for (var r = 0u; r < look.counts.x; r += 1u) {
        let run = look.runs[r];
        if run.z != LINE {
            continue;
        }
        if line_distance(run.x, run.y, p, px) <= half {
            color = over(color, look.colors[r]);
        }
    }
    return color;
}
