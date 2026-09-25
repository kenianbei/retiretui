use plurimus::ui::UiStyle;
use retiretui_engine::plan::Dollars;

use super::*;
use crate::commands::tui::present::signed_money;
use crate::commands::tui::support::Headless;

/// The account the poorer plan starts with less in, all its years below.
const POORER: &str =
    "schema = 1\nbase = \"plan.toml\"\n\n[[accounts]]\nid = \"k\"\nbalance = 20000\n";

/// The Compare page over the test plan, a poorer scenario compared with
/// it.
fn comparing_poorer(size: plurimus::core::TerminalSize) -> Headless {
    let dir = scratch_workspace(&test_plan_briefly_run());
    std::fs::write(dir.join("poorer.toml"), POORER).unwrap();
    let mut app = headless_app_at(dir.join("plan.toml"), size);
    show(&mut app, Page::Compare);
    compare_with(&mut app, "poorer");
    redrawn(&mut app);
    app
}

/// The places of the rows drawn dimmed.
fn dimmed(app: &mut App) -> Vec<usize> {
    let dim = app.world().resource::<Theme>().dimmed();
    let mut rows = app.world_mut().query::<(&plans::PlanRow, &UiStyle)>();
    let mut places: Vec<usize> = rows
        .iter(app.world())
        .filter(|(_, style)| style.0 == dim)
        .map(|(row, _)| row.0)
        .collect();
    places.sort_unstable();
    places
}

fn baseline(app: &App) -> Option<String> {
    let compared = app.world().resource::<Compared>();
    let path = compared.baseline.as_deref();
    path.map(|path| session::file_name(path).into_owned())
}

/// The frame's line holding `text`.
fn line_of(app: &App, text: &str) -> String {
    let (_, row) = cell_of(app, text);
    let frame = composed_frame(app);
    frame.lines().nth(usize::from(row)).unwrap().to_owned()
}

/// Net worth each year, in today's dollars: the document's, then the
/// compared plan's.
fn net_worths(app: &App) -> [Vec<(i16, Dollars)>; 2] {
    let each = |projection: &Projection| {
        let years = projection.years.iter();
        years
            .map(|row| {
                let amount = basis_amount(Metric::NetWorth.value(row), row.deflator, false);
                (row.year, amount)
            })
            .collect()
    };
    let document = &app.world().resource::<Projected>().projection;
    let compared = &app.world().resource::<Compared>().docs[0]
        .projected
        .projection;
    [each(document), each(compared)]
}

#[test]
fn b_makes_the_highlighted_plan_the_baseline_and_removing_it_returns_to_the_document() {
    let mut app = comparing_poorer(SIZE);
    assert_eq!(baseline(&app), None);
    assert_eq!(dimmed(&mut app), [0], "the document to begin with");
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('b'));
    redrawn(&mut app);
    assert_eq!(baseline(&app).as_deref(), Some("poorer.toml"));
    assert_eq!(dimmed(&mut app), [1]);
    press_key(&mut app, KeyCode::Up);
    press_key(&mut app, KeyCode::Char('b'));
    redrawn(&mut app);
    assert_eq!(baseline(&app), None, "b on the document");
    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('b'));
    press_key(&mut app, KeyCode::Char('x'));
    redrawn(&mut app);
    assert!(compared(&app).is_empty());
    assert_eq!(baseline(&app), None);
    assert_eq!(dimmed(&mut app), [0]);
}

#[test]
fn d_reads_every_plan_but_the_baseline_as_its_difference_in_the_table() {
    let mut app = comparing_poorer(ROOMY);
    let [document, poorer] = net_worths(&app);
    let (ends, poorer_ends) = (document.last().unwrap().1, poorer.last().unwrap().1);
    press_key(&mut app, KeyCode::Char('d'));
    let frame = redrawn(&mut app);
    assert!(
        frame.contains("Plans · against plan.toml · today's dollars"),
        "{frame}"
    );
    let own = line_of(&app, "━━ plan.toml");
    assert!(own.contains(&compact_money(ends)), "absolute: {own}");
    let row = line_of(&app, "━━ poorer.toml");
    let fell = signed_money(poorer_ends - ends);
    assert!(fell.starts_with("-$"), "{fell}");
    assert!(row.contains(&fell), "{fell}: {row}");
    assert!(row.contains("now short in 20"), "{row}");

    press_key(&mut app, KeyCode::Down);
    press_key(&mut app, KeyCode::Char('b'));
    redrawn(&mut app);
    assert!(composed_frame(&app).contains("Plans · against poorer.toml"));
    let own = line_of(&app, "━━ plan.toml");
    let rose = signed_money(ends - poorer_ends);
    assert!(own.contains(&rose), "{rose}: {own}");
    let row = line_of(&app, "━━ poorer.toml");
    assert!(row.contains(&compact_money(poorer_ends)), "{row}");

    press_key(&mut app, KeyCode::Char('d'));
    let frame = redrawn(&mut app);
    assert!(frame.contains("Plans · today's dollars"), "{frame}");
}

#[test]
fn d_charts_each_line_less_the_baselines_within_the_scale() {
    let mut app = comparing_poorer(SIZE);
    let [document, poorer] = net_worths(&app);
    press_key(&mut app, KeyCode::Char('d'));
    redrawn(&mut app);
    let mut charts = app
        .world_mut()
        .query_filtered::<&SeriesChart, With<CompareChart>>();
    let chart = charts.single(app.world()).unwrap();
    let baseline = &chart.series[0].points;
    assert!(
        baseline.iter().all(|&(_, amount)| amount == 0.0),
        "along zero"
    );
    let expected: Vec<(f64, f64)> = poorer
        .iter()
        .zip(&document)
        .map(|(&(year, own), &(_, base))| (f64::from(year), (own - base) as f64))
        .collect();
    assert_eq!(chart.series[1].points, expected);
    let lowest = expected
        .iter()
        .map(|&(_, amount)| amount)
        .fold(0.0, f64::min);
    assert!(lowest < 0.0, "the poorer plan is below");
    assert!(chart.y_bounds[0] <= lowest, "{:?} {lowest}", chart.y_bounds);
    assert!(chart.y_labels[0].starts_with('-'), "{:?}", chart.y_labels);
}

#[test]
fn d_reads_the_by_year_table_as_differences_but_the_baselines_column() {
    let mut app = comparing_poorer(SIZE);
    let [document, poorer] = net_worths(&app);
    press_key(&mut app, KeyCode::Char('d'));
    press_key(&mut app, KeyCode::Char('v'));
    press_key(&mut app, KeyCode::Char('v'));
    set_cursor(&mut app, 2040);
    let at = |years: &[(i16, Dollars)]| years.iter().find(|row| row.0 == 2040).unwrap().1;
    let row = line_of(&app, "▏ 2040 ");
    let own = compact_money(at(&document));
    let fell = signed_money(at(&poorer) - at(&document));
    let words: Vec<&str> = row.split_whitespace().collect();
    let expected = ["2040", own.as_str(), fell.as_str()];
    assert!(words.windows(3).any(|cells| cells == expected), "{row}");
}
