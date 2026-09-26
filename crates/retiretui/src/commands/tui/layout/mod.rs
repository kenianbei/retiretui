//! The frame every view is laid out in: one `bevy_ui` tree the terminal's
//! size fills.

use bevy_app::{App, PostUpdate, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Bundle, Changed, Commands, Component, Entity, IntoScheduleConfigs, Mut, Query, Ref, Res, With,
};
use bevy_ui::{Display, FlexDirection, JustifyContent, Node, UiSystems, Val};
use plurimus::bui::{BuiPlugin, ComputedNodeRect};
use plurimus::core::ratatui_core::layout::{Position, Rect};
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::{
    DefaultCamera, ResolvedViewport, TerminalCamera, UiArea, UiWidget, local_area,
};
use plurimus::ui::{
    ComputedWidgetArea, ScrollArea, ScrollOffset, UiStyle, apply_offset, max_offset,
};
use plurimus::widgets::ratatui_widgets::block::Block;
use plurimus::widgets::ratatui_widgets::borders::Borders;

mod clip;
mod cursor;
mod list;

pub use clip::{cells_of, clipped, clipped_middle, wrapped};
pub use cursor::{CURSOR_COLS, Rests, list_cursor, table_cursor};
pub use list::{fill_wrapped, row_width, spawn_scrolled_list};

use super::theme::{Repainted, Theme};

/// Rows the frame spends on chrome: the tab row and the hint row.
pub const CHROME_ROWS: u16 = TAB_ROW_ROWS + HINT_ROWS;

/// Rows the tab row takes: a boxed tab is its label between the two rows
/// of its border. `tabbar` asserts its look against this.
pub const TAB_ROW_ROWS: u16 = 3;

/// The frame row the body starts on, which is the row after the tab
/// row's foot. Only the tests that point at a cell need to name it.
#[cfg(test)]
pub const BODY_TOP: u16 = TAB_ROW_ROWS;

/// Rows the hint row takes.
const HINT_ROWS: u16 = 1;

/// The node every row of the frame hangs under.
#[derive(Component, Debug)]
pub struct Root;

/// The row of chrome along the top of the frame: the tab bar, and the
/// status the shell keeps beside it.
#[derive(Component, Debug)]
pub struct TabRow;

/// What the chrome leaves: the node pages are laid out in.
#[derive(Component, Debug)]
pub struct Body;

/// The row of chrome along the bottom of the frame.
#[derive(Component, Debug)]
pub struct HintRow;

pub fn plugin(app: &mut App) {
    app.add_plugins((BuiPlugin, cursor::plugin));
    app.add_systems(Startup, spawn_frame);
    app.add_systems(
        Update,
        (hold_offsets, (draw_rules, emphasise).in_set(Repainted)),
    );
    app.add_systems(PostUpdate, sync_areas.after(UiSystems::PostLayout));
}

/// Holds each scroll area's offset within what it can scroll once its area
/// moves: plurimus clamps an offset as it scrolls and again as it draws,
/// but keeps one a grown area has left past the end, which a click then
/// reads as rows further on than those drawn.
fn hold_offsets(
    mut areas: Query<
        (Entity, &ComputedWidgetArea, &ScrollArea, &mut ScrollOffset),
        Changed<ComputedWidgetArea>,
    >,
    mut commands: Commands,
) {
    for (entity, area, scroll, mut offset) in &mut areas {
        let most = max_offset(scroll.content_size, area.0);
        let held = Position::new(offset.0.x.min(most.x), offset.0.y.min(most.y));
        apply_offset(entity, held, &mut offset, &mut commands);
    }
}

#[must_use]
pub fn filling() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..Node::default()
    }
}

/// Takes what its fixed neighbours leave, shrinking below its own contents
/// rather than pushing them past the frame.
#[must_use]
pub fn growing() -> Node {
    Node {
        flex_grow: 1.0,
        width: Val::Percent(100.0),
        min_height: Val::Px(0.0),
        ..Node::default()
    }
}

/// Chrome of a fixed height across the frame, which a growing neighbour
/// does not shrink.
#[must_use]
pub fn fixed(height: f32) -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Px(height),
        flex_shrink: 0.0,
        ..Node::default()
    }
}

/// A box of its own size, which a full parent does not shrink.
#[must_use]
pub fn sized(cols: f32, rows: f32) -> Node {
    Node {
        width: Val::Px(cols),
        height: Val::Px(rows),
        flex_shrink: 0.0,
        ..Node::default()
    }
}

/// Where a widget sits outside the tab order, reachable by pointer but
/// never stepped to.
pub const NO_STOP: i32 = -1;

/// The `[ ` and ` ]` plurimus paints a button's label between.
const BUTTON_DECORATION: u16 = 4;
pub const BUTTON_GAP: f32 = 1.0;

/// The node of a button saying `label`, as wide as it is drawn.
#[must_use]
pub fn button_node(label: &str) -> Node {
    sized(
        f32::from(cells_of(label).saturating_add(BUTTON_DECORATION)),
        1.0,
    )
}

/// A row of buttons under what `parent` already holds, its last the
/// rightmost; answers with the row they are spawned into.
pub fn spawn_button_row(commands: &mut Commands, parent: Entity) -> Entity {
    let row = Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        column_gap: Val::Px(BUTTON_GAP),
        ..fixed(1.0)
    };
    commands.spawn((row, ChildOf(parent))).id()
}

/// What a button says of its answer beyond its label.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Emphasis {
    /// The answer the asker came to give.
    Primary,
    /// An answer that loses something, which stays marked under the keyboard.
    Destructive,
}

fn emphasise(theme: Res<Theme>, buttons: Query<(Entity, Ref<Emphasis>)>, mut commands: Commands) {
    for (button, emphasis) in &buttons {
        if !theme.is_changed() && !emphasis.is_added() {
            continue;
        }
        let style = match *emphasis {
            Emphasis::Primary => Style::new().add_modifier(Modifier::BOLD),
            Emphasis::Destructive => theme.exceeded(),
        };
        commands.entity(button).try_insert(UiStyle(style));
    }
}

/// A line across its parent, closing one part of it off from the next,
/// and naming the next where it has a title.
#[derive(Component, Debug)]
pub struct Rule(&'static str);

#[must_use]
pub fn rule() -> impl Bundle {
    titled_rule("")
}

#[must_use]
pub fn titled_rule(title: &'static str) -> impl Bundle {
    (
        Rule(title),
        fixed(1.0),
        UiWidget::new(Block::new()),
        placed(),
    )
}

fn draw_rules(theme: Res<Theme>, mut rules: Query<(Ref<Rule>, &mut UiWidget)>) {
    for (rule, mut widget) in &mut rules {
        if theme.is_changed() || rule.is_added() {
            let title = if rule.0.is_empty() {
                String::new()
            } else {
                format!("{} ", rule.0)
            };
            let line = Block::new().borders(Borders::TOP).title(title);
            *widget = UiWidget::new(line.border_style(theme.dimmed()));
        }
    }
}

/// Lays `node` out while `is_shown`, and takes it out of the layout
/// otherwise, touching it only where that changes something.
pub fn set_display(node: &mut Mut<Node>, is_shown: bool) {
    let display = if is_shown {
        Display::Flex
    } else {
        Display::None
    };
    if node.display != display {
        node.display = display;
    }
}

/// A widget drawn where its node is laid out; `sync_areas` keeps the two
/// together.
#[must_use]
pub fn placed() -> UiArea {
    UiArea::Fixed(Rect::ZERO)
}

pub fn spawn_frame(mut commands: Commands) {
    commands.spawn(TerminalCamera::default());
    let root = commands
        .spawn((
            Root,
            Node {
                flex_direction: FlexDirection::Column,
                ..filling()
            },
        ))
        .id();
    commands.spawn((
        TabRow,
        Node {
            flex_direction: FlexDirection::Row,
            ..fixed(f32::from(TAB_ROW_ROWS))
        },
        ChildOf(root),
    ));
    commands.spawn((Body, growing(), ChildOf(root)));
    commands.spawn((
        HintRow,
        fixed(f32::from(HINT_ROWS)),
        UiWidget::default(),
        placed(),
        ChildOf(root),
    ));
}

// Layout speaks in screen cells and `UiArea::Fixed` in camera-local ones.
fn sync_areas(
    default_camera: Res<DefaultCamera>,
    cameras: Query<&ResolvedViewport>,
    mut widgets: Query<
        (&ComputedNodeRect, &mut UiArea),
        (With<UiWidget>, Changed<ComputedNodeRect>),
    >,
) {
    let Some(viewport) = default_camera
        .0
        .and_then(|camera| cameras.get(camera).ok())
        .map(|resolved| resolved.0)
    else {
        return;
    };
    for (node, mut area) in &mut widgets {
        area.set_if_neq(UiArea::Fixed(local_area(node.visible, viewport)));
    }
}
