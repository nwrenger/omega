use std::{env, fs, path::PathBuf};

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{
        Dialog, EditView, ListView, NamedView, OnEventView, Panel, ResizedView, ScrollView,
        TextArea, TextView,
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
                    .content(
                        EditView::new()
                            .content(path_str.clone())
                            .with_name("filepath"),
                    )
                    .button("Save", {
                        move |s: &mut Cursive| {
                            let new_path = s
                                .call_on_name("filepath", |view: &mut EditView| {
                                    PathBuf::from(view.get_content().to_string())
                                })
                                .unwrap_or_default();

                            let content = s
                                .call_on_name("editor", |view: &mut TextArea| {
                                    view.get_content().to_string()
                                })
                                .unwrap_or_default();

                            match fs::write(new_path.clone(), content) {
                                Ok(_) => {}
                                Err(e) => {
                                    Into::<Error>::into(e).to_dialog(s);
                                    return;
                                }
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
                    .fixed_width(path_str.len() * 2)
                    .with_name("save"),
            );
            Ok(false)
        } else {
            let content = s
                .call_on_name("editor", |view: &mut TextArea| {
                    view.get_content().to_string()
                })
                .unwrap_or_default();

            fs::write(
                state.file_path.as_ref().unwrap_or(&PathBuf::default()),
                content,
            )?;

            s.call_on_name(
                "title_text",
                |view: &mut Panel<OnEventView<ResizedView<ScrollView<NamedView<TextArea>>>>>| {
                    view.set_title(
                        state
                            .file_path
                            .as_ref()
                            .unwrap_or(&PathBuf::default())
                            .to_string_lossy(),
                    )
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
        let path_str = env::current_dir()?.to_string_lossy().to_string();
        s.add_layer(
            Dialog::new()
                .title("Open")
                .padding_lrtb(1, 1, 1, 0)
                .content(
                    ListView::new()
                        .child(
                            "Make sure that",
                            TextView::new("you've saved your progress via Ctrl + s"),
                        )
                        .delimiter()
                        .child(
                            "Path",
                            EditView::new()
                                .content(path_str.clone())
                                .with_name("open_new_path"),
                        ),
                )
                .button("Open", move |s| {
                    let new_path = s
                        .call_on_name("open_new_path", |view: &mut EditView| {
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

                    s.pop_layer();
                })
                .dismiss_button("Cancel")
                .fixed_width(path_str.len() * 2)
                .with_name("open"),
        );
        Ok(())
    }
}

/// Copies the line where the cursor currently is
pub fn copy(text_area: &mut TextArea) -> Result<()> {
    let mut clipboard = clippers::Clipboard::get();

    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, _) = get_cursor_line_info(&content, cursor_pos);

    let lines: Vec<&str> = content.split('\n').collect();

    clipboard.write_text(lines[current_line].to_string() + "\n")?;

    Ok(())
}

/// Pasts the current clipboard
pub fn paste(text_area: &mut TextArea) -> Result<()> {
    let mut clipboard = clippers::Clipboard::get();

    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, cursor_in_line) = get_cursor_line_info(&content, cursor_pos);

    let mut lines: Vec<&str> = content.split('\n').collect();
    if let Some(clippers::ClipperData::Text(text)) = clipboard.read() {
        let split = lines[current_line].split_at(cursor_in_line);
        let inserted_line = split.0.to_string() + text.as_str() + split.1;
        lines[current_line] = inserted_line.as_str();

        let new_content: String = lines.join("\n");
        text_area.set_content(new_content);

        text_area.set_cursor(cursor_pos + text.to_string().len());
    }

    Ok(())
}

/// Cuts the line where the cursor currently is
pub fn cut(text_area: &mut TextArea) -> Result<()> {
    let mut clipboard = clippers::Clipboard::get();

    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let (current_line, _) = get_cursor_line_info(&content, cursor_pos);

    let mut lines: Vec<&str> = content.split('\n').collect();
    clipboard
        .write_text(lines[current_line].to_string() + "\n")
        .unwrap();
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
