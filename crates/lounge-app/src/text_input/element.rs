use super::*;
use gpui::{AvailableSpace, Font, TextAlign, WrappedLine};

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
            .map(|range| {
                let selected = utf16_range_to_bytes(new_text, range.clone());
                range.start + selected.start..range.start + selected.end
            })
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
        _bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let layout = self.last_layout.as_ref()?;
        if layout.lines.is_empty() {
            return None;
        }
        let line_ix = self.content[..range.start.min(self.content.len())]
            .matches('\n')
            .count()
            .min(layout.lines.len() - 1);
        let line_start = layout.starts.get(line_ix).copied().unwrap_or(0);
        let line_end = layout.ends.get(line_ix).copied().unwrap_or(line_start);
        let offset = range.start.clamp(line_start, line_end);
        let display_index = if self.masked {
            self.content[line_start..offset].chars().count()
        } else {
            offset - line_start
        };
        let line = layout.lines.get(line_ix)?;
        let (row_in_line, position_x) = line_position(line, display_index);
        let row = layout.row_offsets.get(line_ix).copied().unwrap_or(0) + row_in_line;
        if row < layout.visible_start || row >= layout.visible_end {
            return None;
        }
        let visible_line = row - layout.visible_start;
        let x = layout.bounds.left() + position_x;
        let top = layout.bounds.top() + layout.line_height * visible_line as f32;
        Some(Bounds::from_corners(
            point(x, top),
            point(x, top + layout.line_height),
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

struct ShapedInput {
    lines: Vec<WrappedLine>,
    starts: Vec<usize>,
    ends: Vec<usize>,
    /// First visual row of each logical line, plus the total row count at the end.
    row_offsets: Vec<usize>,
    total_rows: usize,
}

impl ShapedInput {
    fn display_index(&self, input: &TextInput, line: usize, offset: usize) -> usize {
        let line_start = self.starts[line];
        let line_end = self.ends[line];
        let offset = offset.clamp(line_start, line_end);
        if input.masked {
            input.content[line_start..offset].chars().count()
        } else {
            offset - line_start
        }
    }
}

fn line_position(line: &WrappedLine, index: usize) -> (usize, Pixels) {
    let index = index.min(line.len());
    let mut row = 0;
    let mut row_start = 0;
    for boundary in line.wrap_boundaries.iter() {
        let Some(run) = line.unwrapped_layout.runs.get(boundary.run_ix) else {
            break;
        };
        let Some(glyph) = run.glyphs.get(boundary.glyph_ix) else {
            break;
        };
        if index < glyph.index {
            break;
        }
        row += 1;
        row_start = glyph.index;
    }
    let row_start_x = line.unwrapped_layout.x_for_index(row_start);
    let x = line.unwrapped_layout.x_for_index(index) - row_start_x;
    (row, x)
}

fn wrapped_row_starts(line: &WrappedLine) -> Vec<usize> {
    let mut starts = Vec::with_capacity(line.wrap_boundaries.len() + 2);
    starts.push(0);
    for boundary in line.wrap_boundaries.iter() {
        let Some(run) = line.unwrapped_layout.runs.get(boundary.run_ix) else {
            continue;
        };
        let Some(glyph) = run.glyphs.get(boundary.glyph_ix) else {
            continue;
        };
        starts.push(glyph.index);
    }
    starts.push(line.len());
    starts
}

fn shape_input(
    input: &TextInput,
    wrap_width: Option<Pixels>,
    font_size: Pixels,
    font: &Font,
    text_color: Hsla,
    window: &Window,
) -> ShapedInput {
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
    let mut row_offsets = Vec::with_capacity(starts.len() + 1);
    let mut total_rows = 0;

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
            font: font.clone(),
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

        let shaped = window
            .text_system()
            .shape_text(
                SharedString::from(line_text),
                font_size,
                &runs,
                wrap_width,
                None,
            )
            .unwrap_or_default()
            .into_iter()
            .next()
            .unwrap_or_default();

        row_offsets.push(total_rows);
        total_rows += shaped.wrap_boundaries.len() + 1;
        lines.push(shaped);
    }

    row_offsets.push(total_rows);

    ShapedInput {
        lines,
        starts,
        ends,
        row_offsets,
        total_rows,
    }
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
        _cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let font = style.font();
        let color = style.color;
        let line_height = window.line_height();
        let input = self.input.clone();

        let mut layout_style = Style::default();
        layout_style.size.width = relative(1.).into();
        let layout_id =
            window.request_measured_layout(layout_style, move |known, available, window, cx| {
                let wrap_width = known.width.or(match available.width {
                    AvailableSpace::Definite(width) => Some(width),
                    _ => None,
                });
                let input = input.read(cx);
                let shaped = shape_input(input, wrap_width, font_size, &font, color, window);
                let rows = shaped.total_rows.min(input.max_lines.max(1)).max(1);
                size(wrap_width.unwrap_or(px(0.)), line_height * rows as f32)
            });
        (layout_id, ())
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
        let font = style.font();
        let line_height = window.line_height();
        let text_color = style.color;
        let is_placeholder = input.content.is_empty();

        let shaped = shape_input(
            input,
            Some(bounds.size.width),
            font_size,
            &font,
            text_color,
            window,
        );
        let max_rows = input.max_lines.max(1);

        let cursor_offset = input.cursor_offset().min(input.content.len());
        let cursor_line = input.content[..cursor_offset]
            .matches('\n')
            .count()
            .min(shaped.lines.len().saturating_sub(1));
        let cursor_display = if is_placeholder {
            0
        } else {
            shaped.display_index(input, cursor_line, cursor_offset)
        };
        let (cursor_row_in_line, cursor_x) =
            line_position(&shaped.lines[cursor_line], cursor_display);
        let cursor_row = shaped.row_offsets[cursor_line] + cursor_row_in_line;
        let visible_start = cursor_row.saturating_sub(max_rows - 1);
        let visible_end = visible_start + max_rows;

        let selected_range = input.selected_range.clone();
        let mut selections = Vec::new();
        let mut cursor = None;

        if is_placeholder || selected_range.is_empty() {
            if cursor_row >= visible_start && cursor_row < visible_end {
                let y = bounds.top() + line_height * (cursor_row - visible_start) as f32;
                cursor = Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_x, y),
                        size(px(2.), line_height),
                    ),
                    text_color,
                ));
            }
        } else {
            let selection_start_line = input.content
                [..selected_range.start.min(input.content.len())]
                .matches('\n')
                .count();
            let selection_end_line = input.content[..selected_range.end.min(input.content.len())]
                .matches('\n')
                .count();
            for line_ix in selection_start_line..=selection_end_line {
                if line_ix >= shaped.lines.len() {
                    break;
                }
                let line_start = shaped.starts[line_ix];
                let line_end = shaped.ends[line_ix];
                let start = selected_range.start.max(line_start);
                let end = selected_range.end.min(line_end);
                if start >= end {
                    continue;
                }
                let display_start = shaped.display_index(input, line_ix, start);
                let display_end = shaped.display_index(input, line_ix, end);
                let line = &shaped.lines[line_ix];
                let row_starts = wrapped_row_starts(line);
                for (row_ix, pair) in row_starts.windows(2).enumerate() {
                    let (row_start, row_end) = (pair[0], pair[1]);
                    let a = display_start.max(row_start);
                    let b = display_end.min(row_end);
                    if a >= b {
                        continue;
                    }
                    let row = shaped.row_offsets[line_ix] + row_ix;
                    if row < visible_start || row >= visible_end {
                        continue;
                    }
                    let row_start_x = line.unwrapped_layout.x_for_index(row_start);
                    let x1 = line.unwrapped_layout.x_for_index(a) - row_start_x;
                    let x2 = line.unwrapped_layout.x_for_index(b) - row_start_x;
                    if x2 <= x1 {
                        continue;
                    }
                    let y = bounds.top() + line_height * (row - visible_start) as f32;
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
            lines: shaped.lines,
            starts: shaped.starts,
            ends: shaped.ends,
            row_offsets: shaped.row_offsets,
            bounds,
            visible_start,
            visible_end,
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
        let visible_start = prepaint.layout.visible_start;
        let visible_end = prepaint.layout.visible_end;
        for (index, line) in prepaint.layout.lines.iter().enumerate() {
            let rows_before = prepaint.layout.row_offsets[index];
            let rows = prepaint.layout.rows_for_line(index);
            if rows_before >= visible_end || rows_before + rows <= visible_start {
                continue;
            }
            let offset_rows = rows_before as i64 - visible_start as i64;
            let origin = point(
                bounds.left(),
                bounds.top() + line_height * offset_rows as f32,
            );
            line.paint(origin, line_height, TextAlign::Left, None, window, cx)
                .ok();
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
            .overflow_hidden()
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
            .on_action(cx.listener(Self::paste_image))
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
