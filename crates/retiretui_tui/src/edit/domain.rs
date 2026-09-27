//! A form as the TUI runs it: the client's form, the page it shows on, what
//! its buttons say, and what runs once one is pressed.

use std::ops::Deref;

use bevy_ecs::prelude::Commands;
pub use retiretui_client::forms::{BLANK, FieldKind, FieldSpec, ListOps, Target, ToolAnswers};
use retiretui_client::forms::{DomainId, Form};

use super::build::FormButton;
use crate::nav::Page;

/// What a form's buttons say unless it says otherwise.
const ACTIONS: [&str; 2] = ["Discard", "Apply"];

#[derive(Clone, Copy)]
pub struct Ops {
    pub form: Form,
    pub surface: Option<Page>,
    /// What the buttons at the form's foot say: the one that drops its
    /// edits, then the one that stores them.
    pub actions: [&'static str; 2],
    /// Run once the form is applied or dropped, for a tool that acts on
    /// its answers at once rather than at a command.
    pub after: Option<fn(FormButton, &mut Commands)>,
}

impl Deref for Ops {
    type Target = Form;

    fn deref(&self) -> &Form {
        &self.form
    }
}

impl Ops {
    /// A domain's form, on the page that shows the domain.
    pub const fn domain(form: Form) -> Self {
        Self {
            surface: match form.domain {
                Some(id) => Some(page_of(id)),
                None => None,
            },
            form,
            actions: ACTIONS,
            after: None,
        }
    }

    /// A form the shell keeps beside the plan, its fields a table of its
    /// own that `T` parses. Applying it holds the answers; what acts on
    /// them runs after, where the world is in reach.
    pub const fn tool<T: ToolAnswers>(
        surface: Option<Page>,
        title: &'static str,
        fields: &'static [FieldSpec],
    ) -> Self {
        Self {
            form: Form::tool::<T>(title, fields),
            surface,
            actions: ACTIONS,
            after: None,
        }
    }

    /// The same form, its buttons saying `actions` and `after` run once
    /// either has done its work.
    pub const fn acting(
        self,
        actions: [&'static str; 2],
        after: fn(FormButton, &mut Commands),
    ) -> Self {
        Self {
            actions,
            after: Some(after),
            ..self
        }
    }
}

/// The page a domain is edited on.
pub const fn page_of(id: DomainId) -> Page {
    match id {
        DomainId::Accounts => Page::Accounts,
        DomainId::Income => Page::Income,
        DomainId::Expenses => Page::Expenses,
        DomainId::Cliffs => Page::Cliffs,
        DomainId::Transfers => Page::Transfers,
        DomainId::Conversions => Page::Conversions,
        DomainId::Contributions => Page::Contributions,
        DomainId::Events => Page::Events,
        DomainId::People => Page::People,
        DomainId::Residency => Page::Residency,
        DomainId::Household => Page::Household,
        DomainId::Settings => Page::Settings,
        DomainId::Market => Page::Market,
    }
}

#[cfg(test)]
mod tests {
    use retiretui_client::forms::DOMAINS;

    use super::page_of;

    #[test]
    fn a_domain_s_page_is_titled_as_the_domain_is() {
        for form in DOMAINS {
            let domain = form.domain.expect("every listed form is a domain's");
            assert_eq!(page_of(domain).title(), domain.title());
        }
    }
}
