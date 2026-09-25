use std::path::PathBuf;
use std::time::Duration;

use retiretui_engine::plan::Dollars;

use super::*;
use crate::commands::tui::support::let_pass;
use crate::commands::tui::watch::POLL_SECONDS;

/// Longer than the coarse clock file times are stamped from, so a write
/// straight after a read gets a later mtime.
const MTIME_TICK: Duration = Duration::from_millis(50);
/// A scenario whose inflation validation refuses.
const BROKEN: &str = "schema = 1\nbase = \"plan.toml\"\n\n[plan]\ninflation = 9.0\n";
/// What is said of it: its name, and the first issue without the path.
const BROKEN_NOTE: &str = "variant.toml not re-read: must be between -0.1 and 0.5";

fn variant_path(app: &App) -> PathBuf {
    let document = app.world().resource::<Session>().plan_path.clone();
    document.unwrap().with_file_name("variant.toml")
}

/// Rewrites the compared variant, a tick after it was last read.
fn rewrite_variant(app: &App, text: &str) {
    std::thread::sleep(MTIME_TICK);
    std::fs::write(variant_path(app), text).unwrap();
}

fn richer(app: &App) -> String {
    let text = std::fs::read_to_string(variant_path(app)).unwrap();
    format!("{text}\n[[accounts]]\nid = \"cash\"\nbalance = 5000000\n")
}

/// Lets the watch's poll come round, then draws what it re-read.
fn poll(app: &mut App) -> String {
    let_pass(app, Duration::from_secs_f32(POLL_SECONDS));
    redrawn(app)
}

fn final_net(app: &App) -> Dollars {
    let compared = app.world().resource::<Compared>();
    let summary = compared.docs[0].projected.projection.summary(false);
    summary.final_net_worth
}

fn plans_note(app: &mut App) -> String {
    let table = single::<PlansTable>(app);
    let pane = app.world().get::<ChildOf>(table).unwrap().parent();
    app.world().get::<Framed>(pane).unwrap().note.clone()
}

/// Whether the variant's name is drawn in the colour of what went wrong.
fn is_marked(app: &App) -> bool {
    let (swatch, row) = cell_of(app, "━━ variant.toml");
    let col = swatch + crate::commands::tui::tabulate::SWATCH_COLS;
    cell_fg(app, col, row) == app.world().resource::<Theme>().exceeded().fg
}

#[test]
fn a_compared_file_changed_on_disk_is_read_again() {
    let mut app = comparing_variant(SIZE);
    let before = final_net(&app);
    rewrite_variant(&app, &richer(&app));
    poll(&mut app);
    assert!(final_net(&app) > before, "{:?}", said(&app));
}

#[test]
fn a_broken_rewrite_keeps_the_figures_and_marks_the_row_until_it_reads() {
    let mut app = comparing_variant(SIZE);
    let fixed = richer(&app);
    let before = final_net(&app);
    assert!(!is_marked(&app));
    rewrite_variant(&app, BROKEN);
    let frame = poll(&mut app);
    assert_eq!(final_net(&app), before, "the last good figures stay");
    assert!(is_marked(&app), "{frame}");
    assert_eq!(plans_note(&mut app), BROKEN_NOTE, "{frame}");

    rewrite_variant(&app, &fixed);
    poll(&mut app);
    assert!(final_net(&app) > before);
    assert!(!is_marked(&app));
    assert_eq!(plans_note(&mut app), "");
}

#[test]
fn a_reload_that_fails_marks_the_row_as_the_watch_does() {
    let mut app = comparing_variant(SIZE);
    rewrite_variant(&app, BROKEN);
    press_key(&mut app, KeyCode::Char('r'));
    redrawn(&mut app);
    assert!(is_marked(&app), "{}", composed_frame(&app));
    assert_eq!(plans_note(&mut app), BROKEN_NOTE);
}

fn warnings(app: &App) -> usize {
    let said = said(app);
    said.iter().filter(|line| *line == BROKEN_NOTE).count()
}

#[test]
fn a_failed_reread_is_said_once_while_it_stays_broken() {
    let mut app = comparing_variant(SIZE);
    rewrite_variant(&app, BROKEN);
    poll(&mut app);
    assert_eq!(warnings(&app), 1, "{:?}", said(&app));
    poll(&mut app);
    assert_eq!(warnings(&app), 1, "an unchanged file is not read again");
    press_key(&mut app, KeyCode::Char('r'));
    assert_eq!(warnings(&app), 2, "a reload says it again");
}
