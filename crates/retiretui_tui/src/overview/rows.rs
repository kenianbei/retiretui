//! The Overview's lists - Milestones, Needs attention, Could do better,
//! Over the plan and Rests on: rows that each lead somewhere. ⏎ on a row
//! with a year opens the Ledger at that year, on one that leads to a page
//! opens it, and on any other opens its item; `e` opens the item behind
//! any row.

use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Changed, Commands, Component, Entity, On, Query, Res, ResMut, With};
use bevy_ecs::system::SystemParam;
use bevy_input_focus::InputFocus;
use bevy_ui::Node;
use plurimus::core::UiWidget;
use plurimus::core::ratatui_core::style::Style;
use plurimus::core::ratatui_core::text::Line;
use plurimus::ui::{ComputedWidgetArea, ScrollArea};
use plurimus::widgets::ratatui_widgets::paragraph::Paragraph;
use plurimus::widgets::{ActiveDescendant, ValueChange};

use super::attention;
use super::better::{self, Better};
use super::{Viewed, totals};
use crate::command::Outcome;
use crate::edit::{Draft, Turn, page_of};
use crate::hints::Hints;
use crate::layout::{self, fixed, placed};
use crate::nav::Page;
use crate::pane::{Framed, Pane};
use crate::present;
use crate::session::{LedgerRun, RowYear, Shown, YearCursor};
use crate::theme::Theme;
use crate::tools::ladders;
use crate::tools::spending::Spending;
use retiretui_client::overview::{ATTENTION, MILESTONES, OVER_THE_PLAN, RESTS_ON, Row};
use retiretui_client::searches::overview::COULD_DO_BETTER;

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
#[derive(Clone, Debug)]
pub(super) struct Entry {
    /// What stands before the text on its first line, in a column its
    /// later lines hang under.
    pub lead: String,
    pub text: String,
    pub year: Option<i16>,
    /// Whether the year leads the row, where it has one.
    pub is_dated: bool,
    pub opens: Option<Target>,
    pub leads: Option<Lead>,
    pub tone: Tone,
    /// What is read out beneath the list while the row is highlighted.
    pub beneath: Option<String>,
}

impl From<Row> for Entry {
    /// A client row, opening its item's page.
    fn from(row: Row) -> Self {
        Self {
            year: row.year,
            opens: row.place.map(|(domain, index)| (page_of(domain), index)),
            ..Self::plain(row.text)
        }
    }
}

impl Entry {
    pub fn plain(text: String) -> Self {
        Self {
            lead: String::new(),
            text,
            year: None,
            is_dated: true,
            opens: None,
            leads: None,
            tone: Tone::Plain,
            beneath: None,
        }
    }

    pub fn leading(text: String, lead: Lead) -> Self {
        Self {
            leads: Some(lead),
            ..Self::plain(text)
        }
    }

    /// The row's lines at `width`: its text wrapped, running on indented,
    /// or hung under its lead where it has one.
    pub fn lines(&self, width: u16) -> Vec<String> {
        if self.lead.is_empty() {
            return layout::wrapped(&self.line(), width, layout::CONTINUED);
        }
        let lead = u16::try_from(self.lead.len()).unwrap_or(u16::MAX);
        let hung = " ".repeat(self.lead.len());
        let broken = layout::wrapped(&self.text, width.saturating_sub(lead), "");
        let mut leads = std::iter::once(&self.lead).chain(std::iter::repeat(&hung));
        broken
            .into_iter()
            .map(|line| format!("{}{line}", leads.next().unwrap_or(&hung)))
            .collect()
    }

    /// The row as shown, led by its year where it has one.
    pub fn line(&self) -> String {
        match self.year {
            Some(year) if self.is_dated => format!("{year} {}", self.text),
            _ => self.text.clone(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum List {
    Milestones,
    Attention,
    Better,
    Totals,
    RestsOn,
}

impl List {
    const fn title(self) -> &'static str {
        match self {
            Self::Milestones => MILESTONES,
            Self::Attention => ATTENTION,
            Self::Better => COULD_DO_BETTER,
            Self::Totals => OVER_THE_PLAN,
            Self::RestsOn => RESTS_ON,
        }
    }

    /// The list's pane: as tall as its rows where they are always the
    /// same few, else sharing what its band has.
    fn pane(self) -> Pane {
        match self {
            Self::RestsOn => Pane::new(self.title()).tall(RESTS_ROWS),
            _ => Pane::new(self.title()).sharing(1.0),
        }
    }
}

/// Rests on's pane: its six rows and its borders.
const RESTS_ROWS: f32 = 8.0;
/// The rows the totals' make-up is read out in beneath them.
const BENEATH_ROWS: f32 = 2.0;

/// Which pane a list is, and the width it was last drawn at, with
/// whether it had the rows to say everything in place.
#[derive(Component)]
pub(crate) struct Leads {
    list: List,
    drawn: Option<(u16, bool)>,
}

/// What a row has read out beneath its list while it is highlighted.
#[derive(Component)]
pub(super) struct Beneath(String);

/// The lines beneath a list its highlighted row is read out in.
#[derive(Component)]
pub(super) struct BeneathLines;

/// The `lists`, each in a pane of its own in `band`.
pub(super) fn spawn(commands: &mut Commands, band: Entity, lists: &[List]) {
    for &list in lists {
        let pane = list.pane().spawn(commands, band);
        let spawned = layout::spawn_scrolled_list(commands, pane, HINTS);
        let leads = Leads { list, drawn: None };
        commands
            .entity(spawned)
            .insert(leads)
            .observe(handle_chosen);
        if list == List::Totals {
            commands.spawn((
                BeneathLines,
                UiWidget::default(),
                fixed(0.0),
                placed(),
                ChildOf(pane),
            ));
        }
    }
}

/// Rewrites each list's rows whenever what it reads moves - the page as
/// the client says it, the theme and the pane's size for each, the draft
/// and what the searches found for those that read them - leaving the
/// row highlighted as it is otherwise.
pub(super) fn refresh(
    (viewed, shown, draft, theme): (Res<Viewed>, Shown, Res<Draft>, Res<Theme>),
    (better, spending): (Res<Better>, Res<Spending>),
    mut lists: Query<(
        Entity,
        &mut Leads,
        &ScrollArea,
        &ComputedWidgetArea,
        &ChildOf,
    )>,
    (mut frames, mut commands): (Query<&mut Framed>, Commands),
) {
    let Some(view) = &viewed.0 else {
        return;
    };
    let is_moved = viewed.is_changed() || theme.is_changed();
    for (list, mut leads, scroll, area, pane) in &mut lists {
        let is_stale = is_moved
            || match leads.list {
                List::Attention => draft.is_changed() || better.is_changed(),
                List::Better => better.is_changed() || spending.is_changed(),
                List::Milestones | List::Totals | List::RestsOn => false,
            };
        let width = layout::row_width(*scroll, *area);
        let is_roomy = leads.list == List::Totals && totals::fits_in(view, area.0.height);
        if leads.drawn == Some((width, is_roomy)) && !is_stale {
            continue;
        }
        leads.drawn = Some((width, is_roomy));
        let entries = match leads.list {
            List::Milestones => view.milestones.iter().cloned().map(Entry::from).collect(),
            List::Attention => {
                let historical = better.found().and_then(|found| found.historical.as_ref());
                attention::entries(view, &draft, historical)
            }
            List::Better => {
                let nominal = shown.basis.nominal;
                better::entries((&better, &spending), &shown.projected, nominal)
            }
            List::Totals => {
                let basis = present::basis_name(shown.basis.nominal);
                if let Ok(mut framed) = frames.get_mut(pane.parent()) {
                    Framed::retitle(&mut framed, &format!("{OVER_THE_PLAN} · {basis}"));
                }
                totals::entries(&view.totals, is_roomy)
            }
            List::RestsOn => totals::assumed(&view.rests_on),
        };
        spawn_rows(&mut commands, (list, width), &entries, &theme);
    }
}

/// Reads the highlighted row's make-up out beneath a list whose rows carry
/// one, in the lines kept for it, and keeps no line under any other.
pub(super) fn read_beneath(
    lists: Query<(&ActiveDescendant, &ChildOf, &ComputedWidgetArea), Changed<ActiveDescendant>>,
    rows: Query<&Beneath>,
    mut lines: Query<(&mut UiWidget, &mut Node, &ChildOf), With<BeneathLines>>,
    theme: Res<Theme>,
) {
    for (on, pane, area) in &lists {
        let said = on.0.and_then(|row| rows.get(row).ok());
        for (mut widget, mut node, parent) in &mut lines {
            if parent.parent() != pane.parent() {
                continue;
            }
            let Some(Beneath(said)) = said else {
                *node = fixed(0.0);
                continue;
            };
            let wrapped = layout::wrapped(said, area.0.width.saturating_sub(INDENT), "");
            let wrapped = wrapped.into_iter();
            let lines: Vec<Line<'static>> = wrapped
                .map(|line| Line::styled(format!("  {line}"), theme.dimmed()))
                .collect();
            *node = fixed(BENEATH_ROWS);
            *widget = UiWidget::new(Paragraph::new(lines));
        }
    }
}

/// The cells the lines beneath a list are set in by.
const INDENT: u16 = 2;

/// Replaces `list`'s rows with a row per line each entry wraps to at
/// `width`, each line leading where its entry does.
pub(super) fn spawn_rows(
    commands: &mut Commands,
    (list, width): (Entity, u16),
    entries: &[Entry],
    theme: &Theme,
) {
    let lines = entries.iter().enumerate().flat_map(|(at, entry)| {
        let style = match entry.tone {
            Tone::Plain => Style::new(),
            Tone::Warning => theme.exceeded(),
            Tone::Quiet => theme.dimmed(),
        };
        let broken = entry.lines(width).into_iter();
        broken.map(move |line| (at, line, style))
    });
    layout::fill_lines(commands, list, lines, |at, row| {
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
        if let Some(beneath) = &entry.beneath {
            row.insert(Beneath(beneath.clone()));
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
