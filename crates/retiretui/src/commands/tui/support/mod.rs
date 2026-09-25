//! Headless test helpers: a terminal-less app, key injection, and
//! composed-frame snapshots read from the render sub-app. Mirrors
//! plurimus's own unpublished test crate.

mod fixtures;
mod frame;
mod input;
mod year;

pub use fixtures::*;
pub use frame::*;
pub use input::*;
pub use year::*;

use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use bevy_app::App;
use bevy_ecs::system::RunSystemOnce;
use plurimus::core::{CorePlugin, TerminalSize};
use plurimus::term::{InputCapabilities, KeyCode};
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::layer::SubscriberExt as _;

use crate::commands::tui::confirm::Confirm;
use crate::commands::tui::documents::Browsing;
use crate::commands::tui::edit::DraftEditor;
use crate::commands::tui::journal::{self, Inbox, Journal};
use crate::commands::tui::nav::{ActivePage, Page};
use crate::commands::tui::session::{Session, Today};
use crate::commands::tui::settings::Settings;
use crate::commands::tui::tools::Searches;
use crate::commands::tui::watch;

pub const SIZE: TerminalSize = TerminalSize::new(128, 32);

/// A terminal filling a modern screen, which is what most of the shell is
/// actually seen at, and wide enough for a form to dock beside its table.
pub const ROOMY: TerminalSize = TerminalSize::new(200, 55);

/// The year the test plans start in, so the year cursor opens on their
/// first row whenever the suite runs.
pub const TODAY: Today = Today(2026);

/// Applies `edit` to the draft, as an applied item would.
pub fn commit_edit(app: &mut App, edit: impl Fn(&mut Plan) + Send + Sync + 'static) {
    app.world_mut()
        .run_system_once(move |mut editor: DraftEditor| {
            edit(&mut editor.draft.plan);
            editor.commit();
        })
        .unwrap();
}

pub fn is_asking(app: &App) -> bool {
    app.world().resource::<Confirm>().is_open()
}

pub fn is_browsing(app: &App) -> bool {
    app.world().resource::<Browsing>().is_open()
}

/// Answers the question standing: it opens on its last answer, and `back`
/// steps toward the first.
pub fn answer_back(app: &mut App, back: usize) {
    for _ in 0..back {
        press_shift(app, KeyCode::Tab);
    }
    press_key(app, KeyCode::Enter);
}

/// A headless app, and the subscriber that carries what it says to its
/// journal. `tracing`'s default is per thread, so each test hears only
/// itself however many run at once.
pub struct Headless {
    app: App,
    _listening: DefaultGuard,
}

impl Deref for Headless {
    type Target = App;

    fn deref(&self) -> &App {
        &self.app
    }
}

impl DerefMut for Headless {
    fn deref_mut(&mut self) -> &mut App {
        &mut self.app
    }
}

/// The shell over `plan_text`, its Monte Carlo cut to [`FEW_TRIALS`],
/// written at `path`: its searches on, and every one answered.
pub fn searched_app(path: PathBuf, plan_text: &str, size: TerminalSize) -> Headless {
    std::fs::write(&path, format!("{plan_text}{FEW_TRIALS}")).unwrap();
    let mut app = headless_app_at(path, size);
    app.insert_resource(Searches(true));
    crate::commands::tui::tools::settle_all(&mut app);
    app
}

pub fn headless_app(size: TerminalSize) -> Headless {
    headless_app_at(scratch_plan(), size)
}

/// Everything said to the user so far, oldest first.
pub fn said(app: &App) -> Vec<String> {
    app.world()
        .resource::<Journal>()
        .entries()
        .map(|entry| entry.text.clone())
        .collect()
}

pub fn headless_app_at(plan_path: PathBuf, size: TerminalSize) -> Headless {
    headless_app_set(plan_path, size, Settings::still())
}

/// A headless app at `path`: a plan file, or a directory for the empty
/// shell.
pub fn headless_app_set(path: PathBuf, size: TerminalSize, settings: Settings) -> Headless {
    headless_app_on(path, size, settings, TODAY)
}

/// The shell over the plan at `path` in the calendar year `today`.
pub fn headless_app_in(path: PathBuf, size: TerminalSize, today: Today) -> Headless {
    headless_app_on(path, size, Settings::still(), today)
}

fn headless_app_on(
    path: PathBuf,
    size: TerminalSize,
    settings: Settings,
    today: Today,
) -> Headless {
    let session = Session::at(path, TaxTables::embedded());
    let (projected, files) = watch::load_session(&session, today).unwrap();
    let mut app = App::new();
    app.add_plugins(CorePlugin);
    // No backend refines the capabilities headlessly, and the default
    // claims every one present; a plain terminal is the honest model, so
    // modifier keys are synthesized from each message's own bits.
    app.insert_resource(InputCapabilities::none());
    app.insert_resource(size);
    app.insert_resource(session);
    app.insert_resource(settings);
    app.insert_resource(today);
    app.insert_resource(projected);
    app.insert_resource(watch::Watch::new(files));
    crate::commands::tui::add_tui(&mut app);
    app.insert_resource(Searches(false));
    let inbox = app.world().resource::<Inbox>().clone();
    let subscriber = tracing_subscriber::registry().with(journal::layer(inbox));
    let listening = tracing::subscriber::set_default(subscriber);
    app.update();
    Headless {
        app,
        _listening: listening,
    }
}
/// Frames a shown page takes to settle: its root is laid out on the first,
/// what is placed from that layout lands on the second, and the third
/// draws it.
pub const SETTLING_TICKS: usize = 3;

pub fn active_page(app: &App) -> Page {
    app.world().resource::<ActivePage>().0
}

/// Shows `page`, then ticks until it has settled.
pub fn show(app: &mut App, page: Page) {
    app.insert_resource(ActivePage(page));
    for _ in 0..SETTLING_TICKS {
        app.update();
    }
}
