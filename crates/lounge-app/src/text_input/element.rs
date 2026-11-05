use super::*;

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let range = clamp_range(&self.content, range);

        self.content = format!(
            "{}{}{}",
            &self.content[..range.start],
            new_text,
            &self.content[range.end..]
        );
        let cursor = range.start + new_text.len();
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.marked_range.take();
        self.revision += 1;
        self.last_layout = None;
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let range = clamp_range(&self.content, range);

        self.content = format!(
            "{}{}{}",
            &self.content[..range.start],
            new_text,
            &self.content[range.end..]
        );
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .map(|range| range.start + range.start..range.start + range.end)
            .unwrap_or_else(|| {
                let cursor = range.start + new_text.len();
                cursor..cursor
            });
        self.revision += 1;
        self.last_layout = None;
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let line_ix = self.content[..range.start.min(self.content.len())]
            .matches('\n')
            .count();
        let line_height = window.line_height();
        let x = bounds.left();
        Some(Bounds::from_corners(
            point(x, bounds.top() + line_height * line_ix as f32),
            point(x, bounds.top() + line_height * (line_ix + 1) as f32),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let offset = self.index_for_mouse_position(point);
        Some(self.offset_to_utf16(offset))
    }
}

fn clamp_range(text: &str, range: Range<usize>) -> Range<usize> {
    let start = floor_char_boundary(text, range.start.min(text.len()));
    let end = floor_char_boundary(text, range.end.min(text.len()));
    start..end.max(start)
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    layout: InputLayout,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = self.input.read(cx);
        let line_count = if input.multiline {
            input.content.split('\n').count()
        } else {
            1
        };
        let visible_lines = line_count.min(input.max_lines.max(1));
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (window.line_height() * visible_lines).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();
        let text_color = style.color;
        let placeholder_color = Hsla {
            a: 0.45,
            ..text_color
        };

        let display_full = if input.masked {
            "*".repeat(input.content.chars().count())
        } else {
            input.content.clone()
        };
        let is_placeholder = display_full.is_empty();
        let display = if is_placeholder {
            input.placeholder.to_string()
        } else {
            display_full
        };

        let mut starts = Vec::new();
        let mut ends = Vec::new();
        {
            let mut start = 0;
            for line in input.content.split('\n') {
                starts.push(start);
                ends.push(start + line.len());
                start += line.len() + 1;
            }
        }

        let mut lines = Vec::with_capacity(starts.len());
        let cursor_offset = input.cursor_offset();
        let cursor_line = input.content[..cursor_offset.min(input.content.len())]
            .matches('\n')
            .count();
        let visible_start = if cursor_line >= input.max_lines {
            cursor_line + 1 - input.max_lines
        } else {
            0
        };

        for (line_index, line_start) in starts.iter().enumerate() {
            let line_end = ends[line_index];
            let content_line = &input.content[*line_start..line_end];
            let line_text = if is_placeholder {
                if line_index == 0 {
                    display.clone()
                } else {
                    String::new()
                }
            } else if input.masked {
                "*".repeat(content_line.chars().count())
            } else {
                content_line.to_string()
            };

            let run = TextRun {
                len: line_text.len(),
                font: style.font(),
                color: if is_placeholder {
                    placeholder_color
                } else {
                    text_color
                },
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let runs = match &input.marked_range {
                Some(marked) if !is_placeholder => {
                    if input.masked {
                        vec![run]
                    } else {
                        let marked_start = marked
                            .start
                            .clamp(*line_start, line_end)
                            .saturating_sub(*line_start);
                        let marked_end = marked
                            .end
                            .clamp(*line_start, line_end)
                            .saturating_sub(*line_start);
                        vec![
                            TextRun {
                                len: marked_start,
                                ..run.clone()
                            },
                            TextRun {
                                len: marked_end.saturating_sub(marked_start),
                                underline: Some(UnderlineStyle {
                                    color: Some(run.color),
                                    thickness: px(1.0),
                                    wavy: false,
                                }),
                                ..run.clone()
                            },
                            TextRun {
                                len: line_text.len().saturating_sub(marked_end),
                                ..run
                            },
                        ]
                        .into_iter()
                        .filter(|run| run.len > 0)
                        .collect()
                    }
                }
                _ => vec![run],
            };

            let shaped = if line_text.is_empty() {
                window
                    .text_system()
                    .shape_line(SharedString::from(""), font_size, &[], None)
            } else {
                window.text_system().shape_line(
                    SharedString::from(line_text),
                    font_size,
                    &runs,
                    None,
                )
            };
            lines.push(shaped);
        }

        let selected_range = input.selected_range.clone();
        let mut selections = Vec::new();
        let mut cursor = None;
        {
            let cursor_line = cursor_line.min(lines.len().saturating_sub(1));
            let cursor_content_offset = cursor_offset.saturating_sub(starts[cursor_line]);
            let display_cursor = if is_placeholder {
                0
            } else if input.masked {
                input.content[starts[cursor_line]..cursor_offset.min(ends[cursor_line])]
                    .chars()
                    .count()
                    * '*'.len_utf8()
            } else {
                cursor_content_offset.min(lines[cursor_line].len())
            };
            let cursor_x = lines[cursor_line].x_for_index(display_cursor);
            let y = bounds.top() + line_height * (cursor_line - visible_start) as f32;

            if is_placeholder || selected_range.is_empty() {
                cursor = Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_x, y),
                        size(px(2.), line_height),
                    ),
                    text_color,
                ));
            } else {
                let selection_start_line = input.content
                    [..selected_range.start.min(input.content.len())]
                    .matches('\n')
                    .count();
                let selection_end_line = input.content
                    [..selected_range.end.min(input.content.len())]
                    .matches('\n')
                    .count();
                for line_ix in selection_start_line..=selection_end_line {
                    if line_ix >= lines.len() {
                        break;
                    }
                    let line_start = starts[line_ix];
                    let line_end = ends[line_ix];
                    let start = selected_range.start.max(line_start);
                    let end = selected_range.end.min(line_end);
                    if start >= end {
                        continue;
                    }
                    let display_start = if input.masked {
                        input.content[line_start..start].chars().count() * '*'.len_utf8()
                    } else {
                        start - line_start
                    };
                    let display_end = if input.masked {
                        input.content[line_start..end].chars().count() * '*'.len_utf8()
                    } else {
                        end - line_start
                    };
                    let x1 = lines[line_ix].x_for_index(display_start);
                    let x2 = lines[line_ix].x_for_index(display_end);
                    let y = bounds.top() + line_height * (line_ix - visible_start) as f32;
                    selections.push(fill(
                        Bounds::new(point(bounds.left() + x1, y), size(x2 - x1, line_height)),
                        Hsla {
                            a: 0.25,
                            ..text_color
                        },
                    ));
                }
            }
        }

        let layout = InputLayout {
            lines,
            starts,
            ends,
            bounds,
            visible_start,
            line_height,
        };

        Self::PrepaintState {
            layout,
            cursor,
            selections,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        for selection in prepaint.selections.drain(..) {
            window.paint_quad(selection);
        }

        let line_height = prepaint.layout.line_height;
        for (index, line) in prepaint.layout.lines.iter().enumerate() {
            if index < prepaint.layout.visible_start {
                continue;
            }
            let visible_index = index - prepaint.layout.visible_start;
            let origin = point(
                bounds.left(),
                bounds.top() + line_height * visible_index as f32,
            );
            line.paint(origin, line_height, window, cx).ok();
        }

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        let layout = prepaint.layout.clone();
        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(layout);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("TextInput")
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .text_size(px(self.text_size))
            .line_height(px(self.line_height))
            .text_color(self.colors.text)
            .bg(self.colors.background)
            .border_1()
            .border_color(if self.focus_handle.is_focused(window) {
                self.colors.focus_border
            } else {
                self.colors.border
            })
            .rounded(px(6.))
            .w_full()
            .px(px(10.))
            .py(px(6.))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::word_left_action))
            .on_action(cx.listener(Self::word_right_action))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::delete_word_backward))
            .on_action(cx.listener(Self::delete_word_forward))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::complete))
            .on_action(cx.listener(Self::prev_history))
            .on_action(cx.listener(Self::next_history))
            .on_action(cx.listener(Self::escape))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(TextElement { input: cx.entity() })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
