//! The projection as the Ledger and the Overview's charts show it: the
//! year table under a column set, one year in full, and the series charted.

use retiretui_client::actions::NOTHING_SCHEDULED;
use retiretui_client::ledger::{
    Asked, ColumnSet, FLOW_COLUMNS, FLOWS, MONEY_IN, MONEY_OUT, Marks, PAID, TAX, TO_DO, TO_WATCH,
    Table, WORKED_FROM, YEARS, Year, salary_marks,
};
use retiretui_client::overview::{
    ATTENTION, Chart, MILESTONES, NOTHING, OVER_THE_PLAN, RESTS_ON, STALE, STRIP,
};
use retiretui_client::present::{MoneyForm, basis_name, compact_money, money, treatment_word};
use retiretui_client::searches::markets::{market_of, market_said};
use retiretui_client::session::Projected;
use retiretui_client::table::{basis_amount, percentile_label, present_classes};
use retiretui_engine::market::BAND_PERCENTILES;
use retiretui_engine::plan::Dollars;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::{Bases, JsDocument, reply, tables, to_js};

/// Every projected year as the Ledger's table shows it.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Ledger {
    /// What heads the year and the ages reached in it.
    pub text_headers: [&'static str; 2],
    /// What heads each figure: the `leading` every set opens with, then
    /// the set's own, then net worth.
    pub figure_headers: Vec<String>,
    /// How many figures every set opens with.
    pub leading: usize,
    /// Each year, first to last.
    pub rows: Vec<LedgerRow>,
}

/// One year of the Ledger's table.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LedgerRow {
    /// The calendar year.
    pub year: i16,
    /// The ages reached in it.
    pub ages: String,
    /// What sets it apart in the table of years.
    pub marks: Marks,
    /// Its figures, in the order of their headers.
    pub figures: Vec<String>,
    /// Whether the year could not pay for everything.
    pub is_exceeded: bool,
}

/// The projection as the Overview charts it.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChartSeries {
    /// The treatment classes the plan uses, named as a chart keys them, in
    /// the order each year's `classes` are.
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

/// The column set an address names; the one the table opens under for a
/// name that is none.
fn column_set(named: Option<&str>) -> ColumnSet {
    let mut sets = ColumnSet::ALL.into_iter();
    let named = sets.find(|set| Some(set.slug()) == named);
    named.unwrap_or_default()
}

pub fn ledger(projected: &Projected, set: ColumnSet, is_nominal: bool) -> Ledger {
    let table = Table::new(projected, tables(), set, is_nominal);
    let rows = table.rows.into_iter().map(|row| LedgerRow {
        year: row.year,
        ages: row.ages,
        marks: row.marks,
        figures: row.figures.into_iter().map(money).collect(),
        is_exceeded: row.is_exceeded,
    });
    Ledger {
        text_headers: table.text_headers,
        figure_headers: table.figure_headers,
        leading: Table::LEADING,
        rows: rows.collect(),
    }
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
        classes: classes.iter().map(|&class| treatment_word(class)).collect(),
        years: years.collect(),
        marks: marks
            .map(|(label, year)| ChartMark { year, label })
            .collect(),
    }
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// Every projected year as the Ledger's table shows it under the
    /// column set `columns` names, nominal or in today's dollars, in the
    /// plan's own market or the one `market` names; `null` while no valid
    /// draft has been projected.
    ///
    /// # Errors
    ///
    /// Where `market` names no market the plan can be replayed in, or the
    /// table does not convert.
    #[wasm_bindgen(unchecked_return_type = "Ledger | null")]
    pub fn ledger(
        &self,
        nominal: bool,
        market: Option<String>,
        columns: Option<String>,
    ) -> Result<JsValue, JsError> {
        if self.0.projected().is_err() {
            return Ok(JsValue::NULL);
        }
        let set = column_set(columns.as_deref());
        reply(self.0.in_market(market.as_deref(), |projected| {
            Ok(ledger(projected, set, nominal))
        }))
    }

    /// `year` in full, nominal or in today's dollars, in the plan's own
    /// market or the one `market` names, which its title then names and
    /// whose year says no bracket.
    ///
    /// # Errors
    ///
    /// Where no valid draft has been projected, `market` names no market it
    /// can be replayed in, or `year` is outside it.
    #[wasm_bindgen(js_name = ledgerYear, unchecked_return_type = "Year")]
    pub fn ledger_year(
        &self,
        year: i16,
        nominal: bool,
        market: Option<String>,
    ) -> Result<JsValue, JsError> {
        let run = market.as_deref().and_then(market_of).map(market_said);
        let asked = Asked {
            year,
            is_nominal: nominal,
            run: run.as_deref(),
        };
        reply(self.0.in_market(market.as_deref(), |projected| {
            let said = Year::new(projected, tables(), asked);
            said.ok_or_else(|| format!("{year} is outside the plan's years"))
        }))
    }

    /// The projection as the Overview charts it, nominal or in today's
    /// dollars; `null` while no valid draft has been projected.
    ///
    /// # Errors
    ///
    /// Where the series does not convert.
    #[wasm_bindgen(unchecked_return_type = "ChartSeries | null")]
    pub fn chart(&self, nominal: bool) -> Result<JsValue, JsError> {
        let projected = self.0.projected().ok();
        to_js(&projected.map(|projected| chart(projected, nominal)))
    }
}

/// `amount` shortened to fit an axis: `$2.58M`, `-$42k`.
#[wasm_bindgen(js_name = compactMoney)]
#[must_use]
pub fn js_compact_money(amount: f64) -> String {
    compact_money(whole(amount))
}

/// `amount` in full: `$4,437,120`.
#[wasm_bindgen(js_name = money)]
#[must_use]
pub fn js_money(amount: f64) -> String {
    money(whole(amount))
}

/// A difference in full, signed either way: `+$12,345`, `-$50`.
#[wasm_bindgen(js_name = signedMoney)]
#[must_use]
pub fn js_signed_money(amount: f64) -> String {
    MoneyForm::Full.signed(whole(amount))
}

/// The percentiles each band's net worth is at, lowest first.
///
/// # Errors
///
/// Where they do not convert.
#[wasm_bindgen(
    js_name = bandPercentiles,
    unchecked_return_type = "[number, number, number, number, number]"
)]
pub fn band_percentiles() -> Result<JsValue, JsError> {
    to_js(&BAND_PERCENTILES)
}

/// How the market at `percentile` is named.
#[wasm_bindgen(js_name = percentileLabel)]
#[must_use]
pub fn js_percentile_label(percentile: u8) -> String {
    percentile_label(percentile)
}

/// What the Overview, the Ledger and Compare call what they show.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ViewWords {
    /// The dollars figures are shown in.
    pub basis: Bases<&'static str>,
    /// The Ledger's table of years.
    pub years: &'static str,
    /// The year's flows through each account.
    pub flows: &'static str,
    /// The flows table's columns, each beside whether it holds figures.
    pub flow_columns: [(&'static str, bool); 5],
    /// Where a year's money came from.
    pub money_in: &'static str,
    /// Where it went.
    pub money_out: &'static str,
    /// The year's tax.
    pub tax: &'static str,
    /// What a year has the household do.
    pub to_do: &'static str,
    /// What to watch in a year.
    pub to_watch: &'static str,
    /// The tax a year paid.
    pub paid: &'static str,
    /// What a year's tax was worked out from.
    pub worked_from: &'static str,
    /// The year table's column sets in the order they are turned through,
    /// each as an address names it and as a heading says it.
    pub column_sets: Vec<(&'static str, &'static str)>,
    /// The Overview's charts in the order they are turned through, each
    /// beside its title.
    pub charts: Vec<(Chart, &'static str)>,
    /// The Overview's strip: how long the money lasts, how surely, the
    /// least it holds once it stops earning, and what it ends with.
    pub strip: [&'static str; 4],
    /// The Overview's lifetime totals.
    pub over_the_plan: &'static str,
    /// The Overview's assumptions.
    pub rests_on: &'static str,
    /// What is said of the figures while the draft has issues.
    pub stale: &'static str,
    /// The Overview's list of what needs attention.
    pub attention: &'static str,
    /// What it says where nothing does.
    pub nothing_wanting: &'static str,
    /// The Overview's list of the plan's milestones.
    pub milestones: &'static str,
    /// What a year with nothing to do says.
    pub nothing_scheduled: &'static str,
}

/// What the Overview, the Ledger and Compare call what they show.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = viewWords, unchecked_return_type = "ViewWords")]
pub fn view_words() -> Result<JsValue, JsError> {
    to_js(&ViewWords {
        basis: Bases::of(basis_name),
        years: YEARS,
        flows: FLOWS,
        flow_columns: FLOW_COLUMNS,
        money_in: MONEY_IN,
        money_out: MONEY_OUT,
        tax: TAX,
        to_do: TO_DO,
        to_watch: TO_WATCH,
        paid: PAID,
        worked_from: WORKED_FROM,
        column_sets: (ColumnSet::ALL.iter())
            .map(|set| (set.slug(), set.title()))
            .collect(),
        charts: Chart::ALL.map(|chart| (chart, chart.title())).to_vec(),
        strip: STRIP,
        over_the_plan: OVER_THE_PLAN,
        rests_on: RESTS_ON,
        stale: STALE,
        attention: ATTENTION,
        nothing_wanting: NOTHING,
        milestones: MILESTONES,
        nothing_scheduled: NOTHING_SCHEDULED,
    })
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

    use super::*;

    fn projected() -> Projected {
        let plan = Plan::from_toml_str(EXAMPLES[0].2).expect("parses");
        Projected::new(plan, tables())
    }

    #[test]
    fn the_ledger_has_a_row_a_year_and_a_figure_a_header_under_every_set() {
        let projected = projected();
        for set in ColumnSet::ALL {
            let ledger = ledger(&projected, set, true);
            assert_eq!(ledger.rows.len(), projected.projection.years.len());
            let figures = ledger.figure_headers.len();
            assert!(ledger.rows.iter().all(|row| row.figures.len() == figures));
            assert_eq!(ledger.text_headers, ["Year", "Age"]);
            assert_eq!(ledger.figure_headers[ledger.leading - 1], "Withdrawn");
            let first = &ledger.rows[0];
            assert_eq!(first.year, projected.projection.years[0].year);
            assert!(first.figures[0].starts_with('$'), "{first:?}");
        }
        let todays = ledger(&projected, ColumnSet::Tax, false);
        let nominal = ledger(&projected, ColumnSet::Tax, true);
        assert_ne!(todays.rows[9].figures, nominal.rows[9].figures);
    }

    #[test]
    fn a_column_set_is_read_from_its_name_and_any_other_is_the_first() {
        assert_eq!(column_set(Some("tax")), ColumnSet::Tax);
        assert_eq!(column_set(Some("accounts")), ColumnSet::Accounts);
        assert_eq!(column_set(Some("balances")), ColumnSet::Treatments);
        assert_eq!(column_set(None), ColumnSet::Treatments);
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
