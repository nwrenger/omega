use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

use crate::ui::edit_area::EditArea;
use cursive::{
    backends,
    event::{Event, Key},
    theme::{BaseColor, BorderStyle, Color, PaletteColor, Theme},
    view::{Nameable, Resizable, Scrollable},
    views::{LinearLayout, NamedView, OnEventView, Panel, ResizedView, ScrollView},
};
use cursive_buffered_backend::BufferedBackend;
use cursive_tree_view::TreeView;

use crate::{
    error::ResultExt,
    events::{self, open_paths},
    ui::file_tree::{self, TreeEntry},
};

pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PKG_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const PKG_LICENSE: &str = env!("CARGO_PKG_LICENSE");

#[derive(Clone, Debug, Default)]
pub struct State {
    pub project_path: PathBuf,
    pub current_file: Option<PathBuf>,
    pub files: HashMap<PathBuf, FileData>,
    pub files_edited: HashMap<PathBuf, bool>,
}

#[derive(Clone, Debug, Default)]

pub struct FileData {
    pub str: String,
}

impl State {
    pub fn is_file_edited(&self, path: &PathBuf) -> bool {
        self.files_edited.get(path).is_some()
    }

    pub fn is_current_file_edited(&self) -> bool {
        self.is_file_edited(self.current_file.as_ref().unwrap_or(&PathBuf::default()))
    }

    pub fn get_file(&self, path: &PathBuf) -> Option<&FileData> {
        self.files.get(path)
    }

    pub fn get_current_file(&self) -> Option<&FileData> {
        self.get_file(self.current_file.as_ref().unwrap_or(&PathBuf::default()))
    }

    pub fn remove_file(&mut self, path: &PathBuf) {
        self.files.remove(path);
        self.files_edited.remove(path);
        if let Some(current_file) = &self.current_file {
            if current_file == path {
                self.current_file = None;
            }
        }
    }

    pub fn open_new_project(
        &mut self,
        project_path: &Path,
        current_file: Option<&PathBuf>,
    ) -> Self {
        self.project_path = project_path.to_path_buf();
        self.current_file = current_file.cloned();
        self.to_owned()
    }

    pub fn open_new_file(&mut self, current_file: PathBuf, content: FileData) -> Self {
        self.files.insert(current_file.clone(), content);
        self.current_file = Some(current_file);
        self.to_owned()
    }
}

// Helper types of the main/tree panel
pub type EditorPanel = Panel<OnEventView<ResizedView<ScrollView<NamedView<EditArea>>>>>;
pub type TreePanel = ResizedView<Panel<ScrollView<NamedView<TreeView<TreeEntry>>>>>;

/// Starts the app && event loop
pub fn start() {
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
    siv.add_global_callback(Event::CtrlChar('s'), |s| events::save(s, None).handle(s));

    let mut raw_text_area = EditArea::new().disabled();
    // detecting edits on `EditArea`, and updating global state
    raw_text_area.set_on_edit(|siv, content, _| {
        let mut state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap_or_default();
        if let Some(current_file) = &state.current_file {
            let contents = state.files.get_mut(current_file);
            if let Some(contents) = contents {
                contents.str = content.to_string();
                state.files_edited.insert(current_file.to_path_buf(), true);

                // update title
                siv.call_on_name("editor_title", |editor_panel: &mut EditorPanel| {
                    editor_panel.set_title(format!(
                        "{} *",
                        state
                            .clone()
                            .current_file
                            .unwrap_or_default()
                            .to_string_lossy()
                    ));
                })
                .unwrap();
            }
        }
        siv.set_user_data(state);
    });

    let text_area = raw_text_area.with_name("editor").scrollable().full_screen();

    let events = OnEventView::new(text_area)
        .on_pre_event(Event::CtrlChar('c'), move |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
                events::copy(&mut text_area).handle(s);
            }
        })
        .on_pre_event(Event::CtrlChar('v'), move |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
                events::paste(&mut text_area).handle(s);
            }
        })
        .on_pre_event(Event::CtrlChar('x'), move |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
                events::cut(&mut text_area).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Up), |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
                events::move_line(&mut text_area, events::Direction::Up).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Down), |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
                events::move_line(&mut text_area, events::Direction::Down).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Left), |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
                events::move_cursor_end(&mut text_area, events::Direction::Left).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Right), |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
                events::move_cursor_end(&mut text_area, events::Direction::Right).handle(s);
            }
        })
        .on_pre_event(Key::Tab, |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
                events::tabulator(&mut text_area, true).handle(s);
            }
        })
        .on_pre_event(Event::Shift(Key::Tab), |s| {
            if let Some(mut text_area) = s.find_name::<EditArea>("editor") {
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
    theme.palette[PaletteColor::TitleSecondary] = Color::Dark(BaseColor::Red);
    theme.palette[PaletteColor::Highlight] = Color::Light(BaseColor::Red);
    theme.palette[PaletteColor::HighlightInactive] = Color::Dark(BaseColor::Red);

    theme.borders = BorderStyle::Simple;

    siv.set_theme(theme);

    // start event loop
    siv.run_with(|| backend());
}

fn backend() -> Box<BufferedBackend> {
    let crossterm_backend = backends::crossterm::Backend::init().unwrap();
    let buffered_backend = cursive_buffered_backend::BufferedBackend::new(crossterm_backend);
    Box::new(buffered_backend)
}
