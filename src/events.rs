use std::{
    fs::{self, OpenOptions},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{Dialog, EditView, LinearLayout, ListView, TextArea, TextView},
    Cursive,
};
use cursive_tree_view::{Placement, TreeView};

use crate::{
    app::{
        EditorPanel, State, TreePanel, PKG_AUTHORS, PKG_DESCRIPTION, PKG_LICENSE, PKG_NAME,
        PKG_REPOSITORY, PKG_VERSION,
    },
    error::{Error, Result},
    ui::{
        file_tree::{expand_tree, TreeEntry},
        path_input,
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
                        .child("Force Quitting", TextView::new("Ctrl + f"))
                        .child("Opening a new File/Project", TextView::new("Ctrl + o"))
                        .child("Creating a new File/Directory", TextView::new("Ctrl + n"))
                        .child("Renaming a File/Directory", TextView::new("Ctrl + r"))
                        .child("Deleting a File/Directory", TextView::new("Ctrl + d"))
                        .child("Saving File", TextView::new("Ctrl + siv"))
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
    if save(siv).is_ok() {
        siv.quit();
    }

    Ok(())
}

/// Opens a new file safely with saving the current file
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
                .content(
                    LinearLayout::vertical()
                        .child(TextView::new(
                            "Make sure that you've saved your progress via Ctrl + siv",
                        ))
                        .child(TextView::new(" "))
                        .child(path_input::new(
                            &state.project_path,
                            "open_new_path".to_string(),
                            true,
                        )?),
                )
                .button("Open", move |siv| {
                    let inc_path = siv
                        .call_on_name("open_new_path_edit", |view: &mut EditView| {
                            PathBuf::from(view.get_content().to_string())
                        })
                        .unwrap();

                    let mut file_path = None;
                    let mut project_path = PathBuf::from("/");

                    if inc_path.is_file() {
                        file_path = Some(inc_path.clone());
                        project_path = PathBuf::from(inc_path.parent().unwrap_or(Path::new("/")));
                    } else if inc_path.is_dir() {
                        project_path = inc_path;
                    }

                    if let Err(e) = open_paths(siv, &project_path, file_path.as_ref()) {
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
    project_path: &PathBuf,
    file_path: Option<&PathBuf>,
) -> Result<()> {
    if let Some(file_path) = file_path {
        match fs::read_to_string(file_path) {
            Ok(content) => {
                siv.call_on_name("editor", |text_area: &mut TextArea| {
                    text_area.set_content(content);
                    text_area.enable();
                })
                .unwrap();
                siv.call_on_name("editor_title", |view: &mut EditorPanel| {
                    view.set_title(file_path.to_string_lossy())
                })
                .unwrap();
            }
            Err(e) => {
                return Err(e.into());
            }
        };
    } else if project_path.exists() {
        siv.call_on_name("editor", |text_area: &mut TextArea| {
            text_area.set_content(' ');
            text_area.set_cursor(0);
            text_area.disable();
        })
        .unwrap();
        siv.call_on_name("editor_title", |view: &mut EditorPanel| view.set_title(""))
            .unwrap();
    }
    if project_path.exists() {
        siv.call_on_name("tree_title", |view: &mut TreePanel| {
            view.get_inner_mut()
                .set_title(project_path.to_string_lossy())
        })
        .unwrap();

        siv.call_on_name("tree", |tree: &mut TreeView<TreeEntry>| {
            tree.clear();
            expand_tree(tree, 0, project_path, Placement::Before)
        });

        siv.set_user_data(State {
            file_path: file_path.cloned(),
            project_path: project_path.to_path_buf(),
        });
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
                            tree.clear();
                            expand_tree(tree, 0, &state.project_path, Placement::Before)
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
                            tree.clear();
                            expand_tree(tree, 0, &state.project_path, Placement::Before)
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
        let layout = LinearLayout::horizontal()
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
            );
        siv.add_layer(
            Dialog::new()
                .title("Rename")
                .padding_lrtb(1, 1, 1, 0)
                .content(LinearLayout::vertical()
                    .child(TextView::new("Make sure to save you progress via Ctrl + siv before rigorously moving files!"))
                    .child(TextView::new(" "))
                    .child(layout)
                )
                .button("Confirm", |siv| {
                    let state = siv.with_user_data(|state: &mut State| state.clone()).unwrap();
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

                    siv.call_on_name("tree", |tree: &mut TreeView<TreeEntry>| {
                        tree.clear();
                        expand_tree(tree, 0, &state.project_path, Placement::Before)
                    });

                    if from != to && state.project_path == from {
                        siv.pop_layer();
                        Into::<Error>::into(io::Error::new(ErrorKind::NotFound, "Couldn't find project. It got moved")).to_dialog(siv);
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
                    let state = siv
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

                    siv.call_on_name("tree", |tree: &mut TreeView<TreeEntry>| {
                        tree.clear();
                        expand_tree(tree, 0, &state.project_path, Placement::Before)
                    });

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
pub fn save(siv: &mut Cursive) -> Result<()> {
    let state = siv
        .with_user_data(|state: &mut State| state.clone())
        .unwrap();
    if let Some(file_path) = state.file_path {
        let content = siv
            .call_on_name("editor", |view: &mut TextArea| {
                view.get_content().to_string()
            })
            .unwrap();

        fs::write(file_path.clone(), content)?;

        siv.call_on_name("editor_title", |view: &mut EditorPanel| {
            view.set_title(file_path.to_string_lossy())
        })
        .unwrap();
    }
    Ok(())
}

/// Copies the line where the cursor currently is
pub fn copy(text_area: &mut TextArea) -> Result<()> {
    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, _) = get_cursor_line_info(&content, cursor_pos);

    let lines: Vec<&str> = content.split('\n').collect();

    crate::clipboard::set_content(lines[current_line].to_string() + "\n")?;

    Ok(())
}

/// Pasts the current clipboard
pub fn paste(text_area: &mut TextArea) -> Result<()> {
    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, cursor_in_line) = get_cursor_line_info(&content, cursor_pos);

    let mut lines: Vec<&str> = content.split('\n').collect();
    let text = crate::clipboard::get_content()?;
    let split = lines[current_line].split_at(cursor_in_line);
    let inserted_line = split.0.to_string() + text.as_str() + split.1;
    lines[current_line] = inserted_line.as_str();

    let new_content: String = lines.join("\n");
    text_area.set_content(new_content);

    text_area.set_cursor(cursor_pos + text.to_string().len());

    Ok(())
}

/// Cuts the line where the cursor currently is
pub fn cut(text_area: &mut TextArea) -> Result<()> {
    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, _) = get_cursor_line_info(&content, cursor_pos);

    let mut lines: Vec<&str> = content.split('\n').collect();
    crate::clipboard::set_content(lines[current_line].to_string() + "\n")?;
    lines.remove(current_line);

    let new_content: String = lines.join("\n");
    text_area.set_content(new_content);

    Ok(())
}

/// Implements the tabulator
pub fn tabulator(text_area: &mut TextArea, ident: bool) -> Result<()> {
    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, _) = get_cursor_line_info(&content, cursor_pos);
    let mut lines: Vec<&str> = content.split('\n').collect();
    let tab_size = 4;

    if ident {
        let str_to_add = " ".repeat(tab_size);
        let new_line = str_to_add + lines[current_line];

        text_area.set_cursor(cursor_pos + tab_size);

        lines[current_line] = &new_line;
        let new_content: String = lines.join("\n");
        text_area.set_content(new_content);
    } else {
        let str_to_add = " ".repeat(tab_size);
        let new_line = lines[current_line].replacen(&str_to_add, "", 1);

        if lines[current_line] != new_line {
            text_area.set_cursor(cursor_pos - tab_size);
        }

        lines[current_line] = &new_line;
        let new_content: String = lines.join("\n");
        text_area.set_content(new_content);
    };

    Ok(())
}

/// Directions for further functions
#[derive(PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Moves the line withing the cursor in the specified direction
pub fn move_line(text_area: &mut TextArea, direction: Direction) -> Result<()> {
    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, cursor_in_line) = get_cursor_line_info(&content, cursor_pos);

    let mut lines: Vec<&str> = content.split('\n').collect();

    if (current_line == 0 && direction == Direction::Up)
        || (current_line == lines.len() - 1 && direction == Direction::Down)
    {
        return Ok(());
    }

    let line_to_move = lines.remove(current_line);
    match direction {
        Direction::Up => lines.insert(current_line - 1, line_to_move),
        Direction::Down => lines.insert(current_line + 1, line_to_move),
        _ => {}
    }

    let new_content: String = lines.join("\n");
    text_area.set_content(new_content);

    let new_cursor_pos = if direction == Direction::Up && current_line > 0 {
        lines
            .iter()
            .take(current_line - 1)
            .map(|line| line.len() + 1)
            .sum::<usize>()
            + cursor_in_line
    } else {
        lines
            .iter()
            .take(current_line + (if direction == Direction::Down { 1 } else { 0 }))
            .map(|line| line.len() + 1)
            .sum::<usize>()
            + cursor_in_line
    };

    text_area.set_cursor(new_cursor_pos);

    Ok(())
}

/// Move cursor to the start or end of the current line
pub fn move_cursor_end(text_area: &mut TextArea, direction: Direction) -> Result<()> {
    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, _) = get_cursor_line_info(&content, cursor_pos);

    let lines: Vec<&str> = content.split('\n').collect();
    match direction {
        Direction::Left => {
            let new_cursor_pos = lines
                .iter()
                .take(current_line)
                .map(|line| line.len() + 1)
                .sum::<usize>();
            text_area.set_cursor(new_cursor_pos);
        }
        Direction::Right => {
            let new_cursor_pos = if current_line < lines.len() {
                lines
                    .iter()
                    .take(current_line + 1)
                    .map(|line| line.len() + 1)
                    .sum::<usize>()
                    - 1
            } else {
                content.len()
            };
            text_area.set_cursor(new_cursor_pos);
        }
        _ => {}
    }

    Ok(())
}

/// Returns the current line number and the cursor's position within that line
fn get_cursor_line_info(content: &str, cursor_pos: usize) -> (usize, usize) {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut current_line = 0;
    let mut cursor_in_line = 0;
    let mut count = 0;

    for (i, line) in lines.iter().enumerate() {
        let line_len = line.len() + 1;
        if count + line_len > cursor_pos {
            current_line = i;
            cursor_in_line = cursor_pos - count;
            break;
        }
        count += line_len;
    }

    (current_line, cursor_in_line)
}
