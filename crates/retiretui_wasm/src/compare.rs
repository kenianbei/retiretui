//! Plans compared side by side as the Compare page shows them: each
//! plan's figures, or against a baseline's; a metric year by year; and
//! what one plan changes of another.

use retiretui_client::compare::{
    self, Figured, Success, THE_BASELINE, THE_SAME, UNREACHED, amounts, figure, less,
};
use retiretui_client::metric::Metric;
use retiretui_client::present::MoneyForm;
use retiretui_client::session::{Projected, Today, YearCursor, span};
use retiretui_engine::plan::Dollars;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::{JsDocument, from_js, refused, reply, to_js};

/// What the page shows the plans in.
#[derive(Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CompareView {
    /// In the dollars of each year, else today's.
    pub nominal: bool,
    /// The metric charted and tabled year by year.
    pub metric: Metric,
    /// The year the table's year column is for.
    pub year: i16,
}

/// What a plan's search through random markets has found so far.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(tag = "kind", rename_all = "lowercase"))]
pub enum Searched {
    /// Not answered yet.
    Waiting,
    /// The share of runs the money lasted through.
    Rate {
        /// The share, from 0 to 1.
        rate: f64,
    },
    /// The search was refused or failed.
    Failed,
}

impl From<Searched> for Success {
    fn from(searched: Searched) -> Self {
        match searched {
            Searched::Waiting => Self::Waiting,
            Searched::Rate { rate } => Self::Rate(rate),
            Searched::Failed => Self::Failed,
        }
    }
}

/// A plan's metric in one year.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct YearFigure {
    /// The calendar year.
    pub year: i16,
    /// The amount, or its difference from the baseline's.
    pub amount: Dollars,
    /// The amount as the table says it.
    pub figure: String,
}

/// A metric the page offers.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MetricChoice {
    /// What the address keeps it by.
    pub key: Metric,
    /// What it is called.
    pub title: &'static str,
}

/// What the Compare page says that no plan does.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CompareWords {
    /// What the baseline changes of itself.
    pub the_baseline: &'static str,
    /// What a plan no different from the baseline changes of it.
    pub the_same: &'static str,
    /// A year a plan does not reach.
    pub unreached: &'static str,
    /// Every metric, in the order they are offered.
    pub metrics: Vec<MetricChoice>,
}

/// `own`'s metric year by year, less `base`'s where there is one.
fn metric_by_year(own: &Projected, base: Option<&Projected>, view: CompareView) -> Vec<YearFigure> {
    let of =
        |projected: &Projected| amounts(&projected.projection.years, view.metric, view.nominal);
    let own = of(own);
    let shown = match base {
        Some(base) => less(&own, &of(base)),
        None => own,
    };
    (shown.into_iter())
        .map(|(year, amount)| YearFigure {
            year,
            amount,
            figure: figure(Some(amount), base.is_some(), MoneyForm::Full),
        })
        .collect()
}

/// `own`'s row of figures, each against `base`'s where there is one.
fn plan_figures(
    own: (&Projected, Success),
    base: Option<(&Projected, Success)>,
    view: CompareView,
) -> Vec<String> {
    let deflated = !view.nominal;
    let year = view.year;
    let by_year = metric_by_year(own.0, base.map(|(projected, _)| projected), view);
    let in_year = by_year.iter().find(|shown| shown.year == year);
    let figured = |(projected, success): (&Projected, Success), in_year: String| Figured {
        summary: projected.projection.summary(deflated),
        success,
        in_year,
    };
    let own = figured(
        own,
        figure(
            in_year.map(|shown| shown.amount),
            base.is_some(),
            MoneyForm::Compact,
        ),
    );
    let base = base.map(|base| figured(base, String::new()));
    compare::cells(&own, base.as_ref())
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// The plan's row of figures in `view`, `searched` its success.
    ///
    /// # Errors
    ///
    /// Where the plan has issues, or an argument does not convert.
    #[wasm_bindgen(js_name = planFigures)]
    pub fn plan_figures(
        &self,
        #[wasm_bindgen(unchecked_param_type = "CompareView")] view: JsValue,
        #[wasm_bindgen(unchecked_param_type = "Searched")] searched: JsValue,
    ) -> Result<Vec<String>, JsError> {
        let own = (self.0.projected().map_err(refused)?, success_of(searched)?);
        Ok(plan_figures(own, None, from_js(view)?))
    }

    /// The plan's row of figures in `view`, `searched` its success, each
    /// against `base`'s, searched as `base_searched`.
    ///
    /// # Errors
    ///
    /// Where either plan has issues, or an argument does not convert.
    #[wasm_bindgen(js_name = planFiguresAgainst)]
    pub fn plan_figures_against(
        &self,
        #[wasm_bindgen(unchecked_param_type = "CompareView")] view: JsValue,
        #[wasm_bindgen(unchecked_param_type = "Searched")] searched: JsValue,
        base: &JsDocument,
        #[wasm_bindgen(unchecked_param_type = "Searched")] base_searched: JsValue,
    ) -> Result<Vec<String>, JsError> {
        let own = (self.0.projected().map_err(refused)?, success_of(searched)?);
        let base = (
            base.0.projected().map_err(refused)?,
            success_of(base_searched)?,
        );
        Ok(plan_figures(own, Some(base), from_js(view)?))
    }

    /// The plan's metric in `view` year by year.
    ///
    /// # Errors
    ///
    /// Where the plan has issues, or the view does not convert.
    #[wasm_bindgen(js_name = byYear, unchecked_return_type = "YearFigure[]")]
    pub fn by_year(
        &self,
        #[wasm_bindgen(unchecked_param_type = "CompareView")] view: JsValue,
    ) -> Result<JsValue, JsError> {
        let view = from_js(view)?;
        reply(
            self.0
                .projected()
                .map(|own| metric_by_year(own, None, view)),
        )
    }

    /// The plan's metric in `view` year by year, less `base`'s, in the
    /// years both reach.
    ///
    /// # Errors
    ///
    /// Where either plan has issues, or the view does not convert.
    #[wasm_bindgen(js_name = byYearAgainst, unchecked_return_type = "YearFigure[]")]
    pub fn by_year_against(
        &self,
        #[wasm_bindgen(unchecked_param_type = "CompareView")] view: JsValue,
        base: &JsDocument,
    ) -> Result<JsValue, JsError> {
        let view = from_js(view)?;
        let (own, base) = (self.0.projected(), base.0.projected());
        reply(own.and_then(|own| Ok(metric_by_year(own, Some(base?), view))))
    }

    /// What this plan changes of `base`, a line per change; none where it
    /// changes nothing.
    ///
    /// # Errors
    ///
    /// Where either plan has issues or cannot be compared.
    #[wasm_bindgen(js_name = changesFrom)]
    pub fn changes_from(&self, base: &JsDocument) -> Result<Vec<String>, JsError> {
        let (own, base) = (
            self.0.projected().map_err(refused)?,
            base.0.projected().map_err(refused)?,
        );
        compare::changes(&base.plan, &own.plan).map_err(refused)
    }

    /// The first and last years the plan reaches; `undefined` while it has
    /// issues.
    #[must_use]
    pub fn years(&self) -> Option<Vec<i16>> {
        let (first, last) = span(&self.0.projection()?.years);
        Some(vec![first, last])
    }
}

/// The year the page shows: `requested`, or `today` held within the
/// document's `planned` years where none, then within the years any
/// compared plan reaches, `shows` - each a first and last year, any year
/// at all where empty.
#[wasm_bindgen(js_name = yearAmong)]
#[must_use]
pub fn year_among(requested: Option<i16>, today: i16, planned: &[i16], shows: &[i16]) -> i16 {
    let pair = |years: &[i16]| match (years.first(), years.last()) {
        (Some(&first), Some(&last)) => (first, last),
        _ => (i16::MIN, i16::MAX),
    };
    YearCursor(requested).resolve(Today(today), pair(planned), pair(shows))
}

/// The plans table's headers: the plan's name, then each figure's, the
/// charted `metric`'s in `year` naming both.
///
/// # Errors
///
/// Where the metric does not convert.
#[wasm_bindgen(js_name = compareHeaders)]
pub fn compare_headers(
    #[wasm_bindgen(unchecked_param_type = "Metric")] metric: JsValue,
    year: i16,
) -> Result<Vec<String>, JsError> {
    Ok(compare::headers(from_js(metric)?, year))
}

/// What the Compare page says that no plan does.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = compareWords, unchecked_return_type = "CompareWords")]
pub fn compare_words() -> Result<JsValue, JsError> {
    let metrics = (Metric::ALL.iter())
        .map(|&key| MetricChoice {
            key,
            title: key.title(),
        })
        .collect();
    to_js(&CompareWords {
        the_baseline: THE_BASELINE,
        the_same: THE_SAME,
        unreached: UNREACHED,
        metrics,
    })
}

fn success_of(searched: JsValue) -> Result<Success, JsError> {
    from_js::<Searched>(searched).map(Success::from)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use retiretui_client::setup::EXAMPLES;

    use super::*;
    use crate::document::Document;

    const RICHER: &str =
        "schema = 1\nbase = \"plan.toml\"\n\n[[accounts]]\nid = \"checking\"\nbalance = 50000\n";

    fn opened(path: &str) -> Document {
        let mut read = |file: &Path| match file.to_str() {
            Some("/plan.toml") => Ok(EXAMPLES[0].2.to_owned()),
            Some("/richer.toml") => Ok(RICHER.to_owned()),
            _ => Err("no such file".to_owned()),
        };
        Document::open(path, &mut read).expect("opens")
    }

    fn view() -> CompareView {
        CompareView {
            nominal: false,
            metric: Metric::NetWorth,
            year: 2030,
        }
    }

    #[test]
    fn a_plan_against_the_baseline_reads_as_its_differences() {
        let (plan, richer) = (opened("/plan.toml"), opened("/richer.toml"));
        let (plan, richer) = (
            plan.projected().expect("plan"),
            richer.projected().expect("richer"),
        );
        let own = plan_figures((richer, Success::Rate(0.95)), None, view());
        let against = plan_figures(
            (richer, Success::Rate(0.95)),
            Some((plan, Success::Rate(0.9))),
            view(),
        );
        assert_eq!(own.len(), compare::COLUMNS.len());
        assert!(own[0].starts_with('$'), "{own:?}");
        assert!(against[0].starts_with('+'), "{against:?}");
        assert_eq!(against[2], "+5.0 pts");
        let itself = plan_figures(
            (plan, Success::Waiting),
            Some((plan, Success::Waiting)),
            view(),
        );
        assert_eq!(itself[0], "$0");
    }

    #[test]
    fn a_metric_by_year_less_the_baseline_is_zero_against_itself() {
        let plan = opened("/plan.toml");
        let plan = plan.projected().expect("plan");
        let own = metric_by_year(plan, None, view());
        let itself = metric_by_year(plan, Some(plan), view());
        assert_eq!(own.len(), plan.projection.years.len());
        assert!(itself.iter().all(|shown| shown.amount == 0));
    }

    #[test]
    fn a_scenario_changes_its_base_by_its_overlay_alone() {
        let (plan, richer) = (opened("/plan.toml"), opened("/richer.toml"));
        let (plan, richer) = (
            plan.projected().expect("plan"),
            richer.projected().expect("richer"),
        );
        let changes = compare::changes(&plan.plan, &richer.plan).expect("compared");
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert!(changes[0].contains("Checking"), "{changes:?}");
        assert!(
            compare::changes(&plan.plan, &plan.plan)
                .expect("same")
                .is_empty()
        );
    }

    #[test]
    fn the_year_shown_is_held_within_every_plan_reached() {
        assert_eq!(year_among(None, 2026, &[2026, 2080], &[2026, 2090]), 2026);
        assert_eq!(
            year_among(Some(2085), 2026, &[2026, 2080], &[2026, 2090]),
            2085
        );
        assert_eq!(
            year_among(Some(2100), 2026, &[2026, 2080], &[2026, 2090]),
            2090
        );
        assert_eq!(year_among(Some(2031), 2026, &[], &[]), 2031);
    }
}
