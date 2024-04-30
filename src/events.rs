use std::{
    fs::{self, OpenOptions},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{Dialog, EditView, LinearLayout, ListView, ScrollView, SelectView, TextView},
    Cursive, Vec2,
};
use cursive_tree_view::TreeView;

use crate::{
    app::{
        EditorPanel, FileData, State, TreePanel, PKG_AUTHORS, PKG_DESCRIPTION, PKG_LICENSE,
        PKG_NAME, PKG_REPOSITORY, PKG_VERSION,
    },
    error::{Error, Result, ResultExt},
    ui::{
        edit_area::{Cursor, EditArea},
        file_tree::{load_parent, TreeEntry},
        open_file, path_input, update_title,
    },
};

/// Shows all commands
pub fn info(siv: &mut Cursive) -> Result<()> {
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
                        .child("Infos", TextView::new("Esc"))
                        .child("Debugger", TextView::new("Ctrl + p"))
                        .child("Quitting", TextView::new("Ctrl + q"))
                        .child("Goto an already opened File", TextView::new("Ctrl + g"))
                        .child("Opening a new File/Project", TextView::new("Ctrl + o"))
                        .child("Creating a new File/Directory", TextView::new("Ctrl + n"))
                        .child("Renaming a File/Directory", TextView::new("Ctrl + r"))
                        .child("Deleting a File/Directory", TextView::new("Ctrl + d"))
                        .child("Saving File", TextView::new("Ctrl + s"))
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

/// Quits safely the app
pub fn quit(siv: &mut Cursive) -> Result<()> {
    let state = siv
        .with_user_data(|state: &mut State| state.clone())
        .unwrap();

    let edited_files = state
        .files_edited
        .into_iter() // Note the change to into_iter to consume the map
        .filter(|(_, edited)| *edited)
        .map(|(path, _)| path)
        .collect::<Vec<PathBuf>>(); // Now owns PathBuf instead of &PathBuf

    if edited_files.is_empty() {
        siv.quit();
    } else {
        let mut layout =
            LinearLayout::vertical().child(TextView::new("You have unsaved changes in: "));
        for i in &edited_files {
            layout.add_child(TextView::new(i.to_string_lossy()));
        }

        // Clone edited_files for use in the Save closure
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

/// Goto an opened file of the current project or more precise a file in
/// the state.files hashmap which is inside the current project directory
///
/// This wont reload any current state just open the current state of the specified file!
pub fn goto(siv: &mut Cursive) -> Result<()> {
    if let Some(pos) = siv.screen_mut().find_layer_from_name("goto") {
        siv.screen_mut().remove_layer(pos);
        Ok(())
    } else {
        let state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap();

        let mut filtered = state
            .files
            .iter()
            .filter(|p| p.0.starts_with(&state.project_path))
            .collect::<Vec<_>>();
        filtered.sort_by(|a, b| b.0.cmp(a.0));

        let opened = filtered
            .iter()
            .filter(|p| p.0.starts_with(&state.project_path))
            .map(|f| {
                f.0.canonicalize()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();

        siv.add_layer(
            Dialog::new()
                .title("Goto")
                .padding_lrtb(1, 1, 1, 0)
                .content(ScrollView::new(
                    SelectView::new()
                        .with_all_str(&opened)
                        .on_submit(move |siv, item: &String| {
                            let goto_file = &PathBuf::from(item);
                            if let Err(e) = open_file(siv, goto_file) {
                                Into::<Error>::into(e).to_dialog(siv);
                                return;
                            }
                            siv.pop_layer();
                        })
                        .selected(
                            opened
                                .iter()
                                .position(|p| {
                                    p == &state
                                        .clone()
                                        .current_file
                                        .unwrap_or_default()
                                        .canonicalize()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string()
                                })
                                .unwrap_or_default(),
                        ),
                ))
                .dismiss_button("Cancel")
                .full_width()
                .with_name("goto"),
        );
        Ok(())
    }
}

/// Opens a new file/project
///
/// This wont override current edits made to files so it can be seen as a `save operation`
///
/// Also notable is that this will reload state so the current file tree, the preferred way
/// to move through all your current opened files without using the file tree is using
/// `goto` (`Ctrl` + `g`)
pub fn open(siv: &mut Cursive) -> Result<()> {
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
                        Error::FileOpen("Path doesn't exists".to_string()).to_dialog(siv);
                        return;
                    };

                    if let Err(e) = open_paths(siv, &project_path, current_file.as_ref()) {
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

/// Updates the ui accordingly to the paths
pub fn open_paths(
    siv: &mut Cursive,
    project_path: &Path,
    current_file: Option<&PathBuf>,
) -> Result<()> {
    let project_path = &project_path.canonicalize().unwrap_or_default();
    if let Some(current_file) = current_file {
        open_file(siv, current_file).handle(siv);
    } else if project_path.exists() {
        siv.call_on_name("editor", |edit_area: &mut EditArea| {
            edit_area.set_content(' ');
            edit_area.set_cursor(Cursor::default());
            edit_area.set_scroll(Vec2::zero());
            edit_area.disable();
        })
        .unwrap();
        siv.call_on_name("editor_title", |view: &mut EditorPanel| view.set_title(""))
            .unwrap();
    }
    if project_path.exists() {
        siv.call_on_name("tree_title", |view: &mut TreePanel| {
            view.get_inner_mut().set_title(
                project_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            );
        })
        .unwrap();

        let mut state = siv
            .with_user_data(|state: &mut State| state.clone())
            .unwrap_or_default();

        siv.call_on_name("tree", |tree: &mut TreeView<TreeEntry>| {
            load_parent(tree, project_path);
        });

        siv.set_user_data(state.open_new_project(project_path, current_file));
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "An invalid/not existing directory/file was specified",
        )
        .into());
    }

    Ok(())
}

/// Creates a new file
pub fn new(siv: &mut Cursive) -> Result<()> {
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
pub fn rename(siv: &mut Cursive) -> Result<()> {
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
                    .child(
                        LinearLayout::vertical()
                            .child(TextView::new("From"))
                            .child(path_input::new(
                                &state.project_path,
                                "from_rename_path".to_string(),
                                true,
                            )?)
                            .full_width(),
                    )
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
                        open_paths(siv, &state.project_path, state.current_file.as_ref())
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
pub fn delete(siv: &mut Cursive) -> Result<()> {
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

                    if let Err(e) = open_paths(siv, &state.project_path, current.as_ref()) {
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
