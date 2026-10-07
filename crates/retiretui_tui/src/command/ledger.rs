//! The Ledger's own commands, each scoped to its page.

use bevy_input::keyboard::Key;
use plurimus::ui::KeyBinding;

use super::keys::character;
use super::{
    CommandSpec, LEDGER_COLUMNS, LEDGER_MARKED, LEDGER_PLAN, LEDGER_TABLE, LEDGER_YEAR,
    LEDGER_YEARS, Register, Scope,
};
use crate::ledger;
use crate::nav::Page;
use crate::session;

/// A row of the Ledger's own.
fn on_ledger(
    name: &'static str,
    doc: &'static str,
    keys: Vec<KeyBinding>,
    register: Register,
) -> CommandSpec {
    CommandSpec {
        name,
        scope: Scope::On(Page::Ledger),
        doc,
        keys,
        hint: None,
        register,
    }
}

/// The Ledger's rows.
pub(super) fn commands() -> Vec<CommandSpec> {
    let mut commands = views();
    commands.extend(years());
    commands
}

/// A run left for the plan, the Ledger's two views and the table's columns.
fn views() -> Vec<CommandSpec> {
    let arrow = KeyBinding::new;
    vec![
        CommandSpec {
            hint: Some("the plan"),
            ..on_ledger(
                LEDGER_PLAN,
                "return the ledger from a market run to the plan",
                vec![arrow(Key::Escape)],
                Box::new(|world| world.register_system(ledger::return_to_plan)),
            )
        },
        on_ledger(
            LEDGER_TABLE,
            "give the year table the whole page, or bring the year back under it",
            vec![character("t")],
            Box::new(|world| world.register_system(ledger::swap_table)),
        ),
        on_ledger(
            LEDGER_YEAR,
            "show the year the table's cursor is on under it",
            vec![arrow(Key::Enter)],
            Box::new(|world| world.register_system(ledger::show_year)),
        ),
        on_ledger(
            LEDGER_COLUMNS,
            "turn the year table to its next set of columns",
            vec![character("c")],
            Box::new(|world| world.register_system(ledger::turn_columns)),
        ),
    ]
}

/// The year stepped by one, or to the nearest marked.
fn years() -> Vec<CommandSpec> {
    let arrow = KeyBinding::new;
    vec![
        on_ledger(
            LEDGER_YEARS[0],
            "move the year a year back",
            vec![character("["), arrow(Key::ArrowLeft)],
            Box::new(|world| world.register_system(session::previous_year)),
        ),
        on_ledger(
            LEDGER_YEARS[1],
            "move the year a year on",
            vec![character("]"), arrow(Key::ArrowRight)],
            Box::new(|world| world.register_system(session::next_year)),
        ),
        on_ledger(
            LEDGER_MARKED[0],
            "move the year back to the nearest with a milestone or a warning",
            vec![character("{")],
            Box::new(|world| world.register_system(ledger::previous_marked)),
        ),
        on_ledger(
            LEDGER_MARKED[1],
            "move the year on to the nearest with a milestone or a warning",
            vec![character("}")],
            Box::new(|world| world.register_system(ledger::next_marked)),
        ),
    ]
}
