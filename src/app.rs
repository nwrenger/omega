use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

use crate::ui::{
    edit_area::{Cursor, EditArea},
    quick_access, update_ui_state,
};
use cursive::{
    event::{Event, Key},
    reexports::log::info,
    view::{Nameable, Resizable},
    views::{LinearLayout, NamedView, Panel, ResizedView, ScrollView},
    Vec2,
};
use cursive_tree_view::TreeView;
use syntect::highlighting::ThemeSet;

use crate::{
    error::ResultExt,
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
    pub scroll_offset: Vec2,
    pub cursor: Cursor,
}

impl State {
    pub fn is_file_edited(&self, path: &PathBuf) -> bool {
        self.files_edited.contains_key(path)
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

    pub fn remove(&mut self, path: &PathBuf) {
        for file_path in self.files.clone().keys() {
            if file_path.starts_with(path) {
                self.files.remove(file_path);
                self.files_edited.remove(file_path);
                if let Some(current_file) = &self.current_file {
                    if current_file == file_path {
                        self.current_file = None;
                    }
                }
            }
        }
    }

    pub fn open_new_project(
        &mut self,
        project_path: &Path,
        current_file: Option<&PathBuf>,
    ) -> Self {
        self.project_path = project_path.canonicalize().unwrap_or_default();
        let canonicalized_current_file =
            current_file.map(|current_file| current_file.canonicalize().unwrap_or_default());
        self.current_file = canonicalized_current_file;
        self.to_owned()
    }

    pub fn open_new_file(&mut self, current_file: PathBuf, content: FileData) -> Self {
        let canonicalized_current_file = current_file.canonicalize().unwrap_or_default();
        self.files
            .insert(canonicalized_current_file.clone(), content);
        self.current_file = Some(canonicalized_current_file);
        self.to_owned()
    }

    pub fn update_paths_after_rename(&mut self, old_parent: &Path, new_parent: &Path) {
        let adjust_path = |path: &PathBuf| -> PathBuf {
            if let Ok(relative) = path.strip_prefix(old_parent) {
                new_parent.join(relative)
            } else {
                path.clone()
            }
        };

        self.files = self
            .files
            .drain()
            .map(|(path, data)| (adjust_path(&path), data))
            .collect();

        self.files_edited = self
            .files_edited
            .drain()
            .map(|(path, edited)| (adjust_path(&path), edited))
            .collect();

        if let Some(current_file) = &self.current_file {
            self.current_file = Some(adjust_path(current_file));
        }

        self.project_path = adjust_path(&self.project_path);
    }
}

// Helper types of the main/tree panel
pub type EditorPanel = Panel<ResizedView<NamedView<EditArea>>>;
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
            println!("An invalid/not existing directory/file was specified!");
            std::process::exit(1);
        }
    }

    // disable/handle global shortcuts
    siv.clear_global_callbacks(Event::CtrlChar('c'));

    siv.clear_global_callbacks(Event::CtrlChar('p'));
    siv.clear_global_callbacks(Event::Key(Key::Esc));

    siv.add_global_callback(Event::CtrlChar('p'), |s| quick_access::new(s).handle(s));
    siv.add_global_callback(Event::Key(Key::Esc), |s| {
        if s.screen().len() > 1 {
            s.pop_layer();
        }
    });

    // The current theme, needs to be passed on the general styling and the editor ui for fitting syntax highlighting style.
    let theme = ThemeSet::load_defaults().themes["base16-eighties.dark"].clone();

    let mut raw_edit_area = EditArea::new(&theme).disabled();

    // Detecting edits on `EditArea` and updating global state.
    raw_edit_area.set_on_edit(|siv, content, scroll_offset, cursor| {
        let mut state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap_or_default();
        if let Some(current_file) = &state.current_file {
            let contents = state.files.get_mut(current_file);
            if let Some(contents) = contents {
                contents.str = content.to_string();
                contents.scroll_offset = scroll_offset;
                contents.cursor = cursor;
                state.files_edited.insert(current_file.clone(), true);

                // Update title.
                let title = state
                    .clone()
                    .current_file
                    .unwrap_or_default()
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
                    + " *";
                siv.call_on_name("editor_title", |view: &mut EditorPanel| {
                    view.set_title(title);
                })
                .unwrap();
            }
        }
        siv.set_user_data(state);
    });

    // Detecting cursor changes and updating global state.
    raw_edit_area.set_on_interact(|siv, _, scroll_offset, cursor| {
        let mut state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap_or_default();
        if let Some(current_file) = &state.current_file {
            let contents = state.files.get_mut(current_file);
            if let Some(contents) = contents {
                contents.scroll_offset = scroll_offset;
                contents.cursor = cursor;
            }
        }
        siv.set_user_data(state);
    });

    // Detecting scrolling and updating global state.
    raw_edit_area.set_on_scroll(|siv, _, scroll_offset, _| {
        let mut state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap_or_default();
        if let Some(current_file) = &state.current_file {
            let contents = state.files.get_mut(current_file);
            if let Some(contents) = contents {
                contents.scroll_offset = scroll_offset;
            }
        }
        siv.set_user_data(state);
    });

    // Setting general styling to theme
    siv.with_theme(|t| {
        t.shadow = false;
        if let Some(background) = theme
            .settings
            .background
            .map(cursive_syntect::translate_color)
        {
            t.palette[cursive::theme::PaletteColor::Background] = background;
            t.palette[cursive::theme::PaletteColor::View] = background;
        }
        if let Some(foreground) = theme
            .settings
            .foreground
            .map(cursive_syntect::translate_color)
        {
            t.palette[cursive::theme::PaletteColor::Primary] = foreground;
            t.palette[cursive::theme::PaletteColor::Secondary] = foreground;
            t.palette[cursive::theme::PaletteColor::Tertiary] = foreground;
            t.palette[cursive::theme::PaletteColor::TitlePrimary] = foreground;
            t.palette[cursive::theme::PaletteColor::TitleSecondary] = foreground;
        }

        if let Some(highlight) = theme
            .settings
            .highlight
            .map(cursive_syntect::translate_color)
        {
            t.palette[cursive::theme::PaletteColor::Highlight] = highlight;
            t.palette[cursive::theme::PaletteColor::HighlightText] = highlight;
        }
    });

    let edit_area = raw_edit_area.with_name("editor").full_screen();

    let editor_panel = Panel::new(edit_area).title("").with_name("editor_title");
    let file_tree_panel = Panel::new(file_tree::new(&project_path))
        .title("")
        .fixed_width(40)
        .with_name("tree_title");

    let layout = LinearLayout::horizontal()
        .child(file_tree_panel)
        .child(editor_panel);

    siv.add_fullscreen_layer(layout);

    // Set initial data.
    update_ui_state(&mut siv, &project_path, file_path.as_ref()).unwrap();

    info!("App up and running. Initial setup finished!");

    // Start event loop.
    siv.run();
    // siv.run_with(|| backend());
}

// /// Initiates a buffered Backend for improved visuals
// ///
// /// For linux and windows it uses `crossterm`, for macos it uses `ncurses`
// fn backend() -> Box<dyn Backend> {
//     #[cfg(not(target_os = "macos"))]
//     {
//         let crossterm_backend = backends::crossterm::Backend::init().unwrap();
//         let buffered_backend = cursive_buffered_backend::BufferedBackend::new(crossterm_backend);
//         Box::new(buffered_backend)
//     }
//     #[cfg(target_os = "macos")]
//     {
//         let ncurses_backend = backends::curses::n::Backend::init().unwrap();
//         let buffered_backend = cursive_buffered_backend::BufferedBackend::new(ncurses_backend);
//         Box::new(buffered_backend)
//     }
// }
