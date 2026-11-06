//! The framed pane a view lays its contents in.

use bevy_app::{App, Update};
use bevy_ecs::bundle::Bundle;
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut, Mut, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, Component, Entity, Has, IntoScheduleConfigs, Query, Res, Resource,
};
use bevy_input_focus::InputFocus;
use bevy_ui::{FlexDirection, Node, Overflow, UiRect, Val};
use plurimus::core::ratatui_core::buffer::Buffer;
use plurimus::core::ratatui_core::layout::Rect;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;
use plurimus::core::ratatui_core::widgets::Widget;
use plurimus::core::{UiArea, UiOrder, UiWidget};
use plurimus::ui::FocusWithin;
use plurimus::widgets::ratatui_widgets::block::Block;
use plurimus::widgets::ratatui_widgets::borders::BorderType;
use plurimus::widgets::ratatui_widgets::clear::Clear;

use super::layout::{filling, placed};
use super::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.init_resource::<FrameRows>();
    app.add_systems(
        Update,
        (
            light_frames.before(Repainted),
            draw_frames.in_set(Repainted),
        ),
    );
}

/// Rows a frame takes of a pane sized by what it holds: the terminal's
/// two border rows, or more where a skin draws a taller header.
#[derive(Resource, Debug, Clone, Copy)]
pub struct FrameRows(pub u16);

impl Default for FrameRows {
    fn default() -> Self {
        Self(BORDERS)
    }
}

/// The cells a pane's border takes across either axis: one on each side.
pub const BORDERS: u16 = 2;

/// The cell a pane keeps for its border on every side.
const FRAME_INSET: f32 = (BORDERS / 2) as f32;

/// One below the floor plurimus gives a node it publishes an area for, so
/// a frame is drawn beneath everything laid out inside it.
const PANE_ORDER: UiOrder = UiOrder(i32::MIN / 2 - 1);

/// What joins a title to what follows it.
pub(crate) const JOIN: &str = " · ";

/// A rounded border titled along its top line, drawn in the accent while
/// the keyboard is somewhere inside it, or on the frame itself.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
#[require(UiWidget, UiArea = placed(), Lit)]
pub struct Framed {
    pub title: String,
    /// What follows the title while there is something to say.
    pub note: String,
    /// What the title is drawn in, where not the accent.
    pub title_style: Option<Style>,
    pub(crate) is_covering: bool,
}

impl Framed {
    /// A frame drawn around what a view lays out inside it.
    #[must_use]
    pub fn titled(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            note: String::new(),
            title_style: None,
            is_covering: false,
        }
    }

    /// A frame that clears what it covers, for a box standing over the
    /// shell.
    #[must_use]
    pub fn over(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            note: String::new(),
            title_style: None,
            is_covering: true,
        }
    }

    /// Sets the title, marking the frame changed only when it differs.
    pub fn retitle(framed: &mut Mut<Self>, title: &str) {
        if framed.title != title {
            title.clone_into(&mut framed.title);
        }
    }

    /// Sets what the title is drawn in, the accent for `None`, marking the
    /// frame changed only when it differs.
    pub fn restyle(framed: &mut Mut<Self>, style: Option<Style>) {
        if framed.title_style != style {
            framed.title_style = style;
        }
    }

    /// Sets the note, marking the frame changed only when it differs.
    pub fn renote(framed: &mut Mut<Self>, note: &str) {
        if framed.note != note {
            note.clone_into(&mut framed.note);
        }
    }

    /// The title along the top line, the note after it while there is
    /// one, in its own style or the accent.
    pub(crate) fn title_line(&self, theme: &Theme) -> Line<'static> {
        let heading = if self.note.is_empty() {
            format!(" {} ", self.title)
        } else {
            format!(" {}{JOIN}{} ", self.title, self.note)
        };
        Line::styled(heading, self.title_style.unwrap_or(theme.accented()))
    }
}

/// A block drawn over cleared cells.
struct Cover(Block<'static>);

impl Widget for &Cover {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        Clear.render(area, buffer);
        (&self.0).render(area, buffer);
    }
}

/// Whether the keyboard is inside the frame, which every skin draws it
/// by.
#[derive(Component, Default, Debug, PartialEq, Eq)]
pub(crate) struct Lit(pub(crate) bool);

/// `FocusWithin` marks what the keyboard's holder sits in, never the
/// holder, and a form that is its page holds it itself.
fn light_frames(focus: Res<InputFocus>, mut frames: Query<(Entity, Has<FocusWithin>, &mut Lit)>) {
    let holder = focus.get();
    for (frame, is_within, mut lit) in &mut frames {
        lit.set_if_neq(Lit(is_within || holder == Some(frame)));
    }
}

fn draw_frames(theme: Res<Theme>, mut frames: Query<(Ref<Framed>, Ref<Lit>, &mut UiWidget)>) {
    for (framed, lit, mut widget) in &mut frames {
        if !theme.is_changed() && !framed.is_changed() && !lit.is_changed() {
            continue;
        }
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme.edge(lit.0))
            .title_top(framed.title_line(&theme));
        *widget = if framed.is_covering {
            UiWidget::new(Cover(block))
        } else {
            UiWidget::new(block)
        };
    }
}

/// A framed box a view lays its contents in, clipping what will not fit
/// rather than growing into its neighbours; the builder says how it takes
/// up room among them.
#[derive(Debug)]
pub struct Pane {
    frame: Framed,
    node: Node,
}

impl Pane {
    /// A pane filling what its parent gives it, its contents stacked down
    /// it.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            frame: Framed::titled(title),
            node: Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Em(FRAME_INSET), Val::Rem(FRAME_INSET)),
                overflow: Overflow::clip(),
                ..filling()
            },
        }
    }

    /// The same pane as one of its parent's flexed children, taking `share`
    /// of the room against its siblings' rather than a size of its own.
    #[must_use]
    pub fn sharing(mut self, share: f32) -> Self {
        self.node.flex_grow = share;
        self.node.flex_basis = Val::ZERO;
        self
    }

    /// The same pane `rows` tall, borders included, which a full parent
    /// does not shrink.
    #[must_use]
    pub fn tall(mut self, rows: f32) -> Self {
        self.node.height = Val::Rem(rows);
        self.node.flex_shrink = 0.0;
        self
    }

    /// The same pane given up to a full parent, down to `rows`, borders
    /// included.
    #[must_use]
    pub fn shrinking_to(mut self, rows: f32) -> Self {
        self.node.flex_shrink = 1.0;
        self.node.min_height = Val::Rem(rows);
        self
    }

    /// The same pane taking, past its own size, what its siblings leave.
    #[must_use]
    pub fn stretching(mut self) -> Self {
        self.node.flex_grow = 1.0;
        self
    }

    /// The same pane `cols` wide, borders included, which a full parent does
    /// not shrink.
    #[must_use]
    pub fn wide(mut self, cols: f32) -> Self {
        self.node.width = Val::Em(cols);
        self.node.flex_shrink = 0.0;
        self
    }

    /// The same pane never narrower than `cols`, borders included.
    #[must_use]
    pub fn at_least_wide(mut self, cols: f32) -> Self {
        self.node.min_width = Val::Em(cols);
        self
    }

    /// Draws the pane under `parent`, answering with the node its contents
    /// are spawned into.
    pub fn spawn(self, commands: &mut Commands, parent: Entity) -> Entity {
        self.spawn_with(commands, parent, ())
    }

    /// The pane spawned with `extra` beside its frame, for what must be
    /// there as the frame is dressed.
    pub fn spawn_with(self, commands: &mut Commands, parent: Entity, extra: impl Bundle) -> Entity {
        commands
            .spawn((self.node, self.frame, PANE_ORDER, extra, ChildOf(parent)))
            .id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::Page;
    use crate::support::{SIZE, headless_app, show};

    fn lit_titles(app: &mut bevy_app::App) -> Vec<String> {
        let mut frames = app.world_mut().query::<(&Framed, &Lit)>();
        frames
            .iter(app.world())
            .filter(|(_, lit)| lit.0)
            .map(|(framed, _)| framed.title.clone())
            .collect()
    }

    #[test]
    fn the_frame_the_keyboard_stands_in_is_the_one_drawn_in_the_accent() {
        let mut app = headless_app(SIZE);
        app.update();
        assert_eq!(lit_titles(&mut app), ["Success"], "the Overview's tile");
        show(&mut app, Page::Ledger);
        let ledger = "Ledger · today's dollars · Balances by treatment";
        assert_eq!(lit_titles(&mut app), [ledger]);
        show(&mut app, Page::Accounts);
        assert_eq!(lit_titles(&mut app), ["Accounts"]);
    }

    #[test]
    fn a_frame_that_holds_the_keyboard_itself_is_drawn_in_the_accent() {
        let mut app = headless_app(SIZE);
        show(&mut app, Page::Household);
        assert_eq!(lit_titles(&mut app), ["Household"]);
    }
}
