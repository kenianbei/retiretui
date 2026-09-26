//! The modal a file picker stands in: opened on the workspace, closed by a
//! choice or a dismissal, the path chosen handed to the picker's system.

use std::path::PathBuf;

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, In, IntoScheduleConfigs, On, Res, ResMut, Resource};
use bevy_ecs::system::{SystemId, SystemParam};
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{ModalDismiss, ModalOpen};
use plurimus::widgets::ValueChange;
use plurimus_filepicker::{FilePicker, FilePickerLook, FilePickerMatchStyle};

use super::pickers;
use crate::commands::tui::compare::Compared;
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::{growing, list_cursor, placed};
use crate::commands::tui::overlay::{self, Standing};
use crate::commands::tui::pane::Framed;
use crate::commands::tui::picker::PROMPT;
use crate::commands::tui::session::Session;
use crate::commands::tui::theme::Theme;

const WIDTH: u16 = 64;
/// The path row, and the rows of the list under it.
const ROWS: u16 = 11;

pub fn plugin(app: &mut App) {
    app.init_resource::<Browsing>();
    app.add_systems(Update, sync_browse.in_set(overlay::Settles));
}

/// One file picker: what it is called, what it lists, and the system told
/// the path chosen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FilePick {
    pub(super) title: &'static str,
    /// The extension of the files listed; directories always are.
    pub(super) extension: &'static str,
    /// Whether a name no file has is offered as a file to write.
    pub(super) accepts_new: bool,
    pub(super) badges: Badges,
    pub(super) chosen: SystemId<In<PathBuf>>,
}

/// What a picker's rows say beside each file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Badges {
    None,
    /// `plan`, `scenario` or `invalid`, the last dimmed.
    Kind,
    /// The kind, or `compared` where the Compare page holds the file.
    Compared,
}

/// What the picker is dressed with as it is spawned.
#[derive(SystemParam)]
struct Dressing<'w> {
    theme: Res<'w, Theme>,
    compared: Res<'w, Compared>,
}

/// The file picker on show.
#[derive(Resource, Default, Debug)]
pub struct Browsing {
    pick: Option<FilePick>,
    /// The name the path field is opened holding.
    named: Option<&'static str>,
}

impl Browsing {
    pub fn open(&mut self, pick: FilePick) {
        self.open_named(pick, None);
    }

    /// Opens `pick` with `named` already typed, where there is one.
    pub fn open_named(&mut self, pick: FilePick, named: Option<&'static str>) {
        self.pick = Some(pick);
        self.named = named;
    }

    pub fn close(&mut self) {
        self.pick = None;
    }

    pub const fn is_open(&self) -> bool {
        self.pick.is_some()
    }
}

#[derive(Component, Default, Debug)]
struct BrowseRoot;

fn sync_browse(
    browsing: Res<Browsing>,
    session: Res<Session>,
    dressing: Dressing,
    mut standing: Standing<BrowseRoot>,
    mut commands: Commands,
) {
    if !browsing.is_changed() {
        return;
    }
    let Some(pick) = browsing.pick else {
        standing.close(&mut commands);
        return;
    };
    let Some(root) = standing.open(&mut commands) else {
        return;
    };
    commands
        .entity(root)
        .insert((
            overlay::centred(WIDTH, ROWS),
            Framed::over(pick.title),
            ModalOpen,
            Hints(&[
                ("↑↓", "move"),
                ("⏎", "pick"),
                ("⇥", "complete"),
                ("←", "parent"),
                ("esc", "close"),
            ]),
        ))
        .observe(handle_dismiss);
    let dim = dressing.theme.dimmed();
    let look = FilePickerLook::default()
        .with_extensions([pick.extension])
        .with_accepts_new(pick.accepts_new)
        .with_prompt(Line::styled(PROMPT, dim));
    let mut field = FilePicker::new(session.workspace());
    if let Some(named) = browsing.named {
        field.set_path(named);
    }
    let mut picker = commands.spawn((
        field,
        UiWidget::default(),
        look,
        FilePickerMatchStyle(dressing.theme.accented()),
        list_cursor(),
        growing(),
        placed(),
        ChildOf(root),
    ));
    if let Some(decorator) = pickers::decorate(pick.badges, dim, &dressing.compared) {
        picker.insert(decorator);
    }
    let picker = picker.observe(handle_chosen).observe(handle_dismiss).id();
    standing.focus(picker);
}

fn handle_chosen(
    chosen: On<ValueChange<PathBuf>>,
    mut browsing: ResMut<Browsing>,
    mut commands: Commands,
) {
    if let Some(pick) = browsing.pick.take() {
        commands.run_system_with(pick.chosen, chosen.value.clone());
    }
}

/// Esc on the picker and a click outside the frame both land here.
fn handle_dismiss(_dismissed: On<ModalDismiss>, mut browsing: ResMut<Browsing>) {
    browsing.close();
}
