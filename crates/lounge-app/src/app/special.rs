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
        let data = channel.chan.data.clone();
        let rows: Vec<serde_json::Value> = data
            .as_ref()
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        let status_text = data
            .as_ref()
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
                headers = vec![("Hostmask", None), ("Ignored at", Some(px(230.)))];
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
                header_row = header_row.child(
                    div()
                        .when_some(*width, |this, width| this.w(width))
                        .when(width.is_none(), |this| this.flex_1())
                        .child(label.to_string()),
                );
            }
            table = table.child(header_row);
        }

        if let Some(text) = status_text {
            table = table.child(div().p_4().text_color(theme.muted).child(text));
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
                        cells = cells
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .child(json_string(&row, "hostmask")),
                            )
                            .child(
                                div()
                                    .w(px(230.))
                                    .text_color(theme.muted)
                                    .child(special_time(row.get("when"))),
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
