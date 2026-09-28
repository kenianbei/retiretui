//! A domain's page as the plan holds it: its table, ordered by a column
//! for the view alone, and an item read out in the form's words.

use retiretui_client::draft::Draft;
use retiretui_client::forms::cells::Cell;
use retiretui_client::forms::details;
use retiretui_client::forms::sort::Sort;
use retiretui_client::forms::{Form, ListOps};
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::vocabulary::form_at;
use crate::{JsDocument, reply};

/// A domain's items as its table shows them.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DomainTable {
    /// Each column, in the order its cells are.
    pub columns: Vec<TableColumn>,
    /// Each item, in the order asked for.
    pub rows: Vec<TableRow>,
}

/// One column of a domain's table.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TableColumn {
    /// What heads it.
    pub header: &'static str,
    /// Whether it holds numbers, which line up on the right.
    pub is_numeric: bool,
}

/// One item of a domain's table.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TableRow {
    /// Where the item sits in the plan, whatever order the table shows.
    pub index: usize,
    /// Its cells, in column order.
    pub cells: Vec<Cell>,
}

fn list_of(form: &Form) -> Result<ListOps, String> {
    form.list
        .ok_or_else(|| format!("{} is one item, not a table", form.title))
}

/// The table of the domain `form`, ordered by `sort` where one is given.
///
/// # Errors
///
/// Where the domain is a single item rather than a table.
pub fn table(draft: &Draft, form: &Form, sort: Option<Sort>) -> Result<DomainTable, String> {
    let list = list_of(form)?;
    let columns = list.columns.iter().map(|column| TableColumn {
        header: column.header(form.fields),
        is_numeric: column.is_numeric(form.fields),
    });
    let items: Vec<(usize, Vec<Cell>)> = (list.rows)(&draft.plan).into_iter().enumerate().collect();
    let items = match sort {
        Some(sort) => sort.order(items),
        None => items,
    };
    let rows = items
        .into_iter()
        .map(|(index, cells)| TableRow { index, cells });
    Ok(DomainTable {
        columns: columns.collect(),
        rows: rows.collect(),
    })
}

/// Item `index` of the domain `form` read out: each field it has a use for,
/// labelled, in the form's words.
///
/// # Errors
///
/// Where the domain holds no item at `index`.
pub fn read_out(draft: &Draft, form: &Form, index: usize) -> Result<Vec<[String; 2]>, String> {
    let item =
        (form.item)(draft, index).ok_or_else(|| format!("{} holds no item {index}", form.title))?;
    Ok(details::rows(form, &item, &draft.plan))
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// The table of the domain at `slug`, ordered by column `sort` - down
    /// where `isDescending` - or in the plan's own order where none.
    ///
    /// # Errors
    ///
    /// Where no domain is at `slug`, or it is a single item.
    #[wasm_bindgen(unchecked_return_type = "DomainTable")]
    pub fn table(
        &self,
        slug: &str,
        sort: Option<usize>,
        #[wasm_bindgen(js_name = isDescending)] is_descending: bool,
    ) -> Result<JsValue, JsError> {
        let sort = sort.map(|column| Sort::new(column, is_descending));
        reply(form_at(slug).and_then(|form| table(self.0.draft(), form, sort)))
    }

    /// Item `index` of the domain at `slug`, each row its label and what
    /// it holds; a single item is at 0.
    ///
    /// # Errors
    ///
    /// Where no domain is at `slug`, or it holds no item at `index`.
    #[wasm_bindgen(js_name = readOut, unchecked_return_type = "[string, string][]")]
    pub fn read_out(&self, slug: &str, index: usize) -> Result<JsValue, JsError> {
        reply(form_at(slug).and_then(|form| read_out(self.0.draft(), form, index)))
    }
}

#[cfg(test)]
mod tests {
    use retiretui_client::setup::EXAMPLES;
    use retiretui_engine::plan::Plan;

    use super::*;

    fn draft() -> Draft {
        Draft::new(Plan::from_toml_str(EXAMPLES[0].2).expect("parses"), false)
    }

    #[test]
    fn a_table_keeps_each_item_s_plan_index_whatever_its_order() {
        let draft = draft();
        let accounts = form_at("accounts").expect("a domain");
        let plain = table(&draft, accounts, None).expect("a table");
        assert_eq!(plain.columns.len(), plain.rows[0].cells.len());
        let indices: Vec<usize> = plain.rows.iter().map(|row| row.index).collect();
        assert_eq!(indices, (0..draft.plan.accounts.len()).collect::<Vec<_>>());
        let down = table(&draft, accounts, Some(Sort::new(0, true))).expect("a table");
        let mut reordered: Vec<usize> = down.rows.iter().map(|row| row.index).collect();
        reordered.sort_unstable();
        assert_eq!(reordered, indices);
    }

    #[test]
    fn a_single_item_is_read_out_rather_than_tabled() {
        let draft = draft();
        let settings = form_at("settings").expect("a domain");
        assert!(table(&draft, settings, None).is_err());
        let rows = read_out(&draft, settings, 0).expect("the one item");
        assert!(!rows.is_empty());
        assert!(read_out(&draft, form_at("accounts").expect("a domain"), 99).is_err());
    }
}
