//! A bank's CSV export, its columns sniffed from the header, read on RFC
//! 4180: quoted fields may hold the delimiter, a doubled quote or a line
//! break.

use jiff::civil::Date;

use super::Category;
use super::bank::{BankError, Cents, Row};
use crate::plan::PlanDate;

const QUOTE: char = '"';
const DELIMITERS: [char; 3] = [',', ';', '\t'];
const SHORT_YEAR_CENTURY: i16 = 2000;
const DATE_SEPARATORS: [char; 3] = ['-', '/', '.'];
/// What a header calls each column, lowercased, in the order tried.
const DATE_NAMES: [&str; 1] = ["date"];
const PAYEE_NAMES: [&str; 3] = ["payee", "description", "name"];
const AMOUNT_NAMES: [&str; 1] = ["amount"];
const OUTFLOW_NAMES: [&str; 3] = ["debit", "withdrawal", "outflow"];
const INFLOW_NAMES: [&str; 3] = ["credit", "deposit", "inflow"];
const CATEGORY_NAMES: [&str; 1] = ["category"];
const TYPE_NAMES: [&str; 1] = ["type"];
/// What a CSV's type column says of a row, lowercased words matched within
/// it: a card's payment and an account's transfer move money; interest,
/// dividends and rebates bring it in.
const TRANSFER_TYPES: [&str; 2] = ["transfer", "payment"];
const INCOME_TYPES: [&str; 4] = ["interest", "intadjust", "dividend", "rebate"];

/// Where each column stands, resolved once from the header.
struct Columns {
    date: usize,
    date_format: DateFormat,
    payee: usize,
    amount: Amount,
    category: Option<usize>,
    kind: Option<usize>,
}

/// Where the money stands: one signed column, or what left and what came
/// in apart.
enum Amount {
    Signed(usize),
    Split { outflow: usize, inflow: usize },
}

#[derive(Debug, Clone, Copy)]
enum DateFormat {
    YearMonthDay,
    MonthDayYear,
    MonthDayShortYear,
}

pub(super) fn rows(name: &str, text: &str) -> Result<Vec<Row>, BankError> {
    let first_line = text.lines().next().unwrap_or_default();
    let delimiter = DELIMITERS
        .into_iter()
        .max_by_key(|&d| first_line.matches(d).count())
        .unwrap_or(',');
    let mut records = parse(text, delimiter).into_iter().enumerate();
    let header = records.next().map(|(_, header)| header).unwrap_or_default();
    let mut records = records.peekable();
    let first_date = records.peek().map(|(_, fields)| fields.as_slice());
    let columns = Columns::sniff(&header, first_date).ok_or_else(|| BankError::Unreadable {
        name: name.to_owned(),
        header: first_line.to_owned(),
    })?;
    records
        .filter(|(_, fields)| !fields.iter().all(|field| field.trim().is_empty()))
        .map(|(index, fields)| columns.row(name, index + 1, &fields))
        .collect()
}

impl Columns {
    fn sniff(header: &[String], first_row: Option<&[String]>) -> Option<Self> {
        let column = |names: &[&str]| {
            names.iter().find_map(|name| {
                header
                    .iter()
                    .position(|h| h.trim().to_lowercase().contains(name))
            })
        };
        let date = column(&DATE_NAMES)?;
        let amount = match column(&AMOUNT_NAMES) {
            Some(at) => Amount::Signed(at),
            None => Amount::Split {
                outflow: column(&OUTFLOW_NAMES)?,
                inflow: column(&INFLOW_NAMES)?,
            },
        };
        let first_date = first_row
            .and_then(|fields| fields.get(date))
            .map_or("", String::as_str);
        Some(Self {
            date,
            date_format: guess_format(first_date),
            payee: column(&PAYEE_NAMES)?,
            amount,
            category: column(&CATEGORY_NAMES),
            kind: column(&TYPE_NAMES),
        })
    }

    fn row(&self, name: &str, row: usize, fields: &[String]) -> Result<Row, BankError> {
        let field = |at: usize| fields.get(at).map_or("", String::as_str).trim();
        let malformed = |text: &str, expected| BankError::Malformed {
            name: name.to_owned(),
            row,
            text: text.to_owned(),
            expected,
        };
        let date_text = field(self.date);
        let on =
            parse_date(date_text, self.date_format).ok_or_else(|| malformed(date_text, "date"))?;
        let cents = |at: usize| -> Result<Cents, BankError> {
            let text = field(at);
            if text.is_empty() {
                return Ok(Cents(0));
            }
            Cents::parse(text).ok_or_else(|| malformed(text, "amount"))
        };
        let amount = match self.amount {
            Amount::Signed(at) => cents(at)?,
            Amount::Split { outflow, inflow } => cents(inflow)? - cents(outflow)?,
        };
        let optional =
            |at: Option<usize>| at.map(field).filter(|t| !t.is_empty()).map(str::to_owned);
        Ok(Row {
            on,
            amount,
            payee: field(self.payee).to_owned(),
            category: optional(self.category),
            kind: self.kind.map(field).and_then(kind_of),
            bank_id: None,
            export: String::new(),
        })
    }
}

fn kind_of(text: &str) -> Option<Category> {
    let text = text.to_lowercase();
    let says = |words: &[&str]| words.iter().any(|word| text.contains(word));
    if says(&TRANSFER_TYPES) {
        Some(Category::Transfer)
    } else if says(&INCOME_TYPES) {
        Some(Category::Income)
    } else {
        None
    }
}

fn parse(text: &str, delimiter: char) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut is_quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            QUOTE if is_quoted && chars.peek() == Some(&QUOTE) => {
                field.push(QUOTE);
                chars.next();
            }
            QUOTE => is_quoted = !is_quoted,
            _ if is_quoted => field.push(c),
            _ if c == delimiter => fields.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                fields.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut fields));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !fields.is_empty() {
        fields.push(field);
        records.push(fields);
    }
    records
}

fn parse_date(text: &str, format: DateFormat) -> Option<PlanDate> {
    let parts: Vec<i16> = text
        .split(DATE_SEPARATORS)
        .map(|part| part.parse().ok())
        .collect::<Option<_>>()?;
    let [first, second, third] = parts[..] else {
        return None;
    };
    let (year, month, day) = match format {
        DateFormat::YearMonthDay => (first, second, third),
        DateFormat::MonthDayYear => (third, first, second),
        DateFormat::MonthDayShortYear => (SHORT_YEAR_CENTURY + third, first, second),
    };
    Date::new(year, month as i8, day as i8).ok().map(PlanDate)
}

fn guess_format(date: &str) -> DateFormat {
    let parts: Vec<&str> = date.trim().split(DATE_SEPARATORS).collect();
    match parts[..] {
        [year, _, _] if year.len() == 4 => DateFormat::YearMonthDay,
        [_, _, year] if year.len() == 2 => DateFormat::MonthDayShortYear,
        _ => DateFormat::MonthDayYear,
    }
}
