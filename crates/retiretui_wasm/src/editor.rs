//! The item open in a form, for a page: the client's edit session, and the
//! parts a trigger or a list is entered in while they are typed, which no
//! snapshot could hold until they make a value.

use std::collections::BTreeMap;

use retiretui_client::codec::{get_path, is_within, parse_text};
use retiretui_client::draft::Draft;
use retiretui_client::forms::cells::{nth, nth_back, parse_field};
use retiretui_client::forms::edit::ItemEdit;
use retiretui_client::forms::trigger::{self, Piece, SENTENCE, basis_named, kind_of, operand_of};
use retiretui_client::forms::{FieldKind, FieldSpec, Form, lists};
use retiretui_engine::plan::{Operand, TriggerBasis};
use toml::Value;

/// What a trigger's kind is set through, beside its operands' keys.
pub const BASIS: &str = "basis";

/// A trigger as its parts hold it: the kind chosen, and every operand.
pub type TriggerParts = (Option<TriggerBasis>, Vec<(Operand, Option<Value>)>);

/// The item being edited, and the parts it is entered in.
pub struct Editor {
    pub(crate) edit: ItemEdit,
    pub(crate) form: &'static Form,
    pub(crate) triggers: BTreeMap<&'static str, TriggerParts>,
    /// Each list's rows, by the place their kind gives them in it.
    pub(crate) lists: BTreeMap<&'static str, BTreeMap<usize, Option<Value>>>,
}

/// The place a row holds in the list at its key, where it holds one.
pub const fn place_of(kind: FieldKind) -> Option<usize> {
    match kind {
        FieldKind::Listed(place) | FieldKind::Order(_, place) => Some(place),
        _ => None,
    }
}

/// A pick is the word it holds, kept as the file spells it; none is blank.
fn word(text: &str) -> Option<Value> {
    (!text.is_empty()).then(|| Value::String(text.to_owned()))
}

impl Editor {
    /// Item `index` of `form` as `draft` holds it, or a new one where none.
    #[must_use]
    pub fn open(form: &'static Form, draft: &Draft, index: Option<usize>) -> Self {
        let edit = match index {
            Some(index) => ItemEdit::open(*form, draft, index),
            None => ItemEdit::create(*form, draft),
        };
        let mut editor = Self {
            edit,
            form,
            triggers: BTreeMap::new(),
            lists: BTreeMap::new(),
        };
        editor.seed(None);
        editor
    }

    /// Reads every trigger's and list's parts from what the item holds,
    /// those within `within` alone where given.
    fn seed(&mut self, within: Option<&str>) {
        let is_seeded =
            |key: &str| within.is_none_or(|outer| key == outer || is_within(key, outer));
        for spec in self.form.fields.iter().filter(|spec| is_seeded(spec.key)) {
            let value = get_path(self.edit.snapshot(), spec.key);
            match spec.kind {
                FieldKind::Trigger => {
                    let operands = SENTENCE.iter().map(|piece| {
                        let operand = piece.operand();
                        (operand, operand_of(value, operand).cloned())
                    });
                    let parts = (kind_of(value), operands.collect());
                    self.triggers.insert(spec.key, parts);
                }
                FieldKind::Listed(back) => {
                    let part = nth_back(value, back).cloned();
                    self.lists.entry(spec.key).or_default().insert(back, part);
                }
                FieldKind::Order(_, place) => {
                    let part = nth(value, place).cloned();
                    self.lists.entry(spec.key).or_default().insert(place, part);
                }
                _ => {}
            }
        }
    }

    fn spec(&self, key: &str, place: Option<usize>) -> Result<&'static FieldSpec, String> {
        let mut fields = self.form.fields.iter();
        fields
            .find(|spec| spec.key == key && place_of(spec.kind) == place)
            .ok_or_else(|| format!("{} has no field {key}", self.form.title))
    }

    /// Writes `text`, as the field `key` - the row at `place` of a list -
    /// shows it, into the item.
    ///
    /// # Errors
    ///
    /// Where the item has no such field, or it is not entered as text.
    pub fn set(&mut self, key: &str, place: Option<usize>, text: &str) -> Result<(), String> {
        let spec = self.spec(key, place)?;
        let (value, complaint) = match spec.kind {
            FieldKind::Listed(back) => {
                let parts = self.lists.entry(spec.key).or_default();
                parts.insert(back, parse_field(spec.kind, text));
                lists::listed(parts.clone().into_iter())
            }
            FieldKind::Order(_, place) => {
                let parts = self.lists.entry(spec.key).or_default();
                parts.insert(place, word(text));
                lists::ordered(parts.clone().into_iter())
            }
            FieldKind::Choice(_) | FieldKind::Ref(_) => (word(text), None),
            FieldKind::Trigger
            | FieldKind::Flag
            | FieldKind::Presence(_)
            | FieldKind::Remainder => return Err(format!("{} is not typed", spec.label)),
            kind => (parse_field(kind, text), None),
        };
        self.edit.set(spec.key, value, complaint);
        Ok(())
    }

    /// Ticks or clears the field `key`: a flag, or the tick a table stands
    /// for, whose rows then read what the table holds.
    ///
    /// # Errors
    ///
    /// Where the item has no such field, or it is not ticked.
    pub fn tick(&mut self, key: &str, is_ticked: bool) -> Result<(), String> {
        let spec = self.spec(key, None)?;
        match spec.kind {
            FieldKind::Flag => self
                .edit
                .set(spec.key, Some(Value::Boolean(is_ticked)), None),
            FieldKind::Presence(_) => {
                self.edit.tick(spec.key, is_ticked);
                self.seed(Some(spec.key));
            }
            _ => return Err(format!("{} is not ticked", spec.label)),
        }
        Ok(())
    }

    /// Writes `text` into `part` of the trigger `key` - its [`BASIS`] or
    /// an operand's key - and the trigger its parts now make into the item.
    ///
    /// # Errors
    ///
    /// Where the item has no such trigger, or a trigger no such part.
    pub fn set_trigger(&mut self, key: &str, part: &str, text: &str) -> Result<(), String> {
        let spec = self.spec(key, None)?;
        if spec.kind != FieldKind::Trigger {
            return Err(format!("{} is not a trigger", spec.label));
        }
        let (kind, operands) = self.triggers.entry(spec.key).or_default();
        if part == BASIS {
            *kind = basis_named(text);
        } else {
            let piece = SENTENCE.iter().find(|piece| piece.operand().key() == part);
            let piece = piece.ok_or_else(|| format!("a trigger has no part {part}"))?;
            let value = match piece {
                Piece::Text(..) => parse_text(text),
                Piece::Pick(..) => word(text),
            };
            let slot = operands
                .iter_mut()
                .find(|(held, _)| *held == piece.operand());
            if let Some((_, held)) = slot {
                *held = value;
            }
        }
        let (value, complaint) = trigger::held(*kind, operands);
        self.edit.set(spec.key, value, complaint);
        Ok(())
    }

    /// Drops every edit, back to the item as it was opened or last applied.
    pub fn discard(&mut self) {
        self.edit.discard();
        self.seed(None);
    }
}

#[cfg(test)]
mod tests {
    use retiretui_client::setup::EXAMPLES;
    use retiretui_engine::plan::Plan;

    use super::*;
    use crate::vocabulary::form_at;

    fn draft() -> Draft {
        Draft::new(Plan::from_toml_str(EXAMPLES[0].2).expect("parses"), false)
    }

    #[test]
    fn a_trigger_is_made_from_its_parts_and_refused_until_it_is_whole() {
        let draft = draft();
        let mut editor = Editor::open(form_at("events").expect("a domain"), &draft, None);
        editor
            .set_trigger("trigger", BASIS, "age")
            .expect("a trigger");
        assert!(editor.edit.complaint_at("trigger").is_some());
        editor
            .set_trigger("trigger", "age", "60")
            .expect("an operand");
        assert_eq!(editor.edit.complaint_at("trigger"), None);
        let trigger = get_path(editor.edit.snapshot(), "trigger").expect("a trigger");
        assert_eq!(trigger.get("age"), Some(&Value::Integer(60)));
        assert!(editor.set_trigger("trigger", "nothing", "1").is_err());
    }

    #[test]
    fn a_pick_is_kept_as_the_word_the_file_spells() {
        let draft = draft();
        let mut editor = Editor::open(form_at("accounts").expect("a domain"), &draft, Some(0));
        editor.set("kind", None, "true").expect("a pick");
        let kind = get_path(editor.edit.snapshot(), "kind");
        assert_eq!(kind, Some(&Value::String("true".to_owned())));
        assert!(editor.set("nothing", None, "1").is_err());
    }
}
