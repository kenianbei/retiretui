use retiretui_engine::plan::{Conversion, Event, Plan, Transfer};

use super::cells::Column;
use super::domain::{Domain, FieldSpec, GROWTH_HELP};
use super::offers::RefSource;
use crate::commands::tui::nav::Page;

pub struct Transfers;

impl Domain for Transfers {
    type Item = Transfer;
    const PAGE: Page = Page::Transfers;
    const PURPOSE: &'static str = "One-time moves from one account to another";
    const PATH: &'static str = "transfers";
    const SINGULAR: &'static str = "Transfer";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("id", "ID")
            .help("A short handle scenarios address this transfer by. Optional."),
        FieldSpec::text("name", "Name").help("What the transfer is called. Blank shows the ID."),
        FieldSpec::refers("from", "From", RefSource::Account).help("The account the money leaves."),
        FieldSpec::refers("to", "To", RefSource::Account).help("The account the money lands in."),
        FieldSpec::trigger("on", "When").help("The year the transfer happens, once."),
        FieldSpec::money("amount", "Amount")
            .blank("Whole balance")
            .help("In today's dollars. Blank moves the whole balance."),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Transfer"),
        Column::new("from"),
        Column::new("to"),
        Column::new("on"),
        Column::new("amount"),
    ];
    const BLANK: &'static str = r#"
from = ""
to = ""
on = {}
"#;

    fn items(plan: &Plan) -> &[Transfer] {
        &plan.transfers
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Transfer> {
        &mut plan.transfers
    }
}

pub struct Conversions;

impl Domain for Conversions {
    type Item = Conversion;
    const PAGE: Page = Page::Conversions;
    const PURPOSE: &'static str = "Roth conversions, by year";
    const PATH: &'static str = "conversions";
    const SINGULAR: &'static str = "Conversion";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("id", "ID")
            .help("A short handle scenarios address this conversion by. Optional."),
        FieldSpec::text("name", "Name").help("What the conversion is called. Blank shows the ID."),
        FieldSpec::refers("from", "From", RefSource::Account)
            .help("The tax-deferred account converted out of."),
        FieldSpec::refers("to", "To", RefSource::Account).help("The Roth account converted into."),
        FieldSpec::money("amount", "Annual amount").help(
            "Converted per year, in today's dollars. It is taxed as ordinary income that year.",
        ),
        FieldSpec::timing(),
        FieldSpec::starts(),
        FieldSpec::ends(),
        FieldSpec::once(),
        FieldSpec::growth("cola", "Growth").help(GROWTH_HELP),
    ];
    const COLUMNS: &'static [Column] = &[
        Column::new("id").headed("Conversion"),
        Column::new("from").headed("Convert from"),
        Column::new("to").headed("Convert to"),
        Column::new("amount").headed("Amount"),
        Column::new("start").headed("From"),
        Column::new("end").headed("Until"),
    ];
    const BLANK: &'static str = r#"
from = ""
to = ""
amount = 0
"#;

    fn items(plan: &Plan) -> &[Conversion] {
        &plan.conversions
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Conversion> {
        &mut plan.conversions
    }
}

pub struct Events;

impl Domain for Events {
    type Item = Event;
    const PAGE: Page = Page::Events;
    const PURPOSE: &'static str = "Named moments other items are timed by, such as retiring";
    const PATH: &'static str = "events";
    const SINGULAR: &'static str = "Event";
    const FIELDS: &'static [FieldSpec] = &[
        FieldSpec::text("id", "ID")
            .help("The name other items use for this moment, such as retire."),
        FieldSpec::text("name", "Name").help("What the event is called. Blank shows the ID."),
        FieldSpec::trigger("trigger", "When").help("When the event happens."),
    ];
    const COLUMNS: &'static [Column] = &[Column::new("id").headed("Event"), Column::new("trigger")];
    const BLANK: &'static str = "
trigger = {}
";

    fn items(plan: &Plan) -> &[Event] {
        &plan.events
    }

    fn items_mut(plan: &mut Plan) -> &mut Vec<Event> {
        &mut plan.events
    }
}
