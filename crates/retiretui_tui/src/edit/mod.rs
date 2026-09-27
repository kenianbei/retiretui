//! Editing: the draft plan every edit lands in, and the screens that edit
//! it.

mod accounts;
mod applies;
mod build;
mod cells;
mod changes;
mod codec;
mod commands;
mod config;
mod contributions;
mod details;
mod domain;
mod draft;
mod editing;
mod expenses;
mod field;
mod flows;
mod form;
mod group;
mod household;
mod income;
mod market;
mod offers;
mod screen;
mod scroll;
mod search;
mod select;
mod sort;
mod table;
#[cfg(test)]
pub(crate) mod tests;
mod trigger;
mod widths;

use std::cmp::Reverse;

use bevy_app::{App, Startup, Update};
use bevy_ecs::prelude::{Commands, Entity, IntoScheduleConfigs, Query, With, World};
use retiretui_engine::plan::Plan;
use toml::{Table, Value};

use super::hints::Hints;
use super::layout::{self, Body};
use super::nav::Page;

#[cfg(test)]
pub use build::EditForm;
pub use build::{FormButton, SHORTEST_FORM_ROWS, help_fits};
pub use changes::change_words;
pub use codec::from_table;
pub use commands::{Importing, add, delete, import_earnings, record_statement};
use domain::FieldKind;
pub use domain::{FieldSpec, Ops, ToolAnswers};
pub use draft::{Draft, DraftEditor, redo, save, undo, write_draft};
pub use editing::{EditSession, Slot, open_item};
pub use offers::{RefSource, Vocabulary, ref_offers};
pub use sort::sort;
#[cfg(test)]
pub use table::DomainTable;
pub use table::{Row, Turn, table_bundle};

use super::session::Projected;
use super::watch::Watch;

const SCREENS: &[Ops] = &[
    Ops::of::<accounts::Accounts>(),
    Ops::of::<income::Incomes>(),
    Ops::of::<expenses::Expenses>(),
    Ops::of::<expenses::Cliffs>(),
    Ops::of::<flows::Transfers>(),
    Ops::of::<flows::Conversions>(),
    Ops::of::<contributions::Contributions>(),
    Ops::of::<flows::Events>(),
    Ops::of::<household::People>(),
    Ops::of::<household::Residencies>(),
    Ops::single::<household::Household>(),
    Ops::single::<config::Config>(),
    Ops::single::<market::MarketSettings>(),
];

/// The session settles on its item and fills the form before anything
/// reads what that wrote: a trigger's slots follow the kind filled in, and
/// a table's rows the cursor row the session asked for.
#[derive(bevy_ecs::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditSystems {
    Seed,
    Place,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Importing>();
    app.configure_sets(Update, (EditSystems::Seed, EditSystems::Place).chain());
    app.add_plugins((screen::plugin, form::plugin, scroll::plugin));
    app.add_systems(
        Startup,
        (seed_draft, spawn_screens.after(layout::spawn_frame)),
    );
}

/// The draft over the projected plan, read-only for a scenario.
pub fn seed_draft(world: &mut World) {
    let plan = world.resource::<Projected>().plan.clone();
    let is_scenario = world.resource::<Watch>().is_scenario();
    world.insert_resource(Draft::new(plan, is_scenario));
}

fn spawn_screens(bodies: Query<Entity, With<Body>>, mut commands: Commands) {
    let Ok(body) = bodies.single() else {
        return;
    };
    for &ops in SCREENS {
        screen::spawn_screen(&mut commands, body, ops);
    }
}

/// Whether `page` is a table of items, which is what add and delete act
/// on.
pub fn lists_items(page: Page) -> bool {
    SCREENS
        .iter()
        .any(|ops| ops.surface == Some(page) && ops.list.is_some())
}

/// How many items `page`'s domain holds; `None` for a page that lists
/// none.
pub fn item_count(page: Page, plan: &Plan) -> Option<usize> {
    let list = SCREENS.iter().find(|ops| ops.surface == Some(page))?.list?;
    Some((list.count)(plan))
}

const _: () = {
    let mut at = 0;
    while at < SCREENS.len() {
        assert!(
            build::help_fits(SCREENS[at]),
            "a field's help is missing or too long"
        );
        at += 1;
    }
};

/// Fills `pane` with a tool's answers to read, which ⏎ opens the form
/// over, as a single-item domain's page is.
pub fn spawn_details(commands: &mut Commands, pane: Entity, ops: Ops) {
    details::spawn_into(commands, pane, ops, None, TOOL_HINTS);
}

const TOOL_HINTS: Hints = Hints(&[("⏎", "edit")]);

/// What an issue's path points at: the domain, the item where the path
/// indexes one, and the field where the form has one for it.
struct Located {
    ops: &'static Ops,
    index: Option<usize>,
    field: Option<&'static FieldSpec>,
    /// The place the path names in the field's list: `2` of
    /// `plan.withdrawal_order[2]`.
    place: Option<usize>,
}

impl Located {
    /// Whether the issue is against `spec` of `item`: its field, and where
    /// the path names a place of a list several rows share, that one row.
    fn is_against(&self, spec: &FieldSpec, item: Option<&Table>) -> bool {
        self.field.is_some_and(|field| field.key == spec.key)
            && self
                .place
                .is_none_or(|place| holds_place(spec, place, item))
    }
}

/// Whether `spec`, one row of the list at its key, holds `place` of that
/// list as `item` states it.
fn holds_place(spec: &FieldSpec, place: usize, item: Option<&Table>) -> bool {
    match spec.kind {
        FieldKind::Order(_, at) => at == place,
        FieldKind::Listed(back) => {
            let list = item.and_then(|item| codec::get_path(item, spec.key));
            list.and_then(Value::as_array)
                .is_some_and(|list| list.len().checked_sub(back + 1) == Some(place))
        }
        _ => true,
    }
}

/// The longest root wins, so `household.people` is not the household. A
/// root that is itself a field - `medicare` - is that field.
fn locate(path: &str) -> Option<Located> {
    let (ops, root) = SCREENS
        .iter()
        .flat_map(|ops| ops.paths.iter().map(move |root| (ops, *root)))
        .filter(|(_, root)| is_at_or_within(path, root))
        .max_by_key(|(_, root)| root.len())?;
    let rest = &path[root.len()..];
    let (index, within) = match rest.strip_prefix('[').and_then(|rest| rest.split_once(']')) {
        Some((digits, within)) => (digits.parse().ok(), within),
        None => (None, rest),
    };
    // A root may itself be a key of the item, as `medicare` is.
    let keyed_root = root.rsplit('.').next().map_or(0, str::len);
    let field = field_at(ops.fields, &codec::as_key(within.trim_start_matches('.')))
        .or_else(|| field_at(ops.fields, &path[root.len() - keyed_root..]));
    let place = field.and_then(|spec| codec::list_place(path, spec.key));
    Some(Located {
        ops,
        index,
        field,
        place,
    })
}

/// The field an issue at `path` within an item is about: the one with the
/// longest key the path is or sits under, else the first that sits under it.
fn field_at<'a>(fields: &'a [FieldSpec], path: &str) -> Option<&'a FieldSpec> {
    let holding = fields.iter().filter(|spec| is_at_or_within(path, spec.key));
    holding
        .min_by_key(|spec| Reverse(spec.key.len()))
        .or_else(|| fields.iter().find(|spec| is_at_or_within(spec.key, path)))
}

fn is_at_or_within(path: &str, outer: &str) -> bool {
    path == outer || codec::is_within(path, outer)
}

/// The page that shows what an issue at `path` is about, and the item's
/// row where the path indexes one.
pub fn issue_target(path: &str) -> Option<(Page, Option<usize>)> {
    let located = locate(path)?;
    Some((located.ops.surface?, located.index))
}

const PLACE_SEPARATOR: &str = " \u{203a} ";

/// An issue in the words the forms use: the page, the item by its display
/// name, and the field's label, ahead of the engine's own message. A path
/// no page shows reads as the engine wrote it.
pub fn issue_words(issue: &retiretui_engine::plan::Issue, draft: &Draft) -> String {
    let Some(located) = locate(&issue.path) else {
        return issue.to_string();
    };
    let index = located.index.or(located.ops.list.is_none().then_some(0));
    let item = index.and_then(|index| (located.ops.item)(draft, index));
    let place = place_words(&located, item.as_ref());
    format!("{}: {}", place.join(PLACE_SEPARATOR), issue.message)
}

/// The page, `item` by its display name, and the field's label, of what
/// `located` points at.
fn place_words(located: &Located, item: Option<&Table>) -> Vec<String> {
    let identity = located.ops.list.map(|list| list.identity);
    let name = item
        .zip(identity)
        .and_then(|(item, identity)| offers::display_name(item, identity, located.ops.fields));
    let page = Some(located.ops.title.to_owned());
    let fields = located.ops.fields.iter();
    let row = fields.clone().find(|spec| located.is_against(spec, item));
    let field = row.or(located.field).map(|spec| spec.label.to_owned());
    [page, name, field].into_iter().flatten().collect()
}

/// An issue beside what its path points at, found once for everything
/// that asks about the same draft.
type LocatedIssue<'a> = (Located, &'a str);

fn located_issues(draft: &Draft) -> Vec<LocatedIssue<'_>> {
    let issues = draft.issues().iter();
    issues
        .filter_map(|issue| Some((locate(&issue.path)?, issue.message.as_str())))
        .collect()
}

/// What `located` holds against the field `spec` of `item`, item `index`
/// of `ops`; `None` is the one item a domain without a table has.
fn field_issue<'a>(
    located: &[LocatedIssue<'a>],
    (ops, index): (Ops, Option<usize>),
    spec: &FieldSpec,
    item: Option<&Table>,
) -> Option<&'a str> {
    let domain = ops.surface.filter(|page| page.is_domain())?;
    located.iter().find_map(|(located, message)| {
        let is_here = located.ops.surface == Some(domain)
            && located.index == index
            && located.is_against(spec, item);
        is_here.then_some(*message)
    })
}
