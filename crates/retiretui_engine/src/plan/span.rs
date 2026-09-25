use super::{Cliff, Contribution, Conversion, Expense, Income, Trigger};

/// The activity triggers of one plan item.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Span<'a> {
    /// First-year trigger, if any.
    pub start: Option<&'a Trigger>,
    /// Last-year trigger, if any.
    pub end: Option<&'a Trigger>,
    /// One-time trigger, if any; wins over the window.
    pub on: Option<&'a Trigger>,
}

impl<'a> Span<'a> {
    const fn new(
        start: Option<&'a Trigger>,
        end: Option<&'a Trigger>,
        on: Option<&'a Trigger>,
    ) -> Self {
        Self { start, end, on }
    }

    /// The triggers that are set, under the key each is written at.
    pub(crate) fn triggers(self) -> impl Iterator<Item = (&'static str, &'a Trigger)> {
        [("start", self.start), ("end", self.end), ("on", self.on)]
            .into_iter()
            .filter_map(|(key, trigger)| Some((key, trigger?)))
    }

    /// The trigger that decides the item's first year.
    pub(crate) fn first(self) -> Option<&'a Trigger> {
        self.start.or(self.on)
    }
}

/// A plan item another item's trigger can refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Node<'a> {
    Event(&'a str),
    Income(&'a str),
}

impl Income {
    pub(crate) fn span(&self) -> Span<'_> {
        Span::new(self.start.as_ref(), self.end.as_ref(), self.on.as_ref())
    }
}

impl Expense {
    pub(crate) fn span(&self) -> Span<'_> {
        Span::new(self.start.as_ref(), self.end.as_ref(), self.on.as_ref())
    }
}

impl Conversion {
    pub(crate) fn span(&self) -> Span<'_> {
        Span::new(self.start.as_ref(), self.end.as_ref(), self.on.as_ref())
    }
}

impl Contribution {
    pub(crate) fn span(&self) -> Span<'_> {
        Span::new(self.start.as_ref(), self.end.as_ref(), self.on.as_ref())
    }
}

impl Cliff {
    pub(crate) fn span(&self) -> Span<'_> {
        Span::new(self.start.as_ref(), self.end.as_ref(), None)
    }
}
