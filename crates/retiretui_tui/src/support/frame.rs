//! Reading what a headless app drew.

use bevy_app::App;
use plurimus::core::ratatui_core::buffer::{Buffer, Cell};
use plurimus::core::ratatui_core::style::{Color, Style};
use plurimus::core::{FrameBuffer, TerminalRenderApp};

/// The frame after what a change spawned has been drawn: rows spawned by
/// commands land a frame later than the change.
pub fn redrawn(app: &mut App) -> String {
    app.update();
    app.update();
    composed_frame(app)
}

/// The style the cell at `column`, `row` of the composed frame is drawn in.
pub fn cell_style(app: &App, column: u16, row: u16) -> Style {
    let cell = composed_buffer(app).cell((column, row));
    cell.map(Cell::style).unwrap_or_default()
}

pub fn cell_fg(app: &App, column: u16, row: u16) -> Option<Color> {
    cell_style(app, column, row).fg
}

/// The frame the render sub-app composed on the last tick.
pub fn composed_buffer(app: &App) -> &Buffer {
    &app.sub_app(TerminalRenderApp)
        .world()
        .resource::<FrameBuffer>()
        .0
}

/// Where `text` is drawn in the frame, for a press on it.
pub(crate) fn cell_of(app: &App, text: &str) -> (u16, u16) {
    let frame = composed_frame(app);
    let found = frame
        .lines()
        .enumerate()
        .find_map(|(row, line)| line.find(text).map(|at| (line, row, at)));
    let (line, row, at) = found.unwrap_or_else(|| panic!("{text:?} is not drawn: {frame}"));
    let cells = line[..at].chars().count();
    (
        u16::try_from(cells).expect("a column within the frame"),
        u16::try_from(row).expect("a row within the frame"),
    )
}

/// The composed frame as one line of cell symbols per row.
pub fn composed_frame(app: &App) -> String {
    frame_to_string(composed_buffer(app))
}

pub fn frame_to_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut lines = Vec::with_capacity(area.height as usize);
    for y in area.top()..area.bottom() {
        let mut line = String::with_capacity(area.width as usize);
        for x in area.left()..area.right() {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        lines.push(line);
    }
    lines.join("\n")
}
