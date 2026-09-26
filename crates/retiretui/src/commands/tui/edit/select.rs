//! A value picked from a closed set rather than typed: a button showing
//! the value, and under it the menu of what it may be. Both are plurimus's
//! own menu widgets, so the theme dresses them with no help from here.
//!
//! Enter is left alone, so it applies the item: space and a press open the
//! menu, and left and right step through the set without opening it.

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::{DetectChangesMut, Mut};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{
    Added, Changed, Commands, Component, Entity, EntityEvent, IntoScheduleConfigs, On, Query,
    ResMut, With,
};
use bevy_input::keyboard::{Key, KeyboardInput};
use bevy_input_focus::{FocusCause, FocusedInput, InputFocus};
use plurimus::core::ratatui_core::text::Line;
use plurimus::term::bevy_compat::HeldModifiers;
use plurimus::ui::{KeyBinding, UiLabel, first_bound};
use plurimus::widgets::{Activate, MenuOpen, MenuPopup, menu_button, menu_item, menu_popup};
use retiretui_engine::plan::Plan;
use toml::Value;

use super::build::FormField;
use super::domain::FieldKind;
use super::field::space;
use super::offers::{Offer, Vocabulary, ref_offers};
use super::search;
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout::placed;
use crate::commands::tui::scope::KeyScope;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, search::register);
    app.add_systems(
        Update,
        (sync_selects, open_on_the_chosen).in_set(super::EditSystems::Place),
    );
}

const CHEVRON: &str = "▾";
const STEP_KEYS: &[(KeyBinding, isize)] = &[
    (KeyBinding::new(Key::ArrowLeft), -1),
    (KeyBinding::new(Key::ArrowRight), 1),
];

#[derive(Component)]
pub struct Select {
    kind: FieldKind,
    options: Vec<Offer>,
    /// `None` while the field is empty, which a `None` in the item is.
    chosen: Option<usize>,
    /// What the empty state reads as, on the button and in the menu.
    blank: &'static str,
    /// Whether the menu lists `options` as they now are.
    is_listed: bool,
    /// Whether the field may not be emptied once it holds a value.
    is_required: bool,
    /// Whether the arrows pass the empty state by, leaving it to the menu.
    skips_blank: bool,
}

/// The option a menu row stands for; `None` is the row that empties the
/// field.
#[derive(Component, Clone, Copy)]
struct Choice(Option<usize>);

#[derive(EntityEvent)]
pub struct PickChanged {
    pub entity: Entity,
    pub value: Option<Value>,
}

impl Select {
    pub fn new(kind: FieldKind, blank: &'static str) -> Self {
        let options = match kind {
            FieldKind::Choice(vocabulary) | FieldKind::Order(vocabulary, _) => vocabulary.offers(),
            _ => Vec::new(),
        };
        Self {
            kind,
            options,
            chosen: None,
            blank,
            is_listed: false,
            is_required: false,
            skips_blank: false,
        }
    }

    /// The arrows step through the options alone: emptying a field one key
    /// press past its last option is too easily done where that removes a
    /// value with parts of its own.
    pub fn skipping_blank(self) -> Self {
        Self {
            skips_blank: true,
            ..self
        }
    }

    pub fn required(self, is_required: bool) -> Self {
        Self {
            is_required,
            ..self
        }
    }

    /// Shows `value` among `options`, or among the options held where none
    /// are given. A form is refilled far more often than what it shows
    /// moves, and a select marked changed is relabelled, may be relisted,
    /// and re-places the trigger it is the kind of - so it is marked only
    /// where something did move.
    pub fn fill(select: &mut Mut<Self>, options: Option<Vec<Offer>>, value: Option<&Value>) {
        let held = select.bypass_change_detection();
        let before = (held.chosen, held.is_listed);
        if let Some(options) = options {
            held.set_options(options);
        }
        held.show(value);
        if before != (held.chosen, held.is_listed) {
            select.set_changed();
        }
    }

    /// The ids of the plan the field refers to, where it is a reference.
    pub fn referred(&self, plan: &Plan) -> Option<Vec<Offer>> {
        match self.kind {
            FieldKind::Ref(source) => Some(ref_offers(plan, source)),
            _ => None,
        }
    }

    fn set_options(&mut self, options: Vec<Offer>) {
        if self.options == options {
            return;
        }
        let chosen = self.chosen.and_then(|at| self.options.get(at).cloned());
        self.options = options;
        self.chosen = chosen.and_then(|chosen| self.position(&chosen.value));
        self.is_listed = false;
    }

    /// The value the select stands for; `None` clears the field.
    pub fn value(&self) -> Option<Value> {
        Some(Value::String(self.options.get(self.chosen?)?.value.clone()))
    }

    /// Shows `value`. One the options do not hold is kept as an option
    /// rather than dropped, since the file stated it.
    fn show(&mut self, value: Option<&Value>) {
        let wanted = value.map(|value| match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        });
        self.chosen = wanted.map(|wanted| {
            self.position(&wanted).unwrap_or_else(|| {
                self.options.push(Offer::spelt(wanted));
                self.is_listed = false;
                self.options.len() - 1
            })
        });
    }

    fn position(&self, value: &str) -> Option<usize> {
        self.options.iter().position(|offer| offer.value == value)
    }

    /// Steps through the options, and through the empty state a field
    /// that is not required can be left in.
    fn step(&mut self, step: isize) {
        if self.options.is_empty() {
            return;
        }
        let count = self.options.len() as isize;
        let stops = count + isize::from(!self.is_required && !self.skips_blank);
        let next = match self.chosen {
            Some(chosen) => (chosen as isize + step).rem_euclid(stops),
            None if step > 0 => 0,
            None => count - 1,
        };
        self.chosen = (next < count).then_some(next as usize);
    }

    /// The vocabulary too long for a menu, and searched instead, if the
    /// select's is one.
    pub(super) fn searched(&self) -> Option<Vocabulary> {
        match self.kind {
            FieldKind::Choice(long @ (Vocabulary::Country | Vocabulary::UsState)) => Some(long),
            _ => None,
        }
    }

    /// What the select offers, and the word for holding none of it where
    /// it may.
    pub(super) fn offered(&self) -> (&[Offer], Option<&'static str>) {
        (&self.options, (!self.is_required).then_some(self.blank))
    }

    /// Holds the option at `chosen`, or none; the value it then stands for.
    pub(super) fn choose(&mut self, chosen: Option<usize>) -> Option<Value> {
        self.chosen = chosen.filter(|&chosen| chosen < self.options.len());
        self.value()
    }

    fn shown(&self) -> &str {
        let chosen = self.chosen.and_then(|chosen| self.options.get(chosen));
        chosen.map_or(self.blank, |offer| offer.label.as_str())
    }
}

pub fn spawn_select(commands: &mut Commands, select: Select, field: FormField) -> Entity {
    let is_searched = select.searched().is_some();
    let select = commands
        .spawn((menu_button(""), space(), select, field, placed()))
        .observe(handle_step_key)
        .id();
    if is_searched {
        commands.entity(select).observe(search::handle_search);
        return select;
    }
    commands.spawn((
        menu_popup(select),
        KeyScope::All,
        Hints(&[("↑↓", "move"), ("⏎", "pick"), ("esc", "close")]),
        ChildOf(select),
    ));
    select
}

pub fn sync_selects(
    mut selects: Query<(&mut Select, &mut UiLabel, Option<&Children>), Changed<Select>>,
    popups: Query<Option<&Children>, With<MenuPopup>>,
    mut commands: Commands,
) {
    for (mut select, mut label, children) in &mut selects {
        label.0 = Line::from(format!("{} {CHEVRON}", select.shown())).left_aligned();
        if select.is_listed {
            continue;
        }
        let mut listed = children.into_iter().flatten().copied();
        let Some(popup) = listed.find(|&child| popups.contains(child)) else {
            continue;
        };
        select.bypass_change_detection().is_listed = true;
        for &row in popups.get(popup).ok().flatten().into_iter().flatten() {
            commands.entity(row).despawn();
        }
        let (offers, blank) = select.offered();
        let listed = offers.iter().enumerate();
        let rows = listed.map(|(at, option)| (Choice(Some(at)), option.label.as_str()));
        let emptying = blank.map(|blank| (Choice(None), blank));
        // What only the menu empties - a trigger's kind - is empty first,
        // where what it stands for comes before any when.
        let first = emptying.filter(|_| select.skips_blank);
        let last = emptying.filter(|_| !select.skips_blank);
        for (choice, text) in first.into_iter().chain(rows).chain(last) {
            commands
                .spawn((menu_item(text.to_owned()), choice, ChildOf(popup)))
                .observe(handle_choice);
        }
    }
}

/// A menu opens on the row of the value the field holds.
fn open_on_the_chosen(
    opened: Query<(&Children, &ChildOf), Added<MenuOpen>>,
    selects: Query<&Select>,
    rows: Query<&Choice>,
    mut focus: ResMut<InputFocus>,
) {
    for (children, select) in &opened {
        let Ok(select) = selects.get(select.parent()) else {
            continue;
        };
        let held = children
            .iter()
            .copied()
            .find(|&row| rows.get(row).is_ok_and(|choice| choice.0 == select.chosen));
        if let Some(held) = held {
            focus.set(held, FocusCause::Navigated);
        }
    }
}

fn handle_choice(
    chosen: On<Activate>,
    rows: Query<(&Choice, &ChildOf)>,
    popups: Query<&ChildOf, With<MenuPopup>>,
    mut selects: Query<&mut Select>,
    mut commands: Commands,
) {
    let Ok((choice, popup)) = rows.get(chosen.entity) else {
        return;
    };
    let Ok(entity) = popups.get(popup.parent()).map(ChildOf::parent) else {
        return;
    };
    let Ok(mut select) = selects.get_mut(entity) else {
        return;
    };
    let value = select.choose(choice.0);
    commands.trigger(PickChanged { entity, value });
}

/// Left and right step a closed select.
fn handle_step_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    mut selects: Query<&mut Select>,
    mut commands: Commands,
) {
    let entity = input.focused_entity;
    let Ok(mut select) = selects.get_mut(entity) else {
        return;
    };
    let Some(step) = first_bound(STEP_KEYS, &input.input, held.get()) else {
        return;
    };
    input.propagate(false);
    select.step(step);
    let value = select.value();
    commands.trigger(PickChanged { entity, value });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tui::edit::domain::{BLANK, FieldSpec};
    use crate::commands::tui::edit::offers::Vocabulary;

    fn filing() -> Select {
        Select::new(FieldKind::Choice(Vocabulary::FilingStatus), BLANK)
    }

    #[test]
    fn a_choice_steps_through_its_options_and_can_be_cleared() {
        let mut choice = filing();
        choice.show(Some(&Value::String("single".to_owned())));
        assert_eq!(choice.shown(), "Single", "the label, not the word stored");
        choice.step(1);
        let joint = Some(Value::String("married-joint".to_owned()));
        assert_eq!(choice.value(), joint);
        choice.step(1);
        assert_eq!(choice.value(), None, "one stop past the options is empty");
        assert_eq!(choice.shown(), BLANK);
        choice.step(-1);
        assert_eq!(choice.value(), joint);
    }

    #[test]
    fn a_required_choice_never_steps_to_empty_and_lists_no_row_that_empties_it() {
        let mut choice = filing().required(true);
        choice.step(1);
        let first = choice.value();
        assert!(first.is_some(), "an empty one steps onto its first option");
        for _ in 0..choice.options.len() {
            choice.step(1);
            assert!(choice.value().is_some());
        }
        assert_eq!(choice.value(), first, "and comes back round to it");
        assert_eq!(emptying_rows(true), 0);
        assert_eq!(emptying_rows(false), 1, "one that is not still has both");
    }

    /// How many rows of a select's menu empty the field.
    fn emptying_rows(is_required: bool) -> usize {
        let rows = menu_rows(filing().required(is_required));
        rows.iter().filter(|row| row.is_none()).count()
    }

    /// The options `select`'s menu lists, in order; `None` empties it.
    fn menu_rows(select: Select) -> Vec<Option<usize>> {
        use bevy_ecs::prelude::World;
        use bevy_ecs::system::RunSystemOnce;

        let mut world = World::new();
        let field = FormField {
            spec: FieldSpec::choice("filing", "", Vocabulary::FilingStatus),
        };
        spawn_select(&mut world.commands(), select, field);
        world.flush();
        world.run_system_once(sync_selects).unwrap();
        let mut popups = world.query_filtered::<&Children, With<MenuPopup>>();
        let rows = popups.single(&world).unwrap();
        rows.iter()
            .map(|&row| world.get::<Choice>(row).unwrap().0)
            .collect()
    }

    #[test]
    fn only_a_menu_that_alone_empties_its_field_lists_empty_first() {
        let rows = menu_rows(filing());
        assert_eq!(rows.last(), Some(&None), "{rows:?}");
        let rows = menu_rows(filing().skipping_blank());
        assert_eq!(rows.first(), Some(&None), "{rows:?}");
    }

    #[test]
    fn refilling_with_what_is_already_shown_marks_nothing_changed() {
        use bevy_ecs::change_detection::DetectChanges;
        use bevy_ecs::prelude::World;

        let mut world = World::new();
        let entity = world.spawn(filing()).id();
        let single = Value::String("single".to_owned());
        let is_marked = |world: &mut World, value: &Value| {
            world.clear_trackers();
            world.increment_change_tick();
            let mut select = world.get_mut::<Select>(entity).unwrap();
            Select::fill(&mut select, None, Some(value));
            select.is_changed()
        };
        assert!(is_marked(&mut world, &single), "a value newly shown");
        world.get_mut::<Select>(entity).unwrap().is_listed = true;
        assert!(!is_marked(&mut world, &single), "the same value again");
        let joint = Value::String("married-joint".to_owned());
        assert!(is_marked(&mut world, &joint));
    }

    #[test]
    fn a_value_the_options_do_not_hold_is_kept_rather_than_dropped() {
        let mut choice = filing();
        choice.is_listed = true;
        choice.show(Some(&Value::String("legacy".to_owned())));
        assert_eq!(choice.value(), Some(Value::String("legacy".to_owned())));
        assert!(!choice.is_listed, "and the menu is owed the new row");
    }
}
