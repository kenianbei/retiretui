//! Where the shell can be: the pages a document is read and edited
//! through, and which of them a command acts on. What is actually drawn
//! is [`surface`]'s question, because without a document that is the
//! form a new plan is composed in rather than a page.

use bevy_app::{App, Update};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::{IntoScheduleConfigs, Res, ResMut, Resource, SystemSet};

pub(super) mod surface;

pub use surface::{FocusStop, ShownSurface, SurfaceRoot, shown_in, spawn_surface};

pub fn plugin(app: &mut App) {
    app.init_resource::<ActivePage>();
    app.init_resource::<LastShown>();
    app.configure_sets(Update, (PageSystems::Turn, PageSystems::Show).chain());
    app.add_systems(Update, remember_the_page.in_set(PageSystems::Turn));
    app.add_systems(Update, surface::show_the_surface.in_set(PageSystems::Show));
}

/// A page is turned to before it is shown, so that what may refuse the
/// turn runs between the two and the page refused is never drawn.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PageSystems {
    Turn,
    Show,
}

/// One thing the body can show: a view of the projection, a tool run over
/// it, or one domain of the plan being edited.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Page {
    /// Summary figures, charts, and the year's actions.
    Overview,
    /// The scrollable year table over the cursor year's flows, income and
    /// tax.
    Ledger,
    /// The document beside other workspace files, summarised and charted.
    Compare,
    /// Roth conversion ladders searched under constraints.
    RothConversions,
    /// Social Security claim ages searched, jointly for the household.
    SsaBenefits,
    /// The plan run through many random markets.
    MonteCarlo,
    /// The plan run from every historical start year.
    Historical,
    Accounts,
    Income,
    Expenses,
    Cliffs,
    Transfers,
    Conversions,
    Contributions,
    Events,
    People,
    Residency,
    /// Filing status and Medicare.
    Household,
    /// The `[plan]` settings.
    Settings,
    /// What the market tools assume, and how they run.
    Market,
}

/// A tab holding several pages, named beside the one on show by a sidebar
/// that chooses between them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Group {
    /// The tools that act on the plan as a whole.
    Tools,
    /// The plan's editing domains.
    Plan,
}

impl Group {
    /// Every group, in the order their tabs follow the pages of their own.
    pub const ALL: [Self; 2] = [Self::Tools, Self::Plan];

    /// How the group's tab and sidebar are named, which is also the
    /// heading its pages are listed under.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Tools => "Tools",
            Self::Plan => "Plan",
        }
    }

    /// The page the tab shows until one of its own has been.
    pub const fn first(self) -> Page {
        match self {
            Self::Tools => Page::RothConversions,
            Self::Plan => Page::Accounts,
        }
    }

    /// The group's tab: after every page's own.
    pub const fn tab(self) -> usize {
        OWN_TABS + self as usize
    }

    /// The group whose tab `tab` is, which a page's own tab is not.
    const fn at(tab: usize) -> Self {
        Self::ALL[tab - OWN_TABS]
    }
}

impl Page {
    /// Every page, in the order the tabs and the sidebars reach them.
    pub const ALL: [Self; 20] = [
        Self::Overview,
        Self::Ledger,
        Self::Compare,
        Self::RothConversions,
        Self::SsaBenefits,
        Self::MonteCarlo,
        Self::Historical,
        Self::Accounts,
        Self::Income,
        Self::Expenses,
        Self::Cliffs,
        Self::Transfers,
        Self::Conversions,
        Self::Contributions,
        Self::Events,
        Self::People,
        Self::Residency,
        Self::Household,
        Self::Settings,
        Self::Market,
    ];

    /// How the page is named on screen.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Ledger => "Ledger",
            Self::Compare => "Compare",
            Self::RothConversions => "Roth Conversions",
            Self::SsaBenefits => "SSA Benefits",
            Self::MonteCarlo => "Monte Carlo",
            Self::Historical => "Historical",
            Self::Accounts => "Accounts",
            Self::Income => "Income",
            Self::Expenses => "Expenses",
            Self::Cliffs => "Cliffs",
            Self::Transfers => "Transfers",
            Self::Conversions => "Conversions",
            Self::Contributions => "Contributions",
            Self::Events => "Events",
            Self::People => "People",
            Self::Residency => "Residency",
            Self::Household => "Household",
            Self::Settings => "Settings",
            Self::Market => "Market",
        }
    }

    /// The page's handle in the command table: its title, lowercased,
    /// with a hyphen for a space.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Ledger => "ledger",
            Self::Compare => "compare",
            Self::RothConversions => "roth-conversions",
            Self::SsaBenefits => "ssa-benefits",
            Self::MonteCarlo => "monte-carlo",
            Self::Historical => "historical",
            Self::Accounts => "accounts",
            Self::Income => "income",
            Self::Expenses => "expenses",
            Self::Cliffs => "cliffs",
            Self::Transfers => "transfers",
            Self::Conversions => "conversions",
            Self::Contributions => "contributions",
            Self::Events => "events",
            Self::People => "people",
            Self::Residency => "residency",
            Self::Household => "household",
            Self::Settings => "settings",
            Self::Market => "market",
        }
    }

    /// One short phrase, shown beside the page's command.
    pub const fn doc(self) -> &'static str {
        match self {
            Self::Overview => "show the overview",
            Self::Ledger => "show the year ledger",
            Self::Compare => "compare the document with other files",
            Self::RothConversions => "search Roth conversion ladders",
            Self::SsaBenefits => "search Social Security claim ages",
            Self::MonteCarlo => "run the plan through random markets",
            Self::Historical => "run the plan from every historical start year",
            Self::Accounts => "edit the accounts",
            Self::Income => "edit the income sources",
            Self::Expenses => "edit the expenses",
            Self::Cliffs => "edit the MAGI cliffs",
            Self::Transfers => "edit the transfers",
            Self::Conversions => "edit the Roth conversions",
            Self::Contributions => "edit the contributions",
            Self::Events => "edit the named events",
            Self::People => "edit the people",
            Self::Residency => "edit the residency",
            Self::Household => "edit filing status and medicare",
            Self::Settings => "edit the plan settings",
            Self::Market => "edit the market assumptions",
        }
    }

    /// The heading the page is listed under: its group's title, what runs
    /// over the projection under another, and the views of it under none.
    pub const fn heading(self) -> Option<&'static str> {
        if let Some(group) = self.group() {
            return Some(group.title());
        }
        match self {
            Self::Compare => Some(RUN),
            _ => None,
        }
    }

    /// The tab the page shares with others, where it is not one of its
    /// own.
    ///
    /// This is the partition the shell navigates by, so it is matched
    /// exhaustively: a page added to the enum has to say where it sits.
    pub const fn group(self) -> Option<Group> {
        match self {
            Self::Overview | Self::Ledger | Self::Compare => None,
            Self::RothConversions | Self::SsaBenefits | Self::MonteCarlo | Self::Historical => {
                Some(Group::Tools)
            }
            Self::Accounts
            | Self::Income
            | Self::Expenses
            | Self::Cliffs
            | Self::Transfers
            | Self::Conversions
            | Self::Contributions
            | Self::Events
            | Self::People
            | Self::Residency
            | Self::Household
            | Self::Settings
            | Self::Market => Some(Group::Plan),
        }
    }

    /// Whether the page edits one of the plan's domains, rather than
    /// viewing the projection or running something over it.
    pub const fn is_domain(self) -> bool {
        matches!(self.group(), Some(Group::Plan))
    }

    /// The page's place in [`Self::ALL`], which lists them in declaration
    /// order.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The tab the page is reached through: its own, or its group's.
    pub const fn tab(self) -> usize {
        if let Some(group) = self.group() {
            return group.tab();
        }
        let mut tab = 0;
        let mut at = 0;
        while at < self.index() {
            if Self::ALL[at].group().is_none() {
                tab += 1;
            }
            at += 1;
        }
        tab
    }

    /// The page that is tab `tab`'s own, which a group's tab has none of:
    /// it shows whichever of its pages was last shown through it.
    const fn own(tab: usize) -> Option<Self> {
        let mut seen = 0;
        let mut at = 0;
        while at < Self::ALL.len() {
            let page = Self::ALL[at];
            if page.group().is_none() {
                if seen == tab {
                    return Some(page);
                }
                seen += 1;
            }
            at += 1;
        }
        None
    }
}

/// The heading the pages that run over the projection are listed under.
pub const RUN: &str = "Run";

/// The tabs that are a page's own, ahead of the groups'.
const OWN_TABS: usize = count_own_tabs();

/// The tabs the bar shows: one per page of its own, and one per group.
pub const TAB_COUNT: usize = OWN_TABS + Group::ALL.len();

const fn count_own_tabs() -> usize {
    let mut count = 0;
    let mut at = 0;
    while at < Page::ALL.len() {
        if Page::ALL[at].group().is_none() {
            count += 1;
        }
        at += 1;
    }
    count
}

/// The page entering `tab` shows: the tab's own, and for a group's the
/// page last shown through it.
#[must_use]
pub fn entering(tab: usize, last: LastShown) -> Page {
    Page::own(tab).unwrap_or_else(|| last.of(Group::at(tab)))
}

/// The tab `step` places along the bar, wrapping at either end.
#[must_use]
pub fn neighbor_tab(tab: usize, step: isize) -> usize {
    let count = TAB_COUNT as isize;
    (tab as isize + step).rem_euclid(count) as usize
}

/// The digit that selects a tab, which the command table binds and the
/// bar's own label names.
///
/// # Panics
///
/// If the bar ever shows more tabs than there are digits, which
/// `tabbar` asserts it does not.
#[must_use]
pub fn tab_digit(tab: usize) -> char {
    char::from_digit(tab as u32 + 1, 10).expect("a tab per digit")
}

/// How tab `tab` is named on the bar.
pub const fn tab_title(tab: usize) -> &'static str {
    match Page::own(tab) {
        Some(page) => page.title(),
        None => Group::at(tab).title(),
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ActivePage(pub Page);

/// The page each group's tab shows: the one last shown through it, and
/// the first of them until one has been.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct LastShown([Page; Group::ALL.len()]);

impl LastShown {
    #[must_use]
    pub fn of(self, group: Group) -> Page {
        self.0[group as usize]
    }
}

impl Default for LastShown {
    fn default() -> Self {
        Self(Group::ALL.map(Group::first))
    }
}

impl Default for ActivePage {
    fn default() -> Self {
        Self(Page::Overview)
    }
}

/// Whichever way a grouped page is reached, it is the one its tab comes
/// back to.
fn remember_the_page(active: Res<ActivePage>, mut last: ResMut<LastShown>) {
    if let Some(group) = active.0.group()
        && active.is_changed()
    {
        last.0[group as usize] = active.0;
    }
}

#[cfg(test)]
mod tests;
