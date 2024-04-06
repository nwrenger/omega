pub mod edit_area;
pub mod file_tree;
pub mod path_input;

// Here are some general functions of updating the ui

use std::{fs, path::Path};

use cursive::Cursive;

use crate::{
    app::{EditorPanel, FileData, State},
    error::Result,
};

use self::edit_area::EditArea;

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
            edit_area.enable();
        })
        .unwrap();

        siv.set_user_data(state.open_new_file(file_to_open.clone(), FileData { str: content }));
    } else {
        state = State {
            current_file: Some(file_to_open.clone()),
            ..state
        };

        siv.call_on_name("editor", |edit_area: &mut EditArea| {
            edit_area.set_highlighting(&extension);
            edit_area.set_content(&state.get_current_file().unwrap().str);
            edit_area.enable();
        })
        .unwrap();

        siv.set_user_data(state.clone());
    }

    // check if file has been added && update title accordingly
    update_title(siv, &state, &file_to_open);

    Ok(())
}

/// Update the title of the editor panel including the current editing state via adding `*`
pub fn update_title(siv: &mut Cursive, state: &State, path: &Path) {
    let title = if state.is_current_file_edited() {
        format!(
            "{} *",
            path.file_name().unwrap_or_default().to_string_lossy()
        )
    } else {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    };

    siv.call_on_name("editor_title", |view: &mut EditorPanel| {
        view.set_title(title);
    })
    .unwrap();
}
