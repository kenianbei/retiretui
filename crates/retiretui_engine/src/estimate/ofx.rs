//! An OFX or QFX statement, SGML 1.x or XML 2.x alike: a leaf's text runs
//! from its tag to the next tag or line end, so a missing close tag reads
//! as the older form.

use jiff::civil::Date;

use super::bank::{BankError, Cents, Row};
use crate::plan::PlanDate;

const ROOT: &str = "<OFX>";
const TRANSACTION: &str = "<STMTTRN>";
const TRANSACTION_END: &str = "</STMTTRN>";
const POSTED: &str = "<DTPOSTED>";
const AMOUNT: &str = "<TRNAMT>";
const BANK_ID: &str = "<FITID>";
const NAME: &str = "<NAME>";
const MEMO: &str = "<MEMO>";
/// `YYYYMMDD`, which every OFX date starts with.
const DATE_DIGITS: usize = 8;

pub(super) fn rows(name: &str, text: &str) -> Result<Vec<Row>, BankError> {
    if !(text.contains(ROOT) || text.contains(TRANSACTION)) {
        return Err(BankError::NotOfx(name.to_owned()));
    }
    text.split(TRANSACTION)
        .skip(1)
        .enumerate()
        .map(|(index, chunk)| {
            let chunk = chunk.split_once(TRANSACTION_END).map_or(chunk, |(c, _)| c);
            row(name, index + 1, chunk)
        })
        .collect()
}

fn row(name: &str, row: usize, chunk: &str) -> Result<Row, BankError> {
    let malformed = |text: &str, expected| BankError::Malformed {
        name: name.to_owned(),
        row,
        text: text.to_owned(),
        expected,
    };
    let date_text = leaf(chunk, POSTED).unwrap_or_default();
    let on = date(date_text).ok_or_else(|| malformed(date_text, "date"))?;
    let amount_text = leaf(chunk, AMOUNT).unwrap_or_default();
    let amount = cents(amount_text).ok_or_else(|| malformed(amount_text, "amount"))?;
    Ok(Row {
        on,
        amount,
        payee: leaf(chunk, NAME).unwrap_or_default().to_owned(),
        memo: leaf(chunk, MEMO).map(str::to_owned),
        category: None,
        bank_id: leaf(chunk, BANK_ID).map(str::to_owned),
    })
}

/// The text after the first `tag` in `chunk`, up to the next tag or line
/// end.
fn leaf<'a>(chunk: &'a str, tag: &str) -> Option<&'a str> {
    let after = &chunk[chunk.find(tag)? + tag.len()..];
    let text = after
        .split(['<', '\n', '\r'])
        .next()
        .unwrap_or_default()
        .trim();
    (!text.is_empty()).then_some(text)
}

fn date(text: &str) -> Option<PlanDate> {
    let digits = text.get(..DATE_DIGITS)?;
    let year = digits[..4].parse().ok()?;
    let month = digits[4..6].parse().ok()?;
    let day = digits[6..].parse().ok()?;
    Date::new(year, month, day).ok().map(PlanDate)
}

/// An OFX amount, which a bank may write with a comma for the decimal
/// point.
fn cents(text: &str) -> Option<Cents> {
    if text.contains('.') {
        Cents::parse(text)
    } else {
        Cents::parse(&text.replace(',', "."))
    }
}
