//! The Withdrawal Order tool for a page: every order the plan's classes can
//! be drained in ranked, the chosen order taken into the draft or written
//! as a scenario, and what the Overview's Could do better card says of the
//! best.

use retiretui_client::searches::orders::{
    ABOUT, NOTHING_SEARCHED, option_columns, said, take_question, taken,
};
use retiretui_client::searches::overview::order_said;
use retiretui_client::searches::{AGAINST_PLAN, CURRENT_PLAN, option_cells, page_refusal};
use retiretui_engine::market::Progress;
use retiretui_engine::optimize::{
    OrderCandidate, OrderSearch, apply_order, optimize_order, order_overlay,
};
use retiretui_engine::plan::TreatmentClass;
use retiretui_engine::project::Summary;
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::document::Document;
use crate::searches::gated;
use crate::{Bases, JsDocument, from_js, option_figures, refused, reply, tables, to_js};

/// One order the search tried.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrderOption {
    /// Its classes joined, as the address holds it: `taxable-roth-deferred`.
    pub key: String,
    /// The classes, first drained first.
    pub order: Vec<TreatmentClass>,
    /// The order in words.
    pub said: String,
    /// The plan's figures under it.
    pub figures: Bases<Vec<String>>,
    /// What is asked before it is taken into the plan searched.
    pub question: String,
}

/// Every order the plan's classes can be drained in, best first, beside the
/// plan as it stands.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrderOptions {
    /// The order, then what an option ends with against the plan, then each
    /// figure.
    pub columns: Vec<&'static str>,
    /// What the plan's own row is called.
    pub current: &'static str,
    /// The plan's figures as it stands.
    pub baseline: Bases<Vec<String>>,
    /// One option for each distinct outcome, best first.
    pub options: Vec<OrderOption>,
    /// What the best order does better than the plan, in either basis.
    pub better: Bases<String>,
}

fn option_of(candidate: &OrderCandidate, summaries: &Bases<Summary>) -> OrderOption {
    let classes: Vec<&str> = candidate.order.iter().map(|class| class.as_str()).collect();
    OrderOption {
        key: classes.join("-"),
        order: candidate.order.clone(),
        said: said(&candidate.order),
        figures: option_figures(option_cells, &candidate.projection, Some(summaries)),
        question: take_question(&candidate.order),
    }
}

impl OrderOptions {
    fn new(search: &OrderSearch) -> Self {
        let baseline = &search.baseline;
        let summaries = Bases::of(|nominal| baseline.summary(!nominal));
        Self {
            columns: option_columns(),
            current: CURRENT_PLAN,
            baseline: option_figures(option_cells, baseline, None),
            options: (search.candidates.iter())
                .map(|candidate| option_of(candidate, &summaries))
                .collect(),
            better: Bases::of(|nominal| order_said(search, nominal)),
        }
    }
}

/// Every order in `text`, a plan, that its listed classes can be drained
/// in.
///
/// # Errors
///
/// Where the plan does not pass the gate, or has nothing to order.
pub fn orders(text: &str) -> Result<OrderOptions, String> {
    let plan = gated(text)?;
    let search = optimize_order(&plan, tables(), &Progress::default()).map_err(page_refusal)?;
    Ok(OrderOptions::new(&search))
}

/// What the Withdrawal Order tool says of itself.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OrderWords {
    /// What the tool is for, in a line.
    pub about: &'static str,
    /// Before anything is searched.
    pub nothing_searched: &'static str,
    /// The column a phone's row shows beside the order.
    pub against_plan: &'static str,
}

impl Document {
    /// Takes `order` into the draft as one step of history, answering what
    /// was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only.
    pub fn take_order(&mut self, order: &[TreatmentClass]) -> Result<String, String> {
        self.step(|plan| {
            apply_order(plan, order);
            Ok(taken(order))
        })
    }

    /// `order` as a scenario to be written at `out`, its `base` the
    /// document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// resolved from, or the scenario does not serialize.
    pub fn order_scenario(&self, out: &str, order: &[TreatmentClass]) -> Result<String, String> {
        let base = self.scenario_base(out)?;
        order_overlay(&base, order).map_err(|error| error.to_string())
    }
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// Takes `order` into the draft as one step of history, answering what
    /// was taken.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, or `order` is not a list of treatment
    /// classes.
    #[wasm_bindgen(js_name = takeOrder)]
    pub fn take_order(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "TreatmentClass[]")] order: JsValue,
    ) -> Result<String, JsError> {
        let order: Vec<TreatmentClass> = from_js(order)?;
        self.0.take_order(&order).map_err(refused)
    }

    /// `order` as a scenario to be written at `out`, its `base` the
    /// document's file.
    ///
    /// # Errors
    ///
    /// Where the draft has unsaved edits, `out` is a file the document was
    /// made from, `order` is not a list of treatment classes, or the scenario
    /// does not serialize.
    #[wasm_bindgen(js_name = orderScenario)]
    pub fn order_scenario(
        &self,
        out: &str,
        #[wasm_bindgen(unchecked_param_type = "TreatmentClass[]")] order: JsValue,
    ) -> Result<String, JsError> {
        let order: Vec<TreatmentClass> = from_js(order)?;
        self.0.order_scenario(out, &order).map_err(refused)
    }
}

/// What the Withdrawal Order tool says of itself.
///
/// # Errors
///
/// Where the words do not convert.
#[wasm_bindgen(js_name = orderWords, unchecked_return_type = "OrderWords")]
pub fn order_words() -> Result<JsValue, JsError> {
    to_js(&OrderWords {
        about: ABOUT,
        nothing_searched: NOTHING_SEARCHED,
        against_plan: AGAINST_PLAN,
    })
}

/// Every order `plan`'s listed classes can be drained in, best first.
///
/// # Errors
///
/// Where the plan does not pass the gate, or has nothing to order.
#[wasm_bindgen(js_name = orders, unchecked_return_type = "OrderOptions")]
pub fn js_orders(plan: &str) -> Result<JsValue, JsError> {
    reply(orders(plan))
}

#[cfg(test)]
mod tests {
    use retiretui_client::files::OVERLAY_SAVE_FIRST;
    use retiretui_client::searches::FIGURES;
    use retiretui_client::setup::examples::named;
    use retiretui_engine::plan::TreatmentClass::{Deferred, Hsa, Roth, Taxable};

    use super::*;

    const BETTER_ORDERED: &str = "early-retiree.toml";

    fn example(file: &str) -> &'static str {
        named(file).expect("the example").2
    }

    #[test]
    fn every_option_is_keyed_by_its_classes_and_asked_about_in_words() {
        let reply = orders(example(BETTER_ORDERED)).expect("searches");
        assert_eq!(reply.columns.len(), 2 + FIGURES.len());
        assert_eq!(reply.baseline.today.len(), 1 + FIGURES.len());
        let best = reply.options.first().expect("an option");
        assert_eq!(best.order, [Roth, Deferred, Taxable, Hsa]);
        assert_eq!(best.key, "roth-deferred-taxable-hsa");
        assert_eq!(best.said, said(&best.order));
        assert_eq!(best.figures.today.len(), 1 + FIGURES.len());
        assert_eq!(best.question, take_question(&best.order));
        assert!(!reply.better.today.is_empty() && !reply.better.nominal.is_empty());
    }

    #[test]
    fn an_order_is_taken_as_one_step_and_written_only_over_the_saved_file() {
        let mut read = |_: &std::path::Path| Ok(example(BETTER_ORDERED).to_owned());
        let mut document = Document::open("/plan.toml", &mut read).expect("opens");
        let reply = orders(example(BETTER_ORDERED)).expect("searches");
        let chosen = &reply.options[0].order;
        let said = document.take_order(chosen).expect("taken");
        assert_eq!(said, taken(chosen));
        assert_eq!(document.draft().plan.plan.withdrawal_order, *chosen);
        assert!(document.draft().can_undo());
        assert_eq!(
            document.order_scenario("/order.toml", chosen),
            Err(OVERLAY_SAVE_FIRST.to_owned())
        );
        document.save(&mut |_| Ok(())).expect("saved");
        assert!(document.order_scenario("/plan.toml", chosen).is_err());
        let scenario = document
            .order_scenario("/order.toml", chosen)
            .expect("written");
        assert!(scenario.contains("base = \"plan.toml\""), "{scenario}");
        assert!(
            scenario.contains("[plan]\nwithdrawal_order = ["),
            "{scenario}"
        );
        assert!(document.undo());
    }
}
