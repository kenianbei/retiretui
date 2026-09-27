//! What was just said, shown briefly over the shell's lower right corner.
//! A toast never takes the keyboard.

use std::time::Duration;

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Commands, Component, Entity, IntoScheduleConfigs, MessageReader, On, Query, Res, ResMut,
    Resource, With,
};
use bevy_time::{Real, Time};
use bevy_ui::{AlignItems, FlexDirection, Node, PositionType, Val};
use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::widgets::Widget;
use plurimus::core::{TerminalSize, UiOrder, UiWidget};
use plurimus::ui::{Hovered, PointerPress};
use plurimus::widgets::ratatui_widgets::block::Block;
use plurimus::widgets::ratatui_widgets::borders::BorderType;
use plurimus::widgets::ratatui_widgets::clear::Clear;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use tracing::Level;

use super::journal::{Said, Spoken};
use super::layout::{self, Root, placed, sized};
use super::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<Toasts>();
    app.add_systems(Startup, spawn_stack.after(layout::spawn_frame));
    app.add_systems(Update, (take_toasts.after(Spoken), expire).chain());
    app.add_systems(Update, draw_toasts.in_set(Repainted).after(expire));
}

/// The most toasts shown at once; an older one gives way to a newer.
const STACK: usize = 3;
const INFO_LIFE: Duration = Duration::from_secs(4);
const WARN_LIFE: Duration = Duration::from_secs(8);
/// The most of the terminal's width a toast takes, per cent.
const WIDTH_SHARE: u16 = 66;
/// The border and the space inside it, on both sides.
const CHROME_COLS: u16 = 4;
const TOAST_ROWS: f32 = 3.0;
/// Clear of the hint row beneath, and of the frame's edge beside.
const INSET: f32 = 1.0;
/// Over every overlay, however deep the bands run.
const TOAST_ORDER: UiOrder = UiOrder(i32::MAX);

struct Toast {
    text: String,
    level: Level,
    left: Duration,
}

/// The toasts on show, oldest first.
#[derive(Resource, Default)]
pub struct Toasts(Vec<Toast>);

impl Toasts {
    #[cfg(test)]
    pub fn texts(&self) -> Vec<&str> {
        self.0.iter().map(|toast| toast.text.as_str()).collect()
    }
}

/// The node the toasts stack up from the lower right corner.
#[derive(Component)]
struct ToastStack;

/// Which toast, by its place in [`Toasts`], a drawn one is.
#[derive(Component, Clone, Copy)]
struct Shown(usize);

fn spawn_stack(roots: Query<Entity, With<Root>>, mut commands: Commands) {
    let Ok(root) = roots.single() else {
        return;
    };
    commands.spawn((
        ToastStack,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(INSET),
            bottom: Val::Px(INSET),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            ..Node::default()
        },
        ChildOf(root),
    ));
}

fn take_toasts(mut said: MessageReader<Said>, mut toasts: ResMut<Toasts>) {
    for Said(entry) in said.read() {
        let left = if entry.level <= Level::WARN {
            WARN_LIFE
        } else {
            INFO_LIFE
        };
        toasts.0.push(Toast {
            text: entry.text.clone(),
            level: entry.level,
            left,
        });
    }
    let over = toasts.0.len().saturating_sub(STACK);
    if over > 0 {
        toasts.0.drain(..over);
    }
}

// Wall-clock time: virtual time clamps a long frame, and a toast's life is
// counted in what the reader sat through.
fn expire(time: Res<Time<Real>>, mut toasts: ResMut<Toasts>) {
    if toasts.0.is_empty() {
        return;
    }
    let delta = time.delta();
    let lasting = toasts.bypass_change_detection();
    for toast in &mut lasting.0 {
        toast.left = toast.left.saturating_sub(delta);
    }
    let has_expired = lasting.0.iter().any(|toast| toast.left.is_zero());
    if has_expired {
        toasts.0.retain(|toast| !toast.left.is_zero());
    }
}

/// One line in a rounded box, over whatever it covers.
struct ToastBox {
    text: String,
    style: Style,
}

impl Widget for &ToastBox {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        Clear.render(area, buffer);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(self.style);
        Paragraph::new(self.text.as_str())
            .style(self.style)
            .block(block)
            .render(area, buffer);
    }
}

fn draw_toasts(
    toasts: Res<Toasts>,
    theme: Res<Theme>,
    size: Res<TerminalSize>,
    stacks: Query<Entity, With<ToastStack>>,
    mut commands: Commands,
) {
    if !toasts.is_changed() && !theme.is_changed() && !size.is_changed() {
        return;
    }
    let Ok(stack) = stacks.single() else {
        return;
    };
    commands.entity(stack).despawn_related::<Children>();
    let widest = size.cols.saturating_mul(WIDTH_SHARE) / 100;
    for (at, toast) in toasts.0.iter().enumerate() {
        let wanted = u16::try_from(toast.text.chars().count()).unwrap_or(u16::MAX);
        let width = wanted.saturating_add(CHROME_COLS).min(widest);
        let style = if toast.level <= Level::WARN {
            theme.exceeded()
        } else {
            theme.ui_theme().normal
        };
        commands
            .spawn((
                Shown(at),
                sized(f32::from(width), TOAST_ROWS),
                UiWidget::new(ToastBox {
                    text: format!(" {}", toast.text),
                    style,
                }),
                placed(),
                TOAST_ORDER,
                Hovered::default(),
                ChildOf(stack),
            ))
            .observe(handle_press);
    }
}

/// A press on a toast takes it down.
fn handle_press(press: On<PointerPress>, shown: Query<&Shown>, mut toasts: ResMut<Toasts>) {
    if let Ok(&Shown(at)) = shown.get(press.entity)
        && at < toasts.0.len()
    {
        toasts.0.remove(at);
    }
}
