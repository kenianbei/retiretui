use retiretui_engine::plan::{Dollars, Plan, TreatmentClass};
use retiretui_engine::project::{Summary, YearRow, deflate};

/// Column labels matching [`summary_cells`], shared by every table that
/// renders a [`Summary`] row.
pub const SUMMARY_LABELS: [&str; 9] = [
    "final net",
    "peak",
    "peak yr",
    "taxes",
    "medicare",
    "conversions",
    "unfunded",
    "first unf",
    "deferred",
];

/// A table with one summary per row: `leading` labels head the cells each
/// row brings of its own, [`SUMMARY_LABELS`] the figures after them.
pub fn summary_table(leading: &[&str], rows: Vec<(Vec<String>, Summary)>) -> String {
    let header: Vec<String> = leading
        .iter()
        .chain(SUMMARY_LABELS.iter())
        .map(|&label| label.to_owned())
        .collect();
    let rows: Vec<Vec<String>> = rows
        .into_iter()
        .map(|(mut cells, summary)| {
            cells.extend(summary_cells(&summary));
            cells
        })
        .collect();
    align(&header, &rows)
}

/// One table row of a summary's figures, in [`SUMMARY_LABELS`] order.
pub fn summary_cells(summary: &Summary) -> Vec<String> {
    vec![
        plain_dollars(summary.final_net_worth),
        plain_dollars(summary.peak_net_worth),
        summary.peak_year.to_string(),
        plain_dollars(summary.lifetime_taxes),
        plain_dollars(summary.lifetime_medicare),
        plain_dollars(summary.lifetime_conversions),
        plain_dollars(summary.lifetime_unfunded),
        summary
            .first_unfunded_year
            .map_or_else(|| "-".to_owned(), |year| year.to_string()),
        plain_dollars(summary.final_deferred),
    ]
}

const THOUSANDS: usize = 3;

/// Whole dollars with separators, as a person reads them: `$450,000`,
/// `-$5,000`.
pub fn money(amount: Dollars) -> String {
    let sign = if amount < 0 { "-" } else { "" };
    format!("{sign}${}", grouped(amount.unsigned_abs()))
}

/// A count with separators, as a person reads it: `1,000`.
pub fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / THOUSANDS);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(THOUSANDS) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// An account by its display name, or by its id where it states none.
pub fn account_name<'a>(plan: &'a Plan, id: &'a str) -> &'a str {
    let named = plan.accounts.iter().find(|account| account.id == id);
    named
        .and_then(|account| account.name.as_deref())
        .unwrap_or(id)
}

/// An event by its display name, or by its id where it states none.
pub fn event_name<'a>(plan: &'a Plan, id: &'a str) -> &'a str {
    let named = plan.events.iter().find(|event| event.id == id);
    named.and_then(|event| event.name.as_deref()).unwrap_or(id)
}

/// An income by its display name, or by its id where it states none.
pub fn income_name<'a>(plan: &'a Plan, id: &'a str) -> &'a str {
    plan.income_source(id)
        .and_then(|income| income.name.as_deref())
        .unwrap_or(id)
}

const PERCENT: f64 = 100.0;

/// A rate as a percent, to the places it needs: `2.5%`, `6%`.
pub fn rate(rate: f64) -> String {
    let percent = format!("{:.4}", rate * PERCENT);
    let trimmed = percent.trim_end_matches('0').trim_end_matches('.');
    format!("{trimmed}%")
}

pub fn plain_dollars(amount: Dollars) -> String {
    amount.to_string()
}

/// The year's ages joined `/` in household order; a missing person shows 0.
pub fn ages_text(plan: &Plan, row: &YearRow) -> String {
    let ages: Vec<String> = plan
        .household
        .people
        .iter()
        .map(|person| row.ages.get(&person.id).copied().unwrap_or(0).to_string())
        .collect();
    ages.join("/")
}

/// The treatment classes the plan actually uses, in the order every
/// surface shows them.
pub fn present_classes(plan: &Plan) -> Vec<TreatmentClass> {
    TreatmentClass::ALL
        .iter()
        .copied()
        .filter(|class| {
            plan.accounts
                .iter()
                .any(|account| account.treatment() == *class)
        })
        .collect()
}

/// A ledger's balance column: one account, or a treatment class's total.
#[derive(Debug)]
pub enum Column {
    Account(String),
    Class(TreatmentClass),
}

/// A year's ledger figures, in the order every ledger shows them: income,
/// spending, tax, withdrawn, each of `columns`, then net worth.
pub fn year_figures(row: &YearRow, columns: &[Column]) -> Vec<Dollars> {
    let mut figures = vec![
        row.total_income,
        row.expenses,
        row.taxes.total,
        row.total_withdrawals(),
    ];
    figures.extend(columns.iter().map(|column| match column {
        Column::Account(id) => row.balances.get(id).copied().unwrap_or(0),
        Column::Class(class) => row.class_totals.get(*class),
    }));
    figures.push(row.net_worth);
    figures
}

/// The amount on the displayed dollar basis: nominal as written, otherwise
/// deflated to today's dollars.
pub fn basis_amount(amount: Dollars, deflator: f64, nominal: bool) -> Dollars {
    if nominal {
        amount
    } else {
        deflate(amount, deflator)
    }
}

pub fn display_dollars(amount: Dollars, deflator: f64, nominal: bool) -> String {
    basis_amount(amount, deflator, nominal).to_string()
}

pub fn align(header: &[String], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = header.iter().map(String::len).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }
    let mut lines = Vec::with_capacity(rows.len() + 1);
    for cells in std::iter::once(header).chain(rows.iter().map(Vec::as_slice)) {
        let line: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(i, cell)| format!("{cell:>width$}", width = widths[i]))
            .collect();
        lines.push(line.join("  ").trim_end().to_owned());
    }
    lines.join("\n") + "\n"
}
