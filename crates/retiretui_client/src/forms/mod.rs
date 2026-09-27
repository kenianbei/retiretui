//! The form model: each editable part of a plan, its fields in the words a
//! form shows them in, what edits each field, and when a field has a use.
//! What draws a form, and where, is each interface's own.

pub mod accounts;
pub mod applies;
pub mod cells;
pub mod changes;
/// The settings domain.
pub mod config;
pub mod contributions;
pub mod details;
/// The expense and cliff domains.
pub mod expenses;
/// The transfer, conversion and event domains.
pub mod flows;
/// The people, residency and household domains.
pub mod household;
/// The income domain.
pub mod income;
pub mod lists;
pub mod market;
pub mod offers;
pub mod sort;
pub mod trigger;

use retiretui_engine::plan::{ID_KEY, Plan, fresh_id};
use serde::Serialize;
use serde::de::DeserializeOwned;
use toml::{Table, Value};

use self::cells::{Cell, Column, Shown};
use self::offers::{RefSource, Vocabulary};
use crate::codec::{from_table, to_table};
use crate::draft::Draft;
use crate::present;

/// One editable part of the plan, by the name every interface knows it by.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DomainId {
    /// The plan's accounts.
    Accounts,
    /// Its income sources.
    Income,
    /// Its expenses.
    Expenses,
    /// The MAGI cliffs it declares.
    Cliffs,
    /// Scheduled transfers between accounts.
    Transfers,
    /// Roth conversions.
    Conversions,
    /// Contributions into accounts.
    Contributions,
    /// Named events other items start or stop on.
    Events,
    /// The household's people.
    People,
    /// Where the household lives, and from when.
    Residency,
    /// Filing status and Medicare.
    Household,
    /// The `[plan]` settings.
    Settings,
    /// What the market tools assume, and how they run.
    Market,
}

impl DomainId {
    /// The domain's name, as a heading says it.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Accounts => "Accounts",
            Self::Income => "Income",
            Self::Expenses => "Expenses",
            Self::Cliffs => "Cliffs",
            Self::Transfers => "Transfers",
            Self::Conversions => "Conversions",
            Self::Contributions => "Contributions",
            Self::Events => "Events",
            Self::People => "People",
            Self::Residency => "Residency",
            Self::Household => "Household",
            Self::Settings => "Settings",
            Self::Market => "Market",
        }
    }
}

/// Every domain's form, in the order the plan's domains are listed.
pub const DOMAINS: &[Form] = &[
    Form::of::<accounts::Accounts>(),
    Form::of::<income::Incomes>(),
    Form::of::<expenses::Expenses>(),
    Form::of::<expenses::Cliffs>(),
    Form::of::<flows::Transfers>(),
    Form::of::<flows::Conversions>(),
    Form::of::<contributions::Contributions>(),
    Form::of::<flows::Events>(),
    Form::of::<household::People>(),
    Form::of::<household::Residencies>(),
    Form::single::<household::Household>(),
    Form::single::<config::Config>(),
    Form::single::<market::MarketSettings>(),
];

/// One field of an item: the key the file knows it by, the words the
/// form shows for it, and what edits it.
#[derive(Clone, Copy)]
pub struct FieldSpec {
    /// A dotted key reaches into a table the item holds.
    pub key: &'static str,
    /// What the form calls the field.
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
    /// How the field's value is entered.
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
    /// Free text: a name, an id, a date.
    Text,
    /// An amount of dollars.
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
pub(crate) const GROWTH_HELP: &str =
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

    /// A free-text field.
    #[must_use]
    pub const fn text(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Text)
    }

    /// A money field.
    #[must_use]
    pub const fn money(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Money)
    }

    /// A whole-number field.
    #[must_use]
    pub const fn whole(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Whole)
    }

    /// A ticked field.
    #[must_use]
    pub const fn flag(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Flag)
    }

    /// A pick over `vocabulary`.
    #[must_use]
    pub const fn choice(key: &'static str, label: &'static str, vocabulary: Vocabulary) -> Self {
        Self::new(key, label, FieldKind::Choice(vocabulary))
    }

    /// A pick over the plan's own ids from `source`.
    #[must_use]
    pub const fn refers(key: &'static str, label: &'static str, source: RefSource) -> Self {
        Self::new(key, label, FieldKind::Ref(source))
    }

    /// A trigger field.
    #[must_use]
    pub const fn trigger(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Trigger)
    }

    /// A rate field.
    #[must_use]
    pub const fn rate(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Rate)
    }

    /// A share field.
    #[must_use]
    pub const fn share(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Share)
    }

    /// A share left over by the others of its table.
    #[must_use]
    pub const fn remainder(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Remainder)
    }

    /// A field saying how an amount grows, blank following inflation.
    #[must_use]
    pub const fn growth(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, FieldKind::Growth).blank(present::FOLLOWS_INFLATION)
    }

    /// A tick standing for a table, `ticked` being the table it adds.
    #[must_use]
    pub const fn presence(key: &'static str, label: &'static str, ticked: &'static str) -> Self {
        Self::new(key, label, FieldKind::Presence(ticked))
    }

    /// One amount of a list, `back` places from its end.
    #[must_use]
    pub const fn listed(key: &'static str, label: &'static str, back: usize) -> Self {
        Self::new(key, label, FieldKind::Listed(back))
    }

    /// One place, `place`, of an order over `vocabulary`'s words.
    #[must_use]
    pub const fn ordered(
        key: &'static str,
        label: &'static str,
        vocabulary: Vocabulary,
        place: usize,
    ) -> Self {
        Self::new(key, label, FieldKind::Order(vocabulary, place))
    }

    /// The field with its help.
    #[must_use]
    pub const fn help(self, help: &'static str) -> Self {
        Self { help, ..self }
    }

    /// The field with `blank` as what it reads as while empty.
    #[must_use]
    pub const fn blank(self, blank: &'static str) -> Self {
        Self {
            blank: Some(blank),
            ..self
        }
    }

    /// The field shown only where `shown` says the item has a use for it.
    #[must_use]
    pub const fn shown_when(self, shown: fn(&Table) -> bool) -> Self {
        Self {
            shown: Some(shown),
            ..self
        }
    }

    /// The field read from the item by `seed` and written back by `write`.
    #[must_use]
    pub const fn derived(self, seed: fn(&Table) -> Value, write: fn(&mut Table, &Value)) -> Self {
        Self {
            derived: Some(Derived { seed, write }),
            ..self
        }
    }

    /// Whether a pick may not be emptied, so its menu offers no way to:
    /// one with no word for what empty means.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.blank.is_none()
    }

    /// What the field reads as while empty.
    #[must_use]
    pub const fn blank_word(&self) -> &'static str {
        match self.blank {
            Some(word) => word,
            None => BLANK,
        }
    }
}

/// One editable collection of the plan: which items, which fields, and how
/// a list row reads. The runtime works on [`Form`]; this trait only exists
/// to be turned into one.
pub trait Domain: 'static {
    /// The item's type, as the plan holds it.
    type Item: Serialize + DeserializeOwned;
    /// The domain this is.
    const ID: DomainId;
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
    /// The domain's items in `plan`.
    fn items(plan: &Plan) -> &[Self::Item];
    /// The domain's items in `plan`, to change.
    fn items_mut(plan: &mut Plan) -> &mut Vec<Self::Item>;
}

/// A part of the plan there is exactly one of, edited as a form alone.
pub trait Single: 'static {
    /// The item's type, as the plan holds it.
    type Item: Serialize + DeserializeOwned;
    /// The domain this is.
    const ID: DomainId;
    /// The TOML roots the item's fields validate under.
    const PATHS: &'static [&'static str];
    /// The item's fields, in form order.
    const FIELDS: &'static [FieldSpec];
    /// The item as `plan` holds it.
    fn get(plan: &Plan) -> Self::Item;
    /// Writes `item` into `plan`.
    fn set(plan: &mut Plan, item: Self::Item);
}

/// What applying a form writes: the plan, as a step of its history, or a
/// tool's own table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    /// The plan, as a step of its history.
    Draft,
    /// A tool's own table beside the plan.
    Tool,
}

/// A form as the runtime sees it: plain functions over the draft and TOML
/// tables, so what edits it needs no type parameter.
#[derive(Clone, Copy)]
pub struct Form {
    /// The domain it edits; none for a form an interface keeps beside the
    /// plan.
    pub domain: Option<DomainId>,
    /// The form's title.
    pub title: &'static str,
    /// What applying the form writes.
    pub target: Target,
    /// The TOML roots an issue of the domain is filed under.
    pub paths: &'static [&'static str],
    /// The fields, in form order.
    pub fields: &'static [FieldSpec],
    /// Item `index`'s table, as the form opens it.
    pub item: fn(&Draft, usize) -> Option<Table>,
    /// Replaces item `index` from `table`; the error is the schema's.
    pub store: fn(&mut Draft, usize, Table) -> Result<(), String>,
    /// `table` as the item's own type states it, defaults and all; `None`
    /// where the type cannot read it.
    pub typed: fn(Table) -> Option<Table>,
    /// The list beside the form; `None` for a single-item domain.
    pub list: Option<ListOps>,
}

/// The answers a tool's form holds beside the plan, kept under a name of
/// their own so that two tools never read each other's.
pub trait ToolAnswers: DeserializeOwned + 'static {
    /// The name the answers are held under.
    const SLOT: &'static str;
}

/// What a domain with many items adds: the table.
#[derive(Clone, Copy)]
pub struct ListOps {
    /// What the table says while it holds nothing.
    pub purpose: &'static str,
    /// One item, for naming a new one.
    pub singular: &'static str,
    /// The field an item is known by.
    pub identity: &'static str,
    /// The table's columns.
    pub columns: &'static [Column],
    /// How many items the plan holds.
    pub count: fn(&Plan) -> usize,
    /// Every item's cells, in column order.
    pub rows: fn(&Plan) -> Vec<Vec<Cell>>,
    /// A new item's table, the owner already filled in; stored at the
    /// item count, it appends.
    pub blank: fn(&Plan) -> Table,
    /// Removes item `index` from the plan.
    pub remove: fn(&mut Plan, usize),
    /// Rows the item's details end with, beyond its fields.
    pub record: Option<fn(&Table) -> Vec<[String; 2]>>,
}

impl Form {
    /// The form of the domain `D`.
    #[must_use]
    pub const fn of<D: Domain>() -> Self {
        Self {
            domain: Some(D::ID),
            title: D::ID.title(),
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
        }
    }

    /// The form of the one-item domain `S`.
    #[must_use]
    pub const fn single<S: Single>() -> Self {
        Self {
            domain: Some(S::ID),
            title: S::ID.title(),
            target: Target::Draft,
            paths: S::PATHS,
            fields: S::FIELDS,
            item: single_item::<S>,
            store: single_store::<S>,
            typed: typed::<S::Item>,
            list: None,
        }
    }

    /// A form an interface keeps beside the plan, its fields a table of its
    /// own that `T` parses.
    #[must_use]
    pub const fn tool<T: ToolAnswers>(title: &'static str, fields: &'static [FieldSpec]) -> Self {
        Self {
            domain: None,
            title,
            target: Target::Tool,
            paths: &[],
            fields,
            item: tool_item::<T>,
            store: tool_store::<T>,
            typed: Some,
            list: None,
        }
    }
}

fn tool_item<T: ToolAnswers>(draft: &Draft, index: usize) -> Option<Table> {
    (index == 0).then(|| draft.answers::<T>())
}

/// A table its item parses is kept as typed, blank fields and all.
fn tool_store<T: ToolAnswers>(draft: &mut Draft, _: usize, table: Table) -> Result<(), String> {
    from_table::<T>(table.clone())?;
    draft.set_answers::<T>(table);
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
    let taken: Vec<Table> = D::items(plan).iter().map(to_table).collect();
    let ids = taken
        .iter()
        .filter_map(|item| item.get(D::IDENTITY).and_then(Value::as_str));
    let kind = D::SINGULAR
        .split(' ')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    fresh_id(&kind, ids)
}

fn remove<D: Domain>(plan: &mut Plan, index: usize) {
    let items = D::items_mut(plan);
    if index < items.len() {
        items.remove(index);
    }
}
