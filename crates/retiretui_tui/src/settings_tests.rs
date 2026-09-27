//! Settings in the running shell: worn at startup, tried on, kept.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use plurimus::core::TerminalSize;
use plurimus::core::ratatui_core::style::Color;
use plurimus::term::KeyCode;

use super::motion::Motion;
use super::settings::Settings;
use super::support::{
    Headless, ROOMY, SIZE, composed_buffer, composed_frame, headless_app_set, press_key, said,
    scratch_plan, type_text,
};
use super::theme::Theme;

const NORD_ACCENT: Color = Color::Rgb(0x88, 0xc0, 0xd0);

static NEXT_CONFIG: AtomicUsize = AtomicUsize::new(0);

/// A config file of this test's own, holding `text`.
fn config(text: &str) -> PathBuf {
    let ordinal = NEXT_CONFIG.fetch_add(1, Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("retiretui-config-{}-{ordinal}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("config.toml");
    std::fs::write(&path, text).unwrap();
    path
}

fn app_under(path: PathBuf, size: TerminalSize) -> Headless {
    let (mut settings, complaint) = Settings::at(path);
    assert!(complaint.is_none(), "{complaint:?}");
    // A frame is compared still: an arriving overlay's cells are not all
    // there yet.
    settings.motion = Motion::Off;
    let mut app = headless_app_set(scratch_plan(), size, settings);
    app.update();
    app
}

fn accent(app: &Headless) -> Color {
    app.world().resource::<Theme>().accent
}

fn open_theme_picker(app: &mut Headless) {
    press_key(app, KeyCode::Char(':'));
    type_text(app, "theme");
    press_key(app, KeyCode::Enter);
    app.update();
    assert!(composed_frame(app).contains("╭ Theme"));
}

#[test]
fn the_theme_the_config_names_is_worn_from_the_start() {
    let app = app_under(config("[tui.theme]\nname = \"nord\"\n"), SIZE);
    assert_eq!(accent(&app), NORD_ACCENT);
    assert!(said(&app).is_empty());
}

#[test]
fn a_theme_that_does_not_resolve_is_said_and_the_terminals_worn() {
    let app = app_under(config("[tui.theme]\nname = \"nrod\"\n"), SIZE);
    assert_eq!(*app.world().resource::<Theme>(), Theme::terminal());
    assert!(said(&app)[0].contains("nrod"), "{:?}", said(&app));
}

#[test]
fn a_theme_is_tried_on_as_the_cursor_reaches_it_and_put_back_on_escape() {
    let mut app = app_under(config(""), SIZE);
    let worn = accent(&app);
    open_theme_picker(&mut app);
    type_text(&mut app, "nord");
    app.update();
    assert_eq!(accent(&app), NORD_ACCENT, "reaching it tries it on");
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(accent(&app), worn, "escape puts the first back");
    assert!(said(&app).is_empty(), "and keeps nothing");
}

#[test]
fn a_chosen_theme_is_kept_in_the_file_beside_what_was_there() {
    let path = config("# mine\n[tui.theme]\naccent = \"#ff0000\"\n");
    let mut app = app_under(path.clone(), SIZE);
    open_theme_picker(&mut app);
    type_text(&mut app, "nord");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(
        accent(&app),
        Color::Rgb(0xff, 0, 0),
        "their own accent stays on"
    );
    assert_eq!(
        app.world().resource::<Theme>().bg,
        Some(Color::Rgb(0x2e, 0x34, 0x40))
    );
    assert_eq!(said(&app), ["theme nord"]);
    let kept = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        kept,
        "# mine\n[tui.theme]\naccent = \"#ff0000\"\nname = \"nord\"\n"
    );
}

#[test]
fn a_theme_that_cannot_be_kept_is_worn_for_the_session_and_said_so() {
    let path = config("");
    let mut app = app_under(path.clone(), SIZE);
    std::fs::write(&path, "[tui\n").unwrap();
    open_theme_picker(&mut app);
    type_text(&mut app, "nord");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(accent(&app), NORD_ACCENT);
    assert!(
        said(&app)[0].contains("for this session only"),
        "{:?}",
        said(&app)
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "[tui\n");
}

/// The cells of the composed frame a theme with its own grounds left to the
/// terminal: drawn on the terminal's background, or in its foreground.
fn unthemed_cells(app: &Headless) -> Vec<String> {
    let buffer = composed_buffer(app);
    let mut found = Vec::new();
    for row in 0..buffer.area.height {
        for column in 0..buffer.area.width {
            let Some(cell) = buffer.cell((column, row)) else {
                continue;
            };
            let style = cell.style();
            let has_ink = cell.symbol() != " ";
            let is_bare = style.bg.is_none_or(|bg| bg == Color::Reset)
                || (has_ink && style.fg.is_none_or(|fg| fg == Color::Reset));
            if is_bare {
                found.push(format!("({column},{row}) {:?}", cell.symbol()));
            }
        }
    }
    found
}

fn assert_themed(app: &Headless, what: &str) {
    let bare = unthemed_cells(app);
    assert!(
        bare.is_empty(),
        "{what}: {} cells left to the terminal, first {:?}\n{}",
        bare.len(),
        &bare[..bare.len().min(6)],
        composed_frame(app)
    );
}

#[test]
fn a_theme_with_its_own_grounds_leaves_no_cell_to_the_terminal() {
    use super::nav::Page;
    use super::support::show;
    let mut app = app_under(config("[tui.theme]\nname = \"github-light\"\n"), ROOMY);
    for page in [Page::Overview, Page::Ledger, Page::Accounts, Page::Settings] {
        show(&mut app, page);
        assert_themed(&app, page.title());
    }
    show(&mut app, Page::Accounts);
    press_key(&mut app, KeyCode::Tab);
    press_key(&mut app, KeyCode::Enter);
    app.update();
    assert_themed(&app, "an open item");
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Char('d'));
    app.update();
    assert_themed(&app, "the confirm dialog");
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Char(':'));
    type_text(&mut app, "sa");
    app.update();
    assert_themed(&app, "the picker");
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Char('a'));
    press_key(&mut app, KeyCode::Esc);
    press_key(&mut app, KeyCode::Char('m'));
    app.update();
    assert_themed(&app, "the drawer and a toast");
}
