//! The Overview as the client says it, for a page to draw: the strip's
//! readings, the sentence for a plan that runs short, the lists, what the
//! years add up to and what the projection rests on.

use retiretui_client::forms::DomainId;
use retiretui_client::overview::{Leads, Row, Total, View};
use retiretui_client::session::Projected;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Place;
use crate::markets::AssumptionRow;
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

/// The year the money first runs short, said, and the page of what the
/// plan spends.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Shortfall {
    /// The first year it runs short.
    pub year: i16,
    /// That year and what the money cannot cover, as a sentence.
    pub said: String,
    /// The Expenses page's address.
    pub expenses: String,
}

/// Where a lifetime total leads.
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum TotalLeads {
    /// The domain's page.
    Place(Place),
    /// A tool's page, by its address.
    Tool(&'static str),
    /// The Ledger at a year.
    Year(i16),
}

/// A lifetime total: what it sums, what it is made of and where it leads.
#[derive(Serialize, PartialEq, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OverviewTotal {
    /// What it totals.
    pub label: &'static str,
    /// The total, compact.
    pub amount: String,
    /// What it is made of; empty where it is nothing.
    pub made_of: String,
    /// Where it leads, where it leads anywhere.
    pub leads: Option<TotalLeads>,
}

/// What the Overview says of a projection, in one dollar basis.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OverviewView {
    /// How long the money lasts, where the shortfall says by how much:
    /// "Never short", "Through 2041", "Short from the start".
    pub money_lasts: String,
    /// Where it runs short; `null` where it never does.
    pub shortfall: Option<Shortfall>,
    /// The least the household holds once the last salary ends, and when.
    pub low_point: String,
    /// Net worth at the end.
    pub ends_with: String,
    /// What needs attention in the projection, what has no year first.
    pub attention: Vec<OverviewRow>,
    /// The plan's milestones, earliest first.
    pub milestones: Vec<OverviewRow>,
    /// What the years add up to.
    pub totals: Vec<OverviewTotal>,
    /// What the projection hangs from, each at the field it is edited at.
    pub rests_on: Vec<AssumptionRow>,
}

/// A domain's page, and the item of its table where it has one.
fn place_of((domain, index): (DomainId, Option<usize>)) -> Place {
    Place {
        domain: slug_of(domain),
        index,
        field: None,
    }
}

fn row_of(row: Row) -> OverviewRow {
    OverviewRow {
        text: row.text,
        year: row.year,
        place: row.place.map(place_of),
    }
}

fn rows(rows: Vec<Row>) -> Vec<OverviewRow> {
    rows.into_iter().map(row_of).collect()
}

fn total_of(total: Total) -> OverviewTotal {
    let leads = total.leads.map(|leads| match leads {
        Leads::Place(place) => TotalLeads::Place(place_of(place)),
        Leads::Tool(tool) => TotalLeads::Tool(tool.slug()),
        Leads::Year(year) => TotalLeads::Year(year),
    });
    OverviewTotal {
        label: total.label,
        amount: total.amount,
        made_of: total.made_of,
        leads,
    }
}

/// `projected` as the Overview says it, nominal or in today's dollars.
fn overview_view(projected: &Projected, nominal: bool) -> OverviewView {
    let view = View::new(projected, nominal);
    OverviewView {
        money_lasts: view.money_lasts,
        shortfall: view.shortfall.map(|shortfall| Shortfall {
            year: shortfall.year,
            said: shortfall.said,
            expenses: slug_of(shortfall.place.0),
        }),
        low_point: view.low_point,
        ends_with: view.ends_with,
        attention: rows(view.attention),
        milestones: rows(view.milestones),
        totals: view.totals.into_iter().map(total_of).collect(),
        rests_on: view.rests_on.into_iter().map(AssumptionRow::from).collect(),
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
        assert!(view.shortfall.is_none());
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
        let Shortfall {
            year,
            said,
            expenses,
        } = view.shortfall.expect("short");
        assert_eq!(expenses, "expenses");
        assert!(
            said.starts_with(&format!("Runs short from {year}: $")),
            "{said}"
        );
        assert_eq!(year, projected(&spending).plan.plan.start_year);
        assert_eq!(view.money_lasts, "Short from the start");
        let said_again = (view.attention.iter()).any(|row| row.text.contains("unfunded"));
        assert!(!said_again, "the note says it once");
    }

    #[test]
    fn the_totals_lead_to_pages_tools_and_a_year_by_their_addresses() {
        let (_, _, retired) =
            retiretui_client::setup::examples::named("retired-couple.toml").expect("the example");
        let view = overview_view(&projected(retired), false);
        let leads: Vec<Option<&TotalLeads>> =
            view.totals.iter().map(|it| it.leads.as_ref()).collect();
        assert_eq!(view.totals.len(), 7);
        let Some(TotalLeads::Place(income)) = leads[0] else {
            panic!("Income leads to its page: {:?}", leads[0]);
        };
        assert_eq!(income.domain, "income");
        assert_eq!(leads[1], Some(&TotalLeads::Tool("withdrawal-order")));
        assert_eq!(leads[3], Some(&TotalLeads::Tool("tax-tables")));
        assert_eq!(leads[4], Some(&TotalLeads::Tool("roth-conversions")));
        assert!(
            matches!(leads[5], Some(TotalLeads::Year(_))),
            "{:?}",
            leads[5]
        );
    }

    #[test]
    fn what_it_rests_on_is_addressed_to_the_field_it_is_edited_at() {
        let view = overview_view(&projected(EXAMPLES[0].2), false);
        let first = &view.rests_on[0];
        assert_eq!(
            (first.label, first.domain.as_str(), first.field),
            ("Runs through", "settings", Some("horizon_age"))
        );
        assert!(!view.low_point.is_empty());
    }
}
