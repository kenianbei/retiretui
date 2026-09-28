//! The projection as the Ledger and the Overview's charts show it: the
//! year table, a year's flows, income and tax, and the series charted.

use retiretui_client::actions::collect_warnings;
use retiretui_client::ledger::{
    AccountFlows, DetailLine, account_flows, income_and_tax, ledger_headers, salary_marks,
};
use retiretui_client::present::{compact_money, money, treatment_class};
use retiretui_client::replies::year_row;
use retiretui_client::session::Projected;
use retiretui_client::table::{
    Column, ages_text, basis_amount, percentile_label, present_classes, year_figures,
};
use retiretui_engine::plan::Dollars;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::domain::TableColumn;
use crate::{JsDocument, reply, tables};

/// The year and the ages, the columns that are not figures.
const TEXT_COLUMNS: usize = 2;

/// Every projected year as the Ledger's table shows it.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Ledger {
    /// Each column, in the order its cells are.
    pub columns: Vec<TableColumn>,
    /// Each year, first to last.
    pub rows: Vec<LedgerRow>,
}

/// One year of the Ledger's table.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LedgerRow {
    /// The calendar year.
    pub year: i16,
    /// Its cells, in column order.
    pub cells: Vec<String>,
    /// Whether the year could not pay for everything.
    pub is_exceeded: bool,
}

/// One year beside the Ledger's table.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct YearDetail {
    /// Each account the year touches, from its open to its close.
    pub flows: Vec<AccountFlows>,
    /// The year's income by source.
    pub income: Vec<DetailLine>,
    /// Its spending and what it paid besides.
    pub paid: Vec<DetailLine>,
    /// What needs attention, on the same basis.
    pub warnings: Vec<String>,
}

/// The projection as the Overview charts it.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChartSeries {
    /// The treatment classes the plan uses, named, in the order each
    /// year's `classes` are.
    pub classes: Vec<&'static str>,
    /// Each year, first to last.
    pub years: Vec<ChartYear>,
    /// The years each earner's salary ends.
    pub marks: Vec<ChartMark>,
}

/// One year's charted figures.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChartYear {
    /// The calendar year.
    pub year: i16,
    /// What each treatment class holds.
    pub classes: Vec<Dollars>,
    /// Everything the household holds.
    pub net_worth: Dollars,
    /// Everything it earned.
    pub income: Dollars,
    /// Everything it paid in tax.
    pub taxes: Dollars,
}

/// A year a chart points out.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChartMark {
    /// The calendar year.
    pub year: i16,
    /// What happens in it.
    pub label: String,
}

pub fn ledger(projected: &Projected, is_nominal: bool) -> Ledger {
    let classes = present_classes(&projected.plan);
    let columns = ledger_headers(&classes)
        .into_iter()
        .enumerate()
        .map(|(at, header)| TableColumn {
            header,
            is_numeric: at >= TEXT_COLUMNS,
        });
    let balances: Vec<Column> = classes.into_iter().map(Column::Class).collect();
    let rows = projected.projection.years.iter().map(|row| {
        let figures = year_figures(row, &balances).into_iter();
        let figures = figures.map(|amount| money(basis_amount(amount, row.deflator, is_nominal)));
        let leading = [row.year.to_string(), ages_text(&projected.plan, row)];
        LedgerRow {
            year: row.year,
            cells: leading.into_iter().chain(figures).collect(),
            is_exceeded: row.unfunded > 0,
        }
    });
    Ledger {
        columns: columns.collect(),
        rows: rows.collect(),
    }
}

pub fn year_detail(
    projected: &Projected,
    year: i16,
    is_nominal: bool,
) -> Result<YearDetail, String> {
    let Projected { plan, projection } = projected;
    let row = year_row(projection, year)?;
    let previous = projection.row(year - 1);
    let (income, paid) = income_and_tax(plan, row, is_nominal);
    let deflating = (!is_nominal).then_some(projection);
    Ok(YearDetail {
        flows: account_flows(plan, previous, row, is_nominal),
        income,
        paid,
        warnings: collect_warnings(plan, tables(), row, deflating),
    })
}

pub fn chart(projected: &Projected, is_nominal: bool) -> ChartSeries {
    let classes = present_classes(&projected.plan);
    let years = projected.projection.years.iter().map(|row| {
        let at_basis = |amount| basis_amount(amount, row.deflator, is_nominal);
        ChartYear {
            year: row.year,
            classes: classes
                .iter()
                .map(|&class| at_basis(row.class_totals.get(class)))
                .collect(),
            net_worth: at_basis(row.net_worth),
            income: at_basis(row.total_income),
            taxes: at_basis(row.taxes.total),
        }
    });
    let marks = salary_marks(projected).into_iter();
    ChartSeries {
        classes: classes
            .iter()
            .map(|&class| treatment_class(class))
            .collect(),
        years: years.collect(),
        marks: marks
            .map(|(label, year)| ChartMark { year, label })
            .collect(),
    }
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// Every projected year as the Ledger's table shows it, nominal or in
    /// today's dollars.
    ///
    /// # Errors
    ///
    /// Where no valid draft has been projected.
    #[wasm_bindgen(unchecked_return_type = "Ledger")]
    pub fn ledger(&self, nominal: bool) -> Result<JsValue, JsError> {
        reply(
            self.0
                .projected()
                .map(|projected| ledger(projected, nominal)),
        )
    }

    /// `year`'s flows, income, payments and warnings, nominal or in
    /// today's dollars.
    ///
    /// # Errors
    ///
    /// Where no valid draft has been projected, or `year` is outside it.
    #[wasm_bindgen(js_name = yearDetail, unchecked_return_type = "YearDetail")]
    pub fn year_detail(&self, year: i16, nominal: bool) -> Result<JsValue, JsError> {
        reply(
            self.0
                .projected()
                .and_then(|projected| year_detail(projected, year, nominal)),
        )
    }

    /// The projection as the Overview charts it, nominal or in today's
    /// dollars.
    ///
    /// # Errors
    ///
    /// Where no valid draft has been projected.
    #[wasm_bindgen(unchecked_return_type = "ChartSeries")]
    pub fn chart(&self, nominal: bool) -> Result<JsValue, JsError> {
        reply(
            self.0
                .projected()
                .map(|projected| chart(projected, nominal)),
        )
    }
}

/// `amount` in whole dollars with separators, as every surface writes it.
#[wasm_bindgen(js_name = money)]
#[must_use]
pub fn js_money(amount: f64) -> String {
    money(whole(amount))
}

/// `amount` shortened to fit an axis: `$2.58M`, `-$42k`.
#[wasm_bindgen(js_name = compactMoney)]
#[must_use]
pub fn js_compact_money(amount: f64) -> String {
    compact_money(whole(amount))
}

/// How the market at `percentile` is named.
#[wasm_bindgen(js_name = percentileLabel)]
#[must_use]
pub fn js_percentile_label(percentile: u8) -> String {
    percentile_label(percentile)
}

/// A chart's figure as whole dollars; a figure past what a plan can hold is
/// held at the limit.
fn whole(amount: f64) -> Dollars {
    amount.round() as Dollars
}

#[cfg(test)]
mod tests {
    use retiretui_client::setup::EXAMPLES;
    use retiretui_engine::plan::Plan;
    use retiretui_engine::project::project;

    use super::*;

    fn projected() -> Projected {
        let plan = Plan::from_toml_str(EXAMPLES[0].2).expect("parses");
        let projection = project(&plan, tables());
        Projected { plan, projection }
    }

    #[test]
    fn the_ledger_has_a_row_a_year_and_a_cell_a_column() {
        let projected = projected();
        let ledger = ledger(&projected, true);
        assert_eq!(ledger.rows.len(), projected.projection.years.len());
        assert!(
            ledger
                .rows
                .iter()
                .all(|row| row.cells.len() == ledger.columns.len())
        );
        assert_eq!(ledger.columns[0].header, "Year");
        assert!(!ledger.columns[1].is_numeric && ledger.columns[2].is_numeric);
    }

    #[test]
    fn a_year_s_detail_is_the_client_s_on_the_basis_asked() {
        let projected = projected();
        let year = projected.projection.years[3].year;
        let detail = year_detail(&projected, year, false).expect("in range");
        let row = &projected.projection.years[3];
        let previous = Some(&projected.projection.years[2]);
        let flows = account_flows(&projected.plan, previous, row, false);
        assert_eq!(detail.flows, flows);
        assert!(year_detail(&projected, year - 99, false).is_err());
    }

    #[test]
    fn charted_figures_follow_the_basis() {
        let projected = projected();
        let nominal = chart(&projected, true);
        let today = chart(&projected, false);
        assert_eq!(nominal.classes.len(), nominal.years[0].classes.len());
        let pairs = nominal.years.iter().zip(&today.years);
        assert!(
            pairs
                .clone()
                .all(|(nominal, today)| nominal.year == today.year)
        );
        assert!(
            pairs
                .clone()
                .any(|(nominal, today)| nominal.net_worth != today.net_worth)
        );
    }
}
