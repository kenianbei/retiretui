//! The frame every view is laid out in: one `bevy_ui` tree the terminal's
//! size fills, and the notice that stands in for it below the size the
//! shell needs.

use bevy_app::{App, PostUpdate, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Bundle, Changed, Commands, Component, Entity, IntoScheduleConfigs, Mut, Query, Ref, Res, With,
};
use bevy_ui::{Display, FlexDirection, JustifyContent, Node, UiSystems, Val};
use plurimus::bui::{BuiPlugin, ComputedNodeRect};
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::ratatui_core::style::{Modifier, Style};
use plurimus::core::{
    DefaultCamera, ResolvedViewport, TerminalCamera, TerminalSize, UiArea, UiHidden, UiWidget,
    local_area,
};
use plurimus::ui::UiStyle;
use plurimus::widgets::ratatui_widgets::block::Block;
use plurimus::widgets::ratatui_widgets::borders::Borders;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;

mod clip;
mod cursor;
mod list;

pub use clip::{cells_of, clipped, wrapped};
pub use cursor::{CURSOR_COLS, Rests, list_cursor, table_cursor};
pub use list::{fill_wrapped, row_width, spawn_scrolled_list};

use super::theme::{Repainted, Theme};
use super::{edit, tabbar};

/// The smallest terminal anything here is laid out for. What the shell
/// derives it needs only ever raises the minimum above this, never below.
const FLOOR: TerminalSize = TerminalSize::new(128, 32);

/// The smallest terminal the shell lays itself out in: what the tab row
/// and the tallest standing form need, or the floor where they need less.
/// Derived so that a renamed tab or a domain that gains a field cannot
/// leave it silently stale.
pub const MIN_SIZE: TerminalSize = TerminalSize::new(
    raised(FLOOR.cols, tabbar::TABS_COLS + tabbar::status::MIN_COLS),
    raised(FLOOR.rows, CHROME_ROWS + edit::TALLEST_FORM_ROWS),
);

/// Rows the frame spends on chrome: the tab row and the hint row.
const CHROME_ROWS: u16 = TAB_ROW_ROWS + HINT_ROWS;

const fn raised(floor: u16, needed: u16) -> u16 {
    if needed > floor { needed } else { floor }
}

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

/// The line shown in place of the frame below [`MIN_SIZE`].
#[derive(Component, Debug)]
struct Notice;

pub fn plugin(app: &mut App) {
    app.add_plugins((BuiPlugin, cursor::plugin));
    app.add_systems(Startup, spawn_frame);
    app.add_systems(
        Update,
        (guard_size, (draw_rules, emphasise).in_set(Repainted)),
    );
    app.add_systems(PostUpdate, sync_areas.after(UiSystems::PostLayout));
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
const BUTTON_DECORATION: usize = 4;
pub const BUTTON_GAP: f32 = 1.0;

/// The node of a button saying `label`, as wide as it is drawn.
#[must_use]
pub fn button_node(label: &str) -> Node {
    sized((label.chars().count() + BUTTON_DECORATION) as f32, 1.0)
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
    let notice = format!(
        "terminal too small (minimum {}x{})",
        MIN_SIZE.cols, MIN_SIZE.rows
    );
    commands.spawn((
        Notice,
        UiWidget::new(Paragraph::new(notice)),
        UiArea::Fill,
        UiHidden,
    ));
}

#[must_use]
pub fn fits(size: TerminalSize) -> bool {
    size.cols >= MIN_SIZE.cols && size.rows >= MIN_SIZE.rows
}

fn guard_size(
    size: Res<TerminalSize>,
    mut roots: Query<&mut Node, With<Root>>,
    notices: Query<Entity, With<Notice>>,
    mut commands: Commands,
) {
    if !size.is_changed() {
        return;
    }
    let is_fitting = fits(*size);
    for mut root in &mut roots {
        set_display(&mut root, is_fitting);
    }
    for notice in &notices {
        if is_fitting {
            commands.entity(notice).insert(UiHidden);
        } else {
            commands.entity(notice).remove::<UiHidden>();
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_needs_the_minimum_size() {
        assert!(!fits(TerminalSize::new(127, 32)));
        assert!(!fits(TerminalSize::new(128, 31)));
        assert!(fits(MIN_SIZE));
    }
}
