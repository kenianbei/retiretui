//! What a page draws of the item being edited: each field on show, what
//! it holds in the words it is entered in, what it offers, and what is
//! wrong with it.

use retiretui_client::codec::{get_path, share_left, to_text};
use retiretui_client::draft::Draft;
use retiretui_client::forms::cells::field_text;
use retiretui_client::forms::offers::{Offer, RefSource, Vocabulary, ref_offers};
use retiretui_client::forms::trigger::{Piece, SENTENCE, help_of, shows};
use retiretui_client::forms::{BLANK, FieldKind, FieldSpec, lists};
use retiretui_client::issues::{LocatedIssue, field_issue, located_issues};
use retiretui_client::present;
use serde::Serialize;
use toml::Value;

use crate::editor::{Editor, place_of};

/// How a field is entered.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Control {
    /// Typed: a name, an id, a date, or anything else the file's syntax says.
    Text,
    /// Typed dollars.
    Money,
    /// A typed whole number.
    Whole,
    /// Ticked.
    Flag,
    /// Picked from its offers.
    Choice,
    /// A typed rate, or one set on a slider.
    Rate,
    /// A typed share, or one set on a slider up to the whole.
    Share,
    /// Typed: `Inflation`, `Fixed`, or a rate.
    Growth,
    /// A kind picked, then its operands.
    Trigger,
    /// Ticked, its table's rows shown while it is.
    Presence,
    /// One row of a list, typed.
    Listed,
    /// One place of an order, picked from its offers.
    Order,
    /// Shown, never entered.
    Remainder,
}

/// One field on show.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FieldView {
    /// The key it is set by.
    pub key: &'static str,
    /// Its place in the list at its key, where it is one row of several.
    pub place: Option<usize>,
    /// What the form calls it.
    pub label: &'static str,
    /// What it means.
    pub help: &'static str,
    /// How it is entered.
    pub control: Control,
    /// What it shows while the keyboard is elsewhere; a pick's value.
    pub text: String,
    /// What it shows while typed into: money as plain digits.
    pub typed: String,
    /// What it shows while entered empty: `Blank is 65`; none where it
    /// may not be left empty.
    pub placeholder: Option<String>,
    /// What it reads as once left empty: `65, the default`.
    pub unstated: String,
    /// The heading it is gathered under with its neighbours, where it is.
    pub group: Option<&'static str>,
    /// A rate's or a share's number, for its slider.
    pub number: Option<f64>,
    /// Whether a flag or a tick is set.
    pub is_ticked: bool,
    /// Whether a remainder is less than none or more than the whole.
    pub is_exceeded: bool,
    /// What a pick offers, a blank first where it may be emptied.
    pub offers: Vec<Offer>,
    /// A trigger's kind and operands.
    pub trigger: Option<TriggerView>,
    /// What the draft's gate finds wrong with it.
    pub issue: Option<String>,
    /// Why what is entered in it does not yet make a value.
    pub complaint: Option<&'static str>,
}

/// A trigger as its sentence reads.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TriggerView {
    /// The kind chosen, as the file spells it; blank where none is.
    pub basis: String,
    /// Every kind, a blank first.
    pub bases: Vec<Offer>,
    /// The operands the kind has a use for, in the sentence's order.
    pub operands: Vec<OperandView>,
}

/// One operand of a trigger's sentence.
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OperandView {
    /// The part it is set by.
    pub key: &'static str,
    /// The words before it.
    pub before: &'static str,
    /// The words after it.
    pub after: &'static str,
    /// What it means.
    pub help: &'static str,
    /// What it holds; a pick's value.
    pub text: String,
    /// What it offers, where it is picked from the plan's ids.
    pub offers: Option<Vec<Offer>>,
}

const fn control_of(kind: FieldKind) -> Control {
    match kind {
        FieldKind::Text => Control::Text,
        FieldKind::Money => Control::Money,
        FieldKind::Whole => Control::Whole,
        FieldKind::Flag => Control::Flag,
        FieldKind::Choice(_) | FieldKind::Ref(_) => Control::Choice,
        FieldKind::Rate => Control::Rate,
        FieldKind::Share => Control::Share,
        FieldKind::Growth => Control::Growth,
        FieldKind::Trigger => Control::Trigger,
        FieldKind::Presence(_) => Control::Presence,
        FieldKind::Listed(_) => Control::Listed,
        FieldKind::Order(..) => Control::Order,
        FieldKind::Remainder => Control::Remainder,
    }
}

/// What a pick shows it holds: the word the file spells.
fn picked(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(word)) => word.clone(),
        Some(other) => to_text(other),
        None => String::new(),
    }
}

/// `offers` holding `held` whatever it is, since the file stated it, and
/// a blank first unless `spec` may not be emptied.
fn offered(mut offers: Vec<Offer>, held: &str, spec: &FieldSpec) -> Vec<Offer> {
    if !held.is_empty() && !offers.iter().any(|offer| offer.value == held) {
        offers.push(Offer::spelt(held.to_owned()));
    }
    if spec.is_required() {
        offers
    } else {
        blank_first(offers, spec.blank_word())
    }
}

/// `offers` after a blank, which reads as `word`.
fn blank_first(mut offers: Vec<Offer>, word: &str) -> Vec<Offer> {
    let blank = Offer {
        value: String::new(),
        label: word.to_owned(),
    };
    offers.insert(0, blank);
    offers
}

impl Editor {
    /// Every field on show, `focused` the one being typed into: a stale
    /// field cleared and left is no longer shown.
    pub fn view(&mut self, draft: &Draft, focused: Option<&str>) -> Vec<FieldView> {
        self.edit.hide_cleared(focused);
        let located = located_issues(draft);
        let mut complained = Vec::new();
        let fields = self.form.fields.iter();
        let shown = fields.filter(|spec| self.edit.is_on_show(spec.key));
        let mut views: Vec<FieldView> = shown
            .map(|spec| self.field(spec, draft, &located))
            .collect();
        for view in &mut views {
            if complained.contains(&view.key) {
                view.complaint = None;
            } else if view.complaint.is_some() {
                complained.push(view.key);
            }
        }
        views
    }

    fn field(
        &self,
        spec: &'static FieldSpec,
        draft: &Draft,
        located: &[LocatedIssue],
    ) -> FieldView {
        let snapshot = self.edit.snapshot();
        let place = place_of(spec.kind);
        let value = match place {
            Some(place) => self
                .lists
                .get(spec.key)
                .and_then(|parts| parts.get(&place)?.as_ref()),
            None => get_path(snapshot, spec.key),
        };
        let left = matches!(spec.kind, FieldKind::Remainder)
            .then(|| share_left(snapshot, spec.key))
            .flatten();
        let (text, typed) = match spec.kind {
            FieldKind::Choice(_) | FieldKind::Ref(_) | FieldKind::Order(..) => {
                (picked(value), picked(value))
            }
            FieldKind::Remainder => {
                let left = left.map(present::rate).unwrap_or_default();
                (left.clone(), left)
            }
            kind => (
                field_text(kind, value, false),
                field_text(kind, value, true),
            ),
        };
        let offers = self.offers(spec, draft, &text);
        FieldView {
            key: spec.key,
            place,
            label: spec.label,
            help: spec.help,
            control: control_of(spec.kind),
            text,
            typed,
            placeholder: spec.placeholder(),
            unstated: spec.unstated(),
            group: spec.group,
            number: value.and_then(Value::as_float),
            is_ticked: match spec.kind {
                FieldKind::Presence(_) => value.is_some_and(Value::is_table),
                _ => value.and_then(Value::as_bool) == Some(true),
            },
            is_exceeded: left.is_some_and(|left| !(0.0..=1.0).contains(&left)),
            offers,
            trigger: (spec.kind == FieldKind::Trigger).then(|| self.trigger(spec, draft)),
            issue: self.issue(spec, located).map(str::to_owned),
            complaint: self.edit.complaint_at(spec.key),
        }
    }

    fn issue<'a>(&self, spec: &FieldSpec, located: &[LocatedIssue<'a>]) -> Option<&'a str> {
        // A new item is in no issue's path; the one item of a domain is at none.
        let index = match self.form.list {
            Some(_) => self.edit.index().map(Some),
            None => Some(None),
        };
        let snapshot = Some(self.edit.snapshot());
        index.and_then(|index| field_issue(located, (self.form, index), spec, snapshot))
    }

    fn offers(&self, spec: &FieldSpec, draft: &Draft, held: &str) -> Vec<Offer> {
        let offers = match spec.kind {
            FieldKind::Choice(vocabulary) => vocabulary.offers(),
            FieldKind::Ref(source) => ref_offers(&draft.plan, source),
            FieldKind::Order(vocabulary, place) => self.unused(spec.key, vocabulary, place),
            _ => return Vec::new(),
        };
        offered(offers, held, spec)
    }

    /// What place `place` of the order at `key` may still pick.
    fn unused(&self, key: &str, vocabulary: Vocabulary, place: usize) -> Vec<Offer> {
        let parts = self.lists.get(key);
        let own = parts.and_then(|parts| parts.get(&place)?.as_ref());
        let is_held = |word: &Value| {
            let mut others = parts.into_iter().flatten();
            others.any(|(at, held)| *at != place && held.as_ref() == Some(word))
        };
        lists::unused(vocabulary, own, is_held)
    }

    fn trigger(&self, spec: &FieldSpec, draft: &Draft) -> TriggerView {
        let (kind, parts) = self.triggers.get(spec.key).cloned().unwrap_or_default();
        let held = |operand| {
            let part = parts.iter().find(|(each, _)| *each == operand);
            part.and_then(|(_, value)| value.as_ref())
        };
        let shown = SENTENCE.iter().filter(|piece| shows(kind, piece.operand()));
        let operands = shown.map(|piece| {
            let operand = piece.operand();
            let (words, source): ([&str; 2], Option<RefSource>) = match *piece {
                Piece::Text(_, words) => (words, None),
                Piece::Pick(_, source) => (["", ""], Some(source)),
            };
            let text = match source {
                Some(_) => picked(held(operand)),
                None => held(operand).map(to_text).unwrap_or_default(),
            };
            OperandView {
                key: operand.key(),
                before: words[0],
                after: words[1],
                help: help_of(operand),
                text,
                offers: source.map(|source| blank_first(ref_offers(&draft.plan, source), BLANK)),
            }
        });
        TriggerView {
            basis: kind
                .map(|kind| kind.as_str().to_owned())
                .unwrap_or_default(),
            bases: blank_first(Vocabulary::TriggerBasis.offers(), BLANK),
            operands: operands.collect(),
        }
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
    fn a_view_shows_each_field_in_the_words_it_is_entered_in() {
        let draft = draft();
        let mut editor = Editor::open(form_at("accounts").expect("a domain"), &draft, Some(0));
        let views = editor.view(&draft, None);
        let kind = views
            .iter()
            .find(|view| view.key == "kind")
            .expect("a kind");
        assert_eq!(kind.control, Control::Choice);
        assert!(kind.offers.iter().any(|offer| offer.value == kind.text));
        let balance = views.iter().find(|view| view.control == Control::Money);
        let balance = balance.expect("a balance");
        assert!(balance.text.starts_with('$'), "{}", balance.text);
        assert!(!balance.typed.contains(','), "{}", balance.typed);
    }

    #[test]
    fn a_trigger_view_shows_the_operands_its_kind_needs() {
        let draft = draft();
        let mut editor = Editor::open(form_at("events").expect("a domain"), &draft, None);
        editor
            .set_trigger("trigger", crate::editor::BASIS, "age")
            .expect("a kind");
        let views = editor.view(&draft, None);
        let trigger = views
            .iter()
            .find(|view| view.key == "trigger")
            .expect("a trigger");
        let sentence = trigger.trigger.as_ref().expect("a sentence");
        assert_eq!(sentence.basis, "age");
        let keys: Vec<&str> = sentence
            .operands
            .iter()
            .map(|operand| operand.key)
            .collect();
        assert_eq!(keys.len(), 2, "{keys:?}");
        assert!(trigger.complaint.is_some());
    }
}
