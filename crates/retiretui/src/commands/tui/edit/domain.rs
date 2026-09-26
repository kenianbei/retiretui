use std::collections::BTreeSet;

use bevy_ecs::prelude::Commands;
use retiretui_engine::plan::{ID_KEY, Plan};
use serde::Serialize;
use serde::de::DeserializeOwned;
use toml::{Table, Value};

use super::build::FormButton;
use super::cells::{Cell, Column, Shown};
use super::codec::{from_table, to_table};
use super::draft::Draft;
use super::offers::{RefSource, Vocabulary};
use crate::commands::tui::nav::Page;
use crate::commands::tui::present;

/// One field of an item: the key the file knows it by, the words the
/// form shows for it, and what edits it.
#[derive(Clone, Copy)]
pub struct FieldSpec {
    /// A dotted key reaches into a table the item holds.
    pub key: &'static str,
    pub label: &'static str,
    /// What the field means, its unit, and what leaving it blank does.
    pub help: &'static str,
    /// What the field reads as while it holds nothing, where that says
    /// something: a default, or what absence means.
    pub blank: Option<&'static str>,
    /// Whether the item being edited has a use for the field, where that
    /// hangs on another of its fields; `None` always has.
    pub shown: Option<fn(&Table) -> bool>,
    /// A field no file holds, read from the item and written back into
    /// what a file does hold.
    pub derived: Option<Derived>,
    pub kind: FieldKind,
}

/// How a field no file holds is read on open and turned back on write.
#[derive(Clone, Copy)]
pub struct Derived {
    /// The field's value, read from the item as a file states it.
    pub seed: fn(&Table) -> Value,
    /// What the field's value means for the keys a file does hold.
    pub write: fn(&mut Table, &Value),
}

/// How a field's value is entered. Everything the schema states as a
/// closed set is picked rather than typed, so an invalid value cannot be
/// expressed; the rest is TOML value syntax.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Money,
    /// A whole number that is not money: an age, a year, a rank.
    Whole,
    /// A boolean, shown as a two-way pick.
    Flag,
    /// One word of a vocabulary the schema states.
    Choice(Vocabulary),
    /// An id the plan itself declares.
    Ref(RefSource),
    /// A rate, entered on a slider or exactly as text.
    Rate,
    /// A share of a whole, entered as a rate is on a slider that reaches
    /// all of it.
    Share,
    /// How an amount grows: with inflation, not at all, or at its own rate.
    Growth,
    /// A trigger, picked over the schema's own vocabulary.
    Trigger,
    /// A tick that stands for a table being there: ticked, the item holds
    /// the table this TOML describes, and unticked it holds none.
    Presence(&'static str),
    /// An amount of a list, by how many places from the list's end it is.
    Listed(usize),
    /// One place, counted from the first, in an order of a vocabulary's
    /// words that several rows hold between them, none of them twice.
    Order(Vocabulary, usize),
    /// A share no one enters: what the other shares of its table leave of
    /// the whole, shown as they change.
    Remainder,
}

/// What every field that says how an amount grows is described by.
pub const GROWTH_HELP: &str =
    "Inflation follows the plan's rate, Fixed never grows, or a rate of its own such as 3%.";

/// What an empty pick reads as where its field says nothing else.
pub const BLANK: &str = "None";

impl FieldSpec {
    const fn new(key: &'static str, label: &'static str, kind: FieldKind) -> Self {
        Self {
            key,
            label,
            help: "",
            blank: None,
            shown: None,
            derived: None,
            kind,
        }
    }

    pub const fn text(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Text)
    }

    pub const fn money(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Money)
    }

    pub const fn whole(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Whole)
    }

    pub const fn flag(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Flag)
    }

    pub const fn choice(key: &'static str, label: &'static str, vocabulary: Vocabulary) -> Self {
        Self::new(key, label, FieldKind::Choice(vocabulary))
    }

    pub const fn refers(key: &'static str, label: &'static str, source: RefSource) -> Self {
        Self::new(key, label, FieldKind::Ref(source))
    }

    pub const fn trigger(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Trigger)
    }

    pub const fn rate(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Rate)
    }

    pub const fn share(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Share)
    }

    pub const fn remainder(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Remainder)
    }

    pub const fn growth(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Growth).blank(present::FOLLOWS_INFLATION)
    }

    pub const fn presence(key: &'static str, label: &'static str, ticked: &'static str) -> Self {
        Self::new(key, label, FieldKind::Presence(ticked))
    }

    pub const fn listed(key: &'static str, label: &'static str, back: usize) -> Self {
        Self::new(key, label, FieldKind::Listed(back))
    }

    pub const fn ordered(
        key: &'static str,
        label: &'static str,
        vocabulary: Vocabulary,
        place: usize,
    ) -> Self {
        Self::new(key, label, FieldKind::Order(vocabulary, place))
    }

    pub const fn help(self, help: &'static str) -> Self {
        Self { help, ..self }
    }

    pub const fn blank(self, blank: &'static str) -> Self {
        Self {
            blank: Some(blank),
            ..self
        }
    }

    pub const fn shown_when(self, shown: fn(&Table) -> bool) -> Self {
        Self {
            shown: Some(shown),
            ..self
        }
    }

    pub const fn derived(self, seed: fn(&Table) -> Value, write: fn(&mut Table, &Value)) -> Self {
        Self {
            derived: Some(Derived { seed, write }),
            ..self
        }
    }

    /// Whether a pick may not be emptied, so its menu offers no way to:
    /// one with no word for what empty means.
    pub const fn is_required(&self) -> bool {
        self.blank.is_none()
    }

    pub const fn blank_word(&self) -> &'static str {
        match self.blank {
            Some(word) => word,
            None => BLANK,
        }
    }
}

/// One editable collection of the plan: which items, which fields, and how
/// a list row reads. The runtime works on [`Ops`]; this trait only exists
/// to be turned into one.
pub trait Domain: 'static {
    type Item: Serialize + DeserializeOwned;
    const PAGE: Page;
    /// What the domain is for, in the words of someone who has not read
    /// the schema: what its table says while it holds nothing.
    const PURPOSE: &'static str;
    /// The TOML root the items validate under: what an issue's path starts
    /// with.
    const PATH: &'static str;
    /// One item of the domain, for naming a new one.
    const SINGULAR: &'static str;
    /// The field an item is known by, which is not always the column the
    /// table leads with.
    const IDENTITY: &'static str = ID_KEY;
    /// The item's fields, in form order.
    const FIELDS: &'static [FieldSpec];
    /// The table's columns: the field key shown, and its width.
    const COLUMNS: &'static [Column];
    /// A new item, as TOML; an `owner` key is filled with the first person.
    const BLANK: &'static str;
    /// Rows the item's details end with, beyond its fields: what the item
    /// keeps that no field edits.
    const RECORD: Option<fn(&Table) -> Vec<[String; 2]>> = None;
    fn items(plan: &Plan) -> &[Self::Item];
    fn items_mut(plan: &mut Plan) -> &mut Vec<Self::Item>;
}

/// A part of the plan there is exactly one of, edited as a form alone.
pub trait Single: 'static {
    type Item: Serialize + DeserializeOwned;
    const PAGE: Page;
    /// The TOML roots the item's fields validate under.
    const PATHS: &'static [&'static str];
    const FIELDS: &'static [FieldSpec];
    fn get(plan: &Plan) -> Self::Item;
    fn set(plan: &mut Plan, item: Self::Item);
}

/// What applying a form writes: the plan, as a step of its history, or a
/// tool's own table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Draft,
    Tool,
}

/// A domain as the runtime sees it: plain functions over the draft and
/// TOML tables, so the editing systems need no type parameter.
#[derive(Clone, Copy)]
pub struct Ops {
    pub surface: Option<Page>,
    pub title: &'static str,
    pub target: Target,
    /// The TOML roots an issue of the domain is filed under.
    pub paths: &'static [&'static str],
    pub fields: &'static [FieldSpec],
    pub item: fn(&Draft, usize) -> Option<Table>,
    /// Replaces item `index` from `table`; the error is the schema's.
    pub store: fn(&mut Draft, usize, Table) -> Result<(), String>,
    /// `table` as the item's own type states it, defaults and all; `None`
    /// where the type cannot read it.
    pub typed: fn(Table) -> Option<Table>,
    /// The list beside the form; `None` for a single-item domain.
    pub list: Option<ListOps>,
    /// What the buttons at the form's foot say: the one that drops its
    /// edits, then the one that stores them.
    pub actions: [&'static str; 2],
    /// Run once the form is applied or dropped, for a tool that acts on
    /// its answers at once rather than at a command.
    pub after: Option<fn(FormButton, &mut Commands)>,
}

/// What a form's buttons say unless it says otherwise.
const ACTIONS: [&str; 2] = ["Discard", "Apply"];

/// The answers a tool's form holds beside the plan, kept under a name of
/// their own so that two tools never read each other's.
pub trait ToolAnswers: DeserializeOwned + 'static {
    const SLOT: &'static str;
}

/// What a domain with many items adds: the table.
#[derive(Clone, Copy)]
pub struct ListOps {
    pub purpose: &'static str,
    pub singular: &'static str,
    pub identity: &'static str,
    pub columns: &'static [Column],
    pub count: fn(&Plan) -> usize,
    /// Every item's cells, in column order.
    pub rows: fn(&Plan) -> Vec<Vec<Cell>>,
    /// A new item's table, the owner already filled in; stored at the
    /// item count, it appends.
    pub blank: fn(&Plan) -> Table,
    pub remove: fn(&mut Plan, usize),
    /// Rows the item's details end with, beyond its fields.
    pub record: Option<fn(&Table) -> Vec<[String; 2]>>,
}

impl Ops {
    pub const fn of<D: Domain>() -> Self {
        Self {
            surface: Some(D::PAGE),
            title: D::PAGE.title(),
            target: Target::Draft,
            paths: &[D::PATH],
            fields: D::FIELDS,
            item: item::<D>,
            store: store::<D>,
            typed: typed::<D::Item>,
            list: Some(ListOps {
                purpose: D::PURPOSE,
                singular: D::SINGULAR,
                identity: D::IDENTITY,
                columns: D::COLUMNS,
                count: count::<D>,
                rows: rows::<D>,
                blank: blank::<D>,
                remove: remove::<D>,
                record: D::RECORD,
            }),
            actions: ACTIONS,
            after: None,
        }
    }

    pub const fn single<S: Single>() -> Self {
        Self {
            surface: Some(S::PAGE),
            title: S::PAGE.title(),
            target: Target::Draft,
            paths: S::PATHS,
            fields: S::FIELDS,
            item: single_item::<S>,
            store: single_store::<S>,
            typed: typed::<S::Item>,
            list: None,
            actions: ACTIONS,
            after: None,
        }
    }

    /// A form the shell keeps beside the plan, its fields a table of its
    /// own that `T` parses. Applying it holds the answers; what acts on
    /// them runs after, where the world is in reach.
    pub const fn tool<T: ToolAnswers>(
        surface: Option<Page>,
        title: &'static str,
        fields: &'static [FieldSpec],
    ) -> Self {
        Self {
            surface,
            title,
            target: Target::Tool,
            paths: &[],
            fields,
            item: tool_item::<T>,
            store: tool_store::<T>,
            typed: Some,
            list: None,
            actions: ACTIONS,
            after: None,
        }
    }

    /// The same form, its buttons saying `actions` and `after` run once
    /// either has done its work.
    pub const fn acting(
        self,
        actions: [&'static str; 2],
        after: fn(FormButton, &mut Commands),
    ) -> Self {
        Self {
            actions,
            after: Some(after),
            ..self
        }
    }
}

fn tool_item<T: ToolAnswers>(draft: &Draft, index: usize) -> Option<Table> {
    (index == 0).then(|| draft.answers::<T>())
}

/// A table its item parses is kept as typed, blank fields and all.
fn tool_store<T: ToolAnswers>(draft: &mut Draft, _: usize, table: Table) -> Result<(), String> {
    from_table::<T>(table.clone())?;
    draft.tools.insert(T::SLOT.to_owned(), Value::Table(table));
    Ok(())
}

fn typed<Item: Serialize + DeserializeOwned>(table: Table) -> Option<Table> {
    from_table::<Item>(table).ok().map(|item| to_table(&item))
}

fn count<D: Domain>(plan: &Plan) -> usize {
    D::items(plan).len()
}

fn single_item<S: Single>(draft: &Draft, index: usize) -> Option<Table> {
    (index == 0).then(|| to_table(&S::get(&draft.plan)))
}

fn single_store<S: Single>(draft: &mut Draft, _: usize, table: Table) -> Result<(), String> {
    S::set(&mut draft.plan, from_table(table)?);
    Ok(())
}

fn item<D: Domain>(draft: &Draft, index: usize) -> Option<Table> {
    D::items(&draft.plan).get(index).map(to_table)
}

fn rows<D: Domain>(plan: &Plan) -> Vec<Vec<Cell>> {
    let columns: Vec<Shown> = D::COLUMNS
        .iter()
        .map(|column| Shown::of(column, D::FIELDS, D::IDENTITY, plan))
        .collect();
    D::items(plan)
        .iter()
        .map(|item| {
            let table = to_table(item);
            columns
                .iter()
                .map(|shown| shown.cell(&table, plan))
                .collect()
        })
        .collect()
}

fn store<D: Domain>(draft: &mut Draft, index: usize, table: Table) -> Result<(), String> {
    let item = from_table(table)?;
    let items = D::items_mut(&mut draft.plan);
    match items.get_mut(index) {
        Some(slot) => *slot = item,
        None => items.push(item),
    }
    Ok(())
}

const OWNER_KEY: &str = "owner";

fn blank<D: Domain>(plan: &Plan) -> Table {
    let mut blank: Table = D::BLANK.parse().unwrap_or_default();
    if let (Some(slot), Some(person)) = (blank.get_mut(OWNER_KEY), plan.household.people.first()) {
        *slot = Value::String(person.id.clone());
    }
    if D::IDENTITY == ID_KEY {
        blank.insert(D::IDENTITY.to_owned(), Value::String(free_id::<D>(plan)));
    }
    blank
}

/// The lowest `{kind}-{n}` no item of the domain holds, `kind` the first
/// word of its singular.
fn free_id<D: Domain>(plan: &Plan) -> String {
    let taken: BTreeSet<String> = D::items(plan)
        .iter()
        .filter_map(|item| match to_table(item).remove(D::IDENTITY) {
            Some(Value::String(id)) => Some(id),
            _ => None,
        })
        .collect();
    let kind = D::SINGULAR
        .split(' ')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    (1..=taken.len() + 1)
        .map(|n| format!("{kind}-{n}"))
        .find(|id| !taken.contains(id))
        .unwrap_or(kind)
}

fn remove<D: Domain>(plan: &mut Plan, index: usize) {
    let items = D::items_mut(plan);
    if index < items.len() {
        items.remove(index);
    }
}
