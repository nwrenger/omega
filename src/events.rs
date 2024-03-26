use std::{env, fs, path::PathBuf};

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{
        Dialog, EditView, LinearLayout, ListView, NamedView, OnEventView, Panel, ResizedView,
        ScrollView, SelectView, TextArea, TextView,
    },
    Cursive,
};

use crate::{
    error::{Error, Result},
    State, PKG_AUTHORS, PKG_DESCRIPTION, PKG_LICENSE, PKG_NAME, PKG_REPOSITORY, PKG_VERSION,
};

/// Shows all commands
pub fn info(s: &mut Cursive) -> Result<()> {
    if let Some(pos) = s.screen_mut().find_layer_from_name("info") {
        s.screen_mut().remove_layer(pos);
    } else {
        s.add_layer(
            Dialog::new()
                .title(format!("{PKG_NAME} - Info"))
                .padding_lrtb(1, 1, 1, 0)
                .dismiss_button("Close")
                .content(
                    ListView::new()
                        // general info
                        .child(
                            "Not existing Files",
                            TextView::new("are indicated through a '*' in the header"),
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
                        .child("Infos", TextView::new("Ctrl + z"))
                        .child("Debugger", TextView::new("Ctrl + d"))
                        .child("Quitting", TextView::new("Ctrl + q"))
                        .child("Force Quitting", TextView::new("Ctrl + f"))
                        .child("Saving File", TextView::new("Ctrl + s"))
                        .child("Opening a File", TextView::new("Ctrl + o"))
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
pub fn quit(s: &mut Cursive) -> Result<()> {
    if save(s)? {
        s.quit();
    }

    Ok(())
}

/// Save current progress + Handling Title
pub fn save(s: &mut Cursive) -> Result<bool> {
    if let Some(pos) = s.screen_mut().find_layer_from_name("save") {
        s.screen_mut().remove_layer(pos);
        Ok(false)
    } else {
        let state = s.with_user_data(|state: &mut State| state.clone()).unwrap();
        if state.file_path.is_none() {
            let path_str = env::current_dir()?.to_string_lossy().to_string();
            s.add_layer(
                Dialog::new()
                    .title("Save As")
                    .padding_lrtb(1, 1, 1, 0)
                    .content(path_input(&path_str, "filepath".to_string(), false)?)
                    .button("Save", {
                        move |s: &mut Cursive| {
                            let new_path = s
                                .call_on_name("filepath_edit", |view: &mut EditView| {
                                    PathBuf::from(view.get_content().to_string())
                                })
                                .unwrap_or_default();

                            let content = s
                                .call_on_name("editor", |view: &mut TextArea| {
                                    view.get_content().to_string()
                                })
                                .unwrap_or_default();

                            if new_path.is_file() {
                                if !new_path.exists() {
                                    match fs::write(new_path.clone(), content) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            Into::<Error>::into(e).to_dialog(s);
                                            return;
                                        }
                                    }
                                } else {
                                    Error::AlreadyExists.to_dialog(s);
                                    return;
                                }
                            } else {
                                Error::FileOpen.to_dialog(s);
                            }

                            s.call_on_name(
                                "title_text",
                                |view: &mut Panel<
                                    OnEventView<ResizedView<ScrollView<NamedView<TextArea>>>>,
                                >| {
                                    view.set_title(new_path.to_string_lossy())
                                },
                            )
                            .unwrap_or_default();

                            s.set_user_data(State {
                                file_path: Some(new_path.clone()),
                            });

                            s.pop_layer();
                        }
                    })
                    .dismiss_button("Cancel")
                    .full_width()
                    .with_name("save"),
            );
            Ok(false)
        } else {
            let file_path = state.file_path.unwrap_or_default();
            let content = s
                .call_on_name("editor", |view: &mut TextArea| {
                    view.get_content().to_string()
                })
                .unwrap_or_default();

            if file_path.is_file() {
                fs::write(file_path.clone(), content)?;
            } else {
                return Err(Error::FileOpen);
            }

            s.call_on_name(
                "title_text",
                |view: &mut Panel<OnEventView<ResizedView<ScrollView<NamedView<TextArea>>>>>| {
                    view.set_title(file_path.to_string_lossy())
                },
            )
            .unwrap_or_default();

            Ok(true)
        }
    }
}

/// Opens a new file safely with saving the current file
pub fn open(s: &mut Cursive) -> Result<()> {
    if let Some(pos) = s.screen_mut().find_layer_from_name("open") {
        s.screen_mut().remove_layer(pos);
        Ok(())
    } else {
        let state = s.with_user_data(|state: &mut State| state.clone()).unwrap();

        let path_str = if let Some(path) = state.file_path {
            path.to_string_lossy().to_string()
        } else {
            env::current_dir()?.to_string_lossy().to_string()
        };

        s.add_layer(
            Dialog::new()
                .title("Open")
                .padding_lrtb(1, 1, 1, 0)
                .content(
                    LinearLayout::vertical()
                        .child(TextView::new(
                            "Make sure that you've saved your progress via Ctrl + s",
                        ))
                        .child(TextView::new(" "))
                        .child(path_input(&path_str, "open_new_path".to_string(), true)?),
                )
                .button("Open", move |s| {
                    let new_path = s
                        .call_on_name("open_new_path_edit", |view: &mut EditView| {
                            PathBuf::from(view.get_content().to_string())
                        })
                        .unwrap_or_default();

                    match fs::read_to_string(new_path.clone()) {
                        Ok(content) => {
                            s.call_on_name("editor", |text_area: &mut TextArea| {
                                text_area.set_content(content);
                            })
                            .unwrap_or_default();
                        }
                        Err(e) => {
                            Into::<Error>::into(e).to_dialog(s);
                            return;
                        }
                    };

                    s.call_on_name(
                        "title_text",
                        |view: &mut Panel<
                            OnEventView<ResizedView<ScrollView<NamedView<TextArea>>>>,
                        >| { view.set_title(new_path.to_string_lossy()) },
                    )
                    .unwrap_or_default();

                    s.set_user_data(State {
                        file_path: Some(new_path),
                    });

                    s.pop_layer();
                })
                .dismiss_button("Cancel")
                .full_width()
                .with_name("open"),
        );

        Ok(())
    }
}

/// Creates a filepath input view
///
/// The name for the EditView is `name` + `_edit`, for the SelectView `name` + `_select`
fn path_input(path: &String, name: String, files: bool) -> Result<LinearLayout> {
    let mut select = SelectView::<String>::new();
    select.add_all_str(get_paths(path, files).unwrap_or_default());

    let view_name = name.clone() + "_edit";
    let select_name = name.clone() + "_select";
    let select_name2 = name.clone() + "_select";

    Ok(LinearLayout::vertical()
        .child(
            EditView::new()
                .content(path)
                .on_edit(move |s, new_path, _| {
                    s.call_on_all_named(&select_name, |view: &mut SelectView| {
                        view.clear();
                        view.add_all_str(
                            get_paths(&new_path.to_string(), files).unwrap_or_default(),
                        );
                    });
                })
                .with_name(name.to_string() + "_edit"),
        )
        .child(ScrollView::new(
            select
                .on_select(move |s, new_path: &String| {
                    s.call_on_all_named(&view_name, |s: &mut EditView| {
                        s.set_content(new_path);
                    });
                    s.call_on_all_named(&select_name2, |view: &mut SelectView| {
                        view.clear();
                        view.add_all_str(
                            get_paths(&new_path.to_string(), files).unwrap_or_default(),
                        );
                    });
                })
                .with_name(name.to_string() + "_select"),
        )))
}

/// Getting all paths by a path
fn get_paths(path: &String, files: bool) -> Result<Vec<String>> {
    if let Ok(entries) = fs::read_dir(path) {
        entries
            .filter_map(|entry_result| {
                let entry = match entry_result {
                    Ok(entry) => entry,
                    Err(e) => return Some(Err(e.into())),
                };

                match entry.file_type() {
                    Ok(file_type) => {
                        if files || file_type.is_dir() {
                            Some(Ok(entry.path().to_string_lossy().to_string()))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e.into())),
                }
            })
            .collect()
    } else {
        Err(Error::FileOpen)
    }
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
