//! The Overview in the client's words: the strip's readings, the sentence
//! for a plan that runs short, what needs attention and the milestones.

use retiretui_client::overview::{Row, attention, milestones};
use retiretui_client::present::{compact_money, money_lasts, runs_short};
use retiretui_client::session::Projected;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Place;
use crate::vocabulary::slug_of;
use crate::{JsDocument, to_js};

/// A row of Needs attention or Milestones.
#[derive(Serialize, PartialEq, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OverviewRow {
    /// What it says, without its year.
    pub text: String,
    /// The year it is about, where it is about one.
    pub year: Option<i16>,
    /// The item behind it, where it has one.
    pub place: Option<Place>,
}

/// What the Overview says of a projection, in one dollar basis.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OverviewView {
    /// How long the money lasts: "Never short", "Short $772k from 2042".
    pub money_lasts: String,
    /// The first year it runs short; `null` where it never does.
    pub short_year: Option<i16>,
    /// That year and what the money cannot cover, as a sentence.
    pub runs_short: Option<String>,
    /// Net worth at the end.
    pub ends_with: String,
    /// Every year's taxes together.
    pub lifetime_taxes: String,
    /// What needs attention in the projection, what has no year first.
    pub attention: Vec<OverviewRow>,
    /// The plan's milestones, earliest first.
    pub milestones: Vec<OverviewRow>,
}

fn row_of(row: Row) -> OverviewRow {
    let place = row.place.map(|(domain, index)| Place {
        domain: slug_of(domain),
        index,
        field: row.field,
    });
    OverviewRow {
        text: row.text,
        year: row.year,
        place,
    }
}

fn rows(rows: Vec<Row>) -> Vec<OverviewRow> {
    rows.into_iter().map(row_of).collect()
}

/// `projected` as the Overview says it, nominal or in today's dollars.
fn overview_view(projected: &Projected, nominal: bool) -> OverviewView {
    let summary = projected.projection.summary(!nominal);
    OverviewView {
        money_lasts: money_lasts(&summary),
        short_year: summary.first_unfunded_year,
        runs_short: runs_short(&summary),
        ends_with: compact_money(summary.final_net_worth),
        lifetime_taxes: compact_money(summary.lifetime_taxes),
        attention: rows(attention(projected, nominal)),
        milestones: rows(milestones(projected, nominal)),
    }
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// The Overview's strip and lists, nominal or in today's dollars;
    /// `null` while no valid draft has been projected.
    ///
    /// # Errors
    ///
    /// Where the view does not convert.
    #[wasm_bindgen(unchecked_return_type = "OverviewView | null")]
    pub fn overview(&self, nominal: bool) -> Result<JsValue, JsError> {
        let projected = self.0.projected().ok();
        to_js(&projected.map(|projected| overview_view(projected, nominal)))
    }
}

#[cfg(test)]
mod tests {
    use retiretui_client::setup::EXAMPLES;
    use retiretui_engine::plan::Plan;

    use super::*;
    use crate::tables;

    fn projected(text: &str) -> Projected {
        Projected::new(Plan::from_toml_str(text).expect("parses"), tables())
    }

    #[test]
    fn a_lasting_plan_says_so_and_names_its_milestones_places() {
        let view = overview_view(&projected(EXAMPLES[0].2), false);
        assert_eq!(view.money_lasts, "Never short");
        assert_eq!((view.short_year, view.runs_short), (None, None));
        let medicare = view
            .milestones
            .iter()
            .find(|row| row.text.starts_with("Medicare"));
        let place = medicare
            .and_then(|row| row.place.as_ref())
            .expect("a person");
        assert_eq!((place.domain.as_str(), place.index), ("people", Some(0)));
    }

    #[test]
    fn a_plan_spending_past_its_means_runs_short() {
        let starter = EXAMPLES[0].2;
        let spending = starter.replacen("amount = 24000", "amount = 240000", 1);
        assert_ne!(spending, starter, "the starter lives on 24,000");
        let view = overview_view(&projected(&spending), true);
        let year = view.short_year.expect("short");
        let said = view.runs_short.expect("said");
        assert!(
            said.starts_with(&format!("Runs short from {year}: $")),
            "{said}"
        );
        let unfunded = view
            .attention
            .iter()
            .find(|row| row.text.contains("unfunded"));
        assert_eq!(unfunded.and_then(|row| row.year), Some(year));
    }
}
