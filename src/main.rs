use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use cursive::event::Event;
use cursive::event::Key;
use cursive::theme::Theme;
use cursive::traits::*;
use cursive::views::NamedView;
use cursive::views::Panel;
use cursive::views::ResizedView;
use cursive::views::ScrollView;
use cursive::views::{Dialog, EditView, OnEventView, TextArea};

fn main() {
    let mut siv = cursive::default();

    let args: Vec<String> = env::args().collect();

    let file_path = if args.len() > 1 {
        Rc::new(RefCell::new(Some(PathBuf::from(&args[1]))))
    } else {
        Rc::new(RefCell::new(None))
    };

    let file_content = if file_path.borrow().is_some()
        && file_path
            .borrow()
            .as_ref()
            .unwrap_or(&PathBuf::default())
            .exists()
    {
        Some(fs::read_to_string(file_path.borrow().clone().unwrap_or_default()).unwrap_or_default())
    } else {
        None
    };

    let text_view = Panel::new(
        OnEventView::new(
            TextArea::new()
                .content(file_content.clone().unwrap_or_default())
                .with_name("editor")
                .scrollable()
                .full_screen(),
        )
        // todo: make this somehow work
        // .on_pre_event(EventTrigger::any(), {
        //     let file_path = Rc::clone(&file_path);
        //     move |s| {
        //         s.call_on_name(
        //             "title_text",
        //             |view: &mut Panel<
        //                 OnEventView<ResizedView<ScrollView<NamedView<TextArea>>>>,
        //             >| {
        //                 view.set_title(
        //                     file_path
        //                         .borrow()
        //                         .as_ref()
        //                         .unwrap_or(&PathBuf::default())
        //                         .to_string_lossy()
        //                         + " *",
        //                 )
        //             },
        //         )
        //         .unwrap_or_default();
        //     }
        // })
        .on_pre_event(Event::CtrlChar('s'), {
            let file_path = Rc::clone(&file_path);
            move |s| {
                if file_path.borrow().is_none() {
                    let path_str = env::current_dir()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string();

                    s.add_layer(
                        Dialog::new()
                            .title("Save As")
                            .padding_lrtb(1, 1, 1, 0)
                            .content(EditView::new().content(path_str).with_name("filepath"))
                            .button("Save", {
                                let file_path = Rc::clone(&file_path);
                                move |s| {
                                    let new_path = s
                                        .call_on_name("filepath", |view: &mut EditView| {
                                            PathBuf::from(view.get_content().to_string())
                                        })
                                        .unwrap_or_default();
                                    *file_path.borrow_mut() = Some(new_path.clone());

                                    let content = s
                                        .call_on_name("editor", |view: &mut TextArea| {
                                            view.get_content().to_string()
                                        })
                                        .unwrap_or_default();

                                    fs::write(&new_path, content).unwrap_or_default();

                                    s.call_on_name(
                                        "title_text",
                                        |view: &mut Panel<
                                            OnEventView<
                                                ResizedView<ScrollView<NamedView<TextArea>>>,
                                            >,
                                        >| {
                                            view.set_title(
                                                file_path
                                                    .borrow()
                                                    .as_ref()
                                                    .unwrap_or(&PathBuf::default())
                                                    .to_string_lossy(),
                                            )
                                        },
                                    )
                                    .unwrap_or_default();

                                    s.pop_layer();
                                }
                            })
                            .button("Cancel", |s| {
                                s.call_on_name(
                                    "title_text",
                                    |view: &mut Panel<
                                        OnEventView<ResizedView<ScrollView<NamedView<TextArea>>>>,
                                    >| { view.set_title(" *") },
                                )
                                .unwrap_or_default();
                                s.pop_layer();
                            })
                            .full_width(),
                    );
                } else {
                    let content = s
                        .call_on_name("editor", |view: &mut TextArea| {
                            view.get_content().to_string()
                        })
                        .unwrap_or_default();

                    fs::write(
                        file_path.borrow().as_ref().unwrap_or(&PathBuf::default()),
                        content,
                    )
                    .unwrap_or_default();

                    s.call_on_name(
                        "title_text",
                        |view: &mut Panel<
                            OnEventView<ResizedView<ScrollView<NamedView<TextArea>>>>,
                        >| {
                            view.set_title(
                                file_path
                                    .borrow()
                                    .as_ref()
                                    .unwrap_or(&PathBuf::default())
                                    .to_string_lossy(),
                            )
                        },
                    )
                    .unwrap_or_default();
                }
            }
        })
        .on_pre_event(Event::Alt(Key::Up), |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                move_line(&mut text_area, Direction::Up);
            }
        })
        .on_pre_event(Event::Alt(Key::Down), |s| {
            if let Some(mut text_area) = s.find_name::<TextArea>("editor") {
                move_line(&mut text_area, Direction::Down);
            }
        }),
    )
    .title(
        file_path
            .borrow()
            .clone()
            .unwrap_or_default()
            .to_string_lossy()
            + if file_content.is_none() { " *" } else { "" },
    )
    .with_name("title_text");

    siv.add_fullscreen_layer(text_view);
    // theme
    siv.set_theme(Theme::terminal_default());

    siv.run();
}

#[derive(PartialEq)]
enum Direction {
    Up,
    Down,
}

fn move_line(text_area: &mut TextArea, direction: Direction) {
    let content = text_area.get_content().to_string();
    let cursor_pos = text_area.cursor();

    let mut lines: Vec<&str> = content.split('\n').collect();
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

    if (current_line == 0 && direction == Direction::Up)
        || (current_line == lines.len() - 1 && direction == Direction::Down)
    {
        return;
    }

    let line_to_move = lines.remove(current_line);
    match direction {
        Direction::Up => lines.insert(current_line - 1, line_to_move),
        Direction::Down => lines.insert(current_line + 1, line_to_move),
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
}
