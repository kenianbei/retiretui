//! The pickers over the filesystem: the file to open, to save as, to
//! start, to compare with, to write an overlay to, or to record a
//! statement from.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use bevy_ecs::prelude::{Commands, In, Res, ResMut, Resource, World};
use bevy_ecs::system::{IntoSystem, SystemParam};
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;
use plurimus_filepicker::{FilePickerDecorator, RowDecoration};
use retiretui_engine::plan::{Plan, Scenario};

use super::browse::{Badges, Browsing, FilePick};
use crate::commands::tui::command::{Outcome, Pending};
use crate::commands::tui::compare::Compared;
use crate::commands::tui::confirm::Confirm;
use crate::commands::tui::edit::{self, Draft};
use crate::commands::tui::journal;
use crate::commands::tui::session::{self, Session};
use crate::commands::tui::setup;
use crate::commands::tui::tools;
use crate::commands::tui::watch::Watch;

const EXTENSION: &str = "toml";
const STATEMENT_EXTENSION: &str = "xml";
const SAVE_OVER: &str = "Overwrite {}?";
const COMPARED: &str = "compared";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Plan,
    Scenario,
    Invalid,
}

impl Kind {
    fn of(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::Invalid;
        };
        match Scenario::from_toml_str(&text) {
            Ok(Some(_)) => Self::Scenario,
            Ok(None) if Plan::from_toml_str(&text).is_ok() => Self::Plan,
            _ => Self::Invalid,
        }
    }

    const fn badge(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Scenario => "scenario",
            Self::Invalid => "invalid",
        }
    }
}

/// What a picker's rows say beside each file: its kind, dimmed where
/// invalid, and `compared` over that on a file the Compare page holds as
/// the picker opens.
// ponytail: every file is read and parsed per directory read; cache by
// mtime if a workspace grows large.
pub(super) fn decorate(
    badges: Badges,
    dim: Style,
    compared: &Compared,
) -> Option<FilePickerDecorator> {
    let compared: Vec<PathBuf> = match badges {
        Badges::None => return None,
        Badges::Kind => Vec::new(),
        Badges::Compared => compared.paths().map(Path::to_path_buf).collect(),
    };
    Some(FilePickerDecorator(Box::new(move |path| {
        if compared.iter().any(|held| held == path) {
            return RowDecoration::default().with_trailing(Line::styled(COMPARED, dim));
        }
        let kind = Kind::of(path);
        let row = RowDecoration::default().with_trailing(Line::styled(kind.badge(), dim));
        if kind == Kind::Invalid {
            row.with_style(dim)
        } else {
            row
        }
    })))
}

#[derive(Resource, Clone, Copy)]
pub struct Pickers {
    pub open: FilePick,
    save_as: FilePick,
    compare: FilePick,
    /// Where a tool writes its highlighted result as a scenario.
    pub ladder: FilePick,
    pub claims: FilePick,
    new_plan: FilePick,
    /// The Social Security statement to record on a person.
    pub earnings: FilePick,
}

pub fn register(world: &mut World) {
    let pickers = Pickers {
        open: FilePick {
            title: "Open",
            extension: EXTENSION,
            accepts_new: false,
            badges: Badges::Kind,
            chosen: world.register_system(open_chosen),
        },
        save_as: writing(world, "Save as", write_as, SAVE_OVER),
        compare: FilePick {
            title: "Compare with",
            extension: EXTENSION,
            accepts_new: false,
            badges: Badges::Compared,
            chosen: world.register_system(compare_chosen),
        },
        ladder: writing(
            world,
            "Write overlay",
            tools::ladders::write_overlay,
            tools::OVERLAY_OVER,
        ),
        claims: writing(
            world,
            "Write overlay",
            tools::claims::write_overlay,
            tools::OVERLAY_OVER,
        ),
        new_plan: writing(world, "New plan as", setup::write_new, SAVE_OVER),
        earnings: FilePick {
            title: "Import earnings from",
            extension: STATEMENT_EXTENSION,
            accepts_new: false,
            badges: Badges::None,
            chosen: world.register_system(edit::record_statement),
        },
    };
    world.insert_resource(pickers);
}

/// Whether the workspace holds anything the Open picker could offer, so a
/// first launch in a directory with no plans is not shown a list of files
/// the user does not have.
pub fn workspace_holds_a_plan(session: &Session) -> bool {
    std::fs::read_dir(session.workspace())
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new(EXTENSION)))
        .any(|path| Kind::of(&path) != Kind::Invalid)
}

pub fn open(pickers: Res<Pickers>, mut browsing: ResMut<Browsing>) -> Outcome {
    browsing.open(pickers.open);
    Outcome::Done
}

pub fn save_as(
    pickers: Res<Pickers>,
    draft: Res<Draft>,
    mut browsing: ResMut<Browsing>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_invalid() {
        return refusal;
    }
    browsing.open(pickers.save_as);
    Outcome::Done
}

/// Names the plan the new plan's form composed.
pub fn name_new_plan(pickers: Res<Pickers>, mut browsing: ResMut<Browsing>) {
    browsing.open(pickers.new_plan);
}

pub fn compare_with(pickers: Res<Pickers>, mut browsing: ResMut<Browsing>) -> Outcome {
    browsing.open(pickers.compare);
    Outcome::Done
}

fn open_chosen(In(path): In<PathBuf>, mut pending: ResMut<Pending>) {
    pending.defer_open(path);
}

/// Compares `path`, or stops comparing it; the document itself is refused.
fn compare_chosen(In(path): In<PathBuf>, session: Res<Session>, mut compared: ResMut<Compared>) {
    if is_the_document(&session, &path) {
        journal::warn(format!("{} is the document", session.file_name()));
        return;
    }
    compared.toggle(path, &session.tables);
}

fn is_the_document(session: &Session, path: &Path) -> bool {
    let document = session
        .plan_path
        .as_deref()
        .and_then(|document| document.canonicalize().ok());
    document.is_some_and(|document| path.canonicalize().is_ok_and(|path| path == document))
}

/// A picker over the documents whose chosen name `write` is run with, after
/// `question` where the file exists.
fn writing<M: 'static>(
    world: &mut World,
    title: &'static str,
    write: impl IntoSystem<In<PathBuf>, (), M> + Copy + Send + Sync + 'static,
    question: &'static str,
) -> FilePick {
    let chosen = move |In(path): In<PathBuf>, mut asking: Asking| {
        asking.write_or_ask(path, write, question);
    };
    FilePick {
        title,
        extension: EXTENSION,
        accepts_new: true,
        badges: Badges::Kind,
        chosen: world.register_system(chosen),
    }
}

/// What a write to a chosen name asks first: nothing for a new file, and a
/// question naming the one it would replace.
#[derive(SystemParam)]
struct Asking<'w, 's> {
    confirm: ResMut<'w, Confirm>,
    commands: Commands<'w, 's>,
}

impl Asking<'_, '_> {
    fn write_or_ask<Marker: 'static>(
        &mut self,
        path: PathBuf,
        write: impl IntoSystem<In<PathBuf>, (), Marker> + Send + Sync + 'static,
        question: &str,
    ) {
        if !path.exists() {
            self.commands.run_system_cached_with(write, path);
            return;
        }
        let question = question.replacen("{}", &session::file_name(&path), 1);
        self.confirm.ask(question, "Overwrite", move |commands| {
            commands.run_system_cached_with(write, path);
        });
    }
}

/// Writes the draft under `path` and makes that the document, editable
/// whatever it was: a scenario saved as is a plan.
fn write_as(In(path): In<PathBuf>, world: &mut World) {
    if let Err(refusal) = edit::write_draft(world.resource::<Draft>(), &path) {
        journal::warn(refusal);
        return;
    }
    world.resource_mut::<Draft>().saved_as();
    world.insert_resource(Watch::at(path.clone()));
    let mut session = world.resource_mut::<Session>();
    session.plan_path = Some(path);
    journal::say(format!("saved {}", session.file_name()));
}
