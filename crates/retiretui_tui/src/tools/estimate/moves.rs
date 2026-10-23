//! Moving spending from the keyboard: ⏎ on a record offers what can be
//! done with it, `i` ignores a record or a payee, and `c` moves a payee to
//! a category picked by its number, each change kept in the user's
//! patterns file and said in a toast.

use bevy_app::{App, Startup};
use bevy_ecs::prelude::{Commands, In, Query, Res, ResMut, Resource, With, World};
use bevy_input_focus::InputFocus;
use plurimus::widgets::ActiveDescendant;
use retiretui_client::estimate::{Action, Listed, To, reached, record_menu, starting_words};
use retiretui_engine::estimate::Category;

use super::edits::remember;
use super::{BehindTable, Cursor, Held, PatternsFile, ProposalsTable, RecordRow, record_at};
use crate::command::{self, Outcome};
use crate::picker::{Offered, Picker, Picking, ranked};
use crate::session::Session;

pub const IGNORE: &str = "ignore";
pub const MOVE_PAYEE: &str = "move-payee";
pub const RECORD_ACTIONS: &str = "record-actions";
pub const MOVE_CONTAINING: &str = "move-containing";
pub const IGNORE_PAYEE: &str = "ignore-payee";
pub const NAME_PAYEE: &str = "name-payee";
pub const PICK_CATEGORY: &str = "pick-category";

const NOTHING_HERE: &str = "highlight a record, or an unnamed payee's row, first";
const NO_WORDS: &str = "type the words the payees contain first";

pub fn plugin(app: &mut App) {
    app.init_resource::<Acting>();
    app.add_systems(Startup, register);
}

/// The pickers moving spending opens.
#[derive(Resource)]
pub struct MovePickers {
    categories: Picker,
    actions: Picker,
    words: Picker,
    name: Picker,
}

/// What a picker acts on: the row, and the record behind it where there
/// is one, its payee, and the words or name typed so far.
#[derive(Resource, Default)]
pub struct Acting {
    target: Option<(usize, Option<usize>)>,
    payee: String,
    words: Option<String>,
    name: String,
}

fn register(world: &mut World) {
    let pickers = MovePickers {
        categories: Picker::new(
            world,
            ("Move to", "category"),
            list_categories,
            chose_category,
        )
        .choosing_alone(),
        actions: Picker::new(world, ("Record", "action"), list_actions, chose_action),
        words: Picker::new(
            world,
            ("Payees containing", "payee"),
            list_words,
            chose_words,
        ),
        name: Picker::new(world, ("Name the payee", "name"), list_name, chose_name),
    };
    world.insert_resource(pickers);
}

/// The row and record under the keyboard: a record in the Records pane,
/// or an unnamed payee's row in the Proposals table.
type Keyboard<'w, 's> = (
    Res<'w, InputFocus>,
    Query<'w, 's, (), With<ProposalsTable>>,
    Query<'w, 's, &'static ActiveDescendant, With<BehindTable>>,
    Query<'w, 's, &'static RecordRow>,
);

fn target(
    (focus, proposals, records, rows): &Keyboard,
    cursor: Cursor,
    held: &Held,
) -> Option<(usize, Option<usize>)> {
    let holder = focus.get()?;
    if records.contains(holder) {
        return Some((cursor.0, Some(record_at(records, rows)?)));
    }
    let is_payee = matches!(held.0.row(cursor.0), Some(Listed::Payee(_)));
    (proposals.contains(holder) && is_payee).then_some((cursor.0, None))
}

/// Does `action` to `target`, keeping the change and saying it.
fn act(
    (held, memory, session): (&mut Held, &PatternsFile, &Session),
    (at, record): (usize, Option<usize>),
    action: Action,
) -> Outcome {
    match held.0.act(at, record, action) {
        Ok(change) => {
            remember(session, memory.0.as_deref(), &[change]);
            Outcome::Done
        }
        Err(reason) => Outcome::Refused(reason),
    }
}

/// The `ignore` command: ignores the record under the keyboard, or counts
/// it again behind the Ignored row, or ignores an unnamed payee's row.
pub fn ignore(
    keyboard: Keyboard,
    (cursor, memory, session): (Res<Cursor>, Res<PatternsFile>, Res<Session>),
    mut held: ResMut<Held>,
) -> Outcome {
    let Some(target) = target(&keyboard, *cursor, &held) else {
        return Outcome::Refused(NOTHING_HERE.to_owned());
    };
    let action = if target.1.is_some() {
        Action::IgnoreRecord
    } else {
        Action::IgnorePayee
    };
    act((&mut *held, &memory, &session), target, action)
}

/// The `ignore-payee` command: ignores every record of the payee under the
/// keyboard.
pub fn ignore_payee(
    keyboard: Keyboard,
    (cursor, memory, session): (Res<Cursor>, Res<PatternsFile>, Res<Session>),
    mut held: ResMut<Held>,
) -> Outcome {
    let Some(target) = target(&keyboard, *cursor, &held) else {
        return Outcome::Refused(NOTHING_HERE.to_owned());
    };
    act((&mut *held, &memory, &session), target, Action::IgnorePayee)
}

/// Opens `picker` over `target`, with `query` typed.
fn open(
    (acting, picking, held): (&mut Acting, &mut Picking, &Held),
    target: Option<(usize, Option<usize>)>,
    (picker, query): (Picker, &str),
) -> Outcome {
    let Some((at, record)) = target else {
        return Outcome::Refused(NOTHING_HERE.to_owned());
    };
    acting.target = target;
    acting.payee = held.0.payee(at, record).unwrap_or_default();
    picking.open_with(picker, query);
    Outcome::Done
}

/// The `move-payee` command: picks the category the payee under the
/// keyboard is for.
pub fn move_payee(
    (keyboard, cursor, held): (Keyboard, Res<Cursor>, Res<Held>),
    (pickers, mut acting, mut picking): (Res<MovePickers>, ResMut<Acting>, ResMut<Picking>),
) -> Outcome {
    acting.words = None;
    let target = target(&keyboard, *cursor, &held);
    open(
        (&mut acting, &mut picking, &held),
        target,
        (pickers.categories, ""),
    )
}

/// The `record-actions` command: offers what can be done with the record
/// under the keyboard.
pub fn record_actions(
    (keyboard, cursor, held): (Keyboard, Res<Cursor>, Res<Held>),
    (pickers, mut acting, mut picking): (Res<MovePickers>, ResMut<Acting>, ResMut<Picking>),
) -> Outcome {
    let target = target(&keyboard, *cursor, &held).filter(|(_, record)| record.is_some());
    open(
        (&mut acting, &mut picking, &held),
        target,
        (pickers.actions, ""),
    )
}

/// The `move-containing` command: types the words payees like the one
/// under the keyboard contain, starting from its first two.
pub fn move_containing(
    (keyboard, cursor, held): (Keyboard, Res<Cursor>, Res<Held>),
    (pickers, mut acting, mut picking): (Res<MovePickers>, ResMut<Acting>, ResMut<Picking>),
) -> Outcome {
    let target = target(&keyboard, *cursor, &held);
    let payee = target.and_then(|(at, record)| held.0.payee(at, record));
    let start = starting_words(&payee.unwrap_or_default());
    open(
        (&mut acting, &mut picking, &held),
        target,
        (pickers.words, &start),
    )
}

/// The `name-payee` command: types the name the payee under the keyboard
/// is listed under.
pub fn name_payee(
    (keyboard, cursor, held): (Keyboard, Res<Cursor>, Res<Held>),
    (pickers, mut acting, mut picking): (Res<MovePickers>, ResMut<Acting>, ResMut<Picking>),
) -> Outcome {
    let target = target(&keyboard, *cursor, &held);
    open(
        (&mut acting, &mut picking, &held),
        target,
        (pickers.name, ""),
    )
}

/// The `pick-category` command: picks the category the payees containing
/// the words just typed are for.
pub fn pick_category(
    (pickers, acting, mut picking): (Res<MovePickers>, Res<Acting>, ResMut<Picking>),
) -> Outcome {
    if acting.words.is_none() || acting.target.is_none() {
        return Outcome::Refused(NO_WORDS.to_owned());
    }
    picking.open(pickers.categories);
    Outcome::Done
}

/// The bucket as 0, then every category by its number from 1: those whose
/// number starts with the digits typed, else those whose name matches.
fn list_categories(In(query): In<String>, held: Res<Held>) -> Vec<Offered> {
    let bucket = format!("{} (bucket)", held.0.bucket_name());
    let names = std::iter::once(bucket).chain(Category::ALL.iter().map(|it| it.title()));
    let offered = names
        .enumerate()
        .map(|(number, name)| Offered::new(number, format!("{number:>2} {name}")));
    if !query.is_empty() && query.chars().all(|c| c.is_ascii_digit()) {
        return offered
            .filter(|row| row.id.to_string().starts_with(&query))
            .collect();
    }
    ranked(&query, offered)
}

fn chose_category(
    In(number): In<usize>,
    (memory, session): (Res<PatternsFile>, Res<Session>),
    mut acting: ResMut<Acting>,
    mut held: ResMut<Held>,
) {
    let to = match number.checked_sub(1) {
        None => Some(To::Bucket),
        Some(at) => Category::ALL.get(at).copied().map(To::Category),
    };
    let (Some(to), Some(target)) = (to, acting.target) else {
        return;
    };
    let action = match acting.words.take() {
        Some(words) => Action::Containing(words, to),
        None => Action::PayeeIs(to),
    };
    if let Outcome::Refused(reason) = act((&mut *held, &memory, &session), target, action) {
        crate::journal::warn(reason);
    }
}

/// The command each of a record's menu items runs, in its order.
const RECORD_MENU: [&str; 5] = [
    IGNORE,
    MOVE_PAYEE,
    MOVE_CONTAINING,
    IGNORE_PAYEE,
    NAME_PAYEE,
];

fn list_actions(In(query): In<String>, acting: Res<Acting>, held: Res<Held>) -> Vec<Offered> {
    let Some((at, _)) = acting.target else {
        return Vec::new();
    };
    let is_ignored = matches!(held.0.row(at), Some(Listed::Ignored { .. }));
    let labels = record_menu(&acting.payee, is_ignored);
    ranked(
        &query,
        labels
            .into_iter()
            .enumerate()
            .map(|(id, text)| Offered::new(id, text)),
    )
}

fn chose_action(In(id): In<usize>, mut commands: Commands) {
    if let Some(&name) = RECORD_MENU.get(id) {
        command::defer_named(&mut commands, name);
    }
}

/// The one row the words typed make: how far a rule on them reaches.
fn list_words(In(query): In<String>, mut acting: ResMut<Acting>, held: Res<Held>) -> Vec<Offered> {
    acting.words = Some(query.trim().to_owned()).filter(|words| !words.is_empty());
    let Some(words) = &acting.words else {
        return Vec::new();
    };
    let said = reached(held.0.reach(words));
    vec![Offered::new(0, said)]
}

fn chose_words(In(_): In<usize>, mut commands: Commands) {
    command::defer_named(&mut commands, PICK_CATEGORY);
}

/// The one row the name typed makes.
fn list_name(In(query): In<String>, mut acting: ResMut<Acting>) -> Vec<Offered> {
    query.trim().clone_into(&mut acting.name);
    if acting.name.is_empty() {
        return Vec::new();
    }
    vec![Offered::new(0, format!("List it as {}", acting.name))]
}

fn chose_name(
    In(_): In<usize>,
    (memory, session): (Res<PatternsFile>, Res<Session>),
    acting: Res<Acting>,
    mut held: ResMut<Held>,
) {
    let Some(target) = acting.target else {
        return;
    };
    let action = Action::Name(acting.name.clone());
    if let Outcome::Refused(reason) = act((&mut *held, &memory, &session), target, action) {
        crate::journal::warn(reason);
    }
}
