//! A trigger as a form enters it: a closed vocabulary, so it is picked
//! rather than typed - the kind, then the operands that kind needs, read as
//! a sentence with the words that go between them. An operand that names
//! something the plan declares is a pick over the plan's own ids, so a
//! reference cannot be misspelt.

use retiretui_engine::plan::{Operand, TriggerBasis};
use toml::{Table, Value};

use super::offers::RefSource;

/// What a trigger's sentence holds after its kind, in the order it is read.
#[derive(Clone, Copy)]
pub enum Piece {
    /// A typed operand, with the words that go before and after it.
    Text(Operand, [&'static str; 2]),
    /// An operand picked from the plan's own ids.
    Pick(Operand, RefSource),
}

impl Piece {
    /// The operand the piece enters.
    #[must_use]
    pub const fn operand(self) -> Operand {
        match self {
            Self::Text(operand, _) | Self::Pick(operand, _) => operand,
        }
    }
}

/// Every operand a trigger may hold, in the order its sentence reads.
pub const SENTENCE: &[Piece] = &[
    Piece::Text(Operand::Date, ["", "yyyy-mm-dd"]),
    Piece::Text(Operand::Years, ["", "of "]),
    Piece::Pick(Operand::Owner, RefSource::Person),
    Piece::Pick(Operand::EventId, RefSource::Event),
    Piece::Pick(Operand::IncomeId, RefSource::Income),
    Piece::Text(Operand::Offset, [" offset ", "years"]),
];

/// What `operand` means, where the form says it.
#[must_use]
pub const fn help_of(operand: Operand) -> &'static str {
    match operand {
        Operand::Date => "The day, as year-month-day, such as 2032-01-01. Its year is what counts.",
        Operand::Years => "The age reached, in whole years.",
        Operand::Owner => "Whose age it is.",
        Operand::EventId => "The event it follows. Events are named on their own page.",
        Operand::IncomeId => "The income whose first year it follows.",
        Operand::Offset => {
            "Whole years after it. A negative number is years before; blank is none."
        }
    }
}

/// The basis the file spells `word`.
#[must_use]
pub fn basis_named(word: &str) -> Option<TriggerBasis> {
    TriggerBasis::ALL
        .iter()
        .copied()
        .find(|basis| basis.as_str() == word)
}

/// Whether a trigger of `kind` has a use for `operand`.
#[must_use]
pub fn shows(kind: Option<TriggerBasis>, operand: Operand) -> bool {
    kind.is_some_and(|kind| kind.operands().contains(&operand))
}

/// The kind a trigger table states, by the key it carries.
#[must_use]
pub fn kind_of(value: Option<&Value>) -> Option<TriggerBasis> {
    let table = value?.as_table()?;
    TriggerBasis::ALL
        .iter()
        .copied()
        .find(|basis| table.contains_key(basis.as_str()))
}

/// What the trigger `value` states for `operand`.
#[must_use]
pub fn operand_of(value: Option<&Value>, operand: Operand) -> Option<&Value> {
    value?.as_table()?.get(operand.key())
}

/// The trigger `parts` compose, or `None` when no kind is chosen or its
/// own operand is still empty. What another kind's operands hold is left
/// out of it.
fn compose(kind: Option<TriggerBasis>, parts: &[(Operand, Option<Value>)]) -> Option<Value> {
    let basis = kind?;
    let part = |wanted: Operand| {
        parts
            .iter()
            .find(|(operand, _)| *operand == wanted)
            .and_then(|(_, value)| value.clone())
    };
    let mut table = Table::new();
    table.insert(basis.as_str().to_owned(), part(basis.operands()[0])?);
    for &operand in &basis.operands()[1..] {
        if let Some(value) = part(operand) {
            table.insert(operand.key().to_owned(), value);
        }
    }
    Some(Value::Table(table))
}

/// Why a trigger whose kind is chosen cannot be applied yet.
const INCOMPLETE: &str = "the trigger names what it is measured from, but not the value";

/// The trigger a `kind` and its `parts` make between them, since its parts
/// only mean something together, or the complaint where a kind is chosen
/// that they do not yet make a trigger of.
#[must_use]
pub fn held(
    kind: Option<TriggerBasis>,
    parts: &[(Operand, Option<Value>)],
) -> (Option<Value>, Option<&'static str>) {
    let composed = compose(kind, parts);
    let complaint = (kind.is_some() && composed.is_none()).then_some(INCOMPLETE);
    (composed, complaint)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Value {
        Value::Table(text.parse().unwrap())
    }

    #[test]
    fn a_trigger_states_its_kind_by_the_key_it_carries() {
        assert_eq!(kind_of(Some(&table("age = 60"))), Some(TriggerBasis::Age));
        assert_eq!(
            kind_of(Some(&table("event = \"retire\""))),
            Some(TriggerBasis::Event)
        );
        assert_eq!(kind_of(Some(&table("offset = 1"))), None);
        assert_eq!(kind_of(None), None);
    }

    #[test]
    fn each_kind_shows_the_operands_it_needs() {
        assert!(shows(Some(TriggerBasis::Age), Operand::Years));
        assert!(shows(Some(TriggerBasis::Age), Operand::Owner));
        assert!(!shows(Some(TriggerBasis::Age), Operand::EventId));
        assert!(!shows(Some(TriggerBasis::Date), Operand::Owner));
        assert!(shows(Some(TriggerBasis::Event), Operand::Offset));
        assert!(!shows(Some(TriggerBasis::Event), Operand::IncomeId));
        assert!(!shows(None, Operand::Date));
    }

    #[test]
    fn every_operand_has_a_place_in_the_sentence() {
        for basis in TriggerBasis::ALL {
            for operand in basis.operands() {
                let is_placed = SENTENCE.iter().any(|piece| piece.operand() == *operand);
                assert!(is_placed, "{operand:?}");
            }
        }
    }

    #[test]
    fn the_parts_compose_into_the_schema_s_own_shape() {
        let age = compose(
            Some(TriggerBasis::Age),
            &[
                (Operand::Years, Some(Value::Integer(60))),
                (Operand::Owner, Some(Value::String("me".to_owned()))),
                (Operand::EventId, Some(Value::String("retire".to_owned()))),
            ],
        );
        assert_eq!(
            age,
            Some(table("age = 60\nowner = \"me\"")),
            "what another kind keeps is left out"
        );
        let event = compose(
            Some(TriggerBasis::Event),
            &[
                (Operand::EventId, Some(Value::String("retire".to_owned()))),
                (Operand::Offset, Some(Value::Integer(-1))),
            ],
        );
        assert_eq!(event, Some(table("event = \"retire\"\noffset = -1")));
        assert_eq!(compose(None, &[]), None, "no kind is no trigger");
        assert_eq!(
            compose(Some(TriggerBasis::Age), &[]),
            None,
            "a kind with no operand is not a trigger yet"
        );
    }
}
