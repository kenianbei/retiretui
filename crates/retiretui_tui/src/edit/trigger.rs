//! The trigger editor: one row that reads as the client's sentence. Every
//! operand has a widget of its own, so each kind keeps what was entered for
//! it while another is tried; which of them a kind shows is not fixed.

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Changed, Commands, Component, Entity, Query};
use bevy_ui::{Node, UiRect, Val};
use plurimus::widgets::TextInput;
use retiretui_engine::plan::{Operand, Plan, TriggerBasis};
use toml::Value;

use super::build::FormField;
use super::codec::to_text;
use super::domain::{BLANK, FieldKind, FieldSpec};
use super::field::{sharing, show_text, spawn_beside, spawn_text_as};
use super::offers::Vocabulary;
use super::select::{Select, spawn_select};
use crate::layout::{set_display, sized};
use retiretui_client::forms::trigger::{
    BASIS, Piece, SENTENCE, basis_named, help_of, kind_of, operand_of, shows,
};

/// Which part of a trigger a widget edits.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Kind,
    Part(Operand),
}

impl Slot {
    /// The part of the trigger the client's entry takes it as.
    pub const fn part(self) -> &'static str {
        match self {
            Self::Kind => BASIS,
            Self::Part(operand) => operand.key(),
        }
    }
}

/// Cells a date takes, the longest thing typed into a trigger.
const DATE_WIDTH: f32 = 10.0;

/// Cells an age or an offset takes.
const NUMBER_WIDTH: f32 = 4.0;

const fn width_of(operand: Operand) -> f32 {
    match operand {
        Operand::Date => DATE_WIDTH,
        _ => NUMBER_WIDTH,
    }
}

/// The basis a trigger's kind select holds: the one place a basis is
/// read back out of the word a select shows.
fn chosen_basis(select: &Select) -> Option<TriggerBasis> {
    let chosen = select.value();
    basis_named(chosen.as_ref().and_then(Value::as_str)?)
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
        let operand = piece.operand();
        let field = FormField {
            spec: FieldSpec {
                help: help_of(operand),
                ..field.spec
            },
        };
        match piece {
            Piece::Text(_, [before, after]) => {
                let input = commands.spawn_empty().id();
                spawn_beside(commands, row, input, before);
                let held = (sized(width_of(operand), 1.0), field, Slot::Part(operand));
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

/// Fills a trigger's widgets from `value`: its kind, and every operand
/// the value states.
pub fn show_trigger(value: Option<&Value>, plan: &Plan, group: &[Entity], slots: &mut SlotsMut) {
    let mut widgets = slots.iter_many_mut(group);
    while let Some((slot, text, select)) = widgets.fetch_next() {
        let part = match *slot {
            Slot::Kind => kind_of(value).map(|kind| Value::String(kind.as_str().to_owned())),
            Slot::Part(operand) => operand_of(value, operand).cloned(),
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
