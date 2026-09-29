//! The market tools as a page shows them: the plan through random markets
//! or every historical start year, how it fared, each run singled out with
//! its line of net worth, their spread, and what they were made under; and
//! a run replayed whole for the Ledger.

use retiretui_client::present::MoneyForm;
use retiretui_client::searches::markets::{
    self, Ending, Listed, Markets, NOTHING_SEARCHED, PLANNED, Zone, market_key, market_of,
    market_said, zone_of,
};
use retiretui_client::searches::page_refusal;
use retiretui_client::session::Projected;
use retiretui_engine::market::{self, Band, History, Progress, RunName};
use retiretui_engine::plan::{Dollars, Plan};
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::searches::gated;
use crate::vocabulary::slug_of;
use crate::{JsDocument, reply, tables, to_js};

/// What the plan's own row is kept by.
const PLANNED_KEY: &str = "planned";

/// A market tool's runs, as its page shows them.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MarketRuns {
    /// How the plan fared: "money lasts in 87% of 1,000".
    pub verdict: String,
    /// The share of runs it survives and of how many: "87% of 1,000".
    pub success: String,
    /// The zone its share falls in.
    pub zone: Zone,
    /// The share of runs that succeeded.
    pub success_rate: f64,
    /// How many runs there were.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub count: usize,
    /// The runs table's columns.
    pub columns: [&'static str; 3],
    /// The plan in its own market, then each run singled out.
    pub runs: Vec<RunRow>,
    /// Net worth year by year across the runs.
    pub bands: Vec<Band>,
    /// Net worth at each percentile year by year, where the tool shows it.
    pub by_year: Option<Table>,
    /// How many runs end in each bucket, the short first.
    pub endings: Vec<Ending>,
    /// What the runs are made under, and where each is edited.
    pub assumptions: Vec<AssumptionRow>,
}

/// A run in the table.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RunRow {
    /// What a highlight is kept by, which the same place keeps after a
    /// search again: `planned`, `p90`, `worst`, a start year.
    pub key: String,
    /// The market it went through, as the Ledger's address keeps it;
    /// `null` for the plan's own.
    pub market: Option<String>,
    /// What it shows under each column.
    pub cells: Vec<String>,
    /// Its net worth at each year's end, today's dollars.
    pub net_worth: Vec<Dollars>,
}

/// A table of text.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Table {
    /// Its columns.
    pub columns: Vec<String>,
    /// Its rows.
    pub rows: Vec<Vec<String>>,
}

/// What the runs were made under, and where it is edited.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AssumptionRow {
    /// What it is.
    pub label: &'static str,
    /// What the plan says of it.
    pub value: String,
    /// The domain's page address.
    pub domain: String,
    /// The field it is edited at, where it is one.
    pub field: Option<&'static str>,
}

/// What the market tools say that no search does.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MarketWords {
    /// What the runs say before their first search answers.
    pub nothing_searched: &'static str,
}

fn row(plan: &Plan, key: String, first: String, run: &market::Run) -> RunRow {
    RunRow {
        key,
        market: market_key(run.name),
        net_worth: run.net_worth.clone(),
        cells: markets::run_cells(plan, first, run, MoneyForm::Full),
    }
}

/// What `found` shows of the plan's runs.
fn runs_of<M: Markets>(plan: &Plan, found: &M) -> MarketRuns {
    let runs = found.runs();
    let planned = row(
        plan,
        PLANNED_KEY.to_owned(),
        PLANNED.to_owned(),
        &runs.planned,
    );
    let listed = found
        .listed()
        .into_iter()
        .map(|Listed { key, first, run }| row(plan, key, first, run));
    let by_year = M::HAS_BY_YEAR.then(|| {
        let (columns, rows) = markets::by_year(runs, MoneyForm::Full);
        Table { columns, rows }
    });
    let assumptions = markets::assumptions::<M>(plan)
        .into_iter()
        .map(|assumption| AssumptionRow {
            label: assumption.label,
            value: assumption.value,
            domain: slug_of(assumption.domain),
            field: assumption.field,
        })
        .collect();
    MarketRuns {
        verdict: format!("{} {}", M::HEADLINE, found.verdict()),
        success: found.verdict(),
        zone: zone_of(runs.success_rate()),
        success_rate: runs.success_rate(),
        count: runs.runs.len(),
        columns: markets::run_columns::<M>(),
        runs: std::iter::once(planned).chain(listed).collect(),
        bands: runs.bands.clone(),
        by_year,
        endings: markets::endings(runs),
        assumptions,
    }
}

/// The plan through the random markets its settings draw.
///
/// # Errors
///
/// Where the plan does not pass the gate, or cannot be run.
pub fn monte_carlo(text: &str) -> Result<MarketRuns, String> {
    let plan = gated(text)?;
    let found = market::monte_carlo(&plan, tables(), History::embedded(), &Progress::default())
        .map_err(page_refusal)?;
    Ok(runs_of(&plan, &found))
}

/// The plan from every historical start year, worst first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or cannot be run.
pub fn historical(text: &str) -> Result<MarketRuns, String> {
    let plan = gated(text)?;
    let runs = market::historical(&plan, tables(), History::embedded(), &Progress::default())
        .map_err(page_refusal)?;
    Ok(runs_of(&plan, &runs))
}

/// The market `key` names.
///
/// # Errors
///
/// Where no run could have gone through it.
pub(crate) fn market_named(key: &str) -> Result<RunName, String> {
    market_of(key).ok_or_else(|| format!("no market is called {key}"))
}

/// `own` projected whole through the market `name`.
///
/// # Errors
///
/// Where history cannot run from a start year.
pub(crate) fn replayed(own: &Projected, name: RunName) -> Result<Projected, String> {
    let projection = market::replay(&own.plan, tables(), History::embedded(), name)
        .ok_or_else(|| format!("{} cannot be replayed", market_said(name)))?;
    Ok(Projected {
        plan: own.plan.clone(),
        projection,
    })
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// The market `market` names, as the Ledger says it: `random market
    /// 423`, `retiring in 1929`.
    ///
    /// # Errors
    ///
    /// Where no run could have gone through it.
    #[wasm_bindgen(js_name = marketSaid)]
    pub fn market_said(&self, market: &str) -> Result<String, JsError> {
        market_named(market)
            .map(market_said)
            .map_err(|message| JsError::new(&message))
    }
}

/// The plan through the random markets its settings draw.
///
/// # Errors
///
/// Where the plan does not pass the gate, or cannot be run.
#[wasm_bindgen(js_name = monteCarlo, unchecked_return_type = "MarketRuns")]
pub fn js_monte_carlo(plan: &str) -> Result<JsValue, JsError> {
    reply(monte_carlo(plan))
}

/// The plan from every historical start year, worst first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or cannot be run.
#[wasm_bindgen(js_name = historical, unchecked_return_type = "MarketRuns")]
pub fn js_historical(plan: &str) -> Result<JsValue, JsError> {
    reply(historical(plan))
}

/// What the market tools say that no search does.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = marketWords, unchecked_return_type = "MarketWords")]
pub fn market_words() -> Result<JsValue, JsError> {
    to_js(&MarketWords {
        nothing_searched: NOTHING_SEARCHED,
    })
}

#[cfg(test)]
mod tests {
    use retiretui_client::setup::EXAMPLES;

    use super::*;

    fn starter() -> &'static str {
        EXAMPLES[0].2
    }

    #[test]
    fn monte_carlo_keys_its_runs_by_their_place() {
        let found = monte_carlo(starter()).expect("runs");
        let keys: Vec<&str> = found.runs.iter().map(|run| run.key.as_str()).collect();
        assert_eq!(
            keys,
            ["planned", "p90", "p75", "p50", "p25", "p10", "worst"]
        );
        assert_eq!(found.runs[0].market, None);
        let trial = found.runs[1].market.as_deref().expect("a trial");
        assert!(trial.starts_with("trial-"), "{trial}");
        assert!(
            found.verdict.starts_with("money lasts in "),
            "{}",
            found.verdict
        );
        assert!(found.by_year.is_some());
        assert_eq!(
            found.endings.iter().map(|each| each.count).sum::<u64>(),
            found.count as u64
        );
        assert_eq!(found.runs[1].net_worth.len(), found.bands.len());
    }

    #[test]
    fn historical_lists_its_start_years_worst_first_without_the_by_year_table() {
        let found = historical(starter()).expect("runs");
        assert!(found.by_year.is_none());
        assert!(found.verdict.starts_with("survived "), "{}", found.verdict);
        let first = &found.runs[1];
        assert_eq!(first.market.as_deref(), Some(first.key.as_str()));
        assert_eq!(found.runs.len(), found.count + 1, "every start year listed");
        let fields: Vec<Option<&str>> = found.assumptions.iter().map(|row| row.field).collect();
        assert!(fields.contains(&Some("historical.from")), "{fields:?}");
        assert_eq!(found.assumptions[0].domain, "market");
    }

    #[test]
    fn a_market_is_replayed_as_the_engine_replays_it() {
        let plan = gated(starter()).expect("valid");
        let own = Projected::new(plan, tables());
        let name = market_named("1929").expect("a start year");
        let replay = market::replay(&own.plan, tables(), History::embedded(), name);
        assert_eq!(Some(replayed(&own, name).expect("runs").projection), replay);
        assert!(market_named("p10").is_err());
        assert!(replayed(&own, RunName::Start(1700)).is_err());
    }
}
