//! The Milestones, Needs attention and Could do better panes: rows that
//! each lead somewhere. ⏎ on a row with a year opens the Ledger at that
//! year, on one that leads to a page opens it, and on any other opens its
//! item; `e` opens the item behind any row.

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::prelude::{Commands, Component, Entity, On, Query, Res, ResMut, With};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::InputFocus;
use plurimus::core::ratatui_core::style::Style;
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use plurimus::widgets::{ActiveDescendant, ValueChange};

use super::better::{self, Better};
use super::{attention, milestones};
use crate::commands::tui::command::Outcome;
use crate::commands::tui::edit::{Draft, Turn};
use crate::commands::tui::hints::Hints;
use crate::commands::tui::layout;
use crate::commands::tui::nav::Page;
use crate::commands::tui::session::{LedgerRun, RowYear, Shown, YearCursor};
use crate::commands::tui::theme::Theme;
use crate::commands::tui::tools::ladders;

const HINTS: Hints = Hints(&[("↑↓", "scroll"), ("⏎", "open")]);
pub(super) const NOTHING_TO_EDIT: &str = "Nothing in the plan to edit here";

/// A page, and the item of its table where it has one.
pub(crate) type Target = (Page, Option<usize>);

/// The item a row is about.
#[derive(Component, Clone, Copy)]
pub(crate) struct Opens(Target);

/// A page a row leads to, and the Roth account it sets the conversion
/// search to fill where it leads there.
pub(super) type Lead = (Page, Option<String>);

#[derive(Component, Clone)]
struct LeadsTo(Lead);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tone {
    Plain,
    Warning,
    Quiet,
}

/// A row: what it says, the year it is about, the item behind it, and
/// the page it leads to.
#[derive(Debug)]
pub(super) struct Entry {
    pub text: String,
    pub year: Option<i16>,
    pub opens: Option<Target>,
    pub leads: Option<Lead>,
    pub tone: Tone,
}

impl Entry {
    pub fn plain(text: String) -> Self {
        Self {
            text,
            year: None,
            opens: None,
            leads: None,
            tone: Tone::Plain,
        }
    }

    pub fn leading(text: String, lead: Lead) -> Self {
        Self {
            leads: Some(lead),
            ..Self::plain(text)
        }
    }

    pub fn dated(year: i16, text: String, opens: Target) -> Self {
        Self {
            year: Some(year),
            opens: Some(opens),
            ..Self::plain(text)
        }
    }

    /// The row as shown, led by its year where it has one.
    pub fn line(&self) -> String {
        match self.year {
            Some(year) => format!("{year} {}", self.text),
            None => self.text.clone(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum List {
    Milestones,
    Attention,
    Better,
}

impl List {
    const fn title(self) -> &'static str {
        match self {
            Self::Milestones => "Milestones",
            Self::Attention => "Needs attention",
            Self::Better => "Could do better",
        }
    }
}

/// Which pane a list is, and the width it was last drawn at.
#[derive(Component)]
pub(crate) struct Leads {
    list: List,
    drawn: Option<u16>,
}

/// The `lists`, sharing `band` evenly.
pub(super) fn spawn(commands: &mut Commands, band: Entity, lists: &[List]) {
    for &list in lists {
        let spawned = super::spawn_list(commands, band, list.title(), 1.0);
        let leads = Leads { list, drawn: None };
        commands
            .entity(spawned)
            .insert((leads, HINTS))
            .observe(handle_chosen);
    }
}

/// Rewrites each list's rows whenever what it reads moves - the plan,
/// the basis, the theme and the pane's width for each, the draft and what
/// the searches found for those that read them; the year walking leaves
/// them, and the row highlighted, as they are.
pub(super) fn refresh(
    (shown, draft, theme, better): (Shown, Res<Draft>, Res<Theme>, Res<Better>),
    mut lists: Query<(Entity, &mut Leads, &ScrollArea, &ComputedWidgetArea)>,
    mut commands: Commands,
) {
    let is_moved = shown.projected.is_changed() || shown.basis.is_changed() || theme.is_changed();
    for (list, mut leads, scroll, area) in &mut lists {
        let is_stale = is_moved
            || match leads.list {
                List::Milestones => false,
                List::Attention => draft.is_changed() || better.is_changed(),
                List::Better => better.is_changed(),
            };
        let width = layout::row_width(*scroll, *area);
        if leads.drawn == Some(width) && !is_stale {
            continue;
        }
        leads.drawn = Some(width);
        let nominal = shown.basis.nominal;
        let projected = &shown.projected;
        let entries = match leads.list {
            List::Milestones => milestones::entries(projected, nominal),
            List::Attention => {
                let historical = better.found().and_then(|found| found.historical.as_ref());
                attention::entries(projected, &draft, (nominal, historical))
            }
            List::Better => better::entries(&better, projected, nominal),
        };
        spawn_rows(&mut commands, (list, width), &entries, &theme);
    }
}

/// Replaces `list`'s rows with a row per line each entry wraps to at
/// `width`, each line leading where its entry does.
pub(super) fn spawn_rows(
    commands: &mut Commands,
    (list, width): (Entity, u16),
    entries: &[Entry],
    theme: &Theme,
) {
    let texts = entries.iter().map(|entry| {
        let style = match entry.tone {
            Tone::Plain => Style::new(),
            Tone::Warning => theme.exceeded(),
            Tone::Quiet => theme.dimmed(),
        };
        (entry.line(), style)
    });
    layout::fill_wrapped(commands, (list, width), texts, |at, row| {
        let entry = &entries[at];
        if let Some(year) = entry.year {
            row.insert(RowYear(year));
        }
        if let Some(target) = entry.opens {
            row.insert(Opens(target));
        }
        if let Some(lead) = &entry.leads {
            row.insert(LeadsTo(lead.clone()));
        }
    });
}

/// What following a row reads and turns.
#[derive(SystemParam)]
struct Follow<'w, 's> {
    rows: Query<
        'w,
        's,
        (
            Option<&'static RowYear>,
            Option<&'static Opens>,
            Option<&'static LeadsTo>,
        ),
    >,
    draft: ResMut<'w, Draft>,
    cursor: ResMut<'w, YearCursor>,
    run: ResMut<'w, LedgerRun>,
    turn: Turn<'w, 's>,
}

/// A row chosen by ⏎ or a click: the Ledger at its year, the plan's own,
/// else the page it leads to, else its item.
fn handle_chosen(chosen: On<ValueChange<Entity>>, mut follow: Follow) {
    let Ok((year, opens, leads)) = follow.rows.get(chosen.value) else {
        return;
    };
    if let (None, Some(LeadsTo((page, aim)))) = (year, leads) {
        let page = *page;
        if let Some(destination) = aim.clone() {
            ladders::aim_at(&mut follow.draft, &destination);
        }
        follow.turn.to(page, None);
        return;
    }
    match (year.copied(), opens.copied()) {
        (Some(RowYear(year)), _) => {
            follow.cursor.set_if_neq(YearCursor(Some(year)));
            if follow.run.0.is_some() {
                follow.run.0 = None;
            }
            follow.turn.to(Page::Ledger, None);
        }
        (None, Some(Opens((page, index)))) => follow.turn.to(page, index),
        (None, None) => {}
    }
}

/// The `overview-edit` command: the item behind the highlighted row, on
/// its page.
pub(crate) fn edit_row(
    focus: Res<InputFocus>,
    lists: Query<&ActiveDescendant, With<Leads>>,
    rows: Query<&Opens>,
    mut turn: Turn,
) -> Outcome {
    let row = focus.get().and_then(|list| lists.get(list).ok()?.0);
    let Some(&Opens((page, index))) = row.and_then(|row| rows.get(row).ok()) else {
        return Outcome::Refused(NOTHING_TO_EDIT.to_owned());
    };
    turn.to(page, index);
    Outcome::Done
}
