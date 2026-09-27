//! An issue's path read back as the domain, item and field it is about,
//! and said in the words the forms use.

use std::cmp::Reverse;

use retiretui_engine::plan::Issue;
use toml::{Table, Value};

use crate::codec;
use crate::draft::Draft;
use crate::forms::offers;
use crate::forms::{DOMAINS, DomainId, FieldKind, FieldSpec, Form};

/// What an issue's path points at: the domain, the item where the path
/// indexes one, and the field where the form has one for it.
pub struct Located {
    pub(crate) form: &'static Form,
    pub(crate) index: Option<usize>,
    pub(crate) field: Option<&'static FieldSpec>,
    /// The place the path names in the field's list: `2` of
    /// `plan.withdrawal_order[2]`.
    pub(crate) place: Option<usize>,
}

impl Located {
    /// Whether the issue is against `spec` of `item`: its field, and where
    /// the path names a place of a list several rows share, that one row.
    #[must_use]
    pub(crate) fn is_against(&self, spec: &FieldSpec, item: Option<&Table>) -> bool {
        self.field.is_some_and(|field| field.key == spec.key)
            && self
                .place
                .is_none_or(|place| holds_place(spec, place, item))
    }
}

/// Whether `spec`, one row of the list at its key, holds `place` of that
/// list as `item` states it.
fn holds_place(spec: &FieldSpec, place: usize, item: Option<&Table>) -> bool {
    match spec.kind {
        FieldKind::Order(_, at) => at == place,
        FieldKind::Listed(back) => {
            let list = item.and_then(|item| codec::get_path(item, spec.key));
            list.and_then(Value::as_array)
                .is_some_and(|list| list.len().checked_sub(back + 1) == Some(place))
        }
        _ => true,
    }
}

/// The longest root wins, so `household.people` is not the household. A
/// root that is itself a field - `medicare` - is that field.
pub(crate) fn locate(path: &str) -> Option<Located> {
    let (form, root) = DOMAINS
        .iter()
        .flat_map(|form| form.paths.iter().map(move |root| (form, *root)))
        .filter(|(_, root)| is_at_or_within(path, root))
        .max_by_key(|(_, root)| root.len())?;
    let rest = &path[root.len()..];
    let (index, within) = match rest.strip_prefix('[').and_then(|rest| rest.split_once(']')) {
        Some((digits, within)) => (digits.parse().ok(), within),
        None => (None, rest),
    };
    // A root may itself be a key of the item, as `medicare` is.
    let keyed_root = root.rsplit('.').next().map_or(0, str::len);
    let field = field_at(form.fields, &codec::as_key(within.trim_start_matches('.')))
        .or_else(|| field_at(form.fields, &path[root.len() - keyed_root..]));
    let place = field.and_then(|spec| codec::list_place(path, spec.key));
    Some(Located {
        form,
        index,
        field,
        place,
    })
}

/// The field an issue at `path` within an item is about: the one with the
/// longest key the path is or sits under, else the first that sits under it.
fn field_at<'a>(fields: &'a [FieldSpec], path: &str) -> Option<&'a FieldSpec> {
    let holding = fields.iter().filter(|spec| is_at_or_within(path, spec.key));
    holding
        .min_by_key(|spec| Reverse(spec.key.len()))
        .or_else(|| fields.iter().find(|spec| is_at_or_within(spec.key, path)))
}

fn is_at_or_within(path: &str, outer: &str) -> bool {
    path == outer || codec::is_within(path, outer)
}

/// The domain an issue at `path` is about, and the item where the path
/// indexes one.
#[must_use]
pub fn issue_place(path: &str) -> Option<(DomainId, Option<usize>)> {
    let located = locate(path)?;
    Some((located.form.domain?, located.index))
}

/// What separates an issue's place words: page, item, field.
pub(crate) const PLACE_SEPARATOR: &str = " \u{203a} ";

/// An issue in the words the forms use: the page, the item by its display
/// name, and the field's label, ahead of the engine's own message. A path
/// no page shows reads as the engine wrote it.
#[must_use]
pub fn issue_words(issue: &retiretui_engine::plan::Issue, draft: &Draft) -> String {
    let Some(located) = locate(&issue.path) else {
        return issue.to_string();
    };
    let index = located.index.or(located.form.list.is_none().then_some(0));
    let item = index.and_then(|index| (located.form.item)(draft, index));
    let place = place_words(&located, item.as_ref());
    format!("{}: {}", place.join(PLACE_SEPARATOR), issue.message)
}

/// The page, `item` by its display name, and the field's label, of what
/// `located` points at.
#[must_use]
pub(crate) fn place_words(located: &Located, item: Option<&Table>) -> Vec<String> {
    let identity = located.form.list.map(|list| list.identity);
    let name = item
        .zip(identity)
        .and_then(|(item, identity)| offers::display_name(item, identity, located.form.fields));
    let page = Some(located.form.title.to_owned());
    let fields = located.form.fields.iter();
    let row = fields.clone().find(|spec| located.is_against(spec, item));
    let field = row.or(located.field).map(|spec| spec.label.to_owned());
    [page, name, field].into_iter().flatten().collect()
}

/// An issue beside what its path points at, found once for everything
/// that asks about the same draft.
pub type LocatedIssue<'a> = (Located, &'a str);

/// The draft's issues, each beside what its path points at.
#[must_use]
pub fn located_issues(draft: &Draft) -> Vec<LocatedIssue<'_>> {
    let issues = draft.issues().iter();
    issues
        .filter_map(|issue| Some((locate(&issue.path)?, issue.message.as_str())))
        .collect()
}

/// What `located` holds against the field `spec` of `item`, item `index`
/// of `ops`; `None` is the one item a domain without a table has.
#[must_use]
pub fn field_issue<'a>(
    located: &[LocatedIssue<'a>],
    (form, index): (&Form, Option<usize>),
    spec: &FieldSpec,
    item: Option<&Table>,
) -> Option<&'a str> {
    let domain = form.domain?;
    located.iter().find_map(|(located, message)| {
        let is_here = located.form.domain == Some(domain)
            && located.index == index
            && located.is_against(spec, item);
        is_here.then_some(*message)
    })
}

/// One issue per line, in validation order.
#[must_use]
pub fn issue_listing(issues: &[Issue]) -> String {
    let listing: Vec<String> = issues.iter().map(ToString::to_string).collect();
    listing.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_issue_path_reads_back_as_its_domain_and_item() {
        let place = |path| issue_place(path);
        assert_eq!(
            place("accounts[1].balance"),
            Some((DomainId::Accounts, Some(1)))
        );
        assert_eq!(
            place("household.people[0].birth"),
            Some((DomainId::People, Some(0))),
            "the longest root wins"
        );
        assert_eq!(place("plan.inflation"), Some((DomainId::Settings, None)));
        assert_eq!(place("nowhere.at_all"), None);
    }
}
