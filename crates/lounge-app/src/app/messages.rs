use super::message_list::{MessageRowsCache, Row, build_rows, nick_column_width};
use super::message_text::{
    build_message_text, is_status_message, select_runs, status_icon, user_mode_prefix_in_message,
    word_range,
};
use super::*;

mod render;

impl LoungeApp {
    pub(super) fn render_messages(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.image_cache_current.borrow_mut().clear();
        let theme = self.theme;
        let Some(active) = self.state.active else {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.muted)
                .child("No channel selected")
                .into_any_element();
        };
        let (rows, nick_width, summary_indent, cache_update) = {
            let Some((network, channel)) = self.state.find_channel(active) else {
                return div().flex_1().into_any_element();
            };
            if let Some(special) = channel.chan.special.clone() {
                return self.render_special(network, channel, &special, cx);
            }
            let more = channel.more_history_available();
            let loading = channel.history_loading;
            let key = (
                channel.chan.id,
                self.state.revision,
                self.config.status_messages,
                more,
                loading,
            );
            let cached = self
                .message_rows_cache
                .as_ref()
                .filter(|cache| {
                    cache.channel_id == key.0
                        && cache.revision == key.1
                        && cache.status == key.2
                        && cache.more == key.3
                        && cache.loading == key.4
                })
                .map(|cache| cache.rows.clone());
            let (rows, cache_update) = if let Some(rows) = cached {
                (rows, None)
            } else {
                let mut rows = build_rows(channel, &self.config);
                if more {
                    rows.insert(0, Row::LoadMore { loading });
                }
                let rows = Rc::new(rows);
                let cache = MessageRowsCache {
                    channel_id: key.0,
                    revision: key.1,
                    status: key.2,
                    more: key.3,
                    loading: key.4,
                    rows: rows.clone(),
                };
                (rows, Some(cache))
            };
            let nick_width = if self.config.nick_column && !self.narrow {
                nick_column_width(channel, self.config.font_size)
            } else {
                0.
            };
            let status_column = if nick_width > 0. { nick_width } else { 22. };
            let summary_indent =
                px(16. + if self.config.show_timestamps { 62. } else { 0. } + status_column + 8.);
            (rows, nick_width, summary_indent, cache_update)
        };
        if let Some(cache) = cache_update {
            self.message_rows_cache = Some(cache);
        }
        let keys = rows.iter().map(|row| self.list_row_key(row)).collect();
        self.sync_message_list(keys);
        if self.scroll_to_bottom_pending {
            self.messages_list.scroll_to(ListOffset {
                item_ix: self.messages_list.item_count(),
                offset_in_item: px(0.),
            });
            self.scroll_to_bottom_pending = false;
            self.messages_at_bottom = true;
        }

        self.trim_image_cache(window, cx);
        if let Some(message_id) = self.pending_focus_message.take() {
            if let Some(index) = self.message_row_index(rows.as_slice(), message_id) {
                self.messages_list.scroll_to_reveal_item(index);
                self.messages_at_bottom = false;
                self.pending_context = None;
            } else if let Some(context) = self.pending_context.take() {
                self.message_context = Some(context);
                window.focus(&self.root_focus);
            } else {
                self.toast("Message is not loaded.");
            }
        }

        let chrome = if self.narrow { 110. } else { 80. };
        let viewport_height = window.window_bounds().get_bounds().size.height;
        let max_list_height = (viewport_height - px(chrome)).max(px(120.));
        let list_height = if self.messages_available > px(0.) {
            self.messages_available.min(max_list_height)
        } else {
            (viewport_height - px(150.)).max(px(120.))
        };
        let unread = self
            .state
            .active_channel()
            .map(|(_, channel)| channel.chan.unread + channel.chan.highlight)
            .unwrap_or(0);
        let entity = cx.entity();
        let item_rows = rows.clone();
        let item_entity = entity.clone();
        let bounds_weak = entity.downgrade();
        let selection_weak = entity.downgrade();
        div()
            .flex()
            .flex_1()
            .w_full()
            .min_h(px(0.))
            .relative()
            .when(
                self.nav_mode && self.nav_section == super::keyboard::NavSection::Main,
                |this| this.border_t_1().border_color(theme.accent),
            )
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(bounds) = bounds.first()
                    && let Some(this) = bounds_weak.upgrade()
                {
                    this.update(cx, |this, _| this.messages_bounds = Some(*bounds));
                }
            })
            .id("messages")
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _: &gpui::MouseUpEvent, _, cx| {
                    if this.selecting_text {
                        this.selecting_text = false;
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(move |event: &gpui::MouseMoveEvent, _, cx| {
                let Some(this) = selection_weak.upgrade() else {
                    return;
                };
                this.update(cx, |this, cx| {
                    if !this.selecting_text || event.pressed_button != Some(MouseButton::Left) {
                        return;
                    }
                    let Some(bounds) = this.messages_bounds else {
                        return;
                    };
                    let edge = px(32.);
                    let delta = if event.position.y < bounds.top() + edge {
                        px(18.)
                    } else if event.position.y > bounds.bottom() - edge {
                        px(-18.)
                    } else {
                        px(0.)
                    };
                    if delta != px(0.) {
                        this.scroll_messages_by(delta, cx);
                    }
                });
            })
            .child(
                list(self.messages_list.clone(), move |index, _, cx| {
                    let rows = item_rows.clone();
                    let entity = item_entity.clone();
                    entity.update(cx, |this, cx| {
                        this.render_list_row(index, &rows, nick_width, summary_indent, cx)
                    })
                })
                .w_full()
                .h(list_height)
                .min_h(px(0.))
                .line_height(relative(self.config.line_height.max(1.3)))
                .text_size(px(self.config.font_size))
                .font_family(self.config.font_family.clone()),
            )
            .when(
                unread > 0 && !self.messages_at_bottom && self.config.features.unread_jump,
                |this| {
                    this.child(
                        div()
                            .id("jump-unread")
                            .absolute()
                            .bottom(px(12.))
                            .right(px(16.))
                            .tab_index(0)
                            .tab_stop(!self.last_overlay_open)
                            .focus(|style| style.bg(theme.hover).text_color(theme.text))
                            .px_3()
                            .py_1()
                            .rounded_full()
                            .bg(theme.accent)
                            .text_color(theme.on_accent)
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .shadow_md()
                            .hover(|style| style.bg(theme.accent.opacity(0.9)))
                            .on_click(cx.listener(|this, _, _, cx| this.jump_to_unread(cx)))
                            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                                if activation_key(event) {
                                    this.jump_to_unread(cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .child(format!("↑ {unread} unread")),
                    )
                },
            )
            .into_any_element()
    }
}
