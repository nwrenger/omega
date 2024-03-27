pub mod clipboard;
pub mod error;
pub mod events;

use std::env;
use std::fs;
use std::path::PathBuf;

use cursive::backends;
use cursive::event::Event;
use cursive::event::Key;

use cursive::logger;
use cursive::theme::BaseColor;
use cursive::theme::BorderStyle;
use cursive::theme::Color;
use cursive::theme::PaletteColor;
use cursive::theme::Theme;
use cursive::traits::*;
use cursive::views::Panel;
use cursive::views::{OnEventView, TextArea};

use cursive_buffered_backend::BufferedBackend;
use error::ResultExt;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const PKG_LICENSE: &str = env!("CARGO_PKG_LICENSE");

#[derive(Clone, Debug)]
struct State {
    file_path: Option<PathBuf>,
}

fn backend() -> Box<BufferedBackend> {
    let crossterm_backend = backends::crossterm::Backend::init().unwrap();
    let buffered_backend = cursive_buffered_backend::BufferedBackend::new(crossterm_backend);
    Box::new(buffered_backend)
}

fn main() {
    logger::init();
    let mut siv = cursive::default();
    let args: Vec<String> = env::args().collect();

    let file_path = if args.len() > 1 {
        Some(PathBuf::from(&args[1]))
    } else {
        None
    };

    let content =
        if file_path.is_some() && file_path.as_ref().unwrap_or(&PathBuf::default()).exists() {
            Some(fs::read_to_string(file_path.clone().unwrap_or_default()).unwrap_or_default())
        } else {
            None
        };

    siv.set_user_data(State {
        file_path: file_path.clone(),
    });

    // disable/handle globally
    siv.clear_global_callbacks(Event::CtrlChar('c'));

    siv.clear_global_callbacks(Event::CtrlChar('z'));
    siv.clear_global_callbacks(Event::CtrlChar('d'));
    siv.clear_global_callbacks(Event::CtrlChar('q'));
    siv.clear_global_callbacks(Event::CtrlChar('f'));
    siv.clear_global_callbacks(Event::CtrlChar('s'));
    siv.clear_global_callbacks(Event::CtrlChar('o'));

    siv.add_global_callback(Event::CtrlChar('z'), |s| events::info(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('d'), |s| s.toggle_debug_console());
    siv.add_global_callback(Event::CtrlChar('q'), |s| events::quit(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('f'), |s| s.quit());
    siv.add_global_callback(Event::CtrlChar('s'), |s| events::save(s).handle(s));
    siv.add_global_callback(Event::CtrlChar('o'), |s| events::open(s).handle(s));

    let text_area = TextArea::new()
        .content(content.clone().unwrap_or_default())
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

    let binding = file_path.clone().unwrap_or_default();
    let file_str = binding.to_string_lossy();
    let label = file_str + if content.is_none() { " *" } else { "" };

    let panel = Panel::new(events).title(label).with_name("title_text");

    siv.add_fullscreen_layer(panel);

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
