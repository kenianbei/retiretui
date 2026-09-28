//! A year's tax tables as the plan's projection applies them.

use retiretui_client::tax_tables::{TablesView, year_tables};
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::{JsDocument, from_js, tables, to_js};

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// The tables `view` asks for, from the draft's plan as it stands, so
    /// they show under issues and for a scenario.
    ///
    /// # Errors
    ///
    /// Where the view or the tables do not convert.
    #[wasm_bindgen(js_name = taxTables, unchecked_return_type = "YearTables")]
    pub fn tax_tables(
        &self,
        #[wasm_bindgen(unchecked_param_type = "TablesView")] view: JsValue,
    ) -> Result<JsValue, JsError> {
        let view: TablesView = from_js(view)?;
        to_js(&year_tables(&self.0.draft().plan, tables(), &view))
    }
}

#[cfg(test)]
mod tests {
    use retiretui_client::setup::EXAMPLES;

    use crate::document::Document;

    #[test]
    fn a_document_s_tables_are_its_plan_s_in_the_year_asked() {
        let (file, _, text) = EXAMPLES[0];
        let path = format!("/{file}");
        let mut read = |_: &std::path::Path| Ok(text.to_owned());
        let document = Document::open(&path, &mut read).unwrap();
        let view = retiretui_client::tax_tables::TablesView {
            year: 2027,
            ..Default::default()
        };
        let said = retiretui_client::tax_tables::year_tables(
            &document.draft().plan,
            crate::tables(),
            &view,
        );
        assert_eq!(said.year, 2027);
        assert!(
            said.sections
                .iter()
                .any(|section| section.title == "Income tax brackets")
        );
    }
}
