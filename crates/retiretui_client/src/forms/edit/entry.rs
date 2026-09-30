//! The item open in a form as it is entered: the edit session, and the
//! parts a trigger or a list is entered in while they are typed, which no
//! snapshot could hold until they make a value.

use std::collections::BTreeMap;

use toml::Value;

use super::ItemEdit;
use crate::codec::{get_path, is_within, parse_text};
use crate::draft::Draft;
use crate::forms::cells::{nth, nth_back, parse_field};
use crate::forms::offers::{Offer, Vocabulary};
use crate::forms::trigger::{
    self, BASIS, Piece, SENTENCE, TriggerParts, basis_named, kind_of, operand_of,
};
use crate::forms::{FieldKind, FieldSpec, Form, lists};

/// The item being edited, and the parts it is entered in.
pub struct Entry {
    edit: ItemEdit,
    form: Form,
    triggers: BTreeMap<&'static str, TriggerParts>,
    /// Each list's rows, by the place their kind gives them in it.
    lists: BTreeMap<&'static str, BTreeMap<usize, Option<Value>>>,
}

/// The place a row holds in the list at its key, where it holds one.
#[must_use]
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

impl Entry {
    /// Item `index` of `form` as `draft` holds it, or a new one where none.
    #[must_use]
    pub fn open(form: Form, draft: &Draft, index: Option<usize>) -> Self {
        let edit = match index {
            Some(index) => ItemEdit::open(form, draft, index),
            None => ItemEdit::create(form, draft),
        };
        let mut entry = Self {
            edit,
            form,
            triggers: BTreeMap::new(),
            lists: BTreeMap::new(),
        };
        entry.seed(None);
        entry
    }

    /// The edit session the parts write into.
    #[must_use]
    pub const fn edit(&self) -> &ItemEdit {
        &self.edit
    }

    /// The edit session, for what is not entered as text: a value already
    /// made, and applying.
    pub const fn edit_mut(&mut self) -> &mut ItemEdit {
        &mut self.edit
    }

    /// The form the item is edited in.
    #[must_use]
    pub const fn form(&self) -> &Form {
        &self.form
    }

    /// What the row at `place` of the list at `key` holds.
    #[must_use]
    pub fn list_part(&self, key: &str, place: usize) -> Option<&Value> {
        self.lists.get(key)?.get(&place)?.as_ref()
    }

    /// What the parts of the trigger at `key` hold.
    #[must_use]
    pub fn trigger_parts(&self, key: &str) -> Option<&TriggerParts> {
        self.triggers.get(key)
    }

    /// What place `place` of the order at `key` may still pick.
    #[must_use]
    pub fn unused(&self, key: &str, vocabulary: Vocabulary, place: usize) -> Vec<Offer> {
        let parts = self.lists.get(key);
        let own = parts.and_then(|parts| parts.get(&place)?.as_ref());
        let is_held = |word: &Value| {
            let mut others = parts.into_iter().flatten();
            others.any(|(at, held)| *at != place && held.as_ref() == Some(word))
        };
        lists::unused(vocabulary, own, is_held)
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
    use retiretui_engine::plan::Plan;

    use super::*;
    use crate::forms::{DOMAINS, DomainId};
    use crate::setup::EXAMPLES;

    fn form(domain: DomainId) -> Form {
        let is_it = |form: &&Form| form.domain == Some(domain);
        *DOMAINS.iter().find(is_it).expect("a domain")
    }

    fn draft() -> Draft {
        Draft::new(Plan::from_toml_str(EXAMPLES[0].2).expect("parses"), false)
    }

    #[test]
    fn a_trigger_is_made_from_its_parts_and_refused_until_it_is_whole() {
        let draft = draft();
        let mut entry = Entry::open(form(DomainId::Events), &draft, None);
        entry
            .set_trigger("trigger", BASIS, "age")
            .expect("a trigger");
        assert!(entry.edit().complaint_at("trigger").is_some());
        entry
            .set_trigger("trigger", "age", "60")
            .expect("an operand");
        assert_eq!(entry.edit().complaint_at("trigger"), None);
        let trigger = get_path(entry.edit().snapshot(), "trigger").expect("a trigger");
        assert_eq!(trigger.get("age"), Some(&Value::Integer(60)));
        assert!(entry.set_trigger("trigger", "nothing", "1").is_err());
    }

    #[test]
    fn a_pick_is_kept_as_the_word_the_file_spells() {
        let draft = draft();
        let mut entry = Entry::open(form(DomainId::Accounts), &draft, Some(0));
        entry.set("kind", None, "true").expect("a pick");
        let kind = get_path(entry.edit().snapshot(), "kind");
        assert_eq!(kind, Some(&Value::String("true".to_owned())));
        assert!(entry.set("nothing", None, "1").is_err());
    }
}
