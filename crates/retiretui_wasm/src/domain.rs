//! A domain's page as the plan holds it: its table, ordered by a column
//! for the view alone, and an item read out in the form's words.

use retiretui_client::draft::Draft;
use retiretui_client::forms::cells::Cell;
use retiretui_client::forms::details::{self, ReadRow};
use retiretui_client::forms::edit::{known_as, list_of, name_at};
use retiretui_client::forms::sort::Sort;
use retiretui_client::forms::{DOMAINS, Form};
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::vocabulary::form_at;
use crate::{JsDocument, from_js, reply, to_js};

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
    /// What the item is called, as a question about it names it.
    pub name: String,
    /// What the item is known by: what removing it checks it by.
    pub known: String,
    /// Its cells, in column order.
    pub cells: Vec<Cell>,
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
    let name_of = |index| name_at(form, draft, index).unwrap_or_else(|| list.singular.to_owned());
    let rows = items.into_iter().map(|(index, cells)| TableRow {
        index,
        name: name_of(index),
        known: known_as(form, draft, index).unwrap_or_default(),
        cells,
    });
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
pub fn read_out(draft: &Draft, form: &Form, index: usize) -> Result<Vec<ReadRow>, String> {
    let item =
        (form.item)(draft, index).ok_or_else(|| format!("{} holds no item {index}", form.title))?;
    Ok(details::rows(form, &item, &draft.plan))
}

/// How many items each domain holds, in the order the domains are named
/// in; none for a domain that is one item.
#[must_use]
pub fn item_counts(draft: &Draft) -> Vec<Option<usize>> {
    let count = |form: &Form| form.list.map(|list| (list.count)(&draft.plan));
    DOMAINS.iter().map(count).collect()
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// How many items each domain holds, in `domains()`' order; `null` for
    /// a domain that is one item.
    ///
    /// # Errors
    ///
    /// Where the counts do not convert.
    #[wasm_bindgen(js_name = itemCounts, unchecked_return_type = "(number | null)[]")]
    pub fn item_counts(&self) -> Result<JsValue, JsError> {
        to_js(&item_counts(self.0.draft()))
    }

    /// The table of the domain at `slug`, ordered by `sort`, or in the
    /// plan's own order where none.
    ///
    /// # Errors
    ///
    /// Where no domain is at `slug`, it is a single item, or `sort` is not
    /// one.
    #[wasm_bindgen(unchecked_return_type = "DomainTable")]
    pub fn table(
        &self,
        slug: &str,
        #[wasm_bindgen(unchecked_param_type = "Sort | null")] sort: JsValue,
    ) -> Result<JsValue, JsError> {
        let sort: Option<Sort> = from_js(sort)?;
        reply(form_at(slug).and_then(|form| table(self.0.draft(), form, sort)))
    }

    /// Item `index` of the domain at `slug`, each row its label, what it
    /// holds and whether that is what a blank stands for; a single item is
    /// at 0.
    ///
    /// # Errors
    ///
    /// Where no domain is at `slug`, or it holds no item at `index`.
    #[wasm_bindgen(js_name = readOut, unchecked_return_type = "ReadRow[]")]
    pub fn read_out(&self, slug: &str, index: usize) -> Result<JsValue, JsError> {
        reply(form_at(slug).and_then(|form| read_out(self.0.draft(), form, index)))
    }
}

/// What a press on `column`'s header makes of the order `held`: up, then
/// down, then the plan's own.
///
/// # Errors
///
/// Where `held` is not an order.
#[wasm_bindgen(js_name = sortPressed, unchecked_return_type = "Sort | null")]
pub fn sort_pressed(
    #[wasm_bindgen(unchecked_param_type = "Sort | null")] held: JsValue,
    column: usize,
) -> Result<JsValue, JsError> {
    let held: Option<Sort> = from_js(held)?;
    to_js(&Sort::pressed(held, column))
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
        let named = draft.plan.accounts[0].name.as_deref();
        assert_eq!(Some(plain.rows[0].name.as_str()), named);
        let down =
            table(&draft, accounts, Sort::pressed(Sort::pressed(None, 0), 0)).expect("a table");
        let mut reordered: Vec<usize> = down.rows.iter().map(|row| row.index).collect();
        reordered.sort_unstable();
        assert_eq!(reordered, indices);
    }

    #[test]
    fn a_domain_s_items_are_counted_and_a_single_item_s_are_not() {
        let mut draft = draft();
        let counted = |draft: &Draft| {
            let titles = DOMAINS.iter().map(|form| form.title);
            titles.zip(item_counts(draft)).collect::<Vec<_>>()
        };
        let before = counted(&draft);
        let accounts = draft.plan.accounts.len();
        assert!(before.contains(&("Accounts", Some(accounts))), "{before:?}");
        assert!(before.contains(&("Settings", None)), "{before:?}");
        assert_eq!(before.len(), crate::vocabulary::domains().len());
        draft.plan.accounts.pop();
        let after = counted(&draft);
        assert!(
            after.contains(&("Accounts", Some(accounts - 1))),
            "{after:?}"
        );
    }

    #[test]
    fn a_single_item_is_read_out_rather_than_tabled() {
        let draft = draft();
        let settings = form_at("settings").expect("a domain");
        assert!(table(&draft, settings, None).is_err());
        let rows = read_out(&draft, settings, 0).expect("the one item");
        assert_ne!(rows, []);
        assert!(read_out(&draft, form_at("accounts").expect("a domain"), 99).is_err());
    }
}
