//! What the SSA Benefits page offers beyond its two panes: the actions ⏎
//! on a person offers, and the line under the page that says what to do
//! next and whom the keys act on.

use bevy_app::{App, Startup, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{
    Commands, In, IntoScheduleConfigs, Local, Query, Res, ResMut, Resource, With, World,
};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::InputFocus;
use plurimus::core::UiWidget;
use retiretui_client::searches::claims::{NOBODY, PersonAction, is_claimed, typed_monthly};
use retiretui_engine::optimize::ClaimSearch;
use retiretui_engine::plan::{Item, Person};

use super::super::options::OptionsTable;
use super::super::{HelpLine, show_help};
use super::people::{HeldClaims, PeopleTable, PersonCursor};
use crate::command::{self, Outcome};
use crate::edit::Draft;
use crate::nav::{self, Page, ShownSurface};
use crate::picker::{Offered, Picker, Picking, ranked};
use crate::present::compact_money;
use crate::theme::{Repainted, Theme};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, register).add_systems(
        Update,
        say_help
            .run_if(nav::shows(Page::SsaBenefits))
            .before(Repainted),
    );
}

/// The command ⏎ on a person runs.
pub const PERSON_ACTIONS: &str = "person-actions";

/// How many of [`PersonAction::ALL`], from the first, have a key of
/// their own.
const KEYED_ACTIONS: usize = 3;

/// The command `action` runs.
const fn command_of(action: PersonAction) -> &'static str {
    match action {
        PersonAction::Import => "import-statement",
        PersonAction::FillCareer => "fill-career",
        PersonAction::ComputeBenefit => "compute-benefit",
        PersonAction::Hold | PersonAction::LetVary => "hold-claim",
        PersonAction::ClearRecord => "clear-record",
        PersonAction::RemoveBenefit => "remove-benefit",
    }
}

/// The picker ⏎ on a person opens.
#[derive(Resource, Clone, Copy)]
pub struct ActionsPicker(Picker);

fn register(world: &mut World) {
    let picker = Picker::new(world, ("Actions", "action"), list_actions, run_action);
    world.insert_resource(ActionsPicker(picker));
}

/// The actions the cursor's person has a use for, each beside its key.
fn list_actions(
    In(query): In<String>,
    draft: Res<Draft>,
    cursor: Res<PersonCursor>,
    held: Res<HeldClaims>,
) -> Vec<Offered> {
    let Some(person) = cursor.person(&draft.plan) else {
        return Vec::new();
    };
    let is_held = held.0.contains(&person.id);
    let offered = PersonAction::ALL
        .into_iter()
        .enumerate()
        .filter(|(_, action)| action.is_offered(&draft.plan, person, is_held))
        .map(|(at, action)| {
            let key = command::named(command_of(action)).map_or("", command::CommandId::key_label);
            Offered::new(at, action.label()).badged(key)
        });
    ranked(&query, offered)
}

fn run_action(In(at): In<usize>, mut commands: Commands) {
    if let Some(&action) = PersonAction::ALL.get(at) {
        command::defer_named(&mut commands, command_of(action));
    }
}

/// The `person-actions` command: offers what can be done for the person
/// under the cursor.
pub fn offer_actions(
    draft: Res<Draft>,
    cursor: Res<PersonCursor>,
    picker: Res<ActionsPicker>,
    mut picking: ResMut<Picking>,
) -> Outcome {
    if let Some(refusal) = draft.refuse_if_read_only() {
        return Outcome::Refused(refusal);
    }
    if cursor.person(&draft.plan).is_none() {
        return Outcome::Refused(NOBODY.to_owned());
    }
    picking.open(picker.0);
    Outcome::Done
}

/// Which of the page's panes holds the keyboard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Place {
    People,
    Strategies,
    Elsewhere,
}

#[derive(SystemParam)]
struct Keyboard<'w, 's> {
    focus: Res<'w, InputFocus>,
    people: Query<'w, 's, (), With<PeopleTable>>,
    strategies: Query<'w, 's, (), With<OptionsTable<ClaimSearch>>>,
}

impl Keyboard<'_, '_> {
    fn place(&self) -> Place {
        match self.focus.get() {
            Some(holder) if self.people.contains(holder) => Place::People,
            Some(holder) if self.strategies.contains(holder) => Place::Strategies,
            _ => Place::Elsewhere,
        }
    }
}

fn say_help(
    state: (Res<Draft>, Res<PersonCursor>, ShownSurface),
    keyboard: Keyboard,
    theme: Res<Theme>,
    mut said: Local<String>,
    mut lines: Query<(&mut UiWidget, &HelpLine)>,
) {
    let (draft, cursor, shown) = state;
    let is_moved = draft.is_changed() || cursor.is_changed() || keyboard.focus.is_changed();
    let is_restyled = theme.is_changed();
    if !(is_moved || is_restyled || shown.is_changed()) {
        return;
    }
    let text = help_line(&draft, cursor.person(&draft.plan), keyboard.place());
    if *said == text && !is_restyled {
        return;
    }
    show_help(&mut lines, Page::SsaBenefits, &text, &theme);
    *said = text;
}

/// What to do next, for whom: a record missing comes first, then what ⏎
/// does where the keyboard is.
pub(super) fn help_line(draft: &Draft, person: Option<&Person>, place: Place) -> String {
    let plan = &draft.plan;
    let Some(person) = person else {
        return String::new();
    };
    let is_unrecorded =
        |person: &&Person| person.earnings.is_empty() && typed_monthly(plan, person).is_none();
    if let Some(missing) = plan.household.people.iter().find(is_unrecorded) {
        let id = missing.display_name();
        return format!(
            "{id} has no earnings record: ⏎ on {id} to import a statement or estimate one from their salary."
        );
    }
    let keys = format!("{} act on {}.", keys_phrase(), person.display_name());
    if place == Place::Strategies {
        return format!("⏎ takes the highlighted option into the plan, after asking. {keys}");
    }
    if let Some(monthly) = typed_monthly(plan, person) {
        return format!(
            "{}'s benefit is typed at {} a month: ⏎ to compute it from their record instead.",
            person.display_name(),
            compact_money(monthly)
        );
    }
    if !is_claimed(plan, person) {
        return format!(
            "⇥ to the claim options and ⏎ on one to set when each benefit starts. {keys}"
        );
    }
    format!("⏎ on a person for what can be done for them. {keys}")
}

/// The keys that act on a person, as the command table binds them: `e, c
/// and k`.
fn keys_phrase() -> String {
    let keys: Vec<&str> = PersonAction::ALL[..KEYED_ACTIONS]
        .iter()
        .filter_map(|&action| command::named(command_of(action)))
        .map(command::CommandId::key_label)
        .filter(|key| !key.is_empty())
        .collect();
    match keys.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        Some((last, _)) => (*last).to_owned(),
        None => String::new(),
    }
}
