//! Editing the open document from JavaScript: the item open in a form,
//! applied, removed, stepped back and forward over, and saved.

use js_sys::Function;
use retiretui_client::forms::DomainId;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

use crate::editor::Editor;
use crate::vocabulary::{form_at, slug_of};
use crate::{JsDocument, refused, thrown_message, to_js};

/// One item open in its form.
#[wasm_bindgen(js_name = Editor)]
pub struct JsEditor(pub(crate) Editor);

#[wasm_bindgen(js_class = Editor)]
impl JsEditor {
    /// Every field on show, `focused` the key of the one being typed into.
    ///
    /// # Errors
    ///
    /// Where the fields do not convert.
    #[wasm_bindgen(unchecked_return_type = "FieldView[]")]
    pub fn view(
        &mut self,
        document: &JsDocument,
        focused: Option<String>,
    ) -> Result<JsValue, JsError> {
        to_js(&self.0.view(document.0.draft(), focused.as_deref()))
    }

    /// Writes `text` into the field `key`, the row at `place` of a list.
    ///
    /// # Errors
    ///
    /// Where the item has no such field, or it is not typed or picked.
    pub fn set(&mut self, key: &str, place: Option<usize>, text: &str) -> Result<(), JsError> {
        self.0.set(key, place, text).map_err(refused)
    }

    /// Ticks or clears the flag or tick `key`.
    ///
    /// # Errors
    ///
    /// Where the item has no such field, or it is not ticked.
    pub fn tick(
        &mut self,
        key: &str,
        #[wasm_bindgen(js_name = isTicked)] is_ticked: bool,
    ) -> Result<(), JsError> {
        self.0.tick(key, is_ticked).map_err(refused)
    }

    /// Writes `text` into `part` of the trigger `key`: `"basis"`, or an
    /// operand's key.
    ///
    /// # Errors
    ///
    /// Where the item has no such trigger, or a trigger no such part.
    #[wasm_bindgen(js_name = setTrigger)]
    pub fn set_trigger(&mut self, key: &str, part: &str, text: &str) -> Result<(), JsError> {
        self.0.set_trigger(key, part, text).map_err(refused)
    }

    /// What the form is called: the item's name, or the kind being made.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn title(&self) -> String {
        self.0.edit.title()
    }

    /// Whether applying would store anything the item does not hold.
    #[wasm_bindgen(getter, js_name = isDirty)]
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.0.edit.is_dirty()
    }

    /// Where the item sits in the plan; `undefined` while it is new.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn index(&self) -> Option<usize> {
        self.0.edit.index()
    }

    /// Drops every edit, back to the item as it was opened.
    pub fn discard(&mut self) {
        self.0.discard();
    }
}

/// The address of the page whose people a statement is recorded on.
#[wasm_bindgen(js_name = statementPage)]
#[must_use]
pub fn statement_page() -> String {
    slug_of(DomainId::People)
}

fn writer(write: &Function) -> impl FnMut(&str) -> Result<(), String> + '_ {
    |text| {
        let text = JsValue::from_str(text);
        write
            .call1(&JsValue::NULL, &text)
            .map(drop)
            .map_err(|thrown| thrown_message(&thrown))
    }
}

#[wasm_bindgen(js_class = Document)]
impl JsDocument {
    /// Item `index` of the domain at `slug` open in its form; a single
    /// item is at 0.
    ///
    /// # Errors
    ///
    /// Where no domain is at `slug`, or it holds no item at `index`.
    pub fn edit(&self, slug: &str, index: usize) -> Result<JsEditor, JsError> {
        let form = form_at(slug).map_err(refused)?;
        let draft = self.0.draft();
        if (form.item)(draft, index).is_none() {
            return Err(refused(format!("{} holds no item {index}", form.title)));
        }
        Ok(JsEditor(Editor::open(form, draft, Some(index))))
    }

    /// A new item of the domain at `slug` open in its form.
    ///
    /// # Errors
    ///
    /// Where no domain is at `slug`.
    pub fn create(&self, slug: &str) -> Result<JsEditor, JsError> {
        let form = form_at(slug).map_err(refused)?;
        Ok(JsEditor(Editor::open(form, self.0.draft(), None)))
    }

    /// Stores `editor`'s item as one step of history, answering where it
    /// now sits; `undefined` where it held no edits.
    ///
    /// # Errors
    ///
    /// Why nothing was stored, in the form's words.
    pub fn apply(&mut self, editor: &mut JsEditor) -> Result<Option<usize>, JsError> {
        self.0.apply(&mut editor.0).map_err(refused)
    }

    /// Records the statement `xml` on the person at `index` of the People
    /// page, where they are still the one called `name`, as one step of
    /// history, answering what it recorded.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, the person there is no longer them,
    /// the statement does not parse, or its birth date is not theirs.
    #[wasm_bindgen(js_name = importEarnings)]
    pub fn import_earnings(
        &mut self,
        index: usize,
        name: &str,
        xml: &str,
    ) -> Result<String, JsError> {
        self.0.import_earnings(index, name, xml).map_err(refused)
    }

    /// Removes item `index` of the domain at `slug`, where it is still the
    /// item called `name`, as one step of history.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, or the item there is no longer it.
    pub fn remove(&mut self, slug: &str, index: usize, name: &str) -> Result<(), JsError> {
        let form = form_at(slug).map_err(refused)?;
        self.0.remove(form, index, name).map_err(refused)
    }

    /// Steps back over the last edit, answering whether there was one.
    pub fn undo(&mut self) -> bool {
        self.0.undo()
    }

    /// Steps forward over the last undone edit, answering whether there
    /// was one.
    pub fn redo(&mut self) -> bool {
        self.0.redo()
    }

    /// Whether there is an edit to step back over.
    #[wasm_bindgen(getter, js_name = canUndo)]
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.0.draft().can_undo()
    }

    /// Whether there is an undone edit to step forward over.
    #[wasm_bindgen(getter, js_name = canRedo)]
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.0.draft().can_redo()
    }

    /// Whether the draft holds edits not yet saved.
    #[wasm_bindgen(getter, js_name = isDirty)]
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.0.draft().is_dirty()
    }

    /// Writes the draft as canonical text through `write`, back where it
    /// came from.
    ///
    /// # Errors
    ///
    /// Where the draft is read-only, has issues, or `write` throws.
    pub fn save(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "(text: string) => void")] write: &Function,
    ) -> Result<(), JsError> {
        self.0.save(&mut writer(write)).map_err(refused)
    }

    /// Writes the draft as a plan of its own at `path` through `write`,
    /// which the document then is.
    ///
    /// # Errors
    ///
    /// Where the draft has issues, or `write` throws.
    #[wasm_bindgen(js_name = saveAs)]
    pub fn save_as(
        &mut self,
        path: &str,
        #[wasm_bindgen(unchecked_param_type = "(text: string) => void")] write: &Function,
    ) -> Result<(), JsError> {
        self.0.save_as(path, &mut writer(write)).map_err(refused)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use retiretui_client::setup::EXAMPLES;
    use retiretui_engine::plan::Item;

    use crate::document::Document;
    use crate::editor::Editor;
    use crate::vocabulary::form_at;

    fn starter() -> &'static str {
        EXAMPLES[0].2
    }

    fn reader<'a>(files: &'a [(&str, &str)]) -> impl FnMut(&Path) -> Result<String, String> + 'a {
        |path| {
            let file = files.iter().find(|(at, _)| Path::new(at) == path);
            file.map(|(_, text)| (*text).to_owned())
                .ok_or_else(|| "no such file".to_owned())
        }
    }

    fn opened() -> Document {
        Document::open("/plan.toml", &mut reader(&[("/plan.toml", starter())])).expect("opens")
    }

    fn saved(document: &mut Document) -> Result<String, String> {
        let mut written = String::new();
        document.save(&mut |text| {
            written = text.to_owned();
            Ok(())
        })?;
        Ok(written)
    }

    #[test]
    fn a_relocated_plan_keeps_its_unsaved_draft_and_a_scenario_is_refused() {
        let mut document = opened();
        let accounts = form_at("accounts").expect("a domain");
        let mut editor = Editor::open(accounts, document.draft(), Some(0));
        editor.set("name", None, "Renamed").expect("a field");
        document.apply(&mut editor).expect("applied");
        document.relocate("/renamed.toml").expect("a plan");
        assert_eq!(
            document.files(),
            [std::path::PathBuf::from("/renamed.toml")]
        );
        assert!(document.draft().is_dirty());
        let scenario = "schema = 1\nbase = \"renamed.toml\"\n".to_owned();
        let text = saved(&mut document).expect("saved");
        let mut read = |path: &std::path::Path| {
            Ok(if path.ends_with("what-if.toml") {
                scenario.clone()
            } else {
                text.clone()
            })
        };
        let mut over = Document::open("/what-if.toml", &mut read).expect("opens");
        assert!(over.relocate("/moved.toml").is_err());
        let rebased =
            retiretui_client::files::rebased(&scenario, "moved.toml").expect("a scenario");
        assert_eq!(
            retiretui_client::files::base_named(&rebased).as_deref(),
            Some("moved.toml")
        );
        assert_eq!(retiretui_client::files::base_named(&text), None);
        assert!(retiretui_client::files::rebased(&text, "moved.toml").is_err());
    }

    #[test]
    fn an_applied_edit_is_one_step_undone_redone_and_saved() {
        let mut document = opened();
        let accounts = form_at("accounts").expect("a domain");
        let mut editor = Editor::open(accounts, document.draft(), Some(0));
        editor.set("name", None, "Renamed").expect("a field");
        assert_eq!(document.apply(&mut editor), Ok(Some(0)));
        let renamed = |document: &Document| document.draft().plan.accounts[0].name.clone();
        assert_eq!(renamed(&document).as_deref(), Some("Renamed"));
        assert!(document.draft().is_dirty() && document.draft().can_undo());
        assert!(document.undo());
        assert_ne!(renamed(&document).as_deref(), Some("Renamed"));
        assert!(document.redo());
        assert_eq!(renamed(&document).as_deref(), Some("Renamed"));
        let written = saved(&mut document).expect("saved");
        assert!(written.contains("Renamed"));
        assert!(!document.draft().is_dirty());
    }

    #[test]
    fn an_invalid_draft_keeps_the_last_good_projection_and_is_not_saved() {
        let mut document = opened();
        let settings = form_at("settings").expect("a domain");
        let mut editor = Editor::open(settings, document.draft(), Some(0));
        editor.set("inflation", None, "500%").expect("a field");
        assert_eq!(document.apply(&mut editor), Ok(Some(0)));
        assert!(!document.issues().is_empty());
        assert!(document.projection().is_some(), "the last good view");
        assert!(saved(&mut document).is_err());
    }

    #[test]
    fn a_scenario_is_saved_only_as_a_plan_of_its_own() {
        let scenario = "schema = 1\nbase = \"../base.toml\"\n";
        let files = [("/base.toml", starter()), ("/what-if/early.toml", scenario)];
        let mut document =
            Document::open("/what-if/early.toml", &mut reader(&files)).expect("opens");
        assert!(saved(&mut document).is_err());
        let over = document.save_as("/what-if/early.toml", &mut |_| Ok(()));
        assert!(over.is_err(), "the scenario's own file is not replaced");
        let mut editor = Editor::open(
            form_at("accounts").expect("a domain"),
            document.draft(),
            Some(0),
        );
        editor.set("name", None, "Renamed").expect("a field");
        assert!(document.apply(&mut editor).is_err(), "read-only");
        document
            .save_as("/early.toml", &mut |_| Ok(()))
            .expect("saved as");
        assert!(!document.is_read_only());
        assert_eq!(document.files(), [PathBuf::from("/early.toml")]);
    }

    #[test]
    fn an_item_is_removed_only_while_it_is_the_one_named() {
        let mut document = opened();
        let accounts = form_at("accounts").expect("a domain");
        let count = document.draft().plan.accounts.len();
        assert!(document.remove(accounts, 0, "Someone else's").is_err());
        let name = document.draft().plan.accounts[0].display_name().to_owned();
        document.remove(accounts, 0, &name).expect("removed");
        assert_eq!(document.draft().plan.accounts.len(), count - 1);
        assert!(document.undo());
        assert_eq!(document.draft().plan.accounts.len(), count);
    }
}
