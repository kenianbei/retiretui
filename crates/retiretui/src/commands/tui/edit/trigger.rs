//! The trigger editor: a closed vocabulary, so it is picked rather than
//! typed. A trigger takes one row that reads as a sentence - the kind, then
//! the operands that kind needs with the words that go between them.
//!
//! Every operand has a widget of its own, so each kind keeps what was
//! entered for it while another is tried; which of them a kind shows is
//! not fixed. An operand that names something the plan declares is a pick
//! over the plan's own ids, so a reference cannot be misspelt.

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Changed, Commands, Component, Entity, Query};
use bevy_ui::{Node, UiRect, Val};
use plurimus::widgets::TextInput;
use retiretui_engine::plan::{Operand, Plan, TriggerBasis};
use toml::{Table, Value};

use super::build::FormField;
use super::codec::{parse_text, to_text};
use super::domain::{BLANK, FieldKind, FieldSpec};
use super::field::{sharing, show_text, spawn_beside, spawn_text_as};
use super::offers::{RefSource, Vocabulary};
use super::select::{Select, spawn_select};
use crate::commands::tui::layout::{set_display, sized};

/// Which part of a trigger a widget edits.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Kind,
    Part(Operand),
}

/// What a trigger's row holds after its kind, in the order it is read.
#[derive(Clone, Copy)]
enum Piece {
    /// A typed operand in so many cells, with the words that go before
    /// and after it wherever it goes.
    Text(Operand, f32, [&'static str; 2]),
    Pick(Operand, RefSource),
}

/// Cells a date takes, the longest thing typed into a trigger.
const DATE_WIDTH: f32 = 10.0;

/// Cells an age or an offset takes.
const NUMBER_WIDTH: f32 = 4.0;

const SENTENCE: &[Piece] = &[
    Piece::Text(Operand::Date, DATE_WIDTH, ["", "yyyy-mm-dd"]),
    Piece::Text(Operand::Years, NUMBER_WIDTH, ["", "of "]),
    Piece::Pick(Operand::Owner, RefSource::Person),
    Piece::Pick(Operand::EventId, RefSource::Event),
    Piece::Pick(Operand::IncomeId, RefSource::Income),
    Piece::Text(Operand::Offset, NUMBER_WIDTH, [" offset ", "years"]),
];

const fn help_of(operand: Operand) -> &'static str {
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

/// The basis a trigger's kind select holds: the one place a basis is
/// read back out of the word a select shows.
fn chosen_basis(select: &Select) -> Option<TriggerBasis> {
    let chosen = select.value();
    let word = chosen.as_ref().and_then(Value::as_str)?;
    TriggerBasis::ALL
        .iter()
        .copied()
        .find(|basis| basis.as_str() == word)
}

fn shows(kind: Option<TriggerBasis>, operand: Operand) -> bool {
    kind.is_some_and(|kind| kind.operands().contains(&operand))
}

/// The kind a trigger table states, by the key it carries.
fn kind_of(value: Option<&Value>) -> Option<TriggerBasis> {
    let table = value?.as_table()?;
    TriggerBasis::ALL
        .iter()
        .copied()
        .find(|basis| table.contains_key(basis.as_str()))
}

/// Cells the kind takes, and the one kept clear after it.
const KIND_WIDTH: f32 = 12.0;
const KIND_GAP: f32 = 1.0;

fn kind_node() -> Node {
    Node {
        margin: UiRect::right(Val::Px(KIND_GAP)),
        ..sized(KIND_WIDTH, 1.0)
    }
}

/// The widgets a trigger takes along `row`, in tab order: its kind, then
/// the sentence. Each operand says what it means for itself.
pub fn spawn(commands: &mut Commands, row: Entity, field: FormField) {
    let kind = FieldKind::Choice(Vocabulary::TriggerBasis);
    let kind = Select::new(kind, field.spec.blank_word()).skipping_blank();
    let kind = spawn_select(commands, kind, field);
    commands
        .entity(kind)
        .insert((Slot::Kind, kind_node(), ChildOf(row)));
    for &piece in SENTENCE {
        let (Piece::Text(operand, ..) | Piece::Pick(operand, _)) = piece;
        let field = FormField {
            spec: FieldSpec {
                help: help_of(operand),
                ..field.spec
            },
        };
        match piece {
            Piece::Text(_, cells, [before, after]) => {
                let input = commands.spawn_empty().id();
                spawn_beside(commands, row, input, before);
                let held = (sized(cells, 1.0), field, Slot::Part(operand));
                spawn_text_as(commands, row, input, held);
                spawn_beside(commands, row, input, after);
            }
            Piece::Pick(_, source) => {
                let select = Select::new(FieldKind::Ref(source), BLANK);
                let pick = spawn_select(commands, select, field);
                commands
                    .entity(pick)
                    .insert((Slot::Part(operand), sharing(), ChildOf(row)));
            }
        }
    }
}

/// A trigger's operands take room only while the chosen kind has a use
/// for them. The kind's pick decides rather than the value, because a kind
/// chosen before its operands are filled composes to nothing yet still has
/// a sentence to show. A word goes where its operand does, as a bracket.
pub fn place_slots(
    kinds: Query<(&Slot, &Select, &ChildOf), Changed<Select>>,
    rows: Query<&Children>,
    mut slots: Query<(&Slot, &mut Node)>,
) {
    for (slot, select, row) in &kinds {
        if *slot != Slot::Kind {
            continue;
        }
        let kind = chosen_basis(select);
        for &widget in rows.get(row.parent()).into_iter().flatten() {
            let Ok((slot, mut node)) = slots.get_mut(widget) else {
                continue;
            };
            // With no kind there is no sentence, so the kind has the row
            // to say what an absent trigger means.
            match *slot {
                Slot::Kind => {
                    let wanted = kind.map_or_else(sharing, |_| kind_node());
                    if *node != wanted {
                        *node = wanted;
                    }
                }
                Slot::Part(operand) => set_display(&mut node, shows(kind, operand)),
            }
        }
    }
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

/// A trigger's widgets, for reading them together.
pub type Slots<'w, 's> = Query<
    'w,
    's,
    (
        &'static Slot,
        Option<&'static TextInput>,
        Option<&'static Select>,
    ),
>;

/// A trigger's widgets, for filling them.
pub type SlotsMut<'w, 's> = Query<
    'w,
    's,
    (
        &'static Slot,
        Option<&'static mut TextInput>,
        Option<&'static mut Select>,
    ),
>;

const INCOMPLETE: &str = "the trigger names what it is measured from, but not the value";

/// The trigger the widgets of `group` hold between them, since its parts
/// only mean something together, or the complaint where a kind is chosen
/// that they do not yet make a trigger of.
pub fn held(group: &[Entity], slots: &Slots) -> (Option<Value>, Option<&'static str>) {
    let mut kind = None;
    let mut parts = Vec::new();
    for (slot, text, select) in slots.iter_many(group) {
        let Slot::Part(operand) = *slot else {
            kind = select.and_then(chosen_basis);
            continue;
        };
        let value = match (text, select) {
            (Some(text), _) => parse_text(text.value()),
            (_, Some(select)) => select.value(),
            _ => None,
        };
        parts.push((operand, value));
    }
    let composed = compose(kind, &parts);
    let complaint = (kind.is_some() && composed.is_none()).then_some(INCOMPLETE);
    (composed, complaint)
}

/// Fills a trigger's widgets from `value`: its kind, and every operand
/// the value states.
pub fn show_trigger(value: Option<&Value>, plan: &Plan, group: &[Entity], slots: &mut SlotsMut) {
    let stated = value.and_then(Value::as_table);
    let mut widgets = slots.iter_many_mut(group);
    while let Some((slot, text, select)) = widgets.fetch_next() {
        let part = match *slot {
            Slot::Kind => kind_of(value).map(|kind| Value::String(kind.as_str().to_owned())),
            Slot::Part(operand) => stated.and_then(|table| table.get(operand.key()).cloned()),
        };
        if let Some(mut text) = text {
            show_text(&mut text, part.as_ref().map(to_text).unwrap_or_default());
        }
        if let Some(mut select) = select {
            let options = select.referred(plan);
            Select::fill(&mut select, options, part.as_ref());
        }
    }
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
                let is_placed = SENTENCE.iter().any(|piece| match piece {
                    Piece::Text(placed, ..) | Piece::Pick(placed, _) => placed == operand,
                });
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
