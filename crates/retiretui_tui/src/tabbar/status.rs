//! The status beside the tab bar: the document's name, the mark saying
//! the draft holds what the file does not, and what the draft is failing.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, Query, Res, Resource, With,
};
use bevy_ui::{Node, UiRect, Val};
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::core::{TerminalSize, UiWidget};
use plurimus::widgets::ratatui_widgets::block::Block;
use plurimus::widgets::ratatui_widgets::borders::Borders;

use crate::edit::Draft;
use crate::layout::{cells_of, clipped_middle, placed, sized};
use crate::present;
use crate::session::Session;
use crate::theme::{Repainted, Theme, say, text};

pub fn plugin(app: &mut App) {
    app.init_resource::<Reserved>();
    app.add_systems(Update, draw_status.in_set(Repainted));
}

/// Drawn beside the plan's file name: dim while the draft matches the
/// file, in the accent while it holds changes the file does not. An issue
/// count follows it while the draft has any.
const BADGE: &str = "●";

/// The cell kept clear after the badge, at the row's trailing edge.
const STATUS_MARGIN: u16 = 1;

/// The cell between the name and what follows it.
const NAME_GAP: u16 = 1;

/// Columns the status takes at its barest, the badge alone: what the
/// smallest terminal must leave beside the tabs.
pub const MIN_COLS: u16 = 1 + STATUS_MARGIN;

#[derive(Component, Debug)]
pub(crate) struct StatusCell;

/// Cells a skin keeps after the status on its row, which the status
/// leaves clear of the name.
#[derive(Resource, Debug, Default)]
pub(crate) struct Reserved(pub(crate) u16);

/// Laid beside the bar rather than inside it, where the bar's own chrome
/// would repaint it, and as tall as the bar so its foot carries the bar's
/// baseline on to the frame's edge; what it says sits on the middle row.
pub fn spawn(commands: &mut Commands, tab_row: Entity) {
    let foot = Block::new().borders(Borders::BOTTOM);
    commands.spawn((
        StatusCell,
        Node {
            height: Val::Percent(100.0),
            padding: UiRect::top(Val::Rem(1.0)),
            ..sized(0.0, 1.0)
        },
        text(),
        UiWidget::new(foot),
        placed(),
        ChildOf(tab_row),
    ));
}

/// The cell is as wide as what fits beside the tabs, measured whenever
/// what it says or the room for it changes.
fn draw_status(
    theme: Res<Theme>,
    shell: (Res<Draft>, Res<Session>, Res<TerminalSize>, Res<Reserved>),
    mut cells: Query<(Entity, &mut Node), With<StatusCell>>,
    mut commands: Commands,
) {
    let (draft, session, size, reserved) = shell;
    if !theme.is_changed() && !draft.is_changed() && !session.is_changed() && !size.is_changed() {
        return;
    }
    let marked = if draft.is_dirty() {
        theme.accented()
    } else {
        theme.dimmed()
    };
    let issues = draft.issues().len();
    // The badge says the draft holds what the file does not, so a shell
    // holding no file has none to show.
    let badge = (!session.is_empty()).then(|| Span::styled(BADGE, marked));
    let count = (issues > 0).then(|| {
        let count = format!(" {}", present::issue_count(issues));
        Span::styled(count, theme.exceeded())
    });
    let marks: Vec<Span<'static>> = [badge, count].into_iter().flatten().collect();
    let taken: u16 = marks.iter().map(|span| cells_of(&span.content)).sum();
    let room = size
        .cols
        .saturating_sub(super::TABS_COLS + NAME_GAP + taken + STATUS_MARGIN + reserved.0);
    let name = clipped_middle(session.file_name().into_owned(), room);
    let mut spans = vec![Span::styled(format!("{name} "), theme.dimmed())];
    spans.extend(marks);
    let line = Line::from(spans);
    let width = Val::Em(f32::from(line.width() as u16 + STATUS_MARGIN));
    let Ok((cell, mut node)) = cells.single_mut() else {
        return;
    };
    say(&mut commands, cell, line, &theme);
    if node.width != width {
        node.width = width;
    }
}
