//! Whether a field applies to the item being edited, where another field's
//! value decides it: what a row is shown by, asked of the item's snapshot
//! through the engine's own predicates, what a field no file holds is read
//! from and written back into, and what an item is stored as once what it
//! has no use for is left out.

use retiretui_engine::plan::{AccountKind, IncomeKind, US};
use toml::{Table, Value};

use super::codec::{get_path, set_path};
use super::domain::{FieldSpec, Ops};
use super::editing::Editing;
use super::offers::Vocabulary;

/// The key of the pick that says whether an item recurs. No file holds it:
/// it is read from whether the item states when it happens once, and it is
/// never written.
pub const HAPPENS: &str = "happens";
pub const EVERY_YEAR: &str = "every-year";
pub const ONCE: &str = "once";

const ON_KEY: &str = "on";
const KIND_KEY: &str = "kind";
const COUNTRY_KEY: &str = "country";
const ROTH_KEY: &str = "roth";

/// An item's timing, read from whether it states when it happens once.
pub fn seed_timing(item: &Table) -> Value {
    let timing = if item.contains_key(ON_KEY) {
        ONCE
    } else {
        EVERY_YEAR
    };
    Value::String(timing.to_owned())
}

/// A derived field whose value the file's own keys already say.
pub fn write_nothing(_: &mut Table, _: &Value) {}

fn word<'a>(item: &'a Table, key: &str) -> Option<&'a str> {
    item.get(key).and_then(Value::as_str)
}

/// A windfall happens once whatever is picked, so it is not asked.
pub fn chooses_timing(item: &Table) -> bool {
    word(item, KIND_KEY) != Some(IncomeKind::Windfall.as_str())
}

pub fn happens_once(item: &Table) -> bool {
    word(item, HAPPENS) == Some(ONCE) || !chooses_timing(item)
}

pub fn recurs(item: &Table) -> bool {
    !happens_once(item)
}

impl FieldSpec {
    /// Whether an item recurs or happens once, which no file states.
    pub const fn timing() -> Self {
        Self::choice(HAPPENS, "Happens", Vocabulary::Timing)
            .shown_when(chooses_timing)
            .derived(seed_timing, write_nothing)
            .help("Every year between two dates, or once in a single year.")
    }

    /// When a recurring item begins.
    pub const fn starts() -> Self {
        Self::trigger("start", "Starts")
            .shown_when(recurs)
            .blank("Plan start")
            .help("When it begins. Blank means the start of the plan.")
    }

    /// The last year a recurring item applies.
    pub const fn ends() -> Self {
        Self::trigger("end", "Ends")
            .shown_when(recurs)
            .blank("Plan end")
            .help("The last year it applies. Blank means the end of the plan.")
    }

    /// The year an item that happens once happens.
    pub const fn once() -> Self {
        Self::trigger(ON_KEY, "On")
            .shown_when(happens_once)
            .help("The year it happens.")
    }
}

fn account_is(item: &Table, asked: impl Fn(AccountKind) -> bool) -> bool {
    let kind = item.get(KIND_KEY).cloned();
    let kind = kind.and_then(|kind| kind.try_into::<AccountKind>().ok());
    kind.is_some_and(asked)
}

pub fn keeps_basis(item: &Table) -> bool {
    let is_roth = item.get(ROTH_KEY).and_then(Value::as_bool) == Some(true);
    account_is(item, |kind| kind.keeps_basis(is_roth))
}

pub fn supports_roth(item: &Table) -> bool {
    account_is(item, AccountKind::supports_roth)
}

pub fn is_in_the_us(item: &Table) -> bool {
    word(item, COUNTRY_KEY) == Some(US)
}

/// The item `pristine` as its form edits it: with each field no file holds
/// read from it.
pub fn opened(ops: Ops, pristine: &Table) -> Table {
    let mut item = pristine.clone();
    for spec in ops.fields {
        if let Some(derived) = spec.derived {
            item.insert(spec.key.to_owned(), (derived.seed)(pristine));
        }
    }
    item
}

/// The fields the item `pristine` says something under yet, as `snapshot`
/// opens it, has no use for. A key the item's type states by default says
/// nothing: the item is the same without it.
pub fn stale(ops: Ops, pristine: &Table, snapshot: &Table) -> Vec<&'static str> {
    let stated = (ops.typed)(pristine.clone());
    let is_stale = |spec: &&FieldSpec| {
        let is_unused = spec.shown.is_some_and(|shown| !shown(snapshot));
        if !is_unused || get_path(pristine, spec.key).is_none() {
            return false;
        }
        let mut without = pristine.clone();
        set_path(&mut without, spec.key, None);
        (ops.typed)(without) != stated
    };
    let stale = ops.fields.iter().filter(is_stale);
    stale.map(|spec| spec.key).collect()
}

const ONCE_UNDATED: &str = "happens once, but does not say when";

impl Editing {
    /// Hides the stale fields cleared by hand but `held`, the one the
    /// keyboard is in, answering whether any went: a row cleared and left
    /// has nothing left to be seen for.
    pub(super) fn hide_cleared(&mut self, held: Option<&str>) -> bool {
        let is_cleared = |key: &&&'static str| {
            held != Some(**key)
                && get_path(&self.snapshot, key).is_none()
                && !self.cleared.contains(key)
        };
        let newly: Vec<&'static str> = self.stale.iter().filter(is_cleared).copied().collect();
        self.cleared.extend(&newly);
        !newly.is_empty()
    }

    fn applies(&self, spec: &FieldSpec) -> bool {
        spec.shown.is_none_or(|shown| shown(&self.snapshot)) || self.stale.contains(&spec.key)
    }

    /// Whether the item has a use for the field `key`.
    pub(super) fn uses(&self, key: &str) -> bool {
        let spec = super::cells::field_of(self.ops.fields, key);
        spec.is_none_or(|spec| self.applies(spec))
    }

    /// The first field on show that does not yet make a value, and what is
    /// wrong with it. Once is only a pick until it has a date, which no
    /// file could state, so it is held back as a half-made trigger is.
    pub(super) fn held_back(&self) -> Option<(&'static str, &'static str)> {
        let mut unmade = self.incomplete.iter().filter(|(key, _)| self.uses(key));
        if let Some((key, complaint)) = unmade.next() {
            return Some((key, complaint));
        }
        let asks_timing = self.snapshot.contains_key(HAPPENS);
        let is_undated = happens_once(&self.snapshot) && !self.snapshot.contains_key(ON_KEY);
        (asks_timing && is_undated).then_some((ON_KEY, ONCE_UNDATED))
    }

    /// `item` less what the item being edited has no use for, and with
    /// each field no file holds turned back into what one does.
    pub(super) fn stripped(&self, mut item: Table) -> Table {
        for spec in self.ops.fields {
            if let Some(derived) = spec.derived {
                if let Some(value) = item.remove(spec.key) {
                    (derived.write)(&mut item, &value);
                }
            } else if !self.applies(spec) {
                set_path(&mut item, spec.key, None);
            }
        }
        item
    }

    /// What applying stores.
    pub(super) fn written(&self) -> Table {
        self.stripped(self.snapshot.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(text: &str) -> Table {
        text.parse().unwrap()
    }

    fn with_timing(mut item: Table) -> Table {
        let timing = seed_timing(&item);
        item.insert(HAPPENS.to_owned(), timing);
        item
    }

    #[test]
    fn an_item_s_timing_is_read_from_whether_it_states_a_once() {
        assert!(happens_once(&with_timing(item("on = { age = 70 }"))));
        assert!(recurs(&with_timing(item("start = { age = 70 }"))));
        assert!(recurs(&with_timing(Table::new())));
    }

    #[test]
    fn a_windfall_happens_once_and_is_not_asked() {
        let windfall = with_timing(item("kind = \"windfall\""));
        assert!(happens_once(&windfall) && !chooses_timing(&windfall));
        assert!(chooses_timing(&with_timing(item("kind = \"salary\""))));
    }

    #[test]
    fn an_account_s_kind_decides_what_it_can_hold() {
        assert!(keeps_basis(&item("kind = \"brokerage\"")));
        assert!(keeps_basis(&item("kind = \"ira\"")));
        assert!(!keeps_basis(&item("kind = \"ira\"\nroth = true")));
        assert!(!keeps_basis(&item("kind = \"cash\"")));
        assert!(supports_roth(&item("kind = \"ira\"")));
        assert!(!keeps_basis(&Table::new()), "no kind holds nothing");
    }
}
