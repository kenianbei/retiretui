//! The command table: every action the shell can be asked to do, named once
//! so that a key, the hint row, and the pickers all read from the same
//! row.

mod keys;
mod pickers;
mod table;
mod tools;

pub use tools::{TAKE_CLAIMS, TAKE_LADDER, WRITE_CLAIMS, WRITE_LADDER};

use std::path::PathBuf;
use std::sync::LazyLock;

use bevy_app::{App, PostStartup, Startup, Update};
use bevy_ecs::prelude::{
    Commands, Entity, IntoScheduleConfigs, On, Res, ResMut, Resource, Single, With, World,
};
use bevy_ecs::system::{SystemId, SystemParam};
use bevy_input::ButtonState;
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::FocusedInput;
use bevy_window::PrimaryWindow;
use plurimus::term::KeyModifiers;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::KeyBinding;

pub use table::COMMANDS;

use super::documents;
use super::journal;
use super::nav::{Page, PageSystems, ShownSurface};
use super::overlay;
use super::scope::{KeyScope, Scoped};
use super::session::NO_DOCUMENT;

pub struct CommandSpec {
    /// The stable kebab-case handle the command picker lists it under.
    pub name: &'static str,
    /// One short phrase, shown beside the command.
    pub doc: &'static str,
    /// The keystrokes that run it directly; the first is the one shown.
    pub keys: Vec<KeyBinding>,
    /// The word the hint bar says beside the key, for the commands worth
    /// a permanent reminder. Shown where [`Self::scope`] covers the page.
    pub hint: Option<&'static str>,
    pub register: Register,
    /// Where the command has something to act on.
    pub scope: Scope,
}

/// Where a command has something to act on, which is where its hint is
/// said.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    /// Anywhere at all: what finds a document, ends the session, or
    /// dresses the shell.
    Shell,
    /// Any page of an open document.
    Anywhere,
    /// A page listing a domain's items.
    Lists,
    /// One page, whose key row is where the command's hint belongs. It
    /// runs from any page, as every command of an open document does.
    On(Page),
}

impl Scope {
    /// Whether the command acts on `shown`, the surface on show.
    pub fn covers(self, shown: Option<Page>) -> bool {
        match self {
            Self::Shell => true,
            Self::Anywhere => shown.is_some(),
            Self::Lists => shown
                .filter(|page| page.is_domain())
                .is_some_and(super::edit::lists_items),
            Self::On(page) => shown == Some(page),
        }
    }
}

/// Registers the one-shot system that runs a command. Boxed rather than a
/// `fn`, so a row generated for a page can carry which.
pub type Register = Box<dyn Fn(&mut World) -> SystemId<(), Outcome> + Send + Sync>;

/// What running a command came to; a refusal is said to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "a dropped refusal is one nobody hears"]
pub enum Outcome {
    Done,
    Refused(String),
}

/// Identifies a command by its place in [`COMMANDS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandId(usize);

/// How each command's first key is shown, indexed as [`COMMANDS`] is;
/// empty for one bound to no key.
static KEY_LABELS: LazyLock<Vec<String>> = LazyLock::new(|| {
    COMMANDS
        .iter()
        .map(|spec| spec.keys.first().map(keys::label).unwrap_or_default())
        .collect()
});

impl CommandId {
    pub fn spec(self) -> &'static CommandSpec {
        &COMMANDS[self.0]
    }

    pub fn key_label(self) -> &'static str {
        &KEY_LABELS[self.0]
    }
}

/// Every command in table order.
pub fn all() -> impl Iterator<Item = CommandId> {
    (0..COMMANDS.len()).map(CommandId)
}

/// The command named `name`, for what runs one without a key.
pub fn named(name: &str) -> Option<CommandId> {
    all().find(|command| command.spec().name == name)
}

/// Runs the command named `name` once the keyboard is back, as its key
/// would.
pub fn defer_named(commands: &mut Commands, name: &'static str) {
    commands.queue(move |world: &mut World| {
        if let Some(command) = named(name) {
            world.resource_mut::<Pending>().defer(command);
        }
    });
}

/// The plain arrow keys a command particular to `page` binds, which the
/// keyboard then cannot also walk the page's panes by.
pub fn arrows_bound_on(page: Page) -> impl Iterator<Item = &'static Key> {
    let is_plain = |binding: &KeyBinding| {
        let held = binding.modifiers;
        !(held.ctrl || held.alt || held.shift)
    };
    table::BINDINGS
        .iter()
        .filter(move |(binding, command)| {
            command.spec().scope == Scope::On(page) && is_plain(binding)
        })
        .map(|(binding, _)| &binding.key)
        .filter(|key| {
            matches!(
                key,
                Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
            )
        })
}

/// The hinted commands that act on `shown` and are not `idle`, each with
/// its first key: the ones particular to the page first, since a narrow
/// row drops hints from the end, then table order.
pub fn page_hints(
    shown: Option<Page>,
    idle: impl Fn(&str) -> bool,
) -> impl Iterator<Item = (&'static str, &'static str)> {
    let live = move |command: &CommandId| {
        let spec = command.spec();
        spec.scope.covers(shown) && !idle(spec.name)
    };
    let is_particular =
        |command: &CommandId| matches!(command.spec().scope, Scope::Lists | Scope::On(_));
    let particular = all().filter(is_particular);
    let general = all().filter(move |command| !is_particular(command));
    particular
        .chain(general)
        .filter(live)
        .filter_map(|command| Some((command.key_label(), command.spec().hint?)))
}

/// What was chosen from something that holds the keyboard - the picker, a
/// menu, a question - and waits to run until the keyboard is back with what
/// held it, since that is what it acts on.
#[derive(Resource, Default)]
pub struct Pending(Option<Deferred>);

enum Deferred {
    Command(CommandId),
    Open(PathBuf),
}

impl Pending {
    pub fn defer(&mut self, command: CommandId) {
        self.0 = Some(Deferred::Command(command));
    }

    /// The document at `path` opened once the keyboard is back.
    pub fn defer_open(&mut self, path: PathBuf) {
        self.0 = Some(Deferred::Open(path));
    }
}

/// Runs what was deferred, once whatever chose it has closed and given the
/// keyboard back.
fn run_pending(mut pending: ResMut<Pending>, registry: Res<Registry>, mut commands: Commands) {
    match pending.0.take() {
        Some(Deferred::Command(command)) => registry.run(&mut commands, command),
        Some(Deferred::Open(path)) => {
            commands.run_system_cached_with(documents::open, path.into());
        }
        None => {}
    }
}

/// Where each command's system ended up, indexed as [`COMMANDS`] is.
#[derive(Resource)]
pub struct Registry(Vec<SystemId<(), Outcome>>);

impl Registry {
    /// Runs `command` when commands are next applied; a refusal is said to
    /// the user.
    pub fn run(&self, commands: &mut Commands, command: CommandId) {
        let system = self.0[command.0];
        commands.queue(move |world: &mut World| {
            let spec = command.spec();
            let shown = super::nav::shown_in(world);
            // Where no page is shown only what acts on the shell can.
            if shown.is_none() && !spec.scope.covers(shown) {
                journal::warn(NO_DOCUMENT);
                return;
            }
            if let Ok(Outcome::Refused(reason)) = world.run_system(system) {
                journal::warn(reason);
            }
        });
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Pending>();
    app.add_systems(Startup, register);
    app.add_systems(
        Update,
        run_pending
            .after(overlay::Settles)
            .in_set(PageSystems::Turn),
    );
    app.add_systems(PostStartup, watch_keys);
}

/// The command table takes the keys nothing else claimed. A focused key
/// bubbles from the widget it was typed at up to the window, so a field
/// that typed the character has already stopped it by the time it arrives
/// here; what a widget cannot stop, a [`KeyScope`] decides.
fn watch_keys(window: Single<Entity, With<PrimaryWindow>>, mut commands: Commands) {
    commands.entity(*window).observe(handle_shell_key);
}

/// Whether a key under `scope` is the command table's. The claimant
/// cannot stop the key itself: the focus navigation that walks a form's
/// fields watches the same entity this handler does.
fn yields(scope: Option<KeyScope>, held: KeyModifiers) -> bool {
    match scope {
        Some(KeyScope::All) => false,
        Some(KeyScope::Plain) => held.ctrl || held.alt,
        None => true,
    }
}

/// What a key is dispatched against: the surface on show and the systems
/// the commands were registered as.
#[derive(SystemParam)]
struct Dispatch<'w> {
    shown: ShownSurface<'w>,
    registry: Res<'w, Registry>,
}

fn handle_shell_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    scoped: Scoped,
    dispatch: Dispatch,
    mut commands: Commands,
) {
    let held = held.get();
    if !yields(scoped.of(input.original_event_target()), held) {
        return;
    }
    let Some(command) = bound_on(dispatch.shown.surface(), &input.input, held) else {
        return;
    };
    input.propagate(false);
    dispatch.registry.run(&mut commands, command);
}

/// The command a key runs: the one particular to the page on show where
/// it binds the key, else the first bound to it everywhere. So two pages
/// may bind one key each, and a page may take a key the shell binds.
fn bound_on(shown: Option<Page>, input: &KeyboardInput, held: KeyModifiers) -> Option<CommandId> {
    if input.state != ButtonState::Pressed {
        return None;
    }
    let bound = || {
        table::BINDINGS
            .iter()
            .filter(|(binding, _)| binding.matches(input, held))
            .map(|(_, command)| *command)
    };
    let is_own = |command: &CommandId| matches!(command.spec().scope, Scope::On(_));
    bound()
        .find(|command| is_own(command) && command.spec().scope.covers(shown))
        .or_else(|| bound().find(|command| !is_own(command)))
}

fn register(world: &mut World) {
    let systems = COMMANDS.iter().map(|spec| (spec.register)(world)).collect();
    world.insert_resource(Registry(systems));
    pickers::register(world);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn a_field_gives_up_chords_and_an_overlay_gives_up_nothing() {
        let plain = KeyModifiers::default();
        let chord = KeyModifiers::default().with_ctrl(true);
        assert!(yields(None, plain));
        assert!(!yields(Some(KeyScope::Plain), plain));
        assert!(yields(Some(KeyScope::Plain), chord));
        assert!(!yields(Some(KeyScope::All), chord));
    }

    #[test]
    fn names_are_unique_and_keys_are_unique_on_any_one_page() {
        let names: BTreeSet<&str> = COMMANDS.iter().map(|spec| spec.name).collect();
        assert_eq!(names.len(), COMMANDS.len());
        let keys: Vec<(&KeyBinding, Scope)> = COMMANDS
            .iter()
            .flat_map(|spec| spec.keys.iter().map(|key| (key, spec.scope)))
            .collect();
        // A page's row may take a key the shell binds; two rows of the
        // shell's, or two of one page's, may not share one.
        let are_apart = |a: Scope, b: Scope| match (a, b) {
            (Scope::On(x), Scope::On(y)) => x != y,
            (Scope::On(_), _) | (_, Scope::On(_)) => true,
            _ => false,
        };
        for (index, (key, scope)) in keys.iter().enumerate() {
            let twice = keys[..index]
                .iter()
                .any(|(earlier, held)| earlier == key && !are_apart(*scope, *held));
            assert!(!twice, "{} is bound twice", keys::label(key));
        }
    }

    #[test]
    fn a_pages_row_takes_a_key_the_shell_binds_while_the_page_is_shown() {
        use bevy_input::keyboard::{Key, KeyCode};
        let esc = KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        };
        let on = |page: Page| {
            let command = bound_on(Some(page), &esc, KeyModifiers::default());
            command.map(|command| command.spec().name)
        };
        assert_eq!(on(Page::Ledger), Some("ledger-plan"));
        assert_eq!(on(Page::Accounts), Some("domains"));
    }

    #[test]
    fn a_key_two_pages_bind_runs_the_command_of_the_page_on_show() {
        use bevy_input::keyboard::{Key, KeyCode};
        let w = KeyboardInput {
            key_code: KeyCode::KeyW,
            logical_key: Key::Character("w".into()),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        };
        let on = |page: Page| {
            let command = bound_on(Some(page), &w, KeyModifiers::default());
            command.map(|command| command.spec().name)
        };
        assert_eq!(on(Page::RothConversions), Some("write-ladder"));
        assert_eq!(on(Page::SsaBenefits), Some("write-claims"));
        assert_eq!(on(Page::Overview), None, "a page's key is its own");
    }

    #[test]
    fn only_the_shell_scope_acts_without_a_document() {
        assert!(Scope::Shell.covers(None));
        assert!(!Scope::Anywhere.covers(None));
        assert!(!Scope::Lists.covers(None));
        assert!(Scope::Anywhere.covers(Some(Page::Overview)));
        let scope_of = |name| {
            COMMANDS
                .iter()
                .find(|spec| spec.name == name)
                .unwrap()
                .scope
        };
        for name in [
            "open",
            "quit",
            "help",
            "palette",
            "focus-next",
            "focus-previous",
        ] {
            assert_eq!(scope_of(name), Scope::Shell, "{name}");
        }
        for name in ["save", "new", "tab-next", "go-to", "overview", "plan"] {
            assert_eq!(scope_of(name), Scope::Anywhere, "{name}");
        }
    }

    #[test]
    fn every_command_documents_itself() {
        for spec in COMMANDS.iter() {
            assert!(!spec.doc.is_empty(), "{} has no doc", spec.name);
        }
    }

    #[test]
    fn a_page_is_hinted_the_commands_that_act_on_it() {
        let words = |page: Page| -> Vec<&str> {
            page_hints(Some(page), |_| false)
                .map(|(_, word)| word)
                .collect()
        };
        let viewing = words(Page::Overview);
        assert!(viewing.contains(&"save"), "{viewing:?}");
        let composing: Vec<&str> = page_hints(None, |_| false).map(|(_, word)| word).collect();
        assert_eq!(composing, ["quit"], "nothing to save without a document");
        assert!(!viewing.contains(&"add"), "nothing to add to: {viewing:?}");
        assert!(words(Page::Accounts).contains(&"add"));
        assert!(
            !words(Page::Settings).contains(&"add"),
            "a form, not a list"
        );
        let keyed = page_hints(Some(Page::Accounts), |_| false).all(|(key, _)| !key.is_empty());
        assert!(keyed, "a hinted command names its key");
    }
}
