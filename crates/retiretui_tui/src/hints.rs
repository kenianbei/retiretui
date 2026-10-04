//! The bottom row: the keys of whatever the keyboard is in, then the
//! page's commands, then the two keys that find everything else.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Component, FromWorld, Has, IntoScheduleConfigs, Local, Query, Res, With, World,
};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::InputFocus;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::core::{TerminalSize, UiWidget};
use plurimus::ui::ModalOpen;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;

use super::command::{self, Keymap};
use super::focus::Ring;
use super::layout::HintRow;
use super::scope::{KeyScope, Scoped};
use super::theme::{Repainted, Theme};
use super::tools::Idle;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, refresh_hints.in_set(Repainted));
}

/// A key and the word for what it does.
pub type Hint = (&'static str, &'static str);

/// The keys a widget, or everything inside a container, answers to.
#[derive(Component, Clone, Copy, Debug)]
pub struct Hints(pub &'static [Hint]);

/// A word beside the keys of the commands it is for, as the keymap binds
/// them.
#[derive(Clone, Copy, Debug)]
pub struct CommandHint {
    /// The commands, by name: one, or the two that step back and forward,
    /// whose keys are run together.
    pub commands: &'static [&'static str],
    /// Which of each command's keys is named, counted from its first,
    /// where it has that many.
    pub key: usize,
    pub word: &'static str,
}

/// The command keys a widget, or everything inside a container, answers
/// to, said after its [`Hints`].
#[derive(Component, Clone, Copy, Debug)]
pub struct CommandHints(pub &'static [CommandHint]);

/// A hint as the row draws it: its word beside one key, or beside two run
/// together, the second empty where there is one.
type Shown = ([&'static str; 2], &'static str);

const fn shown((key, word): Hint) -> Shown {
    ([key, ""], word)
}

impl CommandHint {
    /// The hint under `keymap`; none where no command of it has a key.
    fn shown(&self, keymap: &Keymap) -> Option<Shown> {
        // ponytail: a scan of the table by name for each command, every
        // frame; resolve once, on `Add`, if a widget hints more than a pair.
        let labels = self.commands.iter();
        let mut keys = labels
            .map(|name| keymap.label_named(name, self.key))
            .filter(|key| !key.is_empty());
        Some(([keys.next()?, keys.next().unwrap_or_default()], self.word))
    }
}

/// The shell's own keys as the keymap binds them, which a session does not
/// change.
struct ShellKeys {
    /// The key that walks the page's panes, hinted after the holder's own
    /// keys wherever there is more than one to walk.
    walk: Option<Shown>,
    /// Always shown, at the row's trailing end, while the shell has the
    /// keys.
    finders: Vec<Shown>,
}

impl FromWorld for ShellKeys {
    fn from_world(world: &mut World) -> Self {
        let keymap = world.resource::<Keymap>();
        let hint = |name: &str, word: &'static str| {
            let key = keymap.label_named(name, 0);
            (!key.is_empty()).then_some(shown((key, word)))
        };
        let finders = [
            hint(command::PALETTE, "commands"),
            hint(command::HELP, "help"),
        ];
        Self {
            walk: hint(command::FOCUS_NEXT, "pane"),
            finders: finders.into_iter().flatten().collect(),
        }
    }
}

const HINT_GAP: usize = 2;
const EDGE: usize = 1;

fn width_of(hints: &[Shown]) -> usize {
    let said: usize = hints
        .iter()
        .map(|(keys, word)| {
            let keys: usize = keys.iter().map(|key| key.chars().count()).sum();
            keys + 1 + word.chars().count()
        })
        .sum();
    said + HINT_GAP * hints.len().saturating_sub(1)
}

/// The leading hints that fit beside `trailing` in `cols`: whole hints are
/// dropped from the end, never cut, and the trailing run is never dropped.
fn fitted(mut leading: Vec<Shown>, trailing: &[Shown], cols: usize) -> Vec<Shown> {
    let reserved = width_of(trailing) + 2 * EDGE + HINT_GAP;
    while !leading.is_empty() && width_of(&leading) + reserved > cols {
        leading.pop();
    }
    leading
}

/// The widget holding the keyboard and what stands above it, and the
/// keys the shell answers to where that widget leaves them to it.
#[derive(SystemParam)]
struct Held<'w, 's> {
    focus: Res<'w, InputFocus>,
    parents: Query<'w, 's, &'static ChildOf>,
    hinted: Query<
        'w,
        's,
        (
            Option<&'static Hints>,
            Option<&'static CommandHints>,
            Has<ModalOpen>,
        ),
    >,
    scoped: Scoped<'w, 's>,
    keymap: Res<'w, Keymap>,
    own: Local<'s, ShellKeys>,
}

impl Held<'_, '_> {
    /// What the keyboard is in, walked up from the widget holding it to
    /// the first modal ancestor, and whether it keeps every key from the
    /// shell.
    fn hints(&self) -> (Vec<Shown>, bool) {
        let mut found = Vec::new();
        let Some(held) = self.focus.get() else {
            return (found, false);
        };
        let owns_keys = self.scoped.of(held) == Some(KeyScope::All);
        for entity in std::iter::once(held).chain(self.parents.iter_ancestors(held)) {
            let Ok((hints, commands, is_modal)) = self.hinted.get(entity) else {
                continue;
            };
            let typed = hints.into_iter().flat_map(|hints| hints.0);
            found.extend(typed.copied().map(shown));
            let bound = commands.into_iter().flat_map(|hints| hints.0);
            found.extend(bound.filter_map(|hint| hint.shown(&self.keymap)));
            if is_modal {
                break;
            }
        }
        (found, owns_keys)
    }
}

/// What the row last drew, so it is drawn again only when that moves.
#[derive(Default, PartialEq)]
struct Drawn {
    leading: Vec<Shown>,
    has_finders: bool,
    cols: u16,
}

/// Worked out afresh each frame rather than when the keyboard moves: an
/// overlay is given the keyboard on the frame it is queued, before it has
/// the ancestors its hints are read from. The walk is a few entities. The
/// page's panes are counted only as the page changes.
fn refresh_hints(
    held: Held,
    shell: (Ring, Idle, Res<TerminalSize>, Res<Theme>),
    mut rows: Query<&mut UiWidget, With<HintRow>>,
    mut drawn: Local<Drawn>,
    mut has_many: Local<bool>,
) {
    let (ring, idle, size, theme) = shell;
    let Held { keymap, own, .. } = &held;
    if ring.shown.is_changed() {
        *has_many = ring.has_many();
    }
    let (mut leading, owns_keys) = held.hints();
    if !owns_keys {
        leading.extend(own.walk.filter(|_| *has_many));
        let shown_page = ring.shown.surface();
        let page = command::page_hints(keymap, shown_page, |name| idle.is_idle(name));
        leading.extend(page.map(shown));
    }
    let trailing: &[Shown] = if owns_keys { &[] } else { &own.finders };
    let cols = usize::from(size.cols);
    let wanted = Drawn {
        leading: fitted(leading, trailing, cols),
        has_finders: !owns_keys,
        cols: size.cols,
    };
    if *drawn == wanted && !theme.is_changed() {
        return;
    }
    let gap = cols.saturating_sub(width_of(&wanted.leading) + width_of(trailing) + 2 * EDGE);
    let mut spans = vec![Span::raw(" ".repeat(EDGE))];
    push_run(&mut spans, &wanted.leading, &theme);
    spans.push(Span::raw(" ".repeat(gap)));
    push_run(&mut spans, trailing, &theme);
    for mut widget in &mut rows {
        *widget = UiWidget::new(Paragraph::new(Line::from(spans.clone())));
    }
    *drawn = wanted;
}

fn push_run(spans: &mut Vec<Span<'static>>, hints: &[Shown], theme: &Theme) {
    for (at, (keys, word)) in hints.iter().enumerate() {
        if at > 0 {
            spans.push(Span::raw(" ".repeat(HINT_GAP)));
        }
        spans.extend(keys.map(|key| Span::styled(key, theme.accented())));
        spans.push(Span::styled(format!(" {word}"), theme.dimmed()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEADING: [Shown; 3] = [
        shown(("⏎", "open")),
        (["←", "→"], "year"),
        shown(("ctrl-s", "save")),
    ];
    const FINDERS: [Shown; 2] = [shown((":", "commands")), shown(("?", "help"))];

    #[test]
    fn hints_that_fit_are_all_kept() {
        assert_eq!(fitted(LEADING.to_vec(), &FINDERS, 80), LEADING);
    }

    #[test]
    fn whole_hints_are_dropped_from_the_end_and_the_finders_never() {
        let needed = width_of(&LEADING[..2]) + width_of(&FINDERS) + 2 * EDGE + HINT_GAP;
        assert_eq!(fitted(LEADING.to_vec(), &FINDERS, needed), LEADING[..2]);
        assert_eq!(fitted(LEADING.to_vec(), &FINDERS, needed - 1), LEADING[..1]);
        assert!(fitted(LEADING.to_vec(), &FINDERS, 0).is_empty());
    }

    #[test]
    fn two_keys_run_together_take_the_cells_of_both() {
        assert_eq!(width_of(&LEADING[1..2]), "←→ year".chars().count());
    }
}
