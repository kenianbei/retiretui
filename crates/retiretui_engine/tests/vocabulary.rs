//! The vocabularies the schema states for other surfaces must be the ones
//! serde reads and writes.

use std::collections::BTreeSet;
use std::fmt::Debug;

use retiretui_engine::plan::{
    AccountKind, AssetClass, Draw, FilingStatus, IncomeKind, Operand, PlanDate, TreatmentClass,
    Trigger, TriggerBasis,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

fn assert_spelt_as_serde_spells<T>(all: &[T], as_str: fn(T) -> &'static str)
where
    T: Copy + Debug + PartialEq + Serialize + DeserializeOwned,
{
    let mut seen = BTreeSet::new();
    for &each in all {
        let spelt = as_str(each);
        assert_eq!(serde_json::to_value(each).unwrap(), spelt, "{each:?}");
        let read: T = serde_json::from_value(spelt.into()).unwrap();
        assert_eq!(read, each);
        assert!(seen.insert(spelt.to_owned()), "{spelt} is listed twice");
    }
    assert_eq!(seen, variants_serde_knows::<T>(), "`ALL` misses a variant");
}

/// Every variant of `T`, as serde lists them when refusing an unknown one.
fn variants_serde_knows<T: DeserializeOwned + Debug>() -> BTreeSet<String> {
    const UNKNOWN: &str = "no-such-variant";
    let refusal = serde_json::from_value::<T>(UNKNOWN.into())
        .unwrap_err()
        .to_string();
    let quoted = refusal.split('`').skip(1).step_by(2);
    quoted
        .filter(|word| *word != UNKNOWN)
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_closed_enum_is_spelt_as_serde_spells_it() {
    assert_spelt_as_serde_spells(AccountKind::ALL, AccountKind::as_str);
    assert_spelt_as_serde_spells(IncomeKind::ALL, IncomeKind::as_str);
    assert_spelt_as_serde_spells(FilingStatus::ALL, FilingStatus::as_str);
    assert_spelt_as_serde_spells(TreatmentClass::ALL, TreatmentClass::as_str);
    assert_spelt_as_serde_spells(AssetClass::ALL, AssetClass::as_str);
    assert_spelt_as_serde_spells(Draw::ALL, Draw::as_str);
}

#[test]
fn the_operands_are_exactly_the_keys_a_trigger_has() {
    let full = Trigger {
        date: Some(PlanDate(jiff::civil::date(2030, 1, 1))),
        age: Some(60),
        owner: Some("me".into()),
        event: Some("retire".into()),
        income: Some("job".into()),
        offset: Some(1),
    };
    let written = serde_json::to_value(full).unwrap();
    let keys: BTreeSet<&str> = written
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let stated: BTreeSet<&str> = TriggerBasis::ALL
        .iter()
        .flat_map(|basis| basis.operands())
        .map(|operand| operand.key())
        .collect();
    assert_eq!(stated, keys);
}

#[test]
fn a_trigger_stating_a_basis_and_its_operands_classifies() {
    for &basis in TriggerBasis::ALL {
        let mut table = toml::Table::new();
        for operand in basis.operands() {
            let value = match operand {
                Operand::Date => toml::Value::Datetime("2030-01-01".parse().unwrap()),
                Operand::Years | Operand::Offset => toml::Value::Integer(1),
                Operand::Owner | Operand::EventId | Operand::IncomeId => "id".into(),
            };
            table.insert(operand.key().to_owned(), value);
        }
        let trigger: Trigger = toml::from_str(&toml::to_string(&table).unwrap()).unwrap();
        assert_eq!(trigger.basis(), Ok(basis));
        assert_eq!(basis.as_str(), basis.operands()[0].key());
        trigger.form().unwrap();
    }
}
