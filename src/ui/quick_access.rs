use std::{
    fs::{self, OpenOptions},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{
        DebugView, Dialog, EditView, LinearLayout, ListView, NamedView, ScrollView, SelectView,
        TextView,
    },
    Cursive,
};
use cursive_tree_view::TreeView;

use crate::{
    app::{
        FileData, State, PKG_AUTHORS, PKG_DESCRIPTION, PKG_LICENSE, PKG_NAME, PKG_REPOSITORY,
        PKG_VERSION,
    },
    error::{Error, Result, ResultExt},
    ui::{
        file_tree::{load_parent, TreeEntry},
        open_file, path_input,
    },
};

use super::{update_title, update_ui_state};

const VARIANTS: &[&str] = &[
    "info", "debug", "open", "save", "new", "delete", "rename", "quit",
];

struct Entry {
    str: String,
    ty: EntryType,
}

impl Entry {
    fn file(str: String) -> Self {
        Self {
            str,
            ty: EntryType::File,
        }
    }
    fn command(str: String) -> Self {
        Self {
            str,
            ty: EntryType::Command,
        }
    }
}

enum EntryType {
    File,
    Command,
}

/// Creates a new Quick Access view
///
/// The recent file are visible and can be filtered via the `EditView`.
/// Typing in a `>` shows you all current commands which are `info`, `open`, `new`, `rename`, and `delete`.
///
/// Pressing enter in the `EditView` will auto select the current selected option.
pub fn new(siv: &mut Cursive) -> Result<()> {
    if let Some(pos) = siv.screen_mut().find_layer_from_name("quick_access_view") {
        siv.screen_mut().remove_layer(pos);
    } else {
        let state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap();
        siv.add_layer(
            Dialog::new()
                .padding_lrtb(1, 1, 1, 0)
                .content(
                    LinearLayout::vertical()
                        .child(
                            EditView::new()
                                .on_edit(on_edit)
                                .on_submit(on_submit)
                                .with_name("query"),
                        )
                        .child(
                            SelectView::new()
                                .with_all(
                                    search_fn(&state, "")
                                        .into_iter()
                                        .map(|f| (f.str.clone(), f)),
                                )
                                .on_submit(show_next_window)
                                .with_name("matches")
                                .scrollable(),
                        )
                        .fixed_height(10),
                )
                .dismiss_button("Cancel")
                .title("Quick Access")
                .full_width()
                .with_name("quick_access_view"),
        );
    }
    Ok(())
}

fn on_edit(siv: &mut Cursive, query: &str, _cursor: usize) {
    let state = siv
        .with_user_data(|state: &mut State| state.clone())
        .unwrap();
    let matches = search_fn(&state, query);
    // Update the `matches` view with the filtered array of cities
    siv.call_on_name("matches", |v: &mut SelectView<Entry>| {
        v.clear();
        v.add_all(matches.into_iter().map(|f| (f.str.clone(), f)));
    });
}

fn search_fn(state: &State, query: &'_ str) -> Vec<Entry> {
    if query.chars().next().unwrap_or_default() == '>' {
        let query = query.get(1..).unwrap_or("");
        VARIANTS
            .iter()
            .copied()
            .filter(|&item| {
                let item = item.to_lowercase();
                let query = query.to_lowercase();
                item.contains(&query)
            })
            .map(|f| Entry::command(f.to_string()))
            .collect()
    } else {
        let mut filtered = state
            .files
            .iter()
            .filter(|p| {
                p.0.starts_with(&state.project_path) && {
                    let item = p.0.to_string_lossy().to_lowercase();
                    let query = query.to_lowercase();
                    item.contains(&query)
                }
            })
            .collect::<Vec<_>>();
        filtered.sort_by(|a, b| b.0.cmp(a.0));

        filtered
            .iter()
            .map(|f| Entry::file(f.0.to_string_lossy().to_string()))
            .collect()
    }
}

fn on_submit(siv: &mut Cursive, _: &str) {
    let matches = siv.find_name::<SelectView<Entry>>("matches").unwrap();
    if !matches.is_empty() {
        let entry = &*matches.selection().unwrap();
        show_next_window(siv, entry);
    };
}

fn show_next_window(siv: &mut Cursive, entry: &Entry) {
    match entry.ty {
        EntryType::File => {
            let goto_file = &PathBuf::from(entry.str.clone());
            if let Err(e) = open_file(siv, goto_file) {
                Into::<Error>::into(e).to_dialog(siv);
                return;
            }
            siv.pop_layer();
        }
        EntryType::Command => {
            siv.pop_layer();
            run_command(siv, entry.str.clone());
        }
    }
}

fn run_command(siv: &mut Cursive, str: String) {
    let str = str.as_str();
    match str {
        "info" => info(siv).handle(siv),
        "debug" => debug(siv).handle(siv),
        "open" => open_project(siv).handle(siv),
        "save" => save(siv, None).handle(siv),
        "new" => new_file(siv).handle(siv),
        "delete" => delete_file(siv).handle(siv),
        "rename" => rename_file(siv).handle(siv),
        "quit" => quit(siv).handle(siv),
        _ => unreachable!(),
    }
}

/// Shows all commands
fn info(siv: &mut Cursive) -> Result<()> {
    if let Some(pos) = siv.screen_mut().find_layer_from_name("info") {
        siv.screen_mut().remove_layer(pos);
    } else {
        siv.add_layer(
            Dialog::new()
                .title(format!("{PKG_NAME} - Info"))
                .padding_lrtb(1, 1, 1, 0)
                .dismiss_button("Close")
                .content(
                    ListView::new()
                        // general info
                        .child(
                            "A `*` in the Title indicates that",
                            TextView::new("the current file has been edited"),
                        )
                        .delimiter()
                        // pck info
                        .child("Version", TextView::new(PKG_VERSION))
                        .child("Repository", TextView::new(PKG_REPOSITORY))
                        .child("Authors", TextView::new(PKG_AUTHORS))
                        .child("Description", TextView::new(PKG_DESCRIPTION))
                        .child("License", TextView::new(PKG_LICENSE))
                        .delimiter()
                        // shortcuts
                        // global
                        .child("Open Quick Access", TextView::new("Ctrl + p"))
                        .child("Close current Dialog", TextView::new("Esc"))
                        .delimiter()
                        // quick access commands
                        .child("Open Debugger", TextView::new("debug"))
                        .child("Open Infos", TextView::new("info"))
                        .child("Opening a new File/Project", TextView::new("open"))
                        .child("Saving the current opened File", TextView::new("save"))
                        .child("Creating a new File/Directory", TextView::new("new"))
                        .child("Renaming a File/Directory", TextView::new("rename"))
                        .child("Deleting a File/Directory", TextView::new("delete"))
                        .child("Quitting", TextView::new("quit"))
                        .delimiter()
                        // editor
                        .child("Copying Line", TextView::new("Ctrl + c"))
                        .child("Paste Clipboard", TextView::new("Ctrl + v"))
                        .child("Cut Line", TextView::new("Ctrl + x"))
                        .child("Move Line", TextView::new("Shift + Up/Down"))
                        .child("Move Cursor to EoL", TextView::new("Shift + Left/Right"))
                        .child("Ident", TextView::new("Tab"))
                        .child("Remove Ident", TextView::new("Shift + Tab"))
                        .scrollable()
                        .with_name("info"),
                ),
        );
    }

    Ok(())
}

/// Shows debug
fn debug(siv: &mut Cursive) -> Result<()> {
    if let Some(pos) = siv.screen_mut().find_layer_from_name("debug") {
        siv.screen_mut().remove_layer(pos);
    } else {
        siv.add_layer(
            Dialog::around(
                ScrollView::new(NamedView::new("debug", DebugView::new())).scroll_x(true),
            )
            .padding_lrtb(1, 1, 1, 0)
            .title("Debug Console")
            .dismiss_button("Close")
            .full_width(),
        );
    }

    Ok(())
}

/// Opens a new file/project
///
/// This wont override current edits made to files so it can be seen as a `save operation`
///
/// Also notable is that this will reload state so the current file tree, the preferred way
/// to move through all your current opened files without using the file tree is using
/// `goto` (`Ctrl` + `g`)
fn open_project(siv: &mut Cursive) -> Result<()> {
    if let Some(pos) = siv.screen_mut().find_layer_from_name("open") {
        siv.screen_mut().remove_layer(pos);
        Ok(())
    } else {
        let state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap();
        siv.add_layer(
            Dialog::new()
                .title("Open")
                .padding_lrtb(1, 1, 1, 0)
                .content(path_input::new(
                    &state.project_path,
                    "open_new_path".to_string(),
                    true,
                )?)
                .button("Open", move |siv| {
                    let inc_path = siv
                        .call_on_name("open_new_path_edit", |view: &mut EditView| {
                            PathBuf::from(view.get_content().to_string())
                        })
                        .unwrap();

                    let mut current_file = None;
                    let project_path = if inc_path.is_file() {
                        current_file = Some(inc_path.clone());
                        PathBuf::from(inc_path.parent().unwrap_or(Path::new("/")))
                    } else if inc_path.is_dir() {
                        inc_path
                    } else {
                        Error::FileSystem("Path doesn't exists".to_string()).to_dialog(siv);
                        return;
                    };

                    if let Err(e) = update_ui_state(siv, &project_path, current_file.as_ref()) {
                        Into::<Error>::into(e).to_dialog(siv);
                        return;
                    }

                    siv.pop_layer();
                })
                .dismiss_button("Cancel")
                .full_width()
                .with_name("open"),
        );

        Ok(())
    }
}

/// Save current progress + Handling Title
pub fn save(siv: &mut Cursive, other: Option<(&PathBuf, &String)>) -> Result<()> {
    let mut state = siv
        .with_user_data(|state: &mut State| state.clone())
        .unwrap();

    let binding = FileData::default();
    let content = &state.get_current_file().unwrap_or(&binding).str;

    let current_file = state
        .current_file
        .as_ref()
        .map(|current_file| (current_file, content));

    let data = if let Some(other) = other {
        Some(other)
    } else {
        current_file
    };

    if let Some(data) = data {
        let old_content = fs::read_to_string(data.0)?;

        if &old_content != data.1 {
            // just write when something really changed
            fs::write(data.0.clone(), data.1)?;
        }

        update_title(siv, None, data.0);

        state.files_edited.remove(data.0);

        siv.set_user_data(state);
    }
    Ok(())
}

/// Creates a new file
fn new_file(siv: &mut Cursive) -> Result<()> {
    if let Some(pos) = siv.screen_mut().find_layer_from_name("new") {
        siv.screen_mut().remove_layer(pos);
    } else {
        let state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap();
        siv.add_layer(
            Dialog::new()
                .title("Create As")
                .padding_lrtb(1, 1, 1, 0)
                .content(path_input::new(
                    &state.project_path,
                    "new_path".to_string(),
                    false,
                )?)
                .button("A File", {
                    move |siv: &mut Cursive| {
                        let state = siv
                            .with_user_data(|state: &mut State| state.clone())
                            .unwrap();
                        let new_path = siv
                            .call_on_name("new_path_edit", |view: &mut EditView| {
                                PathBuf::from(view.get_content().to_string())
                            })
                            .unwrap();

                        if let Err(e) = OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(new_path)
                        {
                            Into::<Error>::into(e).to_dialog(siv);
                            return;
                        }

                        siv.call_on_name("tree", |tree: &mut TreeView<TreeEntry>| {
                            load_parent(tree, &state.project_path);
                        });

                        siv.pop_layer();
                    }
                })
                .button("A Directory", {
                    move |siv: &mut Cursive| {
                        let state = siv
                            .with_user_data(|state: &mut State| state.clone())
                            .unwrap();
                        let new_path = siv
                            .call_on_name("new_path_edit", |view: &mut EditView| {
                                PathBuf::from(view.get_content().to_string())
                            })
                            .unwrap();

                        if let Err(e) = fs::create_dir_all(new_path) {
                            Into::<Error>::into(e).to_dialog(siv);
                            return;
                        }

                        siv.call_on_name("tree", |tree: &mut TreeView<TreeEntry>| {
                            load_parent(tree, &state.project_path);
                        });

                        siv.pop_layer();
                    }
                })
                .dismiss_button("Cancel")
                .full_width()
                .with_name("new"),
        );
    }
    Ok(())
}

/// Rename(+move) a file/directory
fn rename_file(siv: &mut Cursive) -> Result<()> {
    if let Some(pos) = siv.screen_mut().find_layer_from_name("rename") {
        siv.screen_mut().remove_layer(pos);
    } else {
        let state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap();
        let layout = LinearLayout::vertical()
            .child(TextView::new(
                "Note the file will be autosaved before it'll be moved/renamed!",
            ))
            .child(TextView::new(" "))
            .child(
                LinearLayout::horizontal()
                    .child(LinearLayout::vertical().child(TextView::new("From")).child(
                        path_input::new(&state.project_path, "from_rename_path".to_string(), true)?,
                    ))
                    .child(TextView::new(" "))
                    .child(
                        LinearLayout::vertical()
                            .child(TextView::new("To"))
                            .child(path_input::new(
                                &state.project_path,
                                "to_rename_path".to_string(),
                                false,
                            )?)
                            .full_width(),
                    ),
            );
        siv.add_layer(
            Dialog::new()
                .title("Rename")
                .padding_lrtb(1, 1, 1, 0)
                .content(layout)
                .button("Confirm", |siv| {
                    let mut state = siv
                        .with_user_data(|state: &mut State| state.clone())
                        .unwrap();
                    let from = siv
                        .call_on_name("from_rename_path_edit", |view: &mut EditView| {
                            PathBuf::from(view.get_content().to_string())
                        })
                        .unwrap();

                    let to = siv
                        .call_on_name("to_rename_path_edit", |view: &mut EditView| {
                            PathBuf::from(view.get_content().to_string())
                        })
                        .unwrap();

                    if !to.exists() {
                        if let Err(e) = fs::rename(&from, &to) {
                            Into::<Error>::into(e).to_dialog(siv);
                            return;
                        }
                    } else {
                        Into::<Error>::into(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            "Destination already exists",
                        ))
                        .to_dialog(siv);
                        return;
                    }

                    state.update_paths_after_rename(&from, &to);
                    siv.set_user_data(state.clone());

                    if let Err(e) =
                        update_ui_state(siv, &state.project_path, state.current_file.as_ref())
                    {
                        Into::<Error>::into(e).to_dialog(siv);
                        return;
                    }

                    siv.pop_layer();
                })
                .dismiss_button("Cancel")
                .full_width()
                .with_name("rename"),
        );
    }
    Ok(())
}

/// Delete a file/directory(recursively)
fn delete_file(siv: &mut Cursive) -> Result<()> {
    if let Some(pos) = siv.screen_mut().find_layer_from_name("delete") {
        siv.screen_mut().remove_layer(pos);
    } else {
        let state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap();
        siv.add_layer(
            Dialog::new()
                .title("Delete")
                .padding_lrtb(1, 1, 1, 0)
                .content(path_input::new(
                    &state.project_path,
                    "delete_path".to_string(),
                    true,
                )?)
                .button("Confirm", |siv| {
                    let mut state = siv
                        .with_user_data(|state: &mut State| state.clone())
                        .unwrap();
                    let delete_path = siv
                        .call_on_name("delete_path_edit", |view: &mut EditView| {
                            PathBuf::from(view.get_content().to_string())
                        })
                        .unwrap();

                    if delete_path.is_dir() {
                        if let Err(e) = fs::remove_dir_all(&delete_path) {
                            Into::<Error>::into(e).to_dialog(siv);
                            return;
                        }
                    } else if let Err(e) = fs::remove_file(&delete_path) {
                        Into::<Error>::into(e).to_dialog(siv);
                        return;
                    }

                    state.remove_file(&delete_path);

                    siv.set_user_data(state.clone());

                    let current = if &delete_path
                        != state.current_file.as_ref().unwrap_or(&PathBuf::default())
                    {
                        state.current_file
                    } else {
                        None
                    };

                    if let Err(e) = update_ui_state(siv, &state.project_path, current.as_ref()) {
                        Into::<Error>::into(e).to_dialog(siv);
                        return;
                    }

                    if state.project_path == delete_path {
                        siv.pop_layer();
                        Into::<Error>::into(io::Error::new(
                            ErrorKind::NotFound,
                            "Couldn't find project. It got deleted",
                        ))
                        .to_dialog(siv);
                        return;
                    }

                    siv.pop_layer();
                })
                .dismiss_button("Cancel")
                .full_width()
                .with_name("delete"),
        );
    }
    Ok(())
}

/// Quits safely the app
pub fn quit(siv: &mut Cursive) -> Result<()> {
    let state = siv
        .with_user_data(|state: &mut State| state.clone())
        .unwrap();

    let edited_files = state
        .files_edited
        .into_iter()
        .filter(|(_, edited)| *edited)
        .map(|(path, _)| path)
        .collect::<Vec<PathBuf>>();

    if edited_files.is_empty() {
        siv.quit();
    } else {
        let mut layout =
            LinearLayout::vertical().child(TextView::new("You have unsaved changes in: "));
        for i in &edited_files {
            layout.add_child(TextView::new(i.to_string_lossy()));
        }

        let edited_files_for_save = edited_files.clone();
        siv.add_layer(
            Dialog::new()
                .content(layout)
                .button("Save", move |siv| {
                    for i in &edited_files_for_save {
                        let binding = &FileData::default();
                        let content = &state.files.get(i).unwrap_or(binding).str;
                        save(siv, Some((i, content))).handle(siv);
                    }
                    siv.quit();
                })
                .button("Dismiss", |siv| {
                    siv.pop_layer();
                    siv.quit();
                })
                .dismiss_button("Cancel Closing"),
        );
    }

    Ok(())
}
