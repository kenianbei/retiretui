//! The frame every view is laid out in: one `bevy_ui` tree the terminal's
//! size fills. Every size is in cells: `Val::Rem` is a cell's height and
//! `Val::Em` a cell's width - one pixel each in the terminal, and a font's
//! line height and advance width in a window - so a pixel is named nowhere.

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Bundle, Changed, Commands, Component, Entity, IntoScheduleConfigs, Mut, Query, Ref, Res,
};
use bevy_ui::{ComputedNode, Display, FlexDirection, JustifyContent, Node, Val};
use plurimus::bui::ComputedNodeRect;
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

pub use clip::{cells_of, clipped, clipped_middle, fitting, wrapped};
pub use cursor::{CURSOR_COLS, GUTTER, Rests, SCROLL_BAR_COLS, cursor_bar, list_cursor};
pub use list::{CONTINUED, fill_lines, fill_wrapped, row_width, spawn_scrolled_list};

use super::theme::{Repainted, Theme, text};

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
    app.add_plugins(cursor::plugin);
    app.add_systems(Startup, spawn_frame);
    app.add_systems(
        Update,
        (hold_offsets, (draw_rules, emphasise).in_set(Repainted)),
    );
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
        min_height: Val::ZERO,
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
        min_height: Val::ZERO,
        ..Node::default()
    }
}

/// Chrome of a fixed height across the frame, which a growing neighbour
/// does not shrink.
#[must_use]
pub fn fixed(height: f32) -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Rem(height),
        flex_shrink: 0.0,
        ..Node::default()
    }
}

/// A box of its own size, which a full parent does not shrink.
#[must_use]
pub fn sized(cols: f32, rows: f32) -> Node {
    Node {
        width: Val::Em(cols),
        height: Val::Rem(rows),
        flex_shrink: 0.0,
        ..Node::default()
    }
}

/// Where a widget sits outside the tab order, reachable by pointer but
/// never stepped to.
pub const NO_STOP: i32 = -1;

/// The `[ ` and ` ]` plurimus paints a button's label between.
/// What a button's label is said between.
pub const BUTTON_BRACKETS: [&str; 2] = ["[ ", " ]"];
const BUTTON_DECORATION: u16 = (BUTTON_BRACKETS[0].len() + BUTTON_BRACKETS[1].len()) as u16;
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
        column_gap: Val::Em(BUTTON_GAP),
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

impl Rule {
    /// The title along the line, a cell before the line goes on.
    pub(crate) fn heading(&self) -> String {
        if self.0.is_empty() {
            String::new()
        } else {
            format!("{} ", self.0)
        }
    }
}

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
            let line = Block::new().borders(Borders::TOP).title(rule.heading());
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

/// The columns a laid-out node spans: its width in logical pixels over
/// its em, which is a cell's width in either target. The node's size is
/// in physical pixels, a scale factor larger on a dense screen.
#[must_use]
pub fn node_cols(node: &ComputedNode) -> u16 {
    (node.size.x * node.inverse_scale_factor / node.em_size.0).round() as u16
}

/// The rows a laid-out node spans: its height in logical pixels over its
/// rem, a cell's height in either target.
#[must_use]
pub fn node_rows(node: &ComputedNode) -> u16 {
    (node.size.y * node.inverse_scale_factor / node.rem_size.0).round() as u16
}

pub fn spawn_frame(mut commands: Commands) {
    commands.spawn(TerminalCamera::default());
    let root = commands
        .spawn((
            Root,
            Node {
                flex_direction: FlexDirection::Column,
                min_width: Val::ZERO,
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
    commands.spawn((HintRow, fixed(f32::from(HINT_ROWS)), text(), ChildOf(root)));
}

/// Layout speaks in screen cells and `UiArea::Fixed` in camera-local ones.
pub(crate) fn sync_areas(
    default_camera: Res<DefaultCamera>,
    cameras: Query<&ResolvedViewport>,
    mut widgets: Query<(&ComputedNodeRect, &mut UiArea), Changed<ComputedNodeRect>>,
) {
    let Some(viewport) = default_viewport(*default_camera, &cameras) else {
        return;
    };
    for (node, mut area) in &mut widgets {
        area.set_if_neq(UiArea::Fixed(local_area(node.visible, viewport)));
    }
}

/// The cells the default camera draws into, once it has resolved them.
pub(crate) fn default_viewport(
    default_camera: DefaultCamera,
    cameras: &Query<&ResolvedViewport>,
) -> Option<Rect> {
    let camera = default_camera.0?;
    cameras.get(camera).ok().map(|resolved| resolved.0)
}

#[cfg(test)]
mod tests {
    use bevy_math::Vec2;
    use plurimus::bui::bevy_text::{EmSize, RemSize};

    use super::*;

    #[test]
    fn a_node_s_cells_are_its_logical_size_over_its_em_and_rem() {
        let dense = ComputedNode {
            size: Vec2::new(1234.0, 291.0),
            inverse_scale_factor: 1.0 / 1.5,
            em_size: EmSize(8.0),
            rem_size: RemSize(20.0),
            ..ComputedNode::default()
        };
        assert_eq!((node_cols(&dense), node_rows(&dense)), (103, 10));
        let plain = ComputedNode {
            inverse_scale_factor: 1.0,
            ..dense
        };
        assert_eq!((node_cols(&plain), node_rows(&plain)), (154, 15));
    }
}
