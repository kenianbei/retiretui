use super::*;

#[test]
fn a_page_is_shown_under_the_name_it_is_commanded_by() {
    for page in Page::ALL {
        assert_eq!(page.title().to_lowercase().replace(' ', "-"), page.label());
    }
}

#[test]
fn a_page_is_listed_at_its_own_index() {
    for (at, page) in Page::ALL.into_iter().enumerate() {
        assert_eq!(page.index(), at, "{page:?}");
    }
}

#[test]
fn every_page_is_a_domain_a_tool_or_a_view() {
    let domains = Page::ALL.into_iter().filter(|page| page.is_domain());
    assert_eq!(domains.count(), 13);
    let tools = Page::ALL
        .into_iter()
        .filter(|page| page.group() == Some(Group::Tools));
    assert_eq!(tools.count(), 4);
    for page in Page::ALL {
        assert_eq!(
            page.is_domain(),
            page.heading() == Some(Group::Plan.title()),
            "{page:?}"
        );
        assert_eq!(
            page.group() == Some(Group::Tools),
            page.heading() == Some(Group::Tools.title()),
            "{page:?}"
        );
    }
}

#[test]
fn every_page_is_reached_through_exactly_one_tab() {
    for page in Page::ALL {
        assert!(page.tab() < TAB_COUNT, "{page:?}");
        if let Some(group) = page.group() {
            assert_eq!(page.tab(), group.tab(), "a grouped page is in its tab");
        }
    }
    for tab in 0..Group::Tools.tab() {
        let page = Page::own(tab).expect("a page of its own");
        assert_eq!(page.tab(), tab);
        assert_eq!(tab_title(tab), page.title());
    }
    for group in Group::ALL {
        assert_eq!(Page::own(group.tab()), None, "its pages share it");
        assert_eq!(tab_title(group.tab()), group.title());
        assert_eq!(group.first().group(), Some(group));
    }
    assert_eq!(Group::Tools.tab(), 3);
    assert_eq!(Group::Plan.tab(), 4);
    assert_eq!(TAB_COUNT, 5);
}

#[test]
fn tabs_wrap_at_either_end() {
    assert_eq!(neighbor_tab(0, 1), 1);
    assert_eq!(neighbor_tab(Group::Plan.tab(), 1), 0, "the last tab wraps");
    assert_eq!(neighbor_tab(0, -1), Group::Plan.tab());
}

#[test]
fn a_group_s_tab_opens_on_the_page_last_shown_through_it() {
    let mut last = LastShown::default();
    assert_eq!(
        entering(Group::Plan.tab(), last),
        Page::Accounts,
        "the first of them"
    );
    assert_eq!(entering(Group::Tools.tab(), last), Page::RothConversions);
    last.0[Group::Plan as usize] = Page::Expenses;
    assert_eq!(entering(Group::Plan.tab(), last), Page::Expenses);
    assert_eq!(entering(0, last), Page::Overview, "and no other tab does");
}
