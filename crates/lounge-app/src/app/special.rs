use super::*;

impl LoungeApp {
    pub(super) fn render_special(
        &self,
        network: &NetworkState,
        channel: &ChannelState,
        special: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let channel_id = channel.chan.id;
        if self.special_channel.get() != Some(channel_id) {
            self.special_channel.set(Some(channel_id));
            self.special_limit.set(100);
            self.special_sort.set(None);
            self.inputs
                .special_filter
                .update(cx, |input, cx| input.set_content("", cx));
        }
        let data = channel.chan.data.as_ref();
        let mut all_rows: Vec<serde_json::Value> = data
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        if special == "list_channels" && self.config.features.channel_list_tools {
            let filter = self
                .inputs
                .special_filter
                .read(cx)
                .content()
                .trim()
                .to_lowercase();
            if !filter.is_empty() {
                all_rows.retain(|row| {
                    json_string(row, "channel").to_lowercase().contains(&filter)
                        || json_string(row, "topic").to_lowercase().contains(&filter)
                });
            }
        }
        if special == "list_channels"
            && self.config.features.channel_list_tools
            && let Some((sort, ascending)) = self.special_sort.get()
        {
            all_rows.sort_by(|left, right| {
                let ordering = match sort {
                    SpecialSort::Channel => json_string(left, "channel")
                        .to_lowercase()
                        .cmp(&json_string(right, "channel").to_lowercase()),
                    SpecialSort::Users => left
                        .get("num_users")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0)
                        .cmp(
                            &right
                                .get("num_users")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0),
                        ),
                    SpecialSort::Topic => json_string(left, "topic")
                        .to_lowercase()
                        .cmp(&json_string(right, "topic").to_lowercase()),
                };
                if ascending {
                    ordering
                } else {
                    ordering.reverse()
                }
            });
        }
        let total_rows = all_rows.len();
        let rows: Vec<serde_json::Value> = all_rows
            .into_iter()
            .take(self.special_limit.get())
            .collect();
        let status_text = data
            .and_then(|value| value.get("text"))
            .and_then(|value| value.as_str())
            .map(str::to_string);

        let mut headers: Vec<(&str, Option<gpui::Pixels>)> = Vec::new();
        match special {
            "list_bans" => {
                headers = vec![
                    ("Banned", None),
                    ("Banned by", Some(px(180.))),
                    ("Banned at", Some(px(210.))),
                ];
            }
            "list_invites" => {
                headers = vec![
                    ("Invited", None),
                    ("Invited by", Some(px(180.))),
                    ("Invited at", Some(px(210.))),
                ];
            }
            "list_ignored" => {
                headers = vec![
                    ("Hostmask", None),
                    ("Ignored at", Some(px(230.))),
                    ("", Some(px(90.))),
                ];
            }
            "list_channels" => {
                headers = vec![
                    ("Channel", Some(px(240.))),
                    ("Users", Some(px(90.))),
                    ("Topic", None),
                ];
            }
            _ => {}
        }

        let mut table = div().flex().flex_col().w_full();
        if special == "list_channels" && self.config.features.channel_list_tools {
            table = table.child(
                div()
                    .p_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(self.inputs.special_filter.clone()),
            );
        }
        if !headers.is_empty() {
            let mut header_row = div()
                .flex()
                .gap_4()
                .px_4()
                .py_2()
                .border_b_1()
                .border_color(theme.border)
                .text_size(px(11.))
                .text_color(theme.muted);
            for (label, width) in &headers {
                let sort = if special == "list_channels" && self.config.features.channel_list_tools
                {
                    match *label {
                        "Channel" => Some(SpecialSort::Channel),
                        "Users" => Some(SpecialSort::Users),
                        "Topic" => Some(SpecialSort::Topic),
                        _ => None,
                    }
                } else {
                    None
                };
                let arrow = sort
                    .filter(|sort| {
                        self.special_sort
                            .get()
                            .is_some_and(|(current, _)| current == *sort)
                    })
                    .map(|_| {
                        if self
                            .special_sort
                            .get()
                            .is_some_and(|(_, ascending)| ascending)
                        {
                            " ↑"
                        } else {
                            " ↓"
                        }
                    })
                    .unwrap_or_default();
                let mut cell = div()
                    .id(ElementId::Name(format!("special-header-{label}").into()))
                    .when_some(*width, |this, width| this.w(width))
                    .when(width.is_none(), |this| this.flex_1());
                if let Some(sort) = sort {
                    cell = cell
                        .cursor_pointer()
                        .hover(|style| style.text_color(theme.text))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.special_sort.set(match this.special_sort.get() {
                                Some((current, true)) if current == sort => Some((sort, false)),
                                Some((current, false)) if current == sort => None,
                                _ => Some((sort, true)),
                            });
                            this.special_limit.set(100);
                            cx.notify();
                        }));
                }
                header_row = header_row.child(cell.child(format!("{label}{arrow}")));
            }
            table = table.child(header_row);
        }

        if let Some(text) = status_text {
            table = table.child(div().p_4().text_color(theme.muted).child(text));
        } else if special == "list_channels" && data.is_none() {
            table = table.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_4()
                    .text_color(theme.muted)
                    .child(["◐", "◓", "◑", "◒"][self.special_spinner % 4])
                    .child("Loading channels..."),
            );
        } else if rows.is_empty() && !headers.is_empty() {
            table = table.child(div().p_4().text_color(theme.muted).child("No entries."));
        } else {
            for row in rows {
                let mut cells = div()
                    .flex()
                    .gap_4()
                    .px_4()
                    .py(px(5.))
                    .border_b_1()
                    .border_color(theme.border)
                    .items_start();
                match special {
                    "list_bans" => {
                        cells = cells
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .child(json_string(&row, "hostmask")),
                            )
                            .child(div().w(px(180.)).child(json_string(&row, "banned_by")))
                            .child(
                                div()
                                    .w(px(210.))
                                    .text_color(theme.muted)
                                    .child(special_time(row.get("banned_at"))),
                            );
                    }
                    "list_invites" => {
                        cells = cells
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .child(json_string(&row, "hostmask")),
                            )
                            .child(div().w(px(180.)).child(json_string(&row, "invited_by")))
                            .child(
                                div()
                                    .w(px(210.))
                                    .text_color(theme.muted)
                                    .child(special_time(row.get("invited_at"))),
                            );
                    }
                    "list_ignored" => {
                        let hostmask = json_string(&row, "hostmask");
                        let unignore = hostmask.clone();
                        let target = channel.chan.id;
                        cells = cells
                            .child(div().flex_1().min_w(px(0.)).child(hostmask))
                            .child(
                                div()
                                    .w(px(230.))
                                    .text_color(theme.muted)
                                    .child(special_time(row.get("when"))),
                            )
                            .child(
                                div().w(px(90.)).flex().justify_end().child(
                                    div()
                                        .id(ElementId::Name(format!("unignore-{unignore}").into()))
                                        .px_2()
                                        .py(px(1.))
                                        .rounded(px(4.))
                                        .bg(theme.hover)
                                        .text_size(px(11.))
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.border))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(conn) = &this.conn {
                                                conn.input(
                                                    target,
                                                    format!("/unignore {unignore}"),
                                                    None,
                                                );
                                                conn.input(target, "/ignorelist", None);
                                            }
                                            cx.notify();
                                        }))
                                        .child("Unignore"),
                                ),
                            );
                    }
                    "list_channels" => {
                        let name = json_string(&row, "channel");
                        let join_name = name.clone();
                        let exists = network
                            .channels
                            .iter()
                            .find(|channel| channel.chan.name.eq_ignore_ascii_case(&name))
                            .map(|channel| channel.chan.id);
                        let lobby = network.channels.first().map(|channel| channel.chan.id);
                        cells = cells
                            .child(
                                div()
                                    .w(px(240.))
                                    .id(ElementId::Name(format!("special-join-{name}").into()))
                                    .text_color(theme.accent)
                                    .cursor_pointer()
                                    .hover(|style| style.underline())
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if let Some(id) = exists {
                                            this.switch_channel(id, window, cx);
                                        } else if let (Some(conn), Some(lobby)) =
                                            (&this.conn, lobby)
                                        {
                                            conn.input(lobby, format!("/join {join_name}"), None);
                                        }
                                    }))
                                    .child(name),
                            )
                            .child(
                                div().w(px(90.)).text_color(theme.muted).child(
                                    row.get("num_users")
                                        .map(|value| value.to_string())
                                        .unwrap_or_default(),
                                ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .text_color(theme.muted)
                                    .child(json_string(&row, "topic")),
                            );
                    }
                    _ => {
                        cells = cells.child(row.to_string());
                    }
                }
                table = table.child(cells);
            }
        }
        if total_rows > self.special_limit.get() {
            let remaining = total_rows - self.special_limit.get();
            let step = remaining.min(100);
            table = table.child(
                div().p_2().flex().justify_center().child(
                    div()
                        .id("special-more")
                        .tab_index(0)
                        .focus(|style| style.bg(theme.border).text_color(theme.text))
                        .px_3()
                        .py_1()
                        .rounded(px(5.))
                        .bg(theme.panel)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.special_limit
                                .set(this.special_limit.get().saturating_add(step));
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                            if activation_key(event) {
                                this.special_limit
                                    .set(this.special_limit.get().saturating_add(step));
                                cx.notify();
                                cx.stop_propagation();
                            }
                        }))
                        .child(format!("Show {step} more")),
                ),
            );
        }

        div()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .text_size(px(12.))
                    .text_color(theme.muted)
                    .child(format!("{} - read-only", channel.chan.name)),
            )
            .child(
                div()
                    .id("special-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .child(table),
            )
            .into_any_element()
    }

    pub(super) fn start_special_spinner(&mut self, cx: &mut Context<Self>) {
        self.special_spinner_generation = self.special_spinner_generation.wrapping_add(1);
        let generation = self.special_spinner_generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(120))
                    .await;
                let keep = this.update(cx, |this, cx| {
                    if this.special_spinner_generation != generation {
                        return false;
                    }
                    let loading = this.state.active_channel().is_some_and(|(_, channel)| {
                        channel.chan.special.as_deref() == Some("list_channels")
                            && channel.chan.data.is_none()
                    });
                    if !loading {
                        return false;
                    }
                    this.special_spinner = (this.special_spinner + 1) % 4;
                    cx.notify();
                    true
                });
                if !matches!(keep, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }
}

fn json_string(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .map(|value| match value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn special_time(value: Option<&serde_json::Value>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let datetime = if let Some(millis) = value.as_i64() {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis)
    } else if let Some(text) = value.as_str() {
        chrono::DateTime::parse_from_rfc3339(text)
            .ok()
            .map(|datetime| datetime.with_timezone(&chrono::Utc))
    } else {
        None
    };
    datetime
        .map(|datetime| {
            datetime
                .with_timezone(&Local)
                .format("%d %B %Y, %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default()
}
