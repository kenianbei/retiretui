//! Settings in the running shell: worn at startup, tried on, kept.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy_ecs::prelude::With;
use plurimus::core::TerminalSize;
use plurimus::core::ratatui_core::style::Color;
use plurimus::term::KeyCode;
use plurimus::ui::{UiLabel, UiStyle};
use plurimus::widgets::ListItem;

use super::motion::Motion;
use super::settings::Settings;
use super::store::DiskStore;
use super::support::{
    Headless, ROOMY, SIZE, USER_THEME, USER_THEME_ACCENT, cell_of, composed_buffer, composed_frame,
    headless_app_set, picker_rows, press_key, said, scratch_plan, tap, type_text,
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
    let (mut settings, complaints) = Settings::at(Arc::new(DiskStore), path);
    assert_eq!(complaints, [""; 0]);
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

fn write_theme(path: &Path, slug: &str, text: &str) {
    let themes = path.with_file_name("themes");
    std::fs::create_dir_all(&themes).unwrap();
    std::fs::write(themes.join(format!("{slug}.toml")), text).unwrap();
}

fn badge_of(app: &mut Headless, slug: &str) -> String {
    let rows = picker_rows(app);
    let row = rows.iter().find(|(text, _)| text == slug);
    row.unwrap_or_else(|| panic!("{slug} in {rows:?}"))
        .1
        .clone()
}

#[test]
fn a_theme_of_the_users_own_is_worn_from_the_start_and_offered_as_theirs() {
    let path = config("[tui.theme]\nname = \"mine\"\n");
    write_theme(&path, "dusk", USER_THEME);
    write_theme(&path, "nord", USER_THEME);
    let mut app = app_under(path, SIZE);
    assert_eq!(accent(&app), USER_THEME_ACCENT, "named by its family");
    assert!(said(&app).is_empty());
    open_theme_picker(&mut app);
    assert_eq!(badge_of(&mut app, "dusk"), "dark, yours");
    assert_eq!(badge_of(&mut app, "nord"), "dark, yours");
    assert_eq!(badge_of(&mut app, "dracula"), "dark");
    assert_eq!(badge_of(&mut app, "terminal"), "");
}

#[test]
fn a_theme_written_while_the_shell_runs_is_tried_on_by_opening_the_picker_again() {
    let path = config("");
    let mut app = app_under(path.clone(), SIZE);
    write_theme(&path, "dusk", USER_THEME);
    open_theme_picker(&mut app);
    type_text(&mut app, "dusk");
    app.update();
    assert_eq!(accent(&app), USER_THEME_ACCENT);
    press_key(&mut app, KeyCode::Esc);

    write_theme(&path, "dusk", &USER_THEME.replace("#010203", "#040506"));
    open_theme_picker(&mut app);
    type_text(&mut app, "dusk");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(accent(&app), Color::Rgb(4, 5, 6));
    assert_eq!(said(&app), ["theme dusk"]);
    let kept = std::fs::read_to_string(&path).unwrap();
    assert!(kept.ends_with("[tui.theme]\nname = \"dusk\"\n"), "{kept}");
}

/// The picker tries its first row on as it opens, so what was put on
/// afresh is what escape goes back to.
#[test]
fn the_theme_worn_is_read_again_as_the_picker_opens_and_is_what_escape_puts_back() {
    let path = config("[tui.theme]\nname = \"dusk\"\n");
    write_theme(&path, "dusk", USER_THEME);
    let mut app = app_under(path.clone(), SIZE);
    write_theme(&path, "dusk", &USER_THEME.replace("#010203", "#040506"));
    open_theme_picker(&mut app);
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(accent(&app), Color::Rgb(4, 5, 6));

    write_theme(&path, "dusk", BROKEN_THEME);
    open_theme_picker(&mut app);
    press_key(&mut app, KeyCode::Esc);
    assert_eq!(accent(&app), Color::Rgb(4, 5, 6), "one that broke stays");
}

const BROKEN_THEME: &str = "family = \"mine\"\nvariant = \"dark\"\naccent = 3\n";
const DUSK_UNREAD: &str = "theme \"dusk\" does not read";

/// The shell beside a theme `dusk` that does not read, and what it wore
/// as it opened.
fn app_beside_a_broken_theme() -> (Headless, PathBuf, Color) {
    let path = config("");
    write_theme(&path, "dusk", BROKEN_THEME);
    let app = app_under(path.clone(), SIZE);
    let complaints = said(&app);
    assert!(
        matches!(&complaints[..], [said] if said.ends_with("dusk.toml: accent: not a colour")),
        "{complaints:?}"
    );
    let worn = accent(&app);
    (app, path, worn)
}

#[test]
fn a_theme_of_the_users_that_does_not_read_is_offered_as_such_and_not_kept() {
    let (mut app, path, worn) = app_beside_a_broken_theme();
    open_theme_picker(&mut app);
    assert_eq!(badge_of(&mut app, "dusk"), "does not read");
    let mut rows = app
        .world_mut()
        .query_filtered::<&UiLabel, (With<ListItem>, With<UiStyle>)>();
    let dim: Vec<String> = rows
        .iter(app.world())
        .map(|label| label.0.to_string())
        .collect();
    assert_eq!(dim, ["dusk"], "and is said dimly");
    type_text(&mut app, "d");
    app.update();
    assert_ne!(accent(&app), worn, "another theme is reached on the way");
    type_text(&mut app, "usk");
    app.update();
    assert_eq!(accent(&app), worn, "reaching it puts the first back");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(accent(&app), worn);
    assert_eq!(said(&app).last().unwrap(), DUSK_UNREAD);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
}

#[test]
fn a_theme_that_does_not_read_tapped_before_the_cursor_rests_on_it_puts_the_first_back() {
    let (mut app, path, worn) = app_beside_a_broken_theme();
    open_theme_picker(&mut app);
    type_text(&mut app, "d");
    app.update();
    assert_ne!(accent(&app), worn);
    let (column, row) = cell_of(&app, "dusk");
    tap(&mut app, column, row);
    assert!(!composed_frame(&app).contains("╭ Theme"));
    assert_eq!(accent(&app), worn);
    assert_eq!(said(&app).last().unwrap(), DUSK_UNREAD);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
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
