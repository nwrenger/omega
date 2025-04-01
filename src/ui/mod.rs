//! Here are some general functions of updating the ui

pub mod edit_area;
pub mod file_tree;
pub mod path_input;
pub mod quick_access;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use cursive::{Cursive, Vec2};
use cursive_tree_view::TreeView;
use file_tree::{load_parent, TreeEntry};

use crate::{
    app::{EditorPanel, FileData, State, TreePanel},
    error::{Result, ResultExt},
};

use self::edit_area::{Cursor, EditArea};

/// Updates the ui accordingly to the paths
pub fn update_ui_state(
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

/// Open a file, reading from fs if needed, updating title and edit_area content/highlighting, updating state, ...
pub fn open_file(siv: &mut Cursive, file_to_open: &Path) -> Result<()> {
    let mut state = siv
        .with_user_data(|state: &mut State| state.clone())
        .unwrap_or_default();
    let file_to_open = file_to_open.canonicalize().unwrap_or_default();
    let extension = file_to_open
        .extension()
        .unwrap_or_default()
        .to_string_lossy();
    if state.get_file(&file_to_open).is_none() {
        let content = fs::read_to_string(file_to_open.clone())?;
        siv.call_on_name("editor", |edit_area: &mut EditArea| {
            edit_area.set_highlighting(&extension);
            edit_area.set_content(content.clone());
            edit_area.set_cursor(Cursor::default());
            edit_area.set_scroll(Vec2::zero());
            edit_area.enable();
        })
        .unwrap();

        siv.set_user_data(state.open_new_file(
            file_to_open.clone(),
            FileData {
                str: content,
                ..Default::default()
            },
        ));
    } else {
        state = State {
            current_file: Some(file_to_open.clone()),
            ..state
        };

        siv.call_on_name("editor", |edit_area: &mut EditArea| {
            edit_area.set_highlighting(&extension);
            edit_area.set_content(&state.get_current_file().unwrap().str);
            edit_area.set_cursor(state.get_current_file().unwrap().cursor);
            edit_area.set_scroll(state.get_current_file().unwrap().scroll_offset);
            edit_area.enable();
        })
        .unwrap();

        siv.set_user_data(state.clone());
    }

    // check if file has been added && update title accordingly
    update_title(siv, Some(&state), &file_to_open);

    Ok(())
}

/// Update the title of the editor panel including the current editing state via adding `*`
pub fn update_title(siv: &mut Cursive, state: Option<&State>, path: &Path) {
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let title = if let Some(state) = state {
        if state.is_file_edited(&path.to_path_buf()) {
            file_name + " *"
        } else {
            file_name
        }
    } else {
        file_name
    };

    siv.call_on_name("editor_title", |view: &mut EditorPanel| {
        view.set_title(title);
    })
    .unwrap();
}
