use super::*;

#[test]
fn picking_a_file_compares_it_and_picking_it_again_stops() {
    let dir = scratch_workspace(&test_plan_briefly_run());
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::Compare);
    let frame = composed_frame(&app);
    assert!(frame.contains("Plans · today's dollars"), "{frame}");
    assert!(frame.contains("Ends with"), "{frame}");
    assert!(frame.contains("plan.toml"), "{frame}");
    assert!(frame.contains(HELP), "{frame}");
    assert!(frame.contains("Net worth by year"), "{frame}");
    assert!(frame.contains("←→ metric"), "{frame}");
    assert!(frame.contains("x remove"), "{frame}");
    assert_eq!(series_count(&mut app), 1);

    compare_with(&mut app, "variant");
    assert_eq!(compared(&app), ["variant.toml"]);
    let frame = redrawn(&mut app);
    assert!(frame.contains("variant.toml"), "{frame}");
    assert_eq!(series_count(&mut app), 2);

    compare_with(&mut app, "broken");
    assert_eq!(compared(&app), ["variant.toml"]);
    assert!(
        said(&app)
            .iter()
            .any(|line| line.starts_with("broken.toml not compared:")),
        "{:?}",
        said(&app)
    );

    compare_with(&mut app, "variant");
    assert!(compared(&app).is_empty());
    assert_eq!(series_count(&mut app), 1);
}

#[test]
fn each_row_is_led_by_its_lines_colour_the_document_first() {
    let app = comparing_variant(SIZE);
    let (doc_col, doc_row) = cell_of(&app, "━━ plan.toml");
    let (variant_col, variant_row) = cell_of(&app, "━━ variant.toml");
    assert_eq!(variant_row, doc_row + 1, "{}", composed_frame(&app));
    let theme = app.world().resource::<Theme>();
    assert_eq!(cell_fg(&app, doc_col, doc_row), Some(theme.series(0)));
    assert_eq!(
        cell_fg(&app, variant_col, variant_row),
        Some(theme.series(1))
    );
}

#[test]
fn a_narrower_pane_shows_fewer_columns() {
    let shown = |size| {
        let app = comparing_variant(size);
        let frame = composed_frame(&app);
        let headers = plans::COLUMNS.iter().map(|column| column.header);
        headers
            .filter(|header| !header.is_empty() && frame.contains(header))
            .count()
    };
    let (narrow, roomy) = (shown(SIZE), shown(ROOMY));
    assert!(narrow < roomy, "{narrow} of {roomy}");
}

#[test]
fn columns_are_kept_while_they_fit_beside_the_cursor_and_the_swatch() {
    use plurimus::core::ratatui_core::layout::Constraint::Length;
    let measured = plurimus::widgets::TableColumns(vec![Length(10), Length(10), Length(10)]);
    let beside = layout::CURSOR_COLS + crate::commands::tui::tabulate::SWATCH_COLS;
    assert_eq!(plans::fitting(&measured, beside + 21), 2);
    assert_eq!(plans::fitting(&measured, beside + 20), 1);
    assert_eq!(plans::fitting(&measured, 0), 1, "the name always shows");
    assert_eq!(plans::fitting(&measured, beside + 32), 3);
}

#[test]
fn x_stops_comparing_the_highlighted_plan_but_not_the_document() {
    let mut app = comparing_variant(SIZE);
    press_key(&mut app, KeyCode::Char('x'));
    assert_eq!(compared(&app), ["variant.toml"]);
    let refused = said(&app);
    assert!(
        refused.iter().any(|line| line.contains("is the document")),
        "{refused:?}"
    );
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('x'));
    assert!(compared(&app).is_empty());
    assert!(said(&app).contains(&"variant.toml no longer compared".to_owned()));
    let table = single::<PlansTable>(&mut app);
    redrawn(&mut app);
    let cursor = app.world().get::<ActiveDescendant>(table).unwrap();
    assert!(cursor.0.is_some(), "the cursor falls back to the document");
}

#[test]
fn the_document_row_follows_the_draft_and_the_basis() {
    let dir = scratch_workspace(&test_plan_briefly_run());
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::Compare);
    let before = composed_frame(&app);
    commit_edit(&mut app, |plan| plan.accounts[0].balance = 5_000_000);
    let edited = redrawn(&mut app);
    assert_ne!(before, edited, "the row follows an applied item");
    press_key(&mut app, KeyCode::Char('n'));
    let nominal = redrawn(&mut app);
    assert_ne!(nominal, edited, "and the basis");
    assert!(nominal.contains("Plans · future dollars"), "{nominal}");
}

#[test]
fn reload_rereads_a_compared_file_and_a_switch_drops_the_set() {
    let dir = scratch_workspace(&test_plan_briefly_run());
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::Compare);
    compare_with(&mut app, "variant");
    let final_net = |app: &App| {
        let compared = app.world().resource::<Compared>();
        compared.docs[0]
            .projected
            .projection
            .summary(false)
            .final_net_worth
    };
    let before = final_net(&app);
    let richer = format!(
        "{}\n[[accounts]]\nid = \"cash\"\nbalance = 5000000\n",
        std::fs::read_to_string(dir.join("variant.toml")).unwrap()
    );
    std::fs::write(dir.join("variant.toml"), richer).unwrap();
    press_key(&mut app, KeyCode::Char('r'));
    assert!(final_net(&app) > before, "{:?}", said(&app));

    app.world_mut()
        .run_system_cached_with(
            crate::commands::tui::documents::open,
            dir.join("variant.toml").into(),
        )
        .unwrap();
    app.update();
    app.update();
    assert!(compared(&app).is_empty(), "{:?}", said(&app));
}
