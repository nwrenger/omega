pub mod clipboard;
pub mod error;
pub mod events;
pub mod file_tree;

use std::env;
use std::path::Path;
use std::path::PathBuf;

use cursive::backends;
use cursive::event::Event;
use cursive::event::Key;

use cursive::logger::reserve_logs;
use cursive::logger::CursiveLogger;
use cursive::reexports::log;
use cursive::theme::BaseColor;
use cursive::theme::BorderStyle;
use cursive::theme::Color;
use cursive::theme::PaletteColor;
use cursive::theme::Theme;
use cursive::traits::*;
use cursive::views::LinearLayout;
use cursive::views::NamedView;
use cursive::views::Panel;
use cursive::views::ResizedView;
use cursive::views::ScrollView;
use cursive::views::{OnEventView, TextArea};

use cursive_buffered_backend::BufferedBackend;
use cursive_tree_view::TreeView;
use error::ResultExt;
use events::open_paths;
use file_tree::TreeEntry;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const PKG_LICENSE: &str = env!("CARGO_PKG_LICENSE");

#[derive(Clone, Debug)]
struct State {
    file_path: Option<PathBuf>,
    project_path: PathBuf,
}

fn backend() -> Box<BufferedBackend> {
    let crossterm_backend = backends::crossterm::Backend::init().unwrap();
    let buffered_backend = cursive_buffered_backend::BufferedBackend::new(crossterm_backend);
    Box::new(buffered_backend)
}

// Helper types of the main/tree panel
type EditorPanel = Panel<OnEventView<ResizedView<NamedView<ScrollView<NamedView<TextArea>>>>>>;
type TreePanel = ResizedView<Panel<ScrollView<NamedView<TreeView<TreeEntry>>>>>;

fn logging() {
    reserve_logs(1_000);
    log::set_logger(&CursiveLogger).unwrap();
    log::set_max_level(log::LevelFilter::Warn);
}

fn main() {
    logging();

    let mut siv = cursive::default();

    // gathering arguments
    let args: Vec<String> = env::args().collect();
    let inc_path = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let mut file_path = None;
    let mut project_path = PathBuf::from("/");

    if let Some(inc_path) = inc_path {
        if inc_path.is_file() {
            file_path = Some(inc_path.clone());
            project_path = PathBuf::from(inc_path.parent().unwrap_or(Path::new("/")));
        } else if inc_path.is_dir() {
            project_path = inc_path;
        } else {
            panic!("An invalid/not existing directory/file was specified!");
        }
    }

    // disable/handle globally
    siv.clear_global_callbacks(Event::CtrlChar('c'));

    siv.clear_global_callbacks(Key::Esc);
    siv.clear_global_callbacks(Event::CtrlChar('p'));
    siv.clear_global_callbacks(Event::CtrlChar('q'));
    siv.clear_global_callbacks(Event::CtrlChar('f'));
    siv.clear_global_callbacks(Event::CtrlChar('o'));
    siv.clear_global_callbacks(Event::CtrlChar('n'));
    siv.clear_global_callbacks(Event::CtrlChar('r'));
    siv.clear_global_callbacks(Event::CtrlChar('d'));
    siv.clear_global_callbacks(Event::CtrlChar('s'));

    siv.add_global_callback(Key::Esc, |s| events::info(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('p'), |s| s.toggle_debug_console());
    siv.add_global_callback(Event::CtrlChar('q'), |s| events::quit(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('f'), |s| s.quit());
    siv.add_global_callback(Event::CtrlChar('o'), |s| events::open(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('n'), |s| events::new(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('r'), |s| events::rename(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('d'), |s| events::delete(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('s'), |s| events::save(s).handle(s));

    let text_area = TextArea::new()
        .disabled()
        .with_name("editor")
        .scrollable()
        .with_name("editor_scroll")
        .full_screen();

    let events = OnEventView::new(text_area)
        .on_pre_event(Event::CtrlChar('c'), move |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::copy(&mut text_area).handle(s);
            }
        })
        .on_pre_event(Event::CtrlChar('v'), move |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::paste(&mut text_area).handle(s);
            }
        })
        .on_pre_event(Event::CtrlChar('x'), move |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::cut(&mut text_area).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Up), |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::move_line(&mut text_area, events::Direction::Up).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Down), |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::move_line(&mut text_area, events::Direction::Down).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Left), |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::move_cursor_end(&mut text_area, events::Direction::Left).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Right), |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::move_cursor_end(&mut text_area, events::Direction::Right).handle(s);
            }
        })
        .on_pre_event(Key::Tab, |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::tabulator(&mut text_area, true).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Tab), |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                events::tabulator(&mut text_area, false).handle(s);
            }
        });

    let editor_panel = Panel::new(events).title("").with_name("editor_title");
    let file_tree_panel = Panel::new(file_tree::new(&project_path))
        .title("")
        .fixed_width(40)
        .with_name("tree_title");

    let layout = LinearLayout::horizontal()
        .child(file_tree_panel)
        .child(editor_panel);

    siv.add_fullscreen_layer(layout);

    // set initial data
    open_paths(&mut siv, &project_path, file_path.as_ref()).unwrap();

    // custom theme
    let mut theme = Theme {
        shadow: false,
        ..Default::default()
    };

    theme.palette[PaletteColor::Background] = Color::Dark(BaseColor::Black);
    theme.palette[PaletteColor::View] = Color::Dark(BaseColor::Black);
    theme.palette[PaletteColor::Primary] = Color::Dark(BaseColor::White);
    theme.palette[PaletteColor::Secondary] = Color::Dark(BaseColor::Blue);
    theme.palette[PaletteColor::Secondary] = Color::Dark(BaseColor::White);
    theme.palette[PaletteColor::Tertiary] = Color::Dark(BaseColor::Black);
    theme.palette[PaletteColor::TitlePrimary] = Color::Light(BaseColor::Red);
    theme.palette[PaletteColor::TitleSecondary] = Color::Dark(BaseColor::Yellow);
    theme.palette[PaletteColor::Highlight] = Color::Light(BaseColor::Red);
    theme.palette[PaletteColor::HighlightInactive] = Color::Dark(BaseColor::Yellow);

    theme.borders = BorderStyle::Simple;

    siv.set_theme(theme);

    // start event loop
    siv.run_with(|| backend());
}
