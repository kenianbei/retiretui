//! The keys in force: which command each runs, and how each command's keys
//! are shown.

use bevy_ecs::prelude::Resource;
use bevy_input::ButtonState;
use bevy_input::keyboard::{Key, KeyboardInput};
use plurimus::term::KeyModifiers;
use plurimus::ui::KeyBinding;

use super::{COMMANDS, CommandId, PALETTE, Scope, all, keys, named};
use crate::nav::Page;

const TABLE: &str = "[tui.keys]";

const PALETTE_KEEPS_A_KEY: &str = "the palette keeps a key, so that every command stays in reach";

/// Whether one key may run a command in each scope: a page's row may take
/// a key the shell binds or another page does, and two rows of the
/// shell's, or two of one page's, may not share one.
fn are_apart(a: Scope, b: Scope) -> bool {
    match (a, b) {
        (Scope::On(x), Scope::On(y)) => x != y,
        (Scope::On(_), _) | (_, Scope::On(_)) => true,
        _ => false,
    }
}

/// What applying a user's entries came to, for the journal.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Remarks {
    /// The entries left out, each with why.
    pub refused: Vec<String>,
    /// The keys an entry took from a command the user did not name.
    pub taken: Vec<String>,
}

/// The keys an entry states: one, or a list of them.
fn stated(value: &toml::Value) -> Result<Vec<KeyBinding>, String> {
    let unread = || "a key or a list of keys is wanted".to_owned();
    match value {
        toml::Value::String(key) => Ok(vec![keys::parse(key)?]),
        toml::Value::Array(keys) => keys
            .iter()
            .map(|key| keys::parse(key.as_str().ok_or_else(unread)?))
            .collect(),
        _ => Err(unread()),
    }
}

/// The table's keys while a user's entries are applied over them.
struct Rebinding {
    /// Indexed as [`COMMANDS`] is.
    keys: Vec<Vec<KeyBinding>>,
    /// Whether each command's keys are kept from every other: the ones the
    /// user stated, and the palette's.
    is_kept: Vec<bool>,
}

impl Rebinding {
    /// The command `key` is kept for, where `command` may not share it.
    fn keeper(&self, command: CommandId, key: &KeyBinding) -> Option<CommandId> {
        all().find(|other| {
            *other != command
                && self.is_kept[other.0]
                && !are_apart(command.spec().scope, other.spec().scope)
                && self.keys[other.0].contains(key)
        })
    }

    fn admit(&mut self, command: CommandId, keys: Vec<KeyBinding>) -> Result<(), String> {
        let kept = keys
            .iter()
            .find_map(|key| Some((key, self.keeper(command, key)?)));
        if let Some((key, keeper)) = kept {
            let name = keeper.spec().name;
            return Err(format!("{} is kept for {name}", keys::label(key)));
        }
        self.keys[command.0] = keys;
        self.is_kept[command.0] = true;
        Ok(())
    }

    /// Takes from each command the user did not name the keys they gave
    /// another, and says each.
    fn take(&mut self) -> Vec<String> {
        let mut said = Vec::new();
        for command in all().filter(|command| !self.is_kept[command.0]) {
            for key in std::mem::take(&mut self.keys[command.0]) {
                let Some(keeper) = self.keeper(command, &key) else {
                    self.keys[command.0].push(key);
                    continue;
                };
                let (winner, loser) = (keeper.spec().name, command.spec().name);
                let key = keys::label(&key);
                said.push(format!("{TABLE} {key} now runs {winner}, not {loser}"));
            }
        }
        said
    }
}

/// Every keystroke bound to a command, and how each is shown.
#[derive(Resource)]
pub struct Keymap {
    /// In table order, which is the order a key is matched in.
    bindings: Vec<(KeyBinding, CommandId)>,
    /// Each command's keys as they are shown, indexed as [`COMMANDS`] is.
    labels: Vec<Vec<&'static str>>,
}

impl Keymap {
    #[cfg(test)]
    pub fn defaults() -> Self {
        Self::with(&toml::Table::new()).0
    }

    /// The command table's keys under the entries of `user`. One that does
    /// not read, or states a key kept for another command, is left out; a
    /// key a command holds unstated is taken from it.
    pub fn with(user: &toml::Table) -> (Self, Remarks) {
        let palette = named(PALETTE);
        let mut rebinding = Rebinding {
            keys: COMMANDS.iter().map(|spec| spec.keys.clone()).collect(),
            is_kept: all().map(|command| Some(command) == palette).collect(),
        };
        let unknown = user.keys().filter(|name| named(name).is_none());
        let mut refused: Vec<String> = unknown
            .map(|name| format!("{TABLE} {name}: no command has that name"))
            .collect();
        // The palette's entry is judged first, so that the key it gives up
        // is free for an entry the table lists ahead of it.
        let rest = all().filter(|command| Some(*command) != palette);
        for command in palette.into_iter().chain(rest) {
            let name = command.spec().name;
            let Some(value) = user.get(name) else {
                continue;
            };
            let admitted = stated(value).and_then(|keys| {
                if keys.is_empty() && Some(command) == palette {
                    return Err(PALETTE_KEEPS_A_KEY.to_owned());
                }
                rebinding.admit(command, keys)
            });
            if let Err(why) = admitted {
                refused.push(format!("{TABLE} {name}: {why}"));
            }
        }
        let taken = rebinding.take();
        (Self::of(rebinding.keys), Remarks { refused, taken })
    }

    /// `keys` indexed as [`COMMANDS`] is. Each label is leaked, so that a
    /// hint stays a `&'static str`; a session builds one keymap.
    fn of(keys: Vec<Vec<KeyBinding>>) -> Self {
        let shown = |keys: &Vec<KeyBinding>| -> Vec<&'static str> {
            keys.iter().map(|key| &*keys::label(key).leak()).collect()
        };
        let labels = keys.iter().map(shown).collect();
        let bindings = all()
            .zip(keys)
            .flat_map(|(command, keys)| keys.into_iter().map(move |key| (key, command)))
            .collect();
        Self { bindings, labels }
    }

    /// How `command`'s first key is shown; empty where it has none.
    pub fn label(&self, command: CommandId) -> &'static str {
        self.labels[command.0].first().copied().unwrap_or_default()
    }

    /// How the `preferred` key of the command named `name` is shown, or its
    /// first where it has fewer; empty where it has none.
    pub fn label_named(&self, name: &str, preferred: usize) -> &'static str {
        let Some(command) = named(name) else {
            return "";
        };
        let labels = &self.labels[command.0];
        let label = labels.get(preferred).or_else(|| labels.first());
        label.copied().unwrap_or_default()
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
mod tests;
