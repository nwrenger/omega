pub mod edit_area;
pub mod file_tree;
pub mod path_input;

// Here are some general functions of updating the ui

use std::{fs, path::Path};

use cursive::{Cursive, Vec2};

use crate::{
    app::{EditorPanel, FileData, State},
    error::Result,
};

use self::edit_area::{Cursor, EditArea};

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
