use super::*;

#[test]
fn tab_walks_the_table_the_changes_and_the_chart_and_arrows_cycle_the_metric_from_either() {
    let dir = scratch_workspace(&test_plan_briefly_run());
    let mut app = headless_app_at(dir.join("plan.toml"), SIZE);
    show(&mut app, Page::Compare);
    let (table, chart) = (
        single::<PlansTable>(&mut app),
        single::<CompareChart>(&mut app),
    );
    assert_eq!(focused(&app), Some(table));
    press_key(&mut app, KeyCode::Right);
    assert!(redrawn(&mut app).contains("Income by year"));
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(single::<ChangesList>(&mut app)));
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(chart));
    press_key(&mut app, KeyCode::Left);
    press_key(&mut app, KeyCode::Left);
    assert!(redrawn(&mut app).contains("Unfunded by year"), "wraps");
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(table));
}

#[test]
fn v_turns_the_view_pane_through_its_views_and_back_and_the_keyboard_follows() {
    let mut app = comparing_variant(SIZE);
    let (chart, grid, table) = (
        single::<CompareChart>(&mut app),
        grid_part(&mut app),
        single::<ByYearTable>(&mut app),
    );
    let only = |app: &App, shown: Entity| {
        [chart, grid, table]
            .into_iter()
            .all(|part| is_shown(app, part) == (part == shown))
    };
    assert!(only(&app, chart));
    assert_eq!(
        view_title(&mut app),
        "Net worth by year · today's dollars · 2026"
    );
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(chart));
    press_key(&mut app, KeyCode::Char('v'));
    redrawn(&mut app);
    assert!(only(&app, grid));
    assert_eq!(focused(&app), Some(grid), "the keyboard goes with the turn");
    assert_eq!(view_title(&mut app), "Money · today's dollars · 2026");
    press_key(&mut app, KeyCode::Char('v'));
    redrawn(&mut app);
    assert!(only(&app, table));
    assert_eq!(focused(&app), Some(table));
    assert_eq!(
        view_title(&mut app),
        "Net worth by year · today's dollars · 2026"
    );
    press_key(&mut app, KeyCode::Char('v'));
    redrawn(&mut app);
    assert!(only(&app, chart));
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(single::<PlansTable>(&mut app)),
        "one stop"
    );
}

#[test]
fn arrows_walk_the_eight_metrics_in_the_chart_and_flip_the_grids_sets() {
    let mut app = comparing_variant(SIZE);
    let mut seen = Vec::new();
    for _ in 0..8 {
        seen.push(view_title(&mut app));
        press_key(&mut app, KeyCode::Right);
        redrawn(&mut app);
    }
    seen.dedup();
    assert_eq!(seen.len(), 8, "{seen:?}");
    assert_eq!(view_title(&mut app), seen[0], "wraps");
    assert!(seen.iter().any(|title| title.starts_with("MAGI by year")));

    press_key(&mut app, KeyCode::Char('v'));
    redrawn(&mut app);
    let small_titles = |app: &mut App| {
        let mut charts = app.world_mut().query_filtered::<Entity, With<GridChart>>();
        let charts: Vec<Entity> = charts.iter(app.world()).collect();
        charts
            .into_iter()
            .map(|chart| title_of(app, chart))
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(
        small_titles(&mut app),
        ["Expenses", "Income", "Net worth", "Withdrawals"]
            .map(str::to_owned)
            .into()
    );
    press_key(&mut app, KeyCode::Left);
    redrawn(&mut app);
    assert!(view_title(&mut app).starts_with("Tax · "));
    assert_eq!(
        small_titles(&mut app),
        ["Conversions", "MAGI", "Taxes", "Unfunded"]
            .map(str::to_owned)
            .into()
    );
    press_key(&mut app, KeyCode::Right);
    redrawn(&mut app);
    assert!(view_title(&mut app).starts_with("Money · "));
}

#[test]
fn the_by_year_table_and_the_year_cursor_move_each_other() {
    let mut app = comparing_variant(SIZE);
    press_key(&mut app, KeyCode::Char('v'));
    press_key(&mut app, KeyCode::Char('v'));
    let frame = redrawn(&mut app);
    assert!(frame.contains("plan.toml  variant.toml"), "{frame}");
    let table = single::<ByYearTable>(&mut app);
    let row_year = |app: &App| {
        let row = app
            .world()
            .get::<ActiveDescendant>(table)
            .unwrap()
            .0
            .unwrap();
        app.world().get::<RowYear>(row).unwrap().0
    };
    assert_eq!(row_year(&app), 2026);
    set_cursor(&mut app, 2040);
    assert_eq!(row_year(&app), 2040, "the table follows the cursor");
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(table));
    press_key(&mut app, KeyCode::Down);
    redrawn(&mut app);
    assert_eq!(cursor_year(&app), Some(2041), "and the cursor the table");
    assert_eq!(row_year(&app), 2041);
    press_key(&mut app, KeyCode::Char('v'));
    set_cursor(&mut app, 2048);
    press_key(&mut app, KeyCode::Char('v'));
    press_key(&mut app, KeyCode::Char('v'));
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("▌ 2048 "),
        "turned to on the cursor's row: {frame}"
    );
}

#[test]
fn every_chart_marks_the_cursor_year_and_names_no_lines() {
    let mut app = comparing_variant(SIZE);
    set_cursor(&mut app, 2040);
    press_key(&mut app, KeyCode::Char('v'));
    redrawn(&mut app);
    let mut charts = app
        .world_mut()
        .query_filtered::<&SeriesChart, Or<(With<CompareChart>, With<GridChart>)>>();
    let charts: Vec<&SeriesChart> = charts.iter(app.world()).collect();
    assert_eq!(charts.len(), 5);
    for chart in charts {
        assert!(chart.marks.iter().any(|mark| mark.year == 2040));
        assert!(chart.is_legend_hidden);
    }
    let frame = composed_frame(&app);
    assert!(!frame.contains("│variant.toml│"), "{frame}");
    show(&mut app, Page::Overview);
    press_key(&mut app, KeyCode::Char('v'));
    assert!(
        redrawn(&mut app).contains("│net worth│"),
        "the Overview keeps its legend"
    );
}

#[test]
fn the_cursor_years_column_follows_the_year_and_the_metric() {
    let mut app = comparing_variant(ROOMY);
    let expected = |app: &App, metric: Metric, year: i16| {
        let projected = app.world().resource::<Projected>();
        let row = projected
            .projection
            .years
            .iter()
            .find(|row| row.year == year);
        let row = row.unwrap();
        compact_money(basis_amount(metric.value(row), row.deflator, false))
    };
    let frame = composed_frame(&app);
    assert!(frame.contains("Net worth 2026"), "{frame}");
    set_cursor(&mut app, 2040);
    let frame = composed_frame(&app);
    assert!(frame.contains("Net worth 2040"), "{frame}");
    let value = expected(&app, Metric::NetWorth, 2040);
    let (_, row) = cell_of(&app, "━━ plan.toml");
    let line = frame.lines().nth(usize::from(row)).unwrap();
    assert!(line.contains(&value), "{value}: {line}");
    press_key(&mut app, KeyCode::Right);
    let frame = redrawn(&mut app);
    assert!(frame.contains("Income 2040"), "{frame}");
    let value = expected(&app, Metric::Income, 2040);
    let line = frame.lines().nth(usize::from(row)).unwrap();
    assert!(line.contains(&value), "{value}: {line}");
}
