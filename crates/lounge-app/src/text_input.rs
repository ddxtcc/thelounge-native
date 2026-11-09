use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId, Hsla, InspectorElementId,
    KeyBinding, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    PaintQuad, Pixels, Point, ShapedLine, SharedString, Style, TextRun, UTF16Selection,
    UnderlineStyle, Window, actions, div, fill, point, prelude::*, px, relative, size,
};
use unicode_segmentation::UnicodeSegmentation;

actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        SelectHome,
        SelectEnd,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        DeleteWordBackward,
        DeleteWordForward,
        Paste,
        Cut,
        Copy,
        Submit,
        Newline,
        Complete,
        PrevHistory,
        NextHistory,
        PasteImage,
        Escape,
    ]
);

pub fn bind_keys(cx: &mut App) {
    let context = Some("TextInput");
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, context),
        KeyBinding::new("delete", Delete, context),
        KeyBinding::new("left", Left, context),
        KeyBinding::new("right", Right, context),
        KeyBinding::new("shift-left", SelectLeft, context),
        KeyBinding::new("shift-right", SelectRight, context),
        KeyBinding::new("ctrl-a", SelectAll, context),
        KeyBinding::new("super-a", SelectAll, context),
        KeyBinding::new("home", Home, context),
        KeyBinding::new("end", End, context),
        KeyBinding::new("shift-home", SelectHome, context),
        KeyBinding::new("shift-end", SelectEnd, context),
        KeyBinding::new("ctrl-left", WordLeft, context),
        KeyBinding::new("alt-left", WordLeft, context),
        KeyBinding::new("ctrl-right", WordRight, context),
        KeyBinding::new("alt-right", WordRight, context),
        KeyBinding::new("ctrl-shift-left", SelectWordLeft, context),
        KeyBinding::new("alt-shift-left", SelectWordLeft, context),
        KeyBinding::new("ctrl-shift-right", SelectWordRight, context),
        KeyBinding::new("alt-shift-right", SelectWordRight, context),
        KeyBinding::new("ctrl-backspace", DeleteWordBackward, context),
        KeyBinding::new("ctrl-delete", DeleteWordForward, context),
        KeyBinding::new("ctrl-v", Paste, context),
        KeyBinding::new("super-v", Paste, context),
        KeyBinding::new("ctrl-shift-v", PasteImage, context),
        KeyBinding::new("super-shift-v", PasteImage, context),
        KeyBinding::new("ctrl-c", Copy, context),
        KeyBinding::new("super-c", Copy, context),
        KeyBinding::new("ctrl-x", Cut, context),
        KeyBinding::new("super-x", Cut, context),
        KeyBinding::new("enter", Submit, context),
        KeyBinding::new("shift-enter", Newline, context),
        KeyBinding::new("up", PrevHistory, context),
        KeyBinding::new("down", NextHistory, context),
        KeyBinding::new("escape", Escape, context),
    ]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputRole {
    Composer,
    LoginServer,
    LoginUser,
    LoginPassword,
    NetworkName,
    NetworkHost,
    NetworkPort,
    NetworkNick,
    NetworkUsername,
    NetworkRealname,
    NetworkPassword,
    Search,
    SettingHighlights,
    SettingHighlightExceptions,
    SettingAwayMessage,
    EmojiSearch,
    FontSearch,
    UsersFilter,
    Palette,
    PasswordOld,
    PasswordNew,
    PasswordVerify,
    ThemeName,
    ThemeHex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAction {
    Submit,
    Tab,
    PrevHistory,
    NextHistory,
    PasteImage,
    MoveLeft,
    MoveRight,
    Escape,
}

#[derive(Clone)]
pub struct InputColors {
    pub text: Hsla,
    pub background: Hsla,
    pub border: Hsla,
    pub focus_border: Hsla,
}

pub struct TextInput {
    focus_handle: FocusHandle,
    content: String,
    placeholder: SharedString,
    masked: bool,
    multiline: bool,
    tab_completion: bool,
    section_navigation: bool,
    max_lines: usize,
    pub revision: u64,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<InputLayout>,
    is_selecting: bool,
    action: Option<InputAction>,
    colors: InputColors,
    text_size: f32,
    line_height: f32,
}

#[derive(Clone)]
struct InputLayout {
    lines: Vec<ShapedLine>,
    starts: Vec<usize>,
    ends: Vec<usize>,
    bounds: Bounds<Pixels>,
    visible_start: usize,
    line_height: Pixels,
}

impl TextInput {
    pub fn new(
        placeholder: impl Into<SharedString>,
        colors: InputColors,
        multiline: bool,
        masked: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: String::new(),
            placeholder: placeholder.into(),
            masked,
            multiline,
            tab_completion: multiline,
            section_navigation: true,
            max_lines: if multiline { 10 } else { 1 },
            revision: 0,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            is_selecting: false,
            action: None,
            colors,
            text_size: 15.0,
            line_height: 22.0,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_content(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.content = text.into();
        let end = self.content.len();
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        self.revision += 1;
        cx.notify();
    }

    pub fn handle_forwarded_key(
        &mut self,
        keystroke: &gpui::Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = keystroke.key.to_lowercase();
        let shift = keystroke.modifiers.shift;
        let control = keystroke.modifiers.control || keystroke.modifiers.platform;
        let alt = keystroke.modifiers.alt;
        let text = keystroke.key_char.as_deref();
        let word = control || alt;
        match key.as_str() {
            "backspace" => {
                if word {
                    self.delete_word_backward(&DeleteWordBackward, window, cx);
                } else {
                    self.backspace(&Backspace, window, cx);
                }
                true
            }
            "delete" => {
                if word {
                    self.delete_word_forward(&DeleteWordForward, window, cx);
                } else {
                    self.delete(&Delete, window, cx);
                }
                true
            }
            "left" => {
                if word && shift {
                    self.select_word_left(&SelectWordLeft, window, cx);
                } else if word {
                    self.word_left_action(&WordLeft, window, cx);
                } else if shift {
                    self.select_left(&SelectLeft, window, cx);
                } else {
                    self.left(&Left, window, cx);
                }
                true
            }
            "right" => {
                if word && shift {
                    self.select_word_right(&SelectWordRight, window, cx);
                } else if word {
                    self.word_right_action(&WordRight, window, cx);
                } else if shift {
                    self.select_right(&SelectRight, window, cx);
                } else {
                    self.right(&Right, window, cx);
                }
                true
            }
            "home" => {
                if shift {
                    self.select_home(&SelectHome, window, cx);
                } else {
                    self.home(&Home, window, cx);
                }
                true
            }
            "end" => {
                if shift {
                    self.select_end(&SelectEnd, window, cx);
                } else {
                    self.end(&End, window, cx);
                }
                true
            }
            "up" => {
                self.prev_history(&PrevHistory, window, cx);
                true
            }
            "down" => {
                self.next_history(&NextHistory, window, cx);
                true
            }
            "enter" => {
                if shift {
                    self.newline(&Newline, window, cx);
                } else {
                    self.submit(&Submit, window, cx);
                }
                true
            }
            "escape" => {
                self.escape(&Escape, window, cx);
                true
            }
            "a" if control => {
                self.select_all(&SelectAll, window, cx);
                true
            }
            "c" if control => {
                self.copy(&Copy, window, cx);
                true
            }
            "x" if control => {
                self.cut(&Cut, window, cx);
                true
            }
            "v" if control => {
                self.paste(&Paste, window, cx);
                true
            }
            _ => {
                if !control
                    && !alt
                    && let Some(text) = text
                    && !text.is_empty()
                    && !text.chars().all(char::is_control)
                {
                    self.insert_text(text, cx);
                    return true;
                }
                false
            }
        }
    }

    pub fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let range = clamp_range(&self.content, self.selected_range.clone());
        self.content = format!(
            "{}{}{}",
            &self.content[..range.start],
            text,
            &self.content[range.end..]
        );
        let cursor = range.start + text.len();
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.marked_range = None;
        self.revision += 1;
        self.last_layout = None;
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.content.clear();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.revision += 1;
        self.last_layout = None;
        cx.notify();
    }

    pub fn set_colors(&mut self, colors: InputColors) {
        self.colors = colors;
    }

    pub fn set_tab_stop(&mut self, tab_stop: bool) {
        self.focus_handle = self.focus_handle.clone().tab_stop(tab_stop);
    }

    pub fn with_tab_index(mut self, index: isize) -> Self {
        self.focus_handle = self.focus_handle.clone().tab_stop(true).tab_index(index);
        self
    }

    pub fn set_text_size(&mut self, size: f32, line_height: f32) {
        self.text_size = size;
        self.line_height = line_height;
    }

    pub fn set_placeholder(&mut self, placeholder: impl Into<SharedString>) {
        self.placeholder = placeholder.into();
    }

    pub fn take_action(&mut self) -> Option<InputAction> {
        self.action.take()
    }

    pub fn emit_action(&mut self, action: InputAction, cx: &mut Context<Self>) {
        self.action = Some(action);
        cx.notify();
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    fn word_left(&self, offset: usize) -> usize {
        let mut chars = self.content[..offset.min(self.content.len())]
            .char_indices()
            .rev()
            .peekable();
        while let Some((index, ch)) = chars.peek().copied() {
            if ch.is_whitespace() {
                chars.next();
                let _ = index;
            } else {
                break;
            }
        }
        while let Some((index, ch)) = chars.peek().copied() {
            if !ch.is_whitespace() {
                chars.next();
                let _ = index;
            } else {
                break;
            }
        }
        chars
            .peek()
            .map(|(index, _)| {
                index
                    + self.content[*index..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    fn word_right(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.content.len());
        while offset < self.content.len() {
            let ch = self.content[offset..].chars().next().unwrap();
            if ch.is_whitespace() {
                break;
            }
            offset += ch.len_utf8();
        }
        while offset < self.content.len() {
            let ch = self.content[offset..].chars().next().unwrap();
            if ch.is_whitespace() {
                offset += ch.len_utf8();
            } else {
                break;
            }
        }
        offset
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
        self.emit_action(InputAction::MoveLeft, cx);
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
        self.emit_action(InputAction::MoveRight, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.selected_range.end), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    fn word_left_action(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.word_left(self.cursor_offset()), cx);
    }

    fn word_right_action(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.word_right(self.cursor_offset()), cx);
    }

    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.word_left(self.cursor_offset()), cx);
    }

    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.word_right(self.cursor_offset()), cx);
    }

    fn delete_word_backward(
        &mut self,
        _: &DeleteWordBackward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.word_left(self.cursor_offset()), cx);
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete_word_forward(
        &mut self,
        _: &DeleteWordForward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.word_right(self.cursor_offset()), cx);
        self.replace_text_in_range(None, "", window, cx);
    }

    fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.line_start(self.cursor_offset());
        self.select_to(offset, cx);
    }

    fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.line_end(self.cursor_offset());
        self.select_to(offset, cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.line_start(self.cursor_offset());
        self.move_to(offset, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.line_end(self.cursor_offset());
        self.move_to(offset, cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.selected_range.end), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let text = if self.multiline {
                text.replace("\r\n", "\n").replace('\r', "\n")
            } else {
                text.replace(['\r', '\n'], " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let range = clamp_range(&self.content, self.selected_range.clone());
        if !range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.content[range].to_string()));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        let range = clamp_range(&self.content, self.selected_range.clone());
        if !range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.content[range].to_string()));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    fn submit(&mut self, _: &Submit, _: &mut Window, cx: &mut Context<Self>) {
        self.emit_action(InputAction::Submit, cx);
    }

    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline {
            self.replace_text_in_range(None, "\n", window, cx);
        } else {
            self.emit_action(InputAction::Submit, cx);
        }
    }

    fn complete(&mut self, _: &Complete, _: &mut Window, cx: &mut Context<Self>) {
        self.emit_action(InputAction::Tab, cx);
    }

    fn prev_history(&mut self, _: &PrevHistory, _: &mut Window, cx: &mut Context<Self>) {
        self.emit_action(InputAction::PrevHistory, cx);
    }

    fn next_history(&mut self, _: &NextHistory, _: &mut Window, cx: &mut Context<Self>) {
        self.emit_action(InputAction::NextHistory, cx);
    }

    fn paste_image(&mut self, _: &PasteImage, _: &mut Window, cx: &mut Context<Self>) {
        self.emit_action(InputAction::PasteImage, cx);
    }

    fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        self.emit_action(InputAction::Escape, cx);
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        let offset = self.index_for_mouse_position(event.position);
        if event.click_count >= 3 {
            self.selected_range = 0..self.content.len();
            self.selection_reversed = false;
            cx.notify();
        } else if event.click_count == 2 {
            let start = self.word_start(offset);
            let end = self.word_end(offset);
            self.selected_range = start..end;
            self.selection_reversed = false;
            cx.notify();
        } else if event.modifiers.shift {
            self.select_to(offset, cx);
        } else {
            self.move_to(offset, cx);
        }
    }

    fn word_start(&self, offset: usize) -> usize {
        let mut start = offset.min(self.content.len());
        while start > 0 {
            let ch = self.content[..start].chars().next_back().unwrap();
            if ch.is_alphanumeric() || ch == '_' {
                start -= ch.len_utf8();
            } else {
                break;
            }
        }
        start
    }

    fn word_end(&self, offset: usize) -> usize {
        let mut end = offset.min(self.content.len());
        while end < self.content.len() {
            let ch = self.content[end..].chars().next().unwrap();
            if ch.is_alphanumeric() || ch == '_' {
                end += ch.len_utf8();
            } else {
                break;
            }
        }
        end
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.section_navigation
            && event.keystroke.key.eq_ignore_ascii_case("tab")
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
            && !event.keystroke.modifiers.platform
        {
            return;
        }
        if event.keystroke.key.eq_ignore_ascii_case("tab")
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.platform
        {
            if event.keystroke.modifiers.shift {
                window.focus_prev();
            } else if self.tab_completion {
                self.emit_action(InputAction::Tab, cx);
            } else {
                window.focus_next();
            }
            cx.stop_propagation();
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let offset = self.index_for_mouse_position(event.position);
            self.select_to(offset, cx);
        }
    }

    fn line_start(&self, offset: usize) -> usize {
        self.content[..offset.min(self.content.len())]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    fn line_end(&self, offset: usize) -> usize {
        self.content[offset.min(self.content.len())..]
            .find('\n')
            .map(|index| offset + index)
            .unwrap_or(self.content.len())
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let Some(layout) = &self.last_layout else {
            return 0;
        };
        if layout.lines.is_empty() {
            return 0;
        }
        if position.y < layout.bounds.top() {
            return 0;
        }
        let line_height = layout.line_height.max(px(1.)).to_f64();
        let y = (position.y - layout.bounds.top()).to_f64();
        let visible_line =
            ((y / line_height).floor() as usize).min(layout.lines.len().saturating_sub(1));
        let line_ix = (layout.visible_start + visible_line).min(layout.lines.len() - 1);
        let line = &layout.lines[line_ix];
        let x = position.x - layout.bounds.left();
        let display_ix = line.closest_index_for_x(x);

        if self.masked {
            let start = layout.starts[line_ix];
            let content_line = &self.content[start..layout.ends[line_ix]];
            let chars = line.text[..display_ix.min(line.text.len())].chars().count();
            char_to_byte(content_line, chars) + start
        } else {
            (layout.starts[line_ix] + display_ix).min(layout.ends[line_ix])
        }
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        self.content[..offset.min(self.content.len())]
            .chars()
            .map(char::len_utf16)
            .sum()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf16_count = 0;
        for (index, ch) in self.content.char_indices() {
            if utf16_count >= offset {
                return index;
            }
            utf16_count += ch.len_utf16();
        }
        self.content.len()
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

fn char_to_byte(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn utf16_range_to_bytes(text: &str, range: Range<usize>) -> Range<usize> {
    let mut start = None;
    let mut end = None;
    let mut utf16 = 0;
    for (byte, ch) in text.char_indices() {
        if start.is_none() && utf16 >= range.start {
            start = Some(byte);
        }
        if end.is_none() && utf16 >= range.end {
            end = Some(byte);
        }
        if start.is_some() && end.is_some() {
            break;
        }
        utf16 += ch.len_utf16();
    }
    start.unwrap_or(text.len())..end.unwrap_or(text.len())
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn clamp_range(text: &str, range: Range<usize>) -> Range<usize> {
    let start = floor_char_boundary(text, range.start.min(text.len()));
    let end = floor_char_boundary(text, range.end.min(text.len()));
    start..end.max(start)
}

mod element;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_composition_range_maps_into_bytes() {
        assert_eq!(utf16_range_to_bytes("a日本", 0..1), 0..1);
        assert_eq!(utf16_range_to_bytes("a日本", 1..3), 1..7);
        assert_eq!(utf16_range_to_bytes("a日本", 3..3), 7..7);
    }

    #[test]
    fn clamp_range_stays_on_char_boundaries() {
        assert_eq!(clamp_range("a日本", 1..2), 1..1);
        assert_eq!(clamp_range("a日本", 1..3), 1..1);
        assert_eq!(clamp_range("a日本", 1..4), 1..4);
    }
}
