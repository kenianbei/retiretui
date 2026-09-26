//! The bottom row: the keys of whatever the keyboard is in, then the
//! page's commands, then the two keys that find everything else.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Component, Has, IntoScheduleConfigs, Local, Query, Res, With};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::InputFocus;
use plurimus::core::ratatui_core::text::{Line, Span};
use plurimus::core::{TerminalSize, UiWidget};
use plurimus::ui::ModalOpen;
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;

use super::command;
use super::focus::Ring;
use super::layout::HintRow;
use super::scope::KeyScope;
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

/// Always shown, at the row's trailing end, while the shell has the keys.
const FINDERS: [Hint; 2] = [(":", "commands"), ("?", "help")];
const HINT_GAP: usize = 2;
const EDGE: usize = 1;

fn width_of(hints: &[Hint]) -> usize {
    let said: usize = hints
        .iter()
        .map(|(key, word)| key.chars().count() + 1 + word.chars().count())
        .sum();
    said + HINT_GAP * hints.len().saturating_sub(1)
}

/// The leading hints that fit beside `trailing` in `cols`: whole hints are
/// dropped from the end, never cut, and the trailing run is never dropped.
fn fitted(mut leading: Vec<Hint>, trailing: &[Hint], cols: usize) -> Vec<Hint> {
    let reserved = width_of(trailing) + 2 * EDGE + HINT_GAP;
    while !leading.is_empty() && width_of(&leading) + reserved > cols {
        leading.pop();
    }
    leading
}

/// The widget holding the keyboard and what stands above it.
#[derive(SystemParam)]
struct Held<'w, 's> {
    focus: Res<'w, InputFocus>,
    parents: Query<'w, 's, &'static ChildOf>,
    chain: Query<
        'w,
        's,
        (
            Option<&'static Hints>,
            Has<ModalOpen>,
            Option<&'static KeyScope>,
        ),
    >,
}

impl Held<'_, '_> {
    /// What the keyboard is in, walked up from the widget holding it to
    /// the first modal ancestor, and whether it keeps every key from the
    /// shell.
    fn hints(&self) -> (Vec<Hint>, bool) {
        let mut found = Vec::new();
        let Some(held) = self.focus.get() else {
            return (found, false);
        };
        let mut widest = None;
        let mut is_below_modal = true;
        for entity in std::iter::once(held).chain(self.parents.iter_ancestors(held)) {
            let Ok((hints, is_modal, scope)) = self.chain.get(entity) else {
                continue;
            };
            widest = widest.max(scope.copied());
            if is_below_modal {
                found.extend(hints.into_iter().flat_map(|hints| hints.0));
                is_below_modal = !is_modal;
            }
        }
        (found, widest == Some(KeyScope::All))
    }
}

/// What the row last drew, so it is drawn again only when that moves.
#[derive(Default, PartialEq)]
struct Drawn {
    leading: Vec<Hint>,
    has_finders: bool,
    cols: u16,
}

/// The key that walks the page's panes, hinted after the holder's own
/// keys wherever there is more than one to walk.
const WALK: Hint = ("⇥", "pane");

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
    if ring.shown.is_changed() {
        *has_many = ring.has_many();
    }
    let (mut leading, owns_keys) = held.hints();
    if !owns_keys {
        if *has_many {
            leading.push(WALK);
        }
        leading.extend(command::page_hints(ring.shown.surface(), |name| {
            idle.is_idle(name)
        }));
    }
    let trailing: &[Hint] = if owns_keys { &[] } else { &FINDERS };
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

fn push_run(spans: &mut Vec<Span<'static>>, hints: &[Hint], theme: &Theme) {
    for (at, (key, word)) in hints.iter().enumerate() {
        if at > 0 {
            spans.push(Span::raw(" ".repeat(HINT_GAP)));
        }
        spans.push(Span::styled(*key, theme.accented()));
        spans.push(Span::styled(format!(" {word}"), theme.dimmed()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEADING: [Hint; 3] = [("⏎", "open"), ("a", "add"), ("ctrl-s", "save")];

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
}
