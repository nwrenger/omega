//! Improved Multi-lines text editor.
//!
//! A `EditArea` will attempt to grow vertically and horizontally
//! dependent on the content.  Wrap it in a `ResizedView` to
//! constrain its size.
//!
//! # Examples
//!
//! ```
//! use cursive_core::traits::{Nameable, Resizable};
//! use cursive_core::views::EditArea;
//!
//! let edit_area = EditArea::new()
//!     .content("Write description here...")
//!     .with_name("edit_area")
//!     .fixed_width(30)
//!     .min_height(5);
//! ```

use cursive::{
    direction::Direction,
    event::{Callback, Event, EventResult, Key, MouseEvent},
    impl_enabled,
    reexports::log::error,
    theme::{BaseColor, Color, ColorStyle, Effect, PaletteColor, PaletteStyle, Style},
    utils::{markup::StyledString, span::SpannedString},
    view::CannotFocus,
    Cursive, Printer, Rect, Vec2, View, With, XY,
};
use cursive::{
    impl_scroller,
    view::{scroll, ScrollStrategy},
};
use ropey::{Rope, RopeSlice};
use std::{
    cmp::{max, min},
    sync::Arc,
};
use syntect::{
    highlighting::Theme,
    parsing::{SyntaxReference, SyntaxSet},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Closure type for callbacks when something happens, for example the content is modified.
///
/// Arguments are the `Cursive`, current content of the input and cursor
/// position
pub type OnChange = dyn Fn(&mut Cursive, &Rope, Vec2, Cursor) + Send + Sync;

/// Computes how many characters (from the start of `s`) take up at most `max_width` terminal columns.
fn grapheme_prefix_length(s: &str, max_width: usize) -> usize {
    let mut current_width = 0;
    let mut char_count = 0;
    for grapheme in s.graphemes(true) {
        let grapheme_width = UnicodeWidthStr::width(grapheme);
        if current_width + grapheme_width > max_width {
            break;
        }
        current_width += grapheme_width;
        char_count += grapheme.chars().count();
    }
    char_count
}

/// Swaps `i`-th with `j`-th line in `rope`
fn swap_lines(rope: &mut Rope, i: usize, j: usize) {
    if i == j {
        return;
    }
    let orig_num_lines = rope.len_lines();
    let (i, j) = if i < j { (i, j) } else { (j, i) };

    let start_i = rope.line_to_char(i);
    let end_i = rope.line_to_char(i + 1);
    let start_j = rope.line_to_char(j);
    let end_j = rope.line_to_char(j + 1);

    let line_i = rope.slice(start_i..end_i).to_string();
    let mut line_j = rope.slice(start_j..end_j).to_string();

    if j == orig_num_lines - 1 && !line_j.ends_with('\n') {
        line_j.push('\n');
    }

    rope.remove(start_j..end_j);
    rope.remove(start_i..end_i);

    rope.insert(start_i, &line_j);
    rope.insert(start_i + line_j.chars().count(), &line_i);
}

/// Checks for special character for `grapheme` and returns a displayable version if available.
fn special_character(grapheme: &str) -> Option<&str> {
    match grapheme {
        "\t" => Some("⇥"),
        _ => None,
    }
}

/// The cursor offset
#[derive(Clone, Copy, Debug, Default)]
pub struct Cursor {
    /// Vertical rows from top, rows are separated with a `\n`
    pub row: usize,
    /// From left to right
    pub column: usize,
    /// Byte offset of the currently selected text
    pub byte_offset: usize,
    /// Character offset of the currently selected text
    pub char_offset: usize,
}

pub struct EditArea {
    // Content Buffer.
    content: Rope,

    /// Index of the longest line
    max_line_index: usize,

    /// Width of the longest line
    max_content_width: usize,

    /// Syntax Set
    syntax: SyntaxSet,

    /// Current Theme for highlighting
    theme: Theme,

    /// Specified through file extension, the applied highlighting
    synref: SyntaxReference,

    /// When `false`, we don't take any input.
    enabled: bool,

    /// Callback when the cursor is moved.
    ///
    /// Will be called with the current content and the cursor position.
    on_interact: Option<Arc<OnChange>>,

    /// Callback when the content is modified.
    ///
    /// Will be called with the current content and the cursor position.
    on_scroll: Option<Arc<OnChange>>,

    /// Callback when the content is modified.
    ///
    /// Will be called with the current content and the cursor position.
    on_edit: Option<Arc<OnChange>>,

    /// Base for scrolling features
    scroll_core: scroll::Core,

    /// Cursor offset view the `struct::Cursor` for further details
    cursor: Cursor,
}

impl_scroller!(EditArea::scroll_core);

impl EditArea {
    impl_enabled!(self.enabled);

    /// Creates a new, empty EditArea with a specified theme.
    pub fn new(theme: &Theme) -> Self {
        EditArea {
            content: Rope::new(),
            max_line_index: 0,
            max_content_width: 0,
            syntax: SyntaxSet::load_defaults_newlines(),
            theme: theme.to_owned(),
            synref: SyntaxSet::load_defaults_newlines()
                .find_syntax_plain_text()
                .clone(),
            enabled: true,
            on_interact: None,
            on_scroll: None,
            on_edit: None,
            scroll_core: scroll::Core::new(),
            cursor: Cursor::default(),
        }
        .with(|area| {
            // Enable scrolling in x direction
            area.scroll_core.set_scroll_x(true);
            area.scroll_core
                .set_scroll_strategy(ScrollStrategy::KeepRow);
            // Fix for scrollbar at bottom to be intractable and content better readable
            area.scroll_core.set_scrollbar_padding((1, 1));
        })
    }

    /// Retrieves the content of the view.
    pub fn get_content(&self) -> RopeSlice {
        self.content.slice(..)
    }

    /// Returns the current scroll offset.
    pub fn scroll(&self) -> Vec2 {
        self.scroll_core.content_viewport().top_left()
    }

    /// Moves the scroll to the given position.
    pub fn set_scroll(&mut self, pos: Vec2) -> Callback {
        // Need to refresh layout, content could have been changed.
        self.layout(self.scroll_core.last_outer_size());

        self.scroll_core.set_offset(pos);

        self.on_scroll_callback().unwrap_or(Callback::dummy())
    }

    /// Returns the `Cursor` in the content string.
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Moves the cursor to the given position.
    ///
    /// # Panics
    ///
    /// This method panics if `cursor` is not the beginning of a character in
    /// the content string.
    pub fn set_cursor(&mut self, cursor: Cursor) -> Callback {
        // Need to refresh layout, content could have been changed.
        self.layout(self.scroll_core.last_outer_size());

        self.cursor = cursor;

        self.on_interact_callback().unwrap_or(Callback::dummy())
    }

    /// Sets the `Cursor` from a given `byte_offset`
    fn set_cursor_from_char_offset(&mut self, char_offset: usize) -> Callback {
        let byte_offset = self.content.char_to_byte(char_offset);
        self.set_cursor(Cursor {
            row: self.row_at(byte_offset),
            column: self.col_at(byte_offset),
            byte_offset,
            char_offset,
        })
    }

    /// Sets the `Cursor` from a given `byte_offset`
    fn set_curser_from_byte_offset(&mut self, byte_offset: usize) -> Callback {
        self.set_cursor(Cursor {
            row: self.row_at(byte_offset),
            column: self.col_at(byte_offset),
            byte_offset,
            char_offset: self.content.byte_to_char(byte_offset),
        })
    }

    /// Only updates the offset from the `byte_offset`
    fn set_offset(&mut self, byte_offset: usize) -> Callback {
        self.set_cursor(Cursor {
            row: self.cursor.row,
            column: self.cursor.column,
            byte_offset,
            char_offset: self.content.byte_to_char(byte_offset),
        })
    }

    /// Sets the content of the view.
    pub fn set_content<S: Into<String>>(&mut self, content: S) -> Callback {
        self.content = content.into().into();

        // First, make sure we are within the bounds.
        self.set_curser_from_byte_offset(min(self.cursor.byte_offset, self.content.len_bytes()));

        // Recaulcuate the available width.
        self.compute_max_content_width(None);

        self.on_edit_callback().unwrap_or_else(Callback::dummy)
    }

    /// Sets the content of the view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn content<S: Into<String>>(mut self, content: S) -> Self {
        self.set_content(content);
        self
    }

    /// Set highlighting style via a file extension
    pub fn set_highlighting(&mut self, extension: &str) {
        self.synref = self
            .syntax
            .find_syntax_by_extension(extension)
            .cloned()
            .unwrap_or(self.syntax.find_syntax_plain_text().clone());
    }

    /// Sets a callback to be called whenever the cursor is modified.
    ///
    /// `callback` will be called with the view
    /// content and the current cursor position.
    ///
    /// This callback can safely trigger itself recursively if needed
    /// (for instance if you call `on_event` on this view from the callback).
    ///
    /// If you need a mutable closure and don't care about the recursive
    /// aspect, see [`set_on_interact_mut`](#method.set_on_interact_mut).
    pub fn set_on_interact<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, &Rope, Vec2, Cursor) + 'static + Send + Sync,
    {
        self.on_interact = Some(Arc::new(callback));
    }

    /// Sets a callback to be called whenever the view is scrolled.
    ///
    /// `callback` will be called with the view
    /// content and the current cursor position.
    ///
    /// This callback can safely trigger itself recursively if needed
    /// (for instance if you call `on_event` on this view from the callback).
    ///
    /// If you need a mutable closure and don't care about the recursive
    /// aspect, see [`set_on_scroll_mut`](#method.set_on_scroll_mut).
    pub fn set_on_scroll<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, &Rope, Vec2, Cursor) + 'static + Send + Sync,
    {
        self.on_scroll = Some(Arc::new(callback));
    }

    /// Sets a callback to be called whenever the content is modified.
    ///
    /// `callback` will be called with the view
    /// content and the current cursor position.
    ///
    /// This callback can safely trigger itself recursively if needed
    /// (for instance if you call `on_event` on this view from the callback).
    ///
    /// If you need a mutable closure and don't care about the recursive
    /// aspect, see [`set_on_edit_mut`](#method.set_on_edit_mut).
    pub fn set_on_edit<F>(&mut self, callback: F)
    where
        F: Fn(&mut Cursive, &Rope, Vec2, Cursor) + 'static + Send + Sync,
    {
        self.on_edit = Some(Arc::new(callback));
    }

    /// Finds the row containing the grapheme at the given offset
    fn row_at(&self, byte_offset: usize) -> usize {
        self.content.byte_to_line(byte_offset)
    }

    fn col_at(&self, byte_offset: usize) -> usize {
        let row_id = self.row_at(byte_offset);
        let start = self.content.line_to_char(row_id);
        let end = self.content.byte_to_char(byte_offset);

        self.content.slice(start..end).len_chars()
    }

    /// Finds the row containing the cursor
    fn selected_row(&self) -> usize {
        self.row_at(self.cursor.byte_offset)
    }

    /// Finds the col containing the cursor
    fn selected_col(&self) -> usize {
        self.col_at(self.cursor.byte_offset)
    }

    /// Calculates the max content width. You can add an `edited_line` to improve performance for large content greatly.
    fn compute_max_content_width(&mut self, edited_line: Option<usize>) {
        let num_lines = self.content.len_lines();
        let line_number_width = num_lines.to_string().len();
        match edited_line {
            None => {
                let (max_line_width, max_index) =
                    self.content
                        .lines()
                        .enumerate()
                        .fold((0, 0), |(max, idx), (i, line)| {
                            let w = line.len_chars();
                            if w > max {
                                (w, i)
                            } else {
                                (max, idx)
                            }
                        });

                self.max_content_width = max_line_width + line_number_width + 1;
                self.max_line_index = max_index;
            }
            Some(i) => {
                let new_width = self.content.line(i).len_chars();
                let current_max_line_width =
                    self.max_content_width.saturating_sub(line_number_width + 1);
                if i == self.max_line_index {
                    if new_width < current_max_line_width {
                        let mut candidate = new_width;
                        let mut candidate_index = i;

                        if i > 0 {
                            let above = self.content.line(i - 1).len_chars();
                            if above > candidate {
                                candidate = above;
                                candidate_index = i - 1;
                            }
                        }
                        if i + 1 < num_lines {
                            let below = self.content.line(i + 1).len_chars();
                            if below > candidate {
                                candidate = below;
                                candidate_index = i + 1;
                            }
                        }

                        self.max_content_width = candidate + line_number_width + 1;
                        self.max_line_index = candidate_index;
                    } else {
                        self.max_content_width = new_width + line_number_width + 1;
                    }
                } else if new_width > current_max_line_width {
                    self.max_content_width = new_width + line_number_width + 1;
                    self.max_line_index = i;
                }
            }
        }
    }

    fn page_up(&mut self) -> Callback {
        for _ in 0..5 {
            self.move_up();
        }

        self.on_interact_callback().unwrap_or(Callback::dummy())
    }

    fn page_down(&mut self) -> Callback {
        for _ in 0..5 {
            self.move_down();
        }

        self.on_interact_callback().unwrap_or(Callback::dummy())
    }

    fn move_up(&mut self) -> Callback {
        let current_row = self.selected_row();
        if current_row == 0 {
            return Callback::dummy();
        }

        let prev_line_start = self.content.line_to_char(current_row - 1);
        let prev_line_end = self.content.line_to_char(current_row);
        let prev_line_len = prev_line_end - prev_line_start - 1;

        let new_col = min(self.cursor.column, prev_line_len);
        let new_char_offset = prev_line_start + new_col;
        let new_byte_offset = self.content.char_to_byte(new_char_offset);

        self.set_offset(new_byte_offset)
    }

    // Broken when going to the last linesÏ
    fn move_down(&mut self) -> Callback {
        let current_row = self.selected_row();
        if current_row + 1 >= self.content.len_lines() {
            return Callback::dummy();
        }

        let next_line_start = self.content.line_to_char(current_row + 1);

        let next_line_end = if current_row + 2 < self.content.len_lines() {
            self.content.line_to_char(current_row + 2)
        } else {
            self.content.len_chars()
        };

        let next_line_len = (next_line_end - next_line_start).saturating_sub(1);
        let new_col = min(self.cursor.column, next_line_len);
        let new_char_offset = next_line_start + new_col;
        let new_byte_offset = self.content.char_to_byte(new_char_offset);

        self.set_offset(new_byte_offset)
    }

    /// Moves the cursor to the left.
    fn move_left(&mut self) -> Callback {
        if self.cursor.char_offset == 0 {
            return Callback::dummy();
        }
        let new_char_offset = self.cursor.char_offset - 1;
        let new_byte_offset = self.content.char_to_byte(new_char_offset);
        self.set_curser_from_byte_offset(new_byte_offset)
    }

    /// Moves the cursor to the right.
    fn move_right(&mut self) -> Callback {
        if self.cursor.char_offset >= self.content.len_chars() {
            return Callback::dummy();
        }
        let new_char_offset = self.cursor.char_offset + 1;
        let new_byte_offset = self.content.char_to_byte(new_char_offset);
        self.set_curser_from_byte_offset(new_byte_offset)
    }

    /// Moves by the mouse position and scroll offset.
    fn move_mouse(&mut self, position: XY<usize>, offset: XY<usize>) -> Callback {
        let content_lines = self.content.len_lines();
        if content_lines != 0 && position.fits_in_rect(offset, self.scroll_core.inner_size()) {
            if let Some(position) = position.checked_sub(offset) {
                let y = min(position.y, content_lines - 1);
                let x = position
                    .x
                    .saturating_sub(content_lines.to_string().len() + 1);

                let row_start = self.content.line_to_char(y);
                let row_end = if y + 1 < content_lines {
                    self.content.line_to_char(y + 1).saturating_sub(1)
                } else {
                    self.content.len_chars()
                };

                let content = self.content.slice(row_start..row_end).to_string();
                let prefix_length = grapheme_prefix_length(&content, x);

                return self.set_cursor_from_char_offset(row_start + prefix_length);
            }
        }
        Callback::dummy()
    }

    fn backspace(&mut self) -> Callback {
        self.move_left();
        self.delete()
    }

    // Broken when deleting multiline text
    fn delete(&mut self) -> Callback {
        if self.cursor.char_offset >= self.content.len_chars() {
            return Callback::dummy();
        }
        let end = self.cursor.char_offset + 1;
        self.content.remove(self.cursor.char_offset..end);

        // Recaulcuate the available width for the current edited line.
        let current_line = self.content.char_to_line(self.cursor.char_offset);
        self.compute_max_content_width(Some(current_line));

        self.on_edit_callback().unwrap_or_else(Callback::dummy)
    }

    fn insert(&mut self, ch: char) -> Callback {
        let old_line = self.content.char_to_line(self.cursor.char_offset);
        self.content.insert_char(self.cursor.char_offset, ch);

        // Then, we shift the indexes of every row after this one.
        let shift = ch.len_utf8();

        // Update cursor
        self.set_curser_from_byte_offset(self.cursor.byte_offset + shift);

        // Check if newline char, if true reset the cached column.
        // Also compute the max_content_width with the old line index.
        if ch == '\n' {
            self.cursor.column = 0;
            self.compute_max_content_width(Some(old_line));
        } else {
            let current_line = self.content.char_to_line(self.cursor.char_offset);
            self.compute_max_content_width(Some(current_line));
        }

        self.on_edit_callback().unwrap_or_else(Callback::dummy)
    }

    /// Copies the line where the cursor currently is.
    fn copy(&mut self) {
        let row = self.row_at(self.cursor.byte_offset);
        let line_slice = self.content.line(row);

        let mut copied = line_slice.to_string();
        if !copied.ends_with('\n') {
            copied.push('\n');
        }

        crate::clipboard::set_content(copied).unwrap_or_else(|e| error!("{e}"));
    }

    /// Pastes the current clipboard at the cursor position.
    fn paste(&mut self) -> Callback {
        let cursor_pos = self.cursor.char_offset;
        if let Ok(text) = crate::clipboard::get_content() {
            self.content.insert(cursor_pos, &text);
            self.set_cursor_from_char_offset(cursor_pos + text.chars().count());
            self.on_edit_callback().unwrap_or(Callback::dummy())
        } else {
            Callback::dummy()
        }
    }

    /// Cuts (copies and removes) the line where the cursor currently is.
    fn cut(&mut self) -> Callback {
        let row = self.row_at(self.cursor.byte_offset);

        let line_slice = self.content.line(row);
        let mut line_text = line_slice.to_string();
        if !line_text.ends_with('\n') {
            line_text.push('\n');
        }

        crate::clipboard::set_content(line_text).unwrap_or_else(|e| error!("{e}"));

        let start = self.content.line_to_char(row);
        let end = if row + 1 < self.content.len_lines() {
            self.content.line_to_char(row + 1)
        } else {
            self.content.len_chars()
        };
        self.content.remove(start..end);

        self.set_cursor_from_char_offset(start);

        self.on_edit_callback().unwrap_or(Callback::dummy())
    }

    /// Implements the tabulator. If `ident` is true, insert (indent) a tab;
    /// otherwise, remove (unindent) a tab if present.
    fn tabulator(&mut self, ident: bool) -> Callback {
        let row = self.row_at(self.cursor.byte_offset);
        let line_start = self.content.line_to_char(row);
        let line_end = if row + 1 < self.content.len_lines() {
            self.content.line_to_char(row + 1)
        } else {
            self.content.len_chars()
        };

        let line_text = self.content.slice(line_start..line_end).to_string();
        let tab_size = 4;
        let tab_str = " ".repeat(tab_size);
        let new_line_text = if ident {
            format!("{}{}", tab_str, line_text)
        } else if line_text.starts_with(&tab_str) {
            line_text[tab_size..].to_string()
        } else {
            line_text.clone()
        };

        if new_line_text != line_text {
            self.content.remove(line_start..line_end);
            self.content.insert(line_start, &new_line_text);

            let current_offset = self.cursor.char_offset - line_start;
            let new_offset = if ident {
                current_offset + tab_size
            } else {
                current_offset.saturating_sub(tab_size)
            };
            self.set_cursor_from_char_offset(line_start + new_offset);
        }

        self.on_edit_callback().unwrap_or(Callback::dummy())
    }

    /// Moves the line containing the cursor up or down.
    fn move_line(&mut self, direction: Key) -> Callback {
        let num_lines = self.content.len_lines();
        let current_line = self.row_at(self.cursor.byte_offset);

        if (current_line == 0 && direction == Key::Up)
            || (current_line == num_lines - 1 && direction == Key::Down)
        {
            return Callback::dummy();
        }

        let target_line = if direction == Key::Up {
            current_line - 1
        } else {
            current_line + 1
        };

        let current_start = self.content.line_to_char(current_line);
        let cursor_in_line = self.cursor.char_offset - current_start;

        swap_lines(&mut self.content, current_line, target_line);

        let new_line_start = self.content.line_to_char(target_line);
        let new_cursor_pos = new_line_start + cursor_in_line;
        self.set_cursor_from_char_offset(new_cursor_pos);

        self.on_edit_callback().unwrap_or(Callback::dummy())
    }

    /// Moves the cursor to the start or end of the current line.
    /// For left, moves to the beginning; for right, to the last character (before newline).
    fn move_cursor_end(&mut self, direction: Key) -> Callback {
        let row = self.row_at(self.cursor.byte_offset);
        let line_start = self.content.line_to_char(row);
        let line_end = if row + 1 < self.content.len_lines() {
            // Subtracting 1 for getting the current line end
            self.content.line_to_char(row + 1).saturating_sub(1)
        } else {
            self.content.len_chars()
        };
        match direction {
            Key::Left => self.set_cursor_from_char_offset(line_start),
            Key::Right => self.set_cursor_from_char_offset(line_end),
            _ => Callback::dummy(),
        }
    }

    fn on_interact_callback(&self) -> Option<Callback> {
        self.on_interact.clone().map(|cb| {
            let content = self.content.clone();
            let scroll_offset = self.scroll_core.content_viewport().top_left();
            let cursor = self.cursor;

            Callback::from_fn(move |s| {
                cb(s, &content, scroll_offset, cursor);
            })
        })
    }

    /// Run any callback after scrolling.
    fn on_scroll_callback(&self) -> Option<Callback> {
        self.on_scroll.clone().map(|cb| {
            let content = self.content.clone();
            let scroll_offset = self.scroll_core.content_viewport().top_left();
            let cursor = self.cursor;

            Callback::from_fn(move |s| {
                cb(s, &content, scroll_offset, cursor);
            })
        })
    }

    fn on_edit_callback(&self) -> Option<Callback> {
        self.on_edit.clone().map(|cb| {
            let content = self.content.clone();
            let scroll_offset = self.scroll_core.content_viewport().top_left();
            let cursor = self.cursor;

            Callback::from_fn(move |s| {
                cb(s, &content, scroll_offset, cursor);
            })
        })
    }

    // Events inside the text field
    fn inner_on_event(&mut self, event: Event) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }

        match event {
            Event::Char(ch) => {
                return EventResult::Consumed(Some(self.insert(ch)));
            }
            Event::Key(Key::Enter) => {
                return EventResult::Consumed(Some(self.insert('\n')));
            }
            Event::Key(Key::Backspace) if self.cursor.byte_offset > 0 => {
                return EventResult::Consumed(Some(self.backspace()));
            }
            Event::Key(Key::Del) if self.cursor.byte_offset < self.content.len_bytes() => {
                return EventResult::Consumed(Some(self.delete()));
            }
            Event::Key(Key::Up) => {
                if self.selected_row() > 0 {
                    return EventResult::Consumed(Some(self.move_up()));
                }
            }
            Event::Key(Key::Down) => {
                if self.selected_row() + 1 < self.content.len_lines() {
                    return EventResult::Consumed(Some(self.move_down()));
                }
            }
            Event::Key(Key::Left) => {
                if self.cursor.byte_offset > 0 {
                    return EventResult::Consumed(Some(self.move_left()));
                }
            }
            Event::Key(Key::Right) => {
                if self.cursor.byte_offset < self.content.len_bytes() {
                    return EventResult::Consumed(Some(self.move_right()));
                }
            }
            Event::Shift(Key::Left) => {
                return EventResult::Consumed(Some(self.move_cursor_end(Key::Left)));
            }
            Event::Shift(Key::Right) => {
                return EventResult::Consumed(Some(self.move_cursor_end(Key::Right)));
            }
            Event::Key(Key::PageUp) => {
                return EventResult::Consumed(Some(self.page_up()));
            }
            Event::Key(Key::PageDown) => {
                return EventResult::Consumed(Some(self.page_down()));
            }
            Event::Shift(Key::PageUp) => {
                return EventResult::Consumed(Some(self.set_curser_from_byte_offset(0)));
            }
            Event::Shift(Key::PageDown) => {
                return EventResult::Consumed(Some(
                    self.set_curser_from_byte_offset(self.content.len_bytes()),
                ));
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } => {
                return EventResult::Consumed(Some(self.move_mouse(position, offset)));
            }
            Event::CtrlChar('c') => self.copy(),
            Event::CtrlChar('v') => {
                return EventResult::Consumed(Some(self.paste()));
            }
            Event::CtrlChar('x') => {
                return EventResult::Consumed(Some(self.cut()));
            }
            Event::Shift(Key::Up) => {
                return EventResult::Consumed(Some(self.move_line(Key::Up)));
            }
            Event::Shift(Key::Down) => {
                return EventResult::Consumed(Some(self.move_line(Key::Down)));
            }
            Event::Key(Key::Tab) => {
                return EventResult::Consumed(Some(self.tabulator(true)));
            }
            Event::Shift(Key::Tab) => {
                return EventResult::Consumed(Some(self.tabulator(false)));
            }
            _ => return EventResult::Ignored,
        }

        EventResult::consumed()
    }

    /// Compute the required size for the content.
    fn inner_required_size(&mut self, vec: Vec2) -> Vec2 {
        Vec2::new(
            max(self.max_content_width + 1, vec.x),
            self.content.len_lines(),
        )
    }

    fn inner_important_area(&self, _: Vec2) -> Rect {
        // The important area is a single character
        let char_width = if self.cursor.char_offset >= self.content.len_chars() {
            // If we're are the end of the content, it'll be a space
            1
        } else {
            // Otherwise it's the selected grapheme
            let start = self.cursor.char_offset;
            let end = start + 1;
            self.content.slice(start..end).len_chars()
        };

        Rect::from_size(
            Vec2::new(self.selected_col(), self.selected_row()),
            (
                char_width + self.content.len_lines().to_string().len() + 2,
                1,
            ),
        )
    }
}

impl View for EditArea {
    fn draw(&self, printer: &Printer) {
        printer.with_style(PaletteStyle::Primary, |printer| {
            scroll::draw_lines(self, printer, |edit_area, printer, i| {
                let row_start = self.content.line_to_byte(i);
                let text = edit_area.content.line(i).to_string();

                let mut highlighter =
                    syntect::easy::HighlightLines::new(&edit_area.synref, &edit_area.theme);

                let styled = cursive_syntect::parse(&text, &mut highlighter, &edit_area.syntax)
                    .unwrap_or_default();

                // Check if file needs to be numbered.
                let numbering = if printer.enabled && edit_area.enabled {
                    // Calculate max digits for better visual representation.
                    let max_lines_count_digits = edit_area.content.len_lines().to_string().len();

                    let line_number = format!("{:width$} ", i + 1, width = max_lines_count_digits);

                    let number_style = if i == edit_area.selected_row() {
                        Style::default()
                    } else {
                        Color::Light(BaseColor::Black).into()
                    };
                    SpannedString::styled(line_number, number_style)
                } else {
                    SpannedString::default()
                };

                let line = StyledString::concatenate(vec![numbering.clone(), styled]);

                let mut x = 0;
                for span in line.spans() {
                    let span_text = span.content;
                    let span_style = span.attr.color.front;
                    for grapheme in span_text.graphemes(true) {
                        // Check for special characters and print faded.
                        if let Some(special) = special_character(grapheme) {
                            printer.with_style(
                                ColorStyle::new(
                                    Color::Light(BaseColor::Black),
                                    PaletteColor::Background,
                                ),
                                |printer| {
                                    printer.print((x, 0), special);
                                },
                            );
                            x += 1;
                        } else {
                            printer.with_style(
                                ColorStyle::new(span_style, PaletteColor::Background),
                                |printer| {
                                    printer.print((x, 0), grapheme);
                                },
                            );
                            x += UnicodeWidthStr::width(grapheme);
                        }
                    }
                }

                if printer.focused
                    && i == edit_area.selected_row()
                    && printer.enabled
                    && edit_area.enabled
                {
                    let cursor_offset = edit_area.cursor.byte_offset - row_start;
                    let mut c = StyledString::new();
                    let mut selected_char = if cursor_offset == text.len()
                        || (text[cursor_offset..].contains("\n")
                            && cursor_offset == text.len().saturating_sub(1))
                    {
                        " "
                    } else {
                        text[cursor_offset..].graphemes(true).next().unwrap_or(" ")
                    };
                    // Check for special characters and overwrite selected_char accordingly.
                    if let Some(special) = special_character(selected_char) {
                        selected_char = special;
                    }
                    c.append_styled(selected_char, Style::primary().combine(Effect::Reverse));
                    let offset = text[..cursor_offset].width() + numbering.width();
                    printer.print_styled((offset, 0), &c);
                }
            });
        });
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        scroll::required_size(self, constraint, true, Self::inner_required_size)
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match scroll::on_event(
            self,
            event,
            Self::inner_on_event,
            Self::inner_important_area,
        ) {
            EventResult::Ignored => EventResult::Ignored,
            // If the event was consumed, then we may have scrolled.
            other => other.and(EventResult::Consumed(self.on_scroll_callback())),
        }
    }

    fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
        self.enabled.then(EventResult::consumed).ok_or(CannotFocus)
    }

    fn layout(&mut self, size: Vec2) {
        scroll::layout(self, size, true, |_s, _size| (), Self::inner_required_size);
    }

    fn important_area(&self, size: Vec2) -> Rect {
        scroll::important_area(self, size, Self::inner_important_area)
    }
}
