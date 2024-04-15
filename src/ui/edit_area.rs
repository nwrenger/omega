use cursive::{
    direction::Direction,
    event::{Callback, Event, EventResult, Key, MouseEvent},
    reexports::log::error,
    theme::{ColorStyle, Effect, PaletteColor, PaletteStyle, Style},
    utils::{
        lines::simple::{simple_prefix, LinesIterator, Row},
        markup::StyledString,
        span::SpannedString,
    },
    view::{CannotFocus, SizeCache},
    Cursive, Printer, Rect, Vec2, View, With, XY,
};
use cursive::{
    impl_scroller,
    view::{scroll, ScrollStrategy},
};
use std::{cmp::min, rc::Rc};
use syntect::{
    highlighting::{Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Improved Multi-lines text editor.
///
/// A `EditArea` will attempt to grow vertically and horizontally
/// dependent on the content.  Wrap it in a `ResizedView` to
/// constrain its size.
///
/// # Examples
///
/// ```
/// use cursive_core::traits::{Nameable, Resizable};
/// use cursive_core::views::EditArea;
///
/// let edit_area = EditArea::new()
///     .content("Write description here...")
///     .with_name("edit_area")
///     .fixed_width(30)
///     .min_height(5);
/// ```

/// Closure type for callbacks when the content is modified.
///
/// Arguments are the `Cursive`, current content of the input and cursor
/// position
pub type OnEdit = dyn Fn(&mut Cursive, &str, usize);

pub struct EditArea {
    // TODO: use a smarter data structure (rope?)
    content: String,

    /// Width of the longest line
    max_content_width: usize,

    /// Byte offsets within `content` representing text rows
    ///
    /// Invariant: never empty.
    rows: Vec<Row>,

    /// Syntax Set
    syntax: SyntaxSet,

    /// Current Theme for highlighting
    pub theme: Theme,

    /// Specified through file extension, the applied highlighting
    synref: SyntaxReference,

    /// When `false`, we don't take any input.
    enabled: bool,

    /// Callback when the cursor is moved.
    ///
    /// Will be called with the current content and the cursor position.
    on_interact: Option<Rc<OnEdit>>,

    /// Callback when the content is modified.
    ///
    /// Will be called with the current content and the cursor position.
    on_edit: Option<Rc<OnEdit>>,

    /// Base for scrolling features
    scroll_core: scroll::Core,

    /// Cache to avoid re-computing layout on no-op events
    size_cache: Option<XY<SizeCache>>,

    /// Byte offset of the currently selected grapheme.
    cursor: usize,
}

impl_scroller!(EditArea::scroll_core);

fn make_rows(text: &str) -> Vec<Row> {
    // Full width, no limits
    let width = usize::MAX;
    LinesIterator::new(text, width).show_spaces().collect()
}

impl Default for EditArea {
    fn default() -> Self {
        Self::new()
    }
}

impl EditArea {
    /// Creates a new, empty EditArea.
    pub fn new() -> Self {
        EditArea {
            content: String::new(),
            max_content_width: 0,
            rows: Vec::new(),
            syntax: SyntaxSet::load_defaults_newlines(),
            theme: ThemeSet::load_defaults().themes["base16-eighties.dark"].clone(),
            synref: SyntaxSet::load_defaults_newlines()
                .find_syntax_plain_text()
                .clone(),
            enabled: true,
            on_interact: None,
            on_edit: None,
            scroll_core: scroll::Core::new(),
            size_cache: None,
            cursor: 0,
        }
        .with(|area| {
            // Make sure we have valid rows, even for empty text.
            area.compute_rows(Vec2::new(1, 1));
            // Enable scrolling in x direction
            area.scroll_core.set_scroll_x(true);
            area.scroll_core
                .set_scroll_strategy(ScrollStrategy::KeepRow);
            // Fix for scrollbar at bottom to be intractable and content better readable
            area.scroll_core.set_scrollbar_padding((1, 1));
        })
    }

    /// Retrieves the content of the view.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Ensures next layout call re-computes the rows.
    fn invalidate(&mut self) {
        self.size_cache = None;
    }

    /// Returns the position of the cursor in the content string.
    ///
    /// This is a byte index.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Moves the cursor to the given position.
    ///
    /// # Panics
    ///
    /// This method panics if `cursor` is not the beginning of a character in
    /// the content string.
    ///
    /// The `fix` var should be set to true if the content was changed
    pub fn set_cursor(&mut self, cursor: usize, fix: bool) -> Callback {
        self.cursor = cursor;

        self.scroll_core.scroll_to(
            (
                self.col_at(cursor),
                if fix {
                    self.row_at(cursor) + 6
                } else {
                    self.row_at(cursor)
                },
            )
                .into(),
        );

        self.make_interact_cb().unwrap_or(Callback::dummy())
    }

    /// Sets the content of the view.
    pub fn set_content<S: Into<String>>(&mut self, content: S) -> Callback {
        self.content = content.into();

        // First, make sure we are within the bounds.
        self.cursor = min(self.cursor, self.content.len());

        // We have no guarantee cursor is now at a correct UTF8 location.
        // So look backward until we find a valid grapheme start.
        while !self.content.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }

        if let Some(size) = self.size_cache.map(|s| s.map(|s| s.value)) {
            self.invalidate();
            self.compute_rows(size);
        }

        self.make_edit_cb().unwrap_or_else(Callback::dummy)
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

    /// Disables this view.
    ///
    /// A disabled view cannot be selected.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Disables this view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn disabled(self) -> Self {
        self.with(Self::disable)
    }

    /// Re-enables this view.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Re-enables this view.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn enabled(self) -> Self {
        self.with(Self::enable)
    }

    /// Returns `true` if this view is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
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
        F: Fn(&mut Cursive, &str, usize) + 'static,
    {
        self.on_interact = Some(Rc::new(callback));
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
        F: Fn(&mut Cursive, &str, usize) + 'static,
    {
        self.on_edit = Some(Rc::new(callback));
    }

    /// Finds the row containing the grapheme at the given offset
    fn row_at(&self, byte_offset: usize) -> usize {
        assert!(!self.rows.is_empty());
        assert!(byte_offset >= self.rows[0].start);

        self.rows
            .iter()
            .enumerate()
            .take_while(|&(_, row)| row.start <= byte_offset)
            .map(|(i, _)| i)
            .last()
            .unwrap()
    }

    fn col_at(&self, byte_offset: usize) -> usize {
        let row_id = self.row_at(byte_offset);
        let row = self.rows[row_id];
        // Number of cells to the left of the cursor
        self.content[row.start..byte_offset].width()
    }

    /// Finds the row containing the cursor
    fn selected_row(&self) -> usize {
        assert!(!self.rows.is_empty(), "Rows should never be empty.");
        self.row_at(self.cursor)
    }

    fn selected_col(&self) -> usize {
        self.col_at(self.cursor)
    }

    fn page_up(&mut self) -> Callback {
        for _ in 0..5 {
            self.move_up();
        }

        self.fix_scroll();

        self.make_interact_cb().unwrap_or(Callback::dummy())
    }

    fn page_down(&mut self) -> Callback {
        for _ in 0..5 {
            self.move_down();
        }

        self.fix_scroll();

        self.make_interact_cb().unwrap_or(Callback::dummy())
    }

    fn move_up(&mut self) -> Callback {
        let row_id = self.selected_row();
        if row_id == 0 {
            return Callback::dummy();
        }

        let prev_row = self.rows[row_id - 1];
        self.cursor = prev_row.start;

        self.fix_scroll();

        self.make_interact_cb().unwrap_or(Callback::dummy())
    }

    fn move_down(&mut self) -> Callback {
        let row_id = self.selected_row();
        if row_id + 1 == self.rows.len() {
            return Callback::dummy();
        }

        let next_row = self.rows[row_id + 1];
        self.cursor = next_row.start;

        self.fix_scroll();

        self.make_interact_cb().unwrap_or(Callback::dummy())
    }

    /// Moves the cursor to the left.
    fn move_left(&mut self) -> Callback {
        self.cursor -= 1;

        self.fix_scroll();

        self.make_interact_cb().unwrap_or(Callback::dummy())
    }

    /// Moves the cursor to the right.
    fn move_right(&mut self) -> Callback {
        self.cursor += 1;

        self.fix_scroll();

        self.make_interact_cb().unwrap_or(Callback::dummy())
    }

    fn is_cache_valid(&self, size: Vec2) -> bool {
        match self.size_cache {
            None => false,
            Some(ref last) => last.x.accept(size.x) && last.y.accept(size.y),
        }
    }

    // If we are editing the text, we add a fake "space" character for the
    // cursor to indicate where the next character will appear.
    // If the current line is full, adding a character will overflow into the
    // next line. To show that, we need to add a fake "ghost" row, just for
    // the cursor.
    fn fix_ghost_row(&mut self) {
        if self.rows.is_empty() || self.rows.last().unwrap().end != self.content.len() {
            // Add a fake, empty row at the end.
            self.rows.push(Row {
                start: self.content.len(),
                end: self.content.len(),
                width: 0,
                is_wrapped: false,
            });
        }
    }

    fn compute_max_content_length(&mut self) {
        self.max_content_width = self.rows.iter().map(|r| r.width).max().unwrap_or(1)
            + self.rows.len().to_string().len()
            + 1;
    }

    fn compute_rows(&mut self, size: Vec2) {
        if self.is_cache_valid(size) {
            return;
        }

        self.rows = make_rows(&self.content);
        self.fix_ghost_row();

        // also compute here the max content length
        self.compute_max_content_length();

        if !self.rows.is_empty() {
            self.size_cache = Some(SizeCache::build(size, size));
        }
    }

    fn backspace(&mut self) -> Callback {
        self.move_left();
        self.delete()
    }

    fn delete(&mut self) -> Callback {
        if self.cursor == self.content.len() {
            return Callback::dummy();
        }
        let len = self.content[self.cursor..]
            .graphemes(true)
            .next()
            .unwrap()
            .len();
        let start = self.cursor;
        let end = self.cursor + len;
        for _ in self.content.drain(start..end) {}

        let selected_row = self.selected_row();
        if self.cursor == self.rows[selected_row].end {
            // We're removing an (implicit) newline.
            // This means merging two rows.
            let new_end = self.rows[selected_row + 1].end;
            self.rows[selected_row].end = new_end;
            self.rows.remove(selected_row + 1);
        }
        self.rows[selected_row].end -= len;

        // update all the rows downstream
        for row in &mut self.rows.iter_mut().skip(1 + selected_row) {
            row.rev_shift(len);
        }

        self.fix_damages();
        self.make_edit_cb().unwrap_or_else(Callback::dummy)
    }

    fn insert(&mut self, ch: char) -> Callback {
        // First, we inject the data, but keep the cursor unmoved
        // (So the cursor is to the left of the injected char)
        self.content.insert(self.cursor, ch);

        // Then, we shift the indexes of every row after this one.
        let shift = ch.len_utf8();

        // The current row grows, every other is just shifted.
        let selected_row = self.selected_row();
        self.rows[selected_row].end += shift;

        for row in &mut self.rows.iter_mut().skip(1 + selected_row) {
            row.shift(shift);
        }
        self.cursor += shift;

        // Finally, rows may not have the correct width anymore, so fix them.
        self.fix_damages();
        self.make_edit_cb().unwrap_or_else(Callback::dummy)
    }

    /// Copies the line where the cursor currently is
    fn copy(&mut self) {
        let content = self.get_content().to_string();
        let cursor_pos = self.cursor();

        let (current_line, _) = Self::get_cursor_line_info(&content, cursor_pos);

        let lines: Vec<&str> = content.split('\n').collect();

        crate::clipboard::set_content(lines[current_line].to_string() + "\n")
            .unwrap_or_else(|e| error!("{e}"));
    }

    /// Pasts the current clipboard
    fn paste(&mut self) -> Callback {
        let content = self.get_content().to_string();
        let cursor_pos = self.cursor();

        let (current_line, cursor_in_line) = Self::get_cursor_line_info(&content, cursor_pos);

        let mut lines: Vec<&str> = content.split('\n').collect();
        if let Ok(text) = crate::clipboard::get_content() {
            let split = lines[current_line].split_at(cursor_in_line);
            let inserted_line = split.0.to_string() + text.as_str() + split.1;
            lines[current_line] = inserted_line.as_str();

            let new_content: String = lines.join("\n");
            if new_content != content {
                self.set_content(new_content);
                self.set_cursor(cursor_pos + text.to_string().len(), false);
                // changed stuff soooo, needing this
                self.make_edit_cb().unwrap_or(Callback::dummy())
            } else {
                Callback::dummy()
            }
        } else {
            Callback::dummy()
        }
    }

    /// Cuts the line where the cursor currently is
    fn cut(&mut self) -> Callback {
        let content = self.get_content().to_string();
        let cursor_pos = self.cursor();

        let (current_line, current_line_pos) = Self::get_cursor_line_info(&content, cursor_pos);

        let mut lines: Vec<&str> = content.split('\n').collect();
        crate::clipboard::set_content(lines[current_line].to_string() + "\n")
            .unwrap_or_else(|e| error!("{e}"));
        lines.remove(current_line);

        let new_content: String = lines.join("\n");
        if new_content != content {
            self.set_cursor(cursor_pos - current_line_pos, false);
            self.set_content(new_content);
            // changed stuff soooo, needing this
            self.make_edit_cb().unwrap_or(Callback::dummy())
        } else {
            Callback::dummy()
        }
    }

    /// Implements the tabulator
    fn tabulator(&mut self, ident: bool) -> Callback {
        let content = self.get_content().to_string();
        let cursor_pos = self.cursor();

        let (current_line, current_line_position) =
            Self::get_cursor_line_info(&content, cursor_pos);
        let mut lines: Vec<&str> = content.split('\n').collect();
        let tab_size = 4;

        let str_to_add = " ".repeat(tab_size);

        let new_content = if ident {
            let new_line = str_to_add + lines[current_line];

            self.set_cursor(cursor_pos + tab_size, false);

            lines[current_line] = &new_line;
            lines.join("\n")
        } else {
            let new_line = lines[current_line].replacen(&str_to_add, "", 1);

            if lines[current_line] != new_line {
                self.set_cursor(cursor_pos - min(current_line_position, tab_size), false);
            }

            lines[current_line] = &new_line;
            lines.join("\n")
        };
        if new_content != content {
            self.set_content(new_content);
            // changed stuff soooo, needing this
            self.make_edit_cb().unwrap_or(Callback::dummy())
        } else {
            Callback::dummy()
        }
    }

    /// Moves the line withing the cursor in the specified direction
    fn move_line(&mut self, direction: Key) -> Callback {
        let content = self.get_content().to_string();
        let cursor_pos = self.cursor();

        let (current_line, cursor_in_line) = Self::get_cursor_line_info(&content, cursor_pos);

        let mut lines: Vec<&str> = content.split('\n').collect();

        if (current_line == 0 && direction == Key::Up)
            || (current_line == lines.len() - 1 && direction == Key::Down)
        {
            return Callback::dummy();
        }

        let line_to_move = lines.remove(current_line);
        match direction {
            Key::Up => lines.insert(current_line - 1, line_to_move),
            Key::Down => lines.insert(current_line + 1, line_to_move),
            _ => {}
        }

        let new_cursor_pos = if direction == Key::Up && current_line > 0 {
            lines
                .iter()
                .take(current_line - 1)
                .map(|line| line.len() + 1)
                .sum::<usize>()
                + cursor_in_line
        } else {
            lines
                .iter()
                .take(current_line + (if direction == Key::Down { 1 } else { 0 }))
                .map(|line| line.len() + 1)
                .sum::<usize>()
                + cursor_in_line
        };

        self.set_cursor(new_cursor_pos, false);

        let new_content: String = lines.join("\n");
        if new_content != content {
            self.set_content(new_content);
            // changed stuff soooo, needing this
            self.make_edit_cb().unwrap_or(Callback::dummy())
        } else {
            Callback::dummy()
        }
    }

    /// Move cursor to the start or end of the current line
    fn move_cursor_end(&mut self, direction: Key) {
        let content = self.get_content().to_string();
        let cursor_pos = self.cursor();

        let (current_line, _) = Self::get_cursor_line_info(&content, cursor_pos);

        let lines: Vec<&str> = content.split('\n').collect();
        match direction {
            Key::Left => {
                let new_cursor_pos = lines
                    .iter()
                    .take(current_line)
                    .map(|line| line.len() + 1)
                    .sum::<usize>();
                self.set_cursor(new_cursor_pos, false);
            }
            Key::Right => {
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
                self.set_cursor(new_cursor_pos, false);
            }
            _ => {}
        }
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

    fn make_interact_cb(&self) -> Option<Callback> {
        self.on_interact.clone().map(|cb| {
            // Get a new Rc on the content
            let content = self.content.clone();
            let cursor = self.cursor;

            Callback::from_fn(move |s| {
                cb(s, &content, cursor);
            })
        })
    }

    fn make_edit_cb(&self) -> Option<Callback> {
        self.on_edit.clone().map(|cb| {
            // Get a new Rc on the content
            let content = self.content.clone();
            let cursor = self.cursor;

            Callback::from_fn(move |s| {
                cb(s, &content, cursor);
            })
        })
    }

    /// Fix scrolling
    fn fix_scroll(&mut self) {
        self.scroll_core
            .scroll_to((self.selected_col(), self.selected_row()).into());
    }

    /// Fix a damage located at the cursor.
    ///
    /// The only damages are assumed to have occured around the cursor.
    ///
    /// This is an optimization to not re-compute the entire rows when an
    /// insert happened.
    fn fix_damages(&mut self) {
        if self.size_cache.is_none() {
            // If we don't know our size, we'll get a layout command soon.
            // So no need to do that here.
            return;
        }

        let size = self.size_cache.unwrap().map(|s| s.value);

        // Find affected text.
        // We know the damage started at this row, so it'll need to go.
        //
        // Actually, if possible, also re-compute the previous row.
        // Indeed, the previous row may have been cut short, and if we now
        // break apart a big word, maybe the first half can go up one level.
        let first_row = self.selected_row().saturating_sub(1);

        let first_byte = self.rows[first_row].start;

        // We don't need to go beyond a newline.
        // If we don't find one, end of the text it is.
        let last_byte = self.content[self.cursor..]
            .find('\n')
            .map(|i| 1 + i + self.cursor);
        let last_row = last_byte.map_or(self.rows.len(), |last_byte| self.row_at(last_byte));
        let last_byte = last_byte.unwrap_or(self.content.len());

        let scrollable = self.rows.len() > size.y;
        // First attempt, if scrollbase status didn't change.
        let new_rows = make_rows(&self.content[first_byte..last_byte]);
        // How much did this add?
        let new_row_count = self.rows.len() + new_rows.len() + first_row - last_row;
        if !scrollable && new_row_count > size.y {
            // We just changed scrollable status.
            // This changes everything.
            // TODO: compute_rows() currently makes a scroll-less attempt.
            // Here, we know it's just no gonna happen.
            self.invalidate();
            self.compute_rows(size);
            return;
        }

        // Otherwise, replace stuff.
        let affected_rows = first_row..last_row;
        let replacement_rows = new_rows.into_iter().map(|row| row.shifted(first_byte));
        self.rows.splice(affected_rows, replacement_rows);
        // other fixes
        self.fix_ghost_row();
        self.fix_scroll();
        // also compute the max length, that could have changed
        self.compute_max_content_length();
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
                self.fix_scroll();
                return EventResult::Consumed(Some(self.insert('\n')));
            }
            Event::Key(Key::Backspace) if self.cursor > 0 => {
                return EventResult::Consumed(Some(self.backspace()));
            }
            Event::Key(Key::Del) if self.cursor < self.content.len() => {
                return EventResult::Consumed(Some(self.delete()));
            }
            Event::Key(Key::End) => {
                let row = self.selected_row();
                self.cursor = self.rows[row].end;
                if row + 1 < self.rows.len() && self.cursor == self.rows[row + 1].start {
                    self.move_left();
                }
                self.fix_scroll();
            }
            Event::Ctrl(Key::Home) => {
                self.cursor = 0;
                self.fix_scroll();
            }
            Event::Ctrl(Key::End) => {
                self.cursor = self.content.len();
                self.fix_scroll();
            }
            Event::Key(Key::Home) => {
                self.cursor = self.rows[self.selected_row()].start;
                self.fix_scroll();
            }
            Event::Key(Key::Up) if self.selected_row() > 0 => {
                return EventResult::Consumed(Some(self.move_up()));
            }
            Event::Key(Key::Down) if self.selected_row() + 1 < self.rows.len() => {
                return EventResult::Consumed(Some(self.move_down()));
            }
            Event::Key(Key::PageUp) => {
                return EventResult::Consumed(Some(self.page_up()));
            }
            Event::Key(Key::PageDown) => {
                return EventResult::Consumed(Some(self.page_down()));
            }
            Event::Key(Key::Left) if self.cursor > 0 => {
                return EventResult::Consumed(Some(self.move_left()));
            }
            Event::Key(Key::Right) if self.cursor < self.content.len() => {
                return EventResult::Consumed(Some(self.move_right()));
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } => {
                if !self.rows.is_empty()
                    && position.fits_in_rect(offset, self.scroll_core.inner_size())
                {
                    if let Some(position) = position.checked_sub(offset) {
                        let y = position.y;
                        let y = min(y, self.rows.len() - 1);
                        let x = position
                            .x
                            .saturating_sub(self.rows.len().to_string().len() + 1);
                        let row = &self.rows[y];
                        let content = &self.content[row.start..row.end];
                        self.cursor = row.start + simple_prefix(content, x).length;

                        return EventResult::Consumed(Some(
                            self.make_interact_cb().unwrap_or(Callback::dummy()),
                        ));
                    }
                }
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
            Event::Shift(Key::Left) => {
                self.move_cursor_end(Key::Left);
            }
            Event::Shift(Key::Right) => {
                self.move_cursor_end(Key::Right);
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
    fn inner_required_size(&mut self, _: Vec2) -> Vec2 {
        Vec2::new(self.max_content_width + 1, self.rows.len())
    }

    fn inner_important_area(&self, _: Vec2) -> Rect {
        // The important area is a single character
        let char_width = if self.cursor >= self.content.len() {
            // If we're are the end of the content, it'll be a space
            1
        } else {
            // Otherwise it's the selected grapheme
            self.content[self.cursor..]
                .graphemes(true)
                .next()
                .unwrap()
                .width()
        };

        Rect::from_size(
            Vec2::new(self.selected_col(), self.selected_row()),
            (char_width + self.rows.len().to_string().len() + 2, 1),
        )
    }
}

impl View for EditArea {
    fn draw(&self, printer: &Printer) {
        printer.with_style(PaletteStyle::Primary, |printer| {
            scroll::draw_lines(self, printer, |edit_area, printer, i| {
                let row = &edit_area.rows[i];
                let text = edit_area.content[row.start..row.end].to_string();

                let mut highlighter =
                    syntect::easy::HighlightLines::new(&edit_area.synref, &edit_area.theme);

                let styled = cursive_syntect::parse(&text, &mut highlighter, &edit_area.syntax)
                    .unwrap_or_default();

                // check if file needs to be numbered
                let numbering = if printer.enabled && edit_area.enabled {
                    // calculation max digits for better visual representation
                    let max_lines_count_digits = edit_area.rows.len().to_string().len();

                    let line_number = format!("{:width$} ", i + 1, width = max_lines_count_digits);

                    let number_style = if i == edit_area.selected_row() {
                        Style::default()
                    } else {
                        Style::default().combine(Style::shadow())
                    };
                    SpannedString::styled(line_number, number_style)
                } else {
                    SpannedString::default()
                };

                let line = StyledString::concatenate(vec![numbering.clone(), styled]);

                let mut x = 0;
                for span in line.spans() {
                    printer.with_style(
                        ColorStyle::new(span.attr.color.front, PaletteColor::Background),
                        |printer| {
                            printer.print((x, 0), span.content);
                            x += span.content.width();
                        },
                    );
                }

                if printer.focused
                    && i == edit_area.selected_row()
                    && printer.enabled
                    && edit_area.enabled
                {
                    let cursor_offset = edit_area.cursor - row.start;
                    let mut c = StyledString::new();
                    let selected_char = if cursor_offset == text.len() {
                        " "
                    } else {
                        text[cursor_offset..]
                            .graphemes(true)
                            .next()
                            .expect("Found no char!")
                    };
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
        scroll::on_event(
            self,
            event,
            Self::inner_on_event,
            Self::inner_important_area,
        )
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
