//! What a select offers: each value the file keeps under the words shown
//! for it, from the schema's closed sets and from the plan's own items.

use retiretui_engine::plan::{
    Account, AccountKind, COUNTRIES, Draw, FilingStatus, Income, IncomeKind, Payer, Plan,
    TreatmentClass, TriggerBasis, US_STATES,
};
use toml::Table;

use super::accounts;
use super::applies;
use super::cells::field_of;
use super::codec::to_text;
use super::contributions;
use super::domain::{FieldKind, FieldSpec};
use crate::commands::tui::present;
use crate::commands::tui::setup::{EXAMPLES, LifeStage};

/// A closed set the schema states, read from the schema rather than
/// restated here.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Vocabulary {
    AccountKind,
    IncomeKind,
    FilingStatus,
    TriggerBasis,
    TreatmentClass,
    /// The new plan's own: where a household is in life.
    LifeStage,
    /// The new plan's own: the example plan it starts from.
    Example,
    Country,
    UsState,
    /// The form's own: whether an item recurs or happens once.
    Timing,
    Payer,
    /// The form's own: which way a contribution states its amount.
    AmountForm,
    /// The form's own: how an account says what it earns.
    Holding,
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
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Offer {
    pub value: String,
    pub label: String,
}

impl Offer {
    /// An offer with no words of its own, shown as the file spells it.
    pub fn spelt(value: String) -> Self {
        Self {
            label: value.clone(),
            value,
        }
    }
}

impl Vocabulary {
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
    Person,
    Account,
    /// A tax-deferred account, which a conversion drains.
    DeferredAccount,
    /// A Roth account, which a conversion fills.
    RothAccount,
    Event,
    Income,
}

/// What `source` offers, in plan order: each id under the item's display
/// name, with the id beside it where two names are the same.
pub fn ref_offers(plan: &Plan, source: RefSource) -> Vec<Offer> {
    let named = |id: &str, name: Option<&String>| Offer {
        value: id.to_owned(),
        label: name.map_or(id, String::as_str).to_owned(),
    };
    let account = |account: &Account| named(&account.id, account.name.as_ref());
    let income = |income: &Income| named(&income.id, income.name.as_ref());
    let accounts_of = |class: TreatmentClass| {
        let held = plan.accounts.iter();
        held.filter(move |account| account.treatment() == class)
            .map(account)
            .collect()
    };
    let people = plan.household.people.iter();
    let offers: Vec<Offer> = match source {
        RefSource::Person => people
            .map(|person| named(&person.id, person.name.as_ref()))
            .collect(),
        RefSource::Account => plan.accounts.iter().map(account).collect(),
        RefSource::DeferredAccount => accounts_of(TreatmentClass::Deferred),
        RefSource::RothAccount => accounts_of(TreatmentClass::Roth),
        RefSource::Event => plan
            .events
            .iter()
            .map(|event| named(&event.id, event.name.as_ref()))
            .collect(),
        RefSource::Income => plan.income.iter().map(income).collect(),
    };
    told_apart(offers)
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

/// The key every item's display name is stated under.
pub(super) const NAME_KEY: &str = "name";

/// What an item is called: its display name where it states one, else
/// what it is known by - under the words for it, where `fields` has that
/// picked from a closed set.
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
