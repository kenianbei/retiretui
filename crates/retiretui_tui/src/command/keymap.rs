//! The keys in force: which command each runs, and how each command's key
//! is shown.

use bevy_ecs::prelude::Resource;
use bevy_input::ButtonState;
use bevy_input::keyboard::{Key, KeyboardInput};
use plurimus::term::KeyModifiers;
use plurimus::ui::KeyBinding;

use super::{COMMANDS, CommandId, Scope, all, keys};
use crate::nav::Page;

/// Every keystroke bound to a command, and each command's first.
#[derive(Resource)]
pub struct Keymap {
    /// In table order, which is the order a key is matched in.
    bindings: Vec<(KeyBinding, CommandId)>,
    /// Indexed as [`COMMANDS`] is; empty for a command bound to no key.
    labels: Vec<&'static str>,
}

impl Keymap {
    /// The keys the command table states.
    pub fn defaults() -> Self {
        Self::of(COMMANDS.iter().map(|spec| spec.keys.clone()).collect())
    }

    /// `keys` indexed as [`COMMANDS`] is. Each label is leaked, so that a
    /// hint stays a `&'static str`; a session builds one keymap.
    fn of(keys: Vec<Vec<KeyBinding>>) -> Self {
        let labels = keys
            .iter()
            .map(|keys| keys.first().map_or("", |key| keys::label(key).leak()))
            .collect();
        let bindings = all()
            .zip(keys)
            .flat_map(|(command, keys)| keys.into_iter().map(move |key| (key, command)))
            .collect();
        Self { bindings, labels }
    }

    /// How `command`'s first key is shown; empty where it has none.
    pub fn label(&self, command: CommandId) -> &'static str {
        self.labels[command.0]
    }

    /// The command a key runs: the one particular to the page on show where
    /// it binds the key, else the first bound to it everywhere. So two pages
    /// may bind one key each, and a page may take a key the shell binds.
    pub fn bound_on(
        &self,
        shown: Option<Page>,
        input: &KeyboardInput,
        held: KeyModifiers,
    ) -> Option<CommandId> {
        if input.state != ButtonState::Pressed {
            return None;
        }
        let bound = || {
            self.bindings
                .iter()
                .filter(|(binding, _)| binding.matches(input, held))
                .map(|(_, command)| *command)
        };
        let is_own = |command: &CommandId| matches!(command.spec().scope, Scope::On(_));
        bound()
            .find(|command| is_own(command) && command.spec().scope.covers(shown))
            .or_else(|| bound().find(|command| !is_own(command)))
    }

    /// The plain arrow keys a command particular to `page` binds, which the
    /// keyboard then cannot also walk the page's panes by.
    pub fn arrows_bound_on(&self, page: Page) -> impl Iterator<Item = &Key> {
        let is_plain = |binding: &KeyBinding| {
            let held = binding.modifiers;
            !(held.ctrl || held.alt || held.shift)
        };
        self.bindings
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
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bevy_ecs::prelude::Entity;
    use bevy_input::keyboard::KeyCode;

    use super::*;

    fn pressed(key_code: KeyCode, logical_key: Key) -> KeyboardInput {
        KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
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
        let keymap = Keymap::defaults();
        let esc = pressed(KeyCode::Escape, Key::Escape);
        let on = |page: Page| {
            let command = keymap.bound_on(Some(page), &esc, KeyModifiers::default());
            command.map(|command| command.spec().name)
        };
        assert_eq!(on(Page::Ledger), Some("ledger-plan"));
        assert_eq!(on(Page::Accounts), Some("domains"));
    }

    #[test]
    fn a_key_two_pages_bind_runs_the_command_of_the_page_on_show() {
        let keymap = Keymap::defaults();
        let w = pressed(KeyCode::KeyW, Key::Character("w".into()));
        let on = |page: Page| {
            let command = keymap.bound_on(Some(page), &w, KeyModifiers::default());
            command.map(|command| command.spec().name)
        };
        assert_eq!(on(Page::RothConversions), Some("write-ladder"));
        assert_eq!(on(Page::SsaBenefits), Some("write-claims"));
        assert_eq!(on(Page::Overview), None, "a page's key is its own");
    }
}
