use super::*;

impl LoungeApp {
    pub(super) fn unread_mentions(&self) -> usize {
        self.state
            .mentions
            .iter()
            .filter(|mention| !self.seen_mentions.contains(&mention.msg_id))
            .count()
    }

    pub(super) fn mark_mentions_seen(&mut self) {
        for mention in &self.state.mentions {
            self.seen_mentions.insert(mention.msg_id);
        }
    }

    pub(super) fn move_mentions(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.state.mentions.len();
        if len == 0 {
            return;
        }
        let current = self.mentions_selection.min(len - 1) as isize;
        self.mentions_selection = (current + delta).rem_euclid(len as isize) as usize;
        self.mentions_scroll.scroll_to_item(self.mentions_selection);
        cx.notify();
    }

    pub(super) fn activate_mentions_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_mention(self.mentions_selection, window, cx);
    }

    pub(super) fn activate_mention(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mention) = self.state.mentions.get(index).cloned() else {
            return;
        };
        if self.state.find_channel(mention.chan_id).is_none() {
            return;
        }
        let channel = self
            .state
            .find_channel(mention.chan_id)
            .map(|(_, channel)| channel.chan.name.clone())
            .unwrap_or_default();
        self.pending_focus_message = Some(mention.msg_id);
        self.pending_context = Some(MessageContext {
            channel,
            nick: mention.from.nick.clone(),
            time: mention.time,
            text: format::clean(&mention.text),
        });
        self.flash_message(mention.msg_id, cx);
        self.switch_channel(mention.chan_id, window, cx);
        self.mentions_open = false;
        cx.notify();
    }

    fn emoji_entries(&self, cx: &mut Context<Self>) -> Vec<&'static crate::emoji::Emoji> {
        let term = self.inputs.emoji_search.read(cx).content().to_string();
        crate::emoji::filter(&term, 240)
    }

    pub(super) fn move_emoji(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.emoji_entries(cx).len();
        if len == 0 {
            return;
        }
        let current = self.emoji_selection.min(len - 1) as isize;
        self.emoji_selection = (current + delta).clamp(0, len as isize - 1) as usize;
        self.emoji_scroll.scroll_to_item(self.emoji_selection);
        cx.notify();
    }

    pub(super) fn insert_selected_emoji(&mut self, cx: &mut Context<Self>) {
        let emoji = self
            .emoji_entries(cx)
            .get(self.emoji_selection)
            .map(|emoji| (emoji.glyph, emoji.shortcode));
        if let Some((glyph, shortcode)) = emoji {
            self.insert_emoji_with_shortcode(glyph, shortcode, cx);
        }
    }

    pub(super) fn move_search(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self
            .state
            .search_results
            .as_ref()
            .map(|results| results.results.len())
            .unwrap_or(0);
        if len == 0 {
            return;
        }
        let current = self.search_selection.min(len - 1) as isize;
        self.search_selection = (current + delta).rem_euclid(len as isize) as usize;
        self.search_scroll.scroll_to_item(self.search_selection);
        cx.notify();
    }

    pub(super) fn activate_search_result(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some((target, message_id, context)) = ({
            let Some(results) = &self.state.search_results else {
                return;
            };
            let Some(result) = results.results.get(index) else {
                return;
            };
            let context = MessageContext {
                channel: results.query.channel_name.clone(),
                nick: result
                    .from
                    .as_ref()
                    .map(|from| from.nick.clone())
                    .unwrap_or_default(),
                time: result.time,
                text: format::clean(&result.text),
            };
            let target = self.state.networks.iter().find_map(|network| {
                if network.uuid != results.query.network_uuid {
                    return None;
                }
                network
                    .channels
                    .iter()
                    .find(|channel| channel.chan.name == results.query.channel_name)
                    .map(|channel| channel.chan.id)
            });
            target.map(|target| (target, result.id, context))
        }) else {
            return;
        };
        self.pending_focus_message = Some(message_id);
        self.pending_context = Some(context);
        self.flash_message(message_id, cx);
        self.search_open = false;
        self.pending_channel_switch = Some(target);
        cx.notify();
    }

    pub(super) fn flash_message(&mut self, message_id: i64, cx: &mut Context<Self>) {
        self.focused_message = Some(message_id);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let _ = this.update(cx, |this, cx| {
                if this.focused_message == Some(message_id) {
                    this.focused_message = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(super) fn submit_search(&mut self, cx: &mut Context<Self>) {
        let term = self.inputs.search.read(cx).content().trim().to_string();
        if term.is_empty() {
            return;
        }
        let Some(active) = self.state.active else {
            return;
        };
        let Some((network, channel)) = self.state.find_channel(active) else {
            return;
        };
        let Some(conn) = &self.conn else {
            return;
        };
        conn.emit(
            "search",
            Some(serde_json::json!({
                "searchTerm": term,
                "networkUuid": network.uuid,
                "channelName": channel.chan.name,
                "offset": 0,
            })),
        );
    }
}
