//! Whether a field applies to the item being edited, where another field's
//! value decides it: what a row is shown by, asked of the item's snapshot
//! through the engine's own predicates, what a field no file holds is read
//! from and written back into, and what an item is stored as once what it
//! has no use for is left out.

use retiretui_engine::plan::{AccountKind, IncomeKind, US};
use toml::{Table, Value};

use super::offers::Vocabulary;
use super::{FieldSpec, Form};
use crate::codec::{get_path, set_path};

/// The key of the pick that says whether an item recurs. No file holds it:
/// it is read from whether the item states when it happens once, and it is
/// never written.
pub const HAPPENS: &str = "happens";
/// The timing that recurs every year.
pub(crate) const EVERY_YEAR: &str = "every-year";
/// The timing that happens once.
pub(crate) const ONCE: &str = "once";

/// The key a one-off item's date is under.
pub const ON_KEY: &str = "on";
const KIND_KEY: &str = "kind";
const COUNTRY_KEY: &str = "country";
const ROTH_KEY: &str = "roth";
const SEPARATED_KEY: &str = "separated";

/// An item's timing, read from whether it states when it happens once.
#[must_use]
pub(crate) fn seed_timing(item: &Table) -> Value {
    let timing = if item.contains_key(ON_KEY) {
        ONCE
    } else {
        EVERY_YEAR
    };
    Value::String(timing.to_owned())
}

/// A derived field whose value the file's own keys already say.
pub(crate) fn write_nothing(_: &mut Table, _: &Value) {}

fn word<'a>(item: &'a Table, key: &str) -> Option<&'a str> {
    item.get(key).and_then(Value::as_str)
}

/// A windfall happens once whatever is picked, so it is not asked.
#[must_use]
pub(crate) fn chooses_timing(item: &Table) -> bool {
    word(item, KIND_KEY) != Some(IncomeKind::Windfall.as_str())
}

/// Whether the income's job can have a workplace plan that covers its owner.
#[must_use]
pub(crate) fn can_be_covered(item: &Table) -> bool {
    let kind = item.get(KIND_KEY).cloned();
    let kind = kind.and_then(|kind| kind.try_into::<IncomeKind>().ok());
    kind.is_some_and(IncomeKind::can_be_covered)
}

/// Whether the item happens once: picked so, or a windfall.
#[must_use]
pub fn happens_once(item: &Table) -> bool {
    word(item, HAPPENS) == Some(ONCE) || !chooses_timing(item)
}

/// Whether the item recurs every year.
#[must_use]
pub fn recurs(item: &Table) -> bool {
    !happens_once(item)
}

impl FieldSpec {
    /// Whether an item recurs or happens once, which no file states.
    pub(crate) const fn timing() -> Self {
        Self::choice(HAPPENS, "Happens", Vocabulary::Timing)
            .shown_when(chooses_timing)
            .derived(seed_timing, write_nothing)
            .help("Every year between two dates, or once in a single year.")
    }

    /// What an item is called, its ID shown while it is blank.
    #[must_use]
    pub const fn name(help: &'static str) -> Self {
        Self::text("name", "Name").blank("The ID").help(help)
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

/// Whether the account keeps a cost basis.
pub(crate) fn keeps_basis(item: &Table) -> bool {
    let is_roth = item.get(ROTH_KEY).and_then(Value::as_bool) == Some(true);
    account_is(item, |kind| kind.keeps_basis(is_roth))
}

/// Whether the account's kind has a Roth side.
pub(crate) fn supports_roth(item: &Table) -> bool {
    account_is(item, AccountKind::supports_roth)
}

/// Whether leaving the job the account is with can free it from the
/// early-withdrawal penalty.
pub(crate) fn frees_on_separation(item: &Table) -> bool {
    let is_roth = item.get(ROTH_KEY).and_then(Value::as_bool) == Some(true);
    account_is(item, |kind| kind.frees_on_separation(is_roth))
}

/// Whether the account says when its job is left, and so has an age to
/// lower.
pub(crate) fn states_separation(item: &Table) -> bool {
    frees_on_separation(item) && item.contains_key(SEPARATED_KEY)
}

/// Whether the residency is in the United States.
#[must_use]
pub(crate) fn is_in_the_us(item: &Table) -> bool {
    word(item, COUNTRY_KEY) == Some(US)
}

/// The item `pristine` as its form edits it: with each field no file holds
/// read from it.
#[must_use]
pub fn opened(ops: &Form, pristine: &Table) -> Table {
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
#[must_use]
pub fn stale(ops: &Form, pristine: &Table, snapshot: &Table) -> Vec<&'static str> {
    let stated = (ops.typed)(pristine.clone());
    let is_stale = |spec: &&FieldSpec| {
        let is_unused = !spec.is_shown_for(snapshot);
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

/// Why a one-off item with no date cannot be applied.
pub const ONCE_UNDATED: &str = "happens once, but does not say when";

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
    fn only_a_salary_is_asked_whether_its_job_covers_its_owner() {
        use super::super::Domain;
        use super::super::income::Incomes;
        let covered = Incomes::FIELDS.iter().find(|field| field.key == "covered");
        let covered = covered.expect("the Income form asks it");
        assert!(covered.is_shown_for(&item("kind = \"salary\"")));
        assert!(!covered.is_shown_for(&item("kind = \"pension\"")));
        assert!(!covered.is_shown_for(&Table::new()));
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

    #[test]
    fn only_a_tax_deferred_workplace_plan_is_asked_when_its_job_is_left() {
        use super::super::Domain;
        use super::super::accounts::Accounts;
        let field = |key: &str| Accounts::FIELDS.iter().find(|field| field.key == key);
        let left = field("separated").expect("the Account form asks it");
        for kind in ["401k", "403b", "414k"] {
            assert!(left.is_shown_for(&item(&format!("kind = \"{kind}\""))));
        }
        for kind in ["457b", "ira", "brokerage"] {
            assert!(!left.is_shown_for(&item(&format!("kind = \"{kind}\""))));
        }
        assert!(!left.is_shown_for(&item("kind = \"401k\"\nroth = true")));
        assert!(!left.is_shown_for(&Table::new()));
        let flag = field("public_safety").expect("the Account form asks it");
        assert!(!flag.is_shown_for(&item("kind = \"401k\"")));
        assert!(flag.is_shown_for(&item("kind = \"401k\"\nseparated = { age = 56 }")));
        assert!(!flag.is_shown_for(&item("kind = \"ira\"\nseparated = { age = 56 }")));
    }
}
