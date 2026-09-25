//! The tools' own commands, each scoped to the tool's page.

use super::keys::character;
use super::{CommandSpec, Scope};
use retiretui_engine::market::{MonteCarlo, Runs};

use crate::commands::tui::nav::Page;
use crate::commands::tui::tools;
use crate::commands::tui::tools::markets::MarketTool;

/// Every row the Tools tab's pages add to the command table.
pub(super) fn commands() -> Vec<CommandSpec> {
    [
        ladders(),
        claims(),
        people(),
        market::<MonteCarlo>(),
        market::<Runs>(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The Roth Conversions page's rows.
fn ladders() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "write-ladder",
            scope: Scope::On(Page::RothConversions),
            doc: "write the highlighted ladder as a scenario over the document",
            keys: vec![character("w")],
            hint: Some("write"),
            register: Box::new(|world| world.register_system(tools::ladders::write_picker)),
        },
        CommandSpec {
            name: "take-ladder",
            scope: Scope::On(Page::RothConversions),
            doc: "take the highlighted ladder into the plan as conversions",
            keys: vec![character("t")],
            hint: Some("take"),
            register: Box::new(|world| world.register_system(tools::ladders::adopt)),
        },
    ]
}

/// The SSA Benefits page's claim and record rows.
fn claims() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "write-claims",
            scope: Scope::On(Page::SsaBenefits),
            doc: "write the highlighted claims as a scenario over the document",
            keys: vec![character("w")],
            hint: Some("write"),
            register: Box::new(|world| world.register_system(tools::claims::write_picker)),
        },
        CommandSpec {
            name: "take-claims",
            scope: Scope::On(Page::SsaBenefits),
            doc: "take the highlighted claims into the plan as each income's start",
            keys: vec![character("t")],
            hint: Some("take"),
            register: Box::new(|world| world.register_system(tools::claims::adopt)),
        },
        CommandSpec {
            name: "import-statement",
            scope: Scope::On(Page::SsaBenefits),
            doc: "record a Social Security statement's earnings on the person under the cursor",
            keys: vec![character("e")],
            hint: Some("statement"),
            register: Box::new(|world| world.register_system(tools::claims::import_statement)),
        },
        CommandSpec {
            name: "fill-career",
            scope: Scope::On(Page::SsaBenefits),
            doc: "estimate the cursor's person's earnings from a career at their salary",
            keys: vec![character("c")],
            hint: Some("career"),
            register: Box::new(|world| world.register_system(tools::claims::fill_career)),
        },
    ]
}

/// The SSA Benefits page's rows for the person under the cursor.
fn people() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: "compute-benefit",
            scope: Scope::On(Page::SsaBenefits),
            doc: "compute the cursor's person's typed Social Security benefit from their record",
            keys: vec![character("k")],
            hint: Some("compute"),
            register: Box::new(|world| world.register_system(tools::claims::compute_benefit)),
        },
        CommandSpec {
            name: "person-actions",
            scope: Scope::On(Page::SsaBenefits),
            doc: "offer what can be done for the person under the cursor",
            keys: vec![],
            hint: None,
            register: Box::new(|world| world.register_system(tools::claims::offer_actions)),
        },
        CommandSpec {
            name: "hold-claim",
            scope: Scope::On(Page::SsaBenefits),
            doc: "hold the claim of the person under the cursor while the others are searched, or let it go",
            keys: vec![],
            hint: None,
            register: Box::new(|world| world.register_system(tools::claims::hold_claim)),
        },
        CommandSpec {
            name: "clear-record",
            scope: Scope::On(Page::SsaBenefits),
            doc: "empty the earnings record of the person under the cursor",
            keys: vec![],
            hint: None,
            register: Box::new(|world| world.register_system(tools::claims::clear_record)),
        },
        CommandSpec {
            name: "remove-benefit",
            scope: Scope::On(Page::SsaBenefits),
            doc: "take the Social Security income of the person under the cursor out of the plan",
            keys: vec![],
            hint: None,
            register: Box::new(|world| world.register_system(tools::claims::remove_benefit)),
        },
    ]
}

/// A market tool's rows: ⏎ on a run and on an assumption, and `v`.
fn market<R: MarketTool>() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            name: R::VIEW,
            scope: Scope::On(R::PAGE),
            doc: "show the next view of the runs' spread",
            keys: vec![character("v")],
            hint: Some("view"),
            register: Box::new(|world| world.register_system(tools::markets::cycle_view::<R>)),
        },
        CommandSpec {
            name: R::OPEN,
            scope: Scope::On(R::PAGE),
            doc: "open the highlighted run in the Ledger",
            keys: vec![],
            hint: None,
            register: Box::new(|world| world.register_system(tools::markets::open_run::<R>)),
        },
        CommandSpec {
            name: R::EDIT,
            scope: Scope::On(R::PAGE),
            doc: "turn to where the highlighted assumption is edited",
            keys: vec![],
            hint: None,
            register: Box::new(|world| world.register_system(tools::markets::edit_assumption::<R>)),
        },
    ]
}
