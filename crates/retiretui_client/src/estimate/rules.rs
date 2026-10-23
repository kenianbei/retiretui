//! What the page changes of the user's patterns: a payee named for a
//! category, merged under a name, or ignored - one record or every one -
//! and an ignore it wrote taken back, each made to the file's text so the
//! rest of it stays as the user wrote it.

use retiretui_engine::estimate::{CENTS_PER_DOLLAR, Category, Cents, Patterns};
use retiretui_engine::plan::PlanDate;
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, TableLike, Value};

const PATTERNS: &str = "patterns";
const PAYEES: &str = "payees";
const IS: &str = "is";
const ON: &str = "on";
const SPENT: &str = "spent";
const IGNORE: &str = "ignore";
const NAME: &str = "name";
const ONE_OF: &str = "one_of";
/// What an un-ignore says where no rule of the page's shape ignores it.
const HAND_WRITTEN: &str =
    "a rule written by hand ignores it; take it out of the patterns file to count it";

/// A change the page makes to the user's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The payee's rows are for the category.
    Name {
        /// The payee, by the name it is listed under.
        payee: String,
        /// What its rows are for.
        category: Category,
    },
    /// The payee is listed under a name, with any others given it.
    Merge {
        /// The payee, as the estimate normalized it.
        payee: String,
        /// The name.
        name: String,
    },
    /// Every row of the payee is counted nowhere.
    IgnorePayee {
        /// The payee, by the name it is listed under.
        payee: String,
    },
    /// One record is counted nowhere.
    IgnoreRecord(Record),
    /// An ignored record counted again: the page's rule for it, else its
    /// payee's, taken out.
    Unignore(Record),
}

/// One record, as a rule singles it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The payee, by the name it is listed under.
    pub payee: String,
    /// Its date.
    pub on: PlanDate,
    /// What it spent, a credit negative.
    pub spent: Cents,
}

impl Change {
    /// The patterns file `text` with the change made.
    ///
    /// # Errors
    ///
    /// Where the file is not TOML, holds `patterns` or `payees` as neither
    /// a list nor tables, an un-ignore finds no rule of the page's own
    /// shape, or the file no longer reads as patterns.
    pub fn applied(&self, text: &str) -> Result<String, String> {
        let mut file: DocumentMut = text
            .parse()
            .map_err(|error| format!("the patterns file is not TOML: {error}"))?;
        match self {
            Self::Name { payee, category } => {
                let mut rule = InlineTable::new();
                rule.insert(IS, payee.as_str().into());
                rule.insert("category", category.slug().into());
                push(&mut file, PATTERNS, rule)?;
            }
            Self::Merge { payee, name } => merge(&mut file, payee, name)?,
            Self::IgnorePayee { payee } => push(&mut file, PATTERNS, ignoring(payee, None))?,
            Self::IgnoreRecord(record) => {
                let rule = ignoring(&record.payee, Some((record.on, record.spent)));
                push(&mut file, PATTERNS, rule)?;
            }
            Self::Unignore(record) => unignore(&mut file, record)?,
        }
        let text = file.to_string();
        Patterns::default()
            .add(&text)
            .map_err(|error| format!("the patterns file does not read: {error}"))?;
        Ok(text)
    }

    /// What is said once it is made.
    #[must_use]
    pub fn said(&self) -> String {
        match self {
            Self::Name { payee, category } => format!("{payee} is {}", category.title()),
            Self::Merge { payee, name } => format!("{payee} is listed as {name}"),
            Self::IgnorePayee { payee } => format!("{payee} is ignored"),
            Self::IgnoreRecord(record) => format!("{} on {} is ignored", record.payee, record.on.0),
            Self::Unignore(record) => format!("{} on {} is counted", record.payee, record.on.0),
        }
    }
}

/// The rule ignoring `payee`, or one record of it.
fn ignoring(payee: &str, record: Option<(PlanDate, Cents)>) -> InlineTable {
    let mut rule = InlineTable::new();
    rule.insert(IS, payee.into());
    if let Some((on, spent)) = record {
        rule.insert(
            ON,
            on.0.to_string()
                .parse::<Value>()
                .expect("a date reads as TOML"),
        );
        let dollars = spent.0 as f64 / CENTS_PER_DOLLAR as f64;
        rule.insert(SPENT, Value::Array(Array::from_iter([dollars, dollars])));
    }
    rule.insert(IGNORE, true.into());
    rule
}

/// Appends `entry` to the list or tables at `key`, made where there are
/// none.
fn push(file: &mut DocumentMut, key: &str, entry: InlineTable) -> Result<(), String> {
    match file.get_mut(key) {
        None => {
            let mut tables = ArrayOfTables::new();
            tables.push(entry.into_table());
            file.insert(key, Item::ArrayOfTables(tables));
        }
        Some(Item::ArrayOfTables(tables)) => tables.push(entry.into_table()),
        Some(Item::Value(Value::Array(list))) => list.push(entry),
        Some(_) => return Err(format!("`{key}` is neither a list nor tables")),
    }
    Ok(())
}

/// Each entry at `key`, as list items or tables.
fn entries<'a>(file: &'a mut DocumentMut, key: &str) -> Vec<&'a mut dyn TableLike> {
    match file.get_mut(key) {
        Some(Item::ArrayOfTables(tables)) => tables
            .iter_mut()
            .map(|table| table as &mut dyn TableLike)
            .collect(),
        Some(Item::Value(Value::Array(list))) => list
            .iter_mut()
            .filter_map(|item| item.as_inline_table_mut())
            .map(|table| table as &mut dyn TableLike)
            .collect(),
        _ => Vec::new(),
    }
}

/// Adds `payee` to the merge named `name`, or makes one.
fn merge(file: &mut DocumentMut, payee: &str, name: &str) -> Result<(), String> {
    let named = entries(file, PAYEES).into_iter().find(|entry| {
        entry.get(NAME).and_then(Item::as_str) == Some(name) && entry.get(ONE_OF).is_some()
    });
    if let Some(list) = named
        .and_then(|entry| entry.get_mut(ONE_OF))
        .and_then(Item::as_array_mut)
    {
        if !list.iter().any(|held| held.as_str() == Some(payee)) {
            list.push(payee);
        }
        return Ok(());
    }
    let mut entry = InlineTable::new();
    entry.insert(NAME, name.into());
    entry.insert(ONE_OF, Value::Array(Array::from_iter([payee])));
    push(file, PAYEES, entry)
}

/// Takes out the page's rule ignoring `record`, else its payee's.
fn unignore(file: &mut DocumentMut, record: &Record) -> Result<(), String> {
    let shapes = [Some((record.on, record.spent)), None];
    for shape in shapes {
        let at = (entries(file, PATTERNS).iter())
            .position(|entry| is_ignoring(&**entry, &record.payee, shape));
        if let Some(at) = at {
            remove(file, at);
            return Ok(());
        }
    }
    Err(HAND_WRITTEN.to_owned())
}

/// Whether `entry` is the page's rule ignoring `payee`, or one record of
/// it, and nothing more.
fn is_ignoring(entry: &dyn TableLike, payee: &str, record: Option<(PlanDate, Cents)>) -> bool {
    let is = |key: &str| entry.get(key).and_then(Item::as_value);
    let keys = if record.is_some() { 4 } else { 2 };
    let is_payee = is(IS)
        .and_then(Value::as_str)
        .is_some_and(|held| held.eq_ignore_ascii_case(payee));
    let is_record = record.is_none_or(|(on, spent)| {
        let date = is(ON).map(|value| value.to_string().trim().to_owned());
        let range = is(SPENT).and_then(Value::as_array).map(|range| {
            let cents = |value: &Value| {
                (value.as_float())
                    .or_else(|| value.as_integer().map(|whole| whole as f64))
                    .map(|dollars| (dollars * CENTS_PER_DOLLAR as f64).round() as i64)
            };
            range.iter().map(cents).collect::<Vec<_>>()
        });
        date == Some(on.0.to_string()) && range == Some(vec![Some(spent.0); 2])
    });
    entry.len() == keys
        && is(IGNORE).and_then(Value::as_bool) == Some(true)
        && is_payee
        && is_record
}

fn remove(file: &mut DocumentMut, at: usize) {
    match file.get_mut(PATTERNS) {
        Some(Item::ArrayOfTables(tables)) => {
            tables.remove(at);
        }
        Some(Item::Value(Value::Array(list))) => {
            list.remove(at);
        }
        _ => {}
    }
}
