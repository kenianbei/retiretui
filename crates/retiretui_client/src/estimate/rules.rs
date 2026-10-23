//! What the page changes of the user's patterns: a payee named for a
//! category, merged under a name, or ignored - one record or every one -
//! and an ignore it wrote taken back, each made to the file's text so the
//! rest of it stays as the user wrote it.

use retiretui_engine::estimate::{Category, Cents, Patterns};
use retiretui_engine::plan::PlanDate;
use serde::Deserialize;
use toml_edit::{
    Array, ArrayOfTables, Date, Datetime, DocumentMut, InlineTable, Item, TableLike, Value,
};

const PATTERNS: &str = "patterns";
const PAYEES: &str = "payees";
const IS: &str = "is";
const ON: &str = "on";
const SPENT: &str = "spent";
const IGNORE: &str = "ignore";
const NAME: &str = "name";
const CATEGORY: &str = "category";
const BUCKET: &str = "bucket";
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
        /// Where its rows go.
        to: To,
    },
    /// The payee is listed under a name, with any others given it.
    Merge {
        /// The payee, as the estimate normalized it.
        payee: String,
        /// The name.
        name: String,
    },
    /// Every row of each payee containing the words is for the category.
    Contains {
        /// The words.
        part: String,
        /// Where the rows go.
        to: To,
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

/// Where a payee's rows go: a category, or the bucket as a payee nothing
/// names goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum To {
    /// The category.
    Category(Category),
    /// The bucket.
    Bucket,
}

impl To {
    /// Writes where the rows go into `rule`.
    fn write(self, rule: &mut InlineTable) {
        match self {
            Self::Category(category) => rule.insert(CATEGORY, category.slug().into()),
            Self::Bucket => rule.insert(BUCKET, true.into()),
        };
    }

    /// Where the rows go, said after one payee or several.
    fn said(self, is_several: bool) -> String {
        match (self, is_several) {
            (Self::Category(category), false) => format!("is {}", category.title()),
            (Self::Category(category), true) => format!("are {}", category.title()),
            (Self::Bucket, false) => "goes to the bucket".to_owned(),
            (Self::Bucket, true) => "go to the bucket".to_owned(),
        }
    }
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
            Self::Name { payee, to } => {
                let mut rule = InlineTable::new();
                rule.insert(IS, payee.as_str().into());
                to.write(&mut rule);
                replace_payees(&mut file, payee, rule)?;
            }
            Self::Contains { part, to } => {
                let mut rule = InlineTable::new();
                rule.insert("match", part.as_str().into());
                to.write(&mut rule);
                push(&mut file, PATTERNS, rule)?;
            }
            Self::Merge { payee, name } => merge(&mut file, payee, name)?,
            Self::IgnorePayee { payee } => replace_payees(&mut file, payee, ignoring(payee, None))?,
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
            Self::Name { payee, to } => format!("{payee} {}", to.said(false)),
            Self::Contains { part, to } => format!("payees containing {part} {}", to.said(true)),
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
        let date = Date {
            year: on.0.year() as u16,
            month: on.0.month() as u8,
            day: on.0.day() as u8,
        };
        let on = Datetime {
            date: Some(date),
            time: None,
            offset: None,
        };
        rule.insert(ON, on.into());
        let dollars = spent.dollars();
        rule.insert(SPENT, Value::Array(Array::from_iter([dollars, dollars])));
    }
    rule.insert(IGNORE, true.into());
    rule
}

/// Puts `rule` in place of the rules the page wrote for all of `payee` - a
/// category or an ignore - so a payee moved again goes where it was last
/// sent.
fn replace_payees(file: &mut DocumentMut, payee: &str, rule: InlineTable) -> Result<(), String> {
    while let Some(at) =
        (entries(file, PATTERNS).iter()).position(|entry| is_for_payee(&**entry, payee))
    {
        remove(file, at);
    }
    push(file, PATTERNS, rule)
}

/// Whether `entry` is a rule the page writes for all of `payee`.
fn is_for_payee(entry: &dyn TableLike, payee: &str) -> bool {
    let is_payee =
        (entry.get(IS).and_then(Item::as_str)).is_some_and(|held| held.eq_ignore_ascii_case(payee));
    let says = entry.get(CATEGORY).is_some()
        || entry.get(BUCKET).is_some()
        || is_ignoring(entry, payee, None);
    is_payee && entry.len() == 2 && says
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

/// Whether `entry` is the rule the page writes ignoring `payee`, or one
/// record of it, and nothing more.
fn is_ignoring(entry: &dyn TableLike, payee: &str, record: Option<(PlanDate, Cents)>) -> bool {
    let wanted = ignoring(payee, record);
    entry.len() == wanted.len()
        && wanted.iter().all(|(key, value)| {
            let held = entry.get(key).and_then(Item::as_value);
            held.is_some_and(|held| is_same(held, value))
        })
}

/// Whether two values say the same, letters of either case alike.
fn is_same(held: &Value, wanted: &Value) -> bool {
    if let (Some(held), Some(wanted)) = (held.as_array(), wanted.as_array()) {
        return held.len() == wanted.len()
            && held.iter().zip(wanted.iter()).all(|(a, b)| is_same(a, b));
    }
    let said = |value: &Value| value.to_string().trim().to_owned();
    said(held).eq_ignore_ascii_case(&said(wanted))
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
