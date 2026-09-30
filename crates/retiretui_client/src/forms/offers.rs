//! What a select offers: each value the file keeps under the words shown
//! for it, from the schema's closed sets and from the plan's own items.

use retiretui_engine::plan::{
    AccountKind, COUNTRIES, Draw, FilingStatus, IncomeKind, Item, Payer, Plan, TreatmentClass,
    TriggerBasis, US_STATES,
};
use serde::Serialize;
use toml::{Table, Value};

use super::accounts;
use super::applies;
use super::cells::field_of;
use super::contributions;
use super::{BLANK, FieldKind, FieldSpec};
use crate::codec::to_text;
use crate::present;
use crate::setup::{EXAMPLES, LifeStage};

/// A closed set the schema states, read from the schema rather than
/// restated here.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Vocabulary {
    /// An account's kind.
    AccountKind,
    /// An income's kind.
    IncomeKind,
    /// A filing status.
    FilingStatus,
    /// What a trigger rests on.
    TriggerBasis,
    /// A tax treatment class.
    TreatmentClass,
    /// The new plan's own: where a household is in life.
    LifeStage,
    /// The new plan's own: the example plan it starts from.
    Example,
    /// A country.
    Country,
    /// A U.S. state.
    UsState,
    /// The form's own: whether an item recurs or happens once.
    Timing,
    /// Who pays a contribution.
    Payer,
    /// The form's own: which way a contribution states its amount.
    AmountForm,
    /// The form's own: how an account says what it earns.
    Holding,
    /// How a market is drawn.
    Draw,
}

const TIMINGS: &[(&str, &str)] = &[(applies::EVERY_YEAR, "Every year"), (applies::ONCE, "Once")];

const HOLDINGS: &[(&str, &str)] = &[
    (accounts::FIXED, "Fixed return"),
    (accounts::MIX, "One mix"),
    (accounts::GLIDE, "Glide path"),
];

const AMOUNT_FORMS: &[(&str, &str)] = &[
    (contributions::DOLLARS, "Dollars"),
    (contributions::SHARE, "Share of income"),
    (contributions::MAXIMUM, "The maximum"),
    (contributions::MATCH, "Employer match"),
];

/// One thing a select offers: the value the file keeps, and the words
/// shown for it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Offer {
    /// What the file keeps.
    pub value: String,
    /// The words shown for it.
    pub label: String,
}

impl Offer {
    /// An offer with no words of its own, shown as the file spells it.
    #[must_use]
    pub fn spelt(value: String) -> Self {
        Self {
            label: value.clone(),
            value,
        }
    }
}

impl Vocabulary {
    /// Every word of the vocabulary, as a pick offers it.
    pub fn offers(self) -> Vec<Offer> {
        fn offers<T: Copy>(
            all: &[T],
            as_str: fn(T) -> &'static str,
            label: fn(T) -> &'static str,
        ) -> Vec<Offer> {
            let offer = |&each: &T| Offer {
                value: as_str(each).to_owned(),
                label: label(each).to_owned(),
            };
            all.iter().map(offer).collect()
        }
        match self {
            Self::AccountKind => {
                offers(AccountKind::ALL, AccountKind::as_str, present::account_kind)
            }
            Self::IncomeKind => offers(IncomeKind::ALL, IncomeKind::as_str, present::income_kind),
            Self::FilingStatus => offers(
                FilingStatus::ALL,
                FilingStatus::as_str,
                present::filing_status,
            ),
            Self::TriggerBasis => offers(
                TriggerBasis::ALL,
                TriggerBasis::as_str,
                present::trigger_basis,
            ),
            Self::TreatmentClass => offers(
                TreatmentClass::ALL,
                TreatmentClass::as_str,
                present::treatment_class,
            ),
            Self::LifeStage => offers(LifeStage::ALL, LifeStage::as_str, LifeStage::label),
            Self::Example => offers(EXAMPLES, |example| example.0, |example| example.1),
            Self::Country => offers(COUNTRIES, |place| place.0, |place| place.1),
            Self::UsState => offers(US_STATES, |place| place.0, |place| place.1),
            Self::Timing => offers(TIMINGS, |timing| timing.0, |timing| timing.1),
            Self::Payer => offers(Payer::ALL, Payer::as_str, present::payer),
            Self::AmountForm => offers(AMOUNT_FORMS, |form| form.0, |form| form.1),
            Self::Holding => offers(HOLDINGS, |form| form.0, |form| form.1),
            Self::Draw => offers(Draw::ALL, Draw::as_str, present::draw),
        }
    }
}

/// Where a reference field's candidates come from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RefSource {
    /// A person of the household.
    Person,
    /// Any account.
    Account,
    /// A tax-deferred account, which a conversion drains.
    DeferredAccount,
    /// A Roth account, which a conversion fills.
    RothAccount,
    /// A named event.
    Event,
    /// An income.
    Income,
}

/// What `source` offers, in plan order: each id under the item's display
/// name, with the id beside it where two names are the same.
pub fn ref_offers(plan: &Plan, source: RefSource) -> Vec<Offer> {
    let accounts_of = |class: TreatmentClass| {
        let held = plan.accounts.iter();
        held.filter(move |account| account.treatment() == class)
            .map(offer)
            .collect()
    };
    let offers: Vec<Offer> = match source {
        RefSource::Person => plan.household.people.iter().map(offer).collect(),
        RefSource::Account => plan.accounts.iter().map(offer).collect(),
        RefSource::DeferredAccount => accounts_of(TreatmentClass::Deferred),
        RefSource::RothAccount => accounts_of(TreatmentClass::Roth),
        RefSource::Event => plan.events.iter().map(offer).collect(),
        RefSource::Income => plan.income.iter().map(offer).collect(),
    };
    told_apart(offers)
}

fn offer(item: &impl Item) -> Offer {
    Offer {
        value: item.id().to_owned(),
        label: item.display_name().to_owned(),
    }
}

/// Adds the id to every label another offer shares.
fn told_apart(offers: Vec<Offer>) -> Vec<Offer> {
    let shared: Vec<bool> = offers
        .iter()
        .map(|offer| {
            offers
                .iter()
                .filter(|each| each.label == offer.label)
                .count()
                > 1
        })
        .collect();
    offers
        .into_iter()
        .zip(shared)
        .map(|(offer, is_shared)| Offer {
            label: if is_shared {
                format!("{} ({})", offer.label, offer.value)
            } else {
                offer.label
            },
            value: offer.value,
        })
        .collect()
}

/// What a pick offers: every candidate, a held word among them, and the
/// word for holding none where it may.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PickOffers {
    /// What may be picked, in the order offered.
    pub offers: Vec<Offer>,
    /// What holding none reads as; none where the pick may not be emptied.
    pub blank: Option<&'static str>,
}

/// The word a pick holding `value` shows: a string as spelt, anything else
/// as the file writes it; none where nothing, or an empty word, is held.
#[must_use]
pub fn held_word(value: Option<&Value>) -> Option<String> {
    let word = match value? {
        Value::String(word) => word.clone(),
        other => to_text(other),
    };
    (!word.is_empty()).then_some(word)
}

/// `candidates`, holding `held` whatever it is since the file stated it.
fn holding(mut candidates: Vec<Offer>, held: Option<&str>) -> Vec<Offer> {
    if let Some(held) = held.filter(|held| !held.is_empty())
        && !candidates.iter().any(|offer| offer.value == held)
    {
        candidates.push(Offer::spelt(held.to_owned()));
    }
    candidates
}

/// `candidates`, holding `held` whatever it is since the file stated it, and
/// `spec`'s blank word unless it may not be emptied.
#[must_use]
pub fn offered(spec: &FieldSpec, candidates: Vec<Offer>, held: Option<&str>) -> PickOffers {
    PickOffers {
        offers: holding(candidates, held),
        blank: (!spec.is_required()).then(|| spec.blank_word()),
    }
}

/// What a field's pick chooses between: its vocabulary, or the plan's ids.
#[must_use]
pub fn candidates(kind: FieldKind, plan: &Plan) -> Vec<Offer> {
    match kind {
        FieldKind::Choice(vocabulary) | FieldKind::Order(vocabulary, _) => vocabulary.offers(),
        FieldKind::Ref(source) => ref_offers(plan, source),
        _ => Vec::new(),
    }
}

/// What a trigger's kind offers: every basis, and `spec`'s blank word,
/// since clearing the kind is how a trigger is cleared.
#[must_use]
pub fn trigger_bases(spec: &FieldSpec) -> PickOffers {
    PickOffers {
        offers: Vocabulary::TriggerBasis.offers(),
        blank: Some(spec.blank_word()),
    }
}

/// What a trigger's operand picked from `source` offers, holding `held`.
#[must_use]
pub fn operand_offers(plan: &Plan, source: RefSource, held: Option<&str>) -> PickOffers {
    PickOffers {
        offers: holding(ref_offers(plan, source), held),
        blank: Some(BLANK),
    }
}

/// The key every item's display name is stated under.
pub const NAME_KEY: &str = "name";

/// What an item is called: its display name where it states one, else
/// what it is known by - under the words for it, where `fields` has that
/// picked from a closed set.
#[must_use]
pub fn display_name(item: &Table, identity: &str, fields: &[FieldSpec]) -> Option<String> {
    let stated = |key: &str| item.get(key).map(to_text).filter(|text| !text.is_empty());
    let known = || {
        let known = stated(identity)?;
        let Some(FieldKind::Choice(vocabulary)) = field_of(fields, identity).map(|spec| spec.kind)
        else {
            return Some(known);
        };
        let mut offers = vocabulary.offers().into_iter();
        let picked = offers.find(|offer| offer.value == known);
        Some(picked.map_or(known, |offer| offer.label))
    };
    stated(NAME_KEY).or_else(known)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::DomainId;
    use crate::forms::edit::form_of;

    fn account_field(key: &str) -> FieldSpec {
        *field_of(form_of(DomainId::Accounts).fields, key).expect("a field")
    }

    #[test]
    fn a_pick_holds_a_word_the_file_stated_but_not_an_empty_one() {
        let kind = account_field("kind");
        let offers = |held| offered(&kind, Vocabulary::AccountKind.offers(), held).offers;
        let all = Vocabulary::AccountKind.offers().len();
        assert_eq!(
            offers(Some("brokerage")).len(),
            all,
            "a candidate is not added twice"
        );
        let stated = offers(Some("pension"));
        assert_eq!(stated.last(), Some(&Offer::spelt("pension".to_owned())));
        assert_eq!(offers(Some("")).len(), all);
        assert_eq!(held_word(Some(&Value::String(String::new()))), None);
        assert_eq!(held_word(Some(&Value::Integer(3))).as_deref(), Some("3"));
    }

    #[test]
    fn a_blank_is_offered_in_the_fields_own_word_unless_it_is_required() {
        let kind = account_field("kind");
        assert_eq!(offered(&kind, Vec::new(), None).blank, None);
        let locked = account_field("locked_until");
        assert_eq!(trigger_bases(&locked).blank, Some("Never"));
        let plan = Plan::from_toml_str(crate::setup::EXAMPLES[0].2).expect("parses");
        let operand = operand_offers(&plan, RefSource::Event, Some("gone"));
        assert_eq!(operand.blank, Some(BLANK));
        assert_eq!(
            operand.offers.last().map(|offer| offer.value.as_str()),
            Some("gone")
        );
    }

    fn offer(value: &str, label: &str) -> Offer {
        Offer {
            value: value.to_owned(),
            label: label.to_owned(),
        }
    }

    #[test]
    fn only_offers_sharing_a_name_show_their_ids() {
        let offers = told_apart(vec![
            offer("ira-a", "Rollover"),
            offer("cash", "Cash"),
            offer("ira-b", "Rollover"),
        ]);
        let labels: Vec<&str> = offers.iter().map(|offer| offer.label.as_str()).collect();
        assert_eq!(labels, ["Rollover (ira-a)", "Cash", "Rollover (ira-b)"]);
        assert_eq!(offers[0].value, "ira-a", "the value stays the id");
    }

    #[test]
    fn an_item_goes_by_its_name_and_falls_back_to_what_it_is_known_by() {
        let named: Table = "id = \"fid\"\nname = \"Fidelity 401(k)\"".parse().unwrap();
        assert_eq!(
            display_name(&named, "id", &[]).as_deref(),
            Some("Fidelity 401(k)")
        );
        let bare: Table = "id = \"fid\"".parse().unwrap();
        assert_eq!(display_name(&bare, "id", &[]).as_deref(), Some("fid"));
        assert_eq!(display_name(&Table::new(), "id", &[]), None);
    }
}
