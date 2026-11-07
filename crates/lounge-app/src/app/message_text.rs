use super::*;

pub(super) fn build_message_text(
    message: &Message,
    network: &NetworkState,
    channel: &ChannelState,
    config: &AppConfig,
    nick_column: bool,
    theme: &Theme,
    dark: bool,
) -> (
    String,
    Vec<TextRun>,
    Vec<std::ops::Range<usize>>,
    Vec<ClickTarget>,
) {
    let font = gpui::font(if config.font_family.is_empty() {
        ".SystemUIFont".to_string()
    } else {
        config.font_family.clone()
    });
    let mono_font = gpui::font(if config.monospace_font.is_empty() {
        "monospace".to_string()
    } else {
        config.monospace_font.clone()
    });
    let _ = (channel, dark);
    let mut builder = RunBuilder::default();
    let mut ranges = Vec::new();
    let mut targets = Vec::new();

    let nick = message
        .from
        .as_ref()
        .map(|from| from.nick.clone())
        .unwrap_or_default();
    let mode = message
        .from
        .as_ref()
        .and_then(|from| user_mode_prefix_in_message(from, network))
        .unwrap_or_default();

    let status_like = is_status_message(message.kind);
    let muted = if status_like {
        theme.muted
    } else if message.is_self {
        theme.self_text
    } else {
        theme.text
    };
    let nick_color = if config.colored_nicks && !nick.is_empty() {
        theme.nick_color(&nick)
    } else {
        theme.text
    };
    let nick_font = font.clone().bold();

    if let Some(reply) = &message.reply_to {
        let label = format!(
            "> {}: {}\n",
            message.reply_to_nick.as_deref().unwrap_or("unknown"),
            message
                .reply_to_text
                .as_deref()
                .map(format::clean)
                .unwrap_or_default()
        );
        let _ = reply;
        builder.push(&label, &font, theme.faint, None, false, false);
    }

    if status_like {
        match message.kind {
            MessageType::Topic => {
                if nick.is_empty() {
                    builder.push("The topic is: ", &font, muted, None, false, false);
                } else {
                    let start = builder.text.len();
                    builder.push(
                        &format!("{mode}{nick}"),
                        &nick_font,
                        nick_color,
                        None,
                        false,
                        false,
                    );
                    ranges.push(start..builder.text.len());
                    targets.push(ClickTarget::Nick(nick.clone()));
                    builder.push(
                        " has changed the topic to: ",
                        &font,
                        muted,
                        None,
                        false,
                        false,
                    );
                }
            }
            MessageType::TopicSetBy => {
                builder.push("Topic set by ", &font, muted, None, false, false);
                if nick.is_empty() {
                    builder.push("an unknown user", &font, muted, None, false, false);
                } else {
                    let start = builder.text.len();
                    builder.push(
                        &format!("{mode}{nick}"),
                        &nick_font,
                        nick_color,
                        None,
                        false,
                        false,
                    );
                    ranges.push(start..builder.text.len());
                    targets.push(ClickTarget::Nick(nick.clone()));
                }
                if let Some(when) = message.when {
                    builder.push(
                        &format!(
                            " on {}",
                            when.with_timezone(&Local).format("%-d %B %Y, %H:%M:%S")
                        ),
                        &font,
                        muted,
                        None,
                        false,
                        false,
                    );
                }
            }
            MessageType::ModeChannel => {
                builder.push("Channel mode is ", &font, muted, None, false, false);
            }
            MessageType::ModeUser => {
                builder.push("Your user mode is ", &font, muted, None, false, false);
            }
            _ => {
                if !nick.is_empty() {
                    let start = builder.text.len();
                    builder.push(
                        &format!("{mode}{nick}"),
                        &nick_font,
                        nick_color,
                        None,
                        false,
                        false,
                    );
                    ranges.push(start..builder.text.len());
                    targets.push(ClickTarget::Nick(nick.clone()));
                    if matches!(
                        message.kind,
                        MessageType::Join | MessageType::Part | MessageType::Quit
                    ) && let Some(hostmask) =
                        message.hostmask.as_ref().filter(|host| !host.is_empty())
                    {
                        builder.push(" (", &font, muted, None, false, false);
                        builder.push(hostmask, &font, theme.muted, None, false, false);
                        builder.push(")", &font, muted, None, false, false);
                    }
                    builder.push(" ", &font, muted, None, false, false);
                }
            }
        }
    } else {
        match message.kind {
            MessageType::Action => {
                builder.push("* ", &font, theme.muted, None, false, false);
                if !nick.is_empty() && !nick_column {
                    let start = builder.text.len();
                    builder.push(
                        &format!("{mode}{nick}"),
                        &nick_font,
                        nick_color,
                        None,
                        false,
                        false,
                    );
                    ranges.push(start..builder.text.len());
                    targets.push(ClickTarget::Nick(nick.clone()));
                    builder.push(" ", &font, muted, None, false, false);
                }
            }
            MessageType::Message => {
                if !nick.is_empty() && !nick_column {
                    let start = builder.text.len();
                    builder.push(
                        &format!("{mode}{nick}"),
                        &nick_font,
                        nick_color,
                        None,
                        false,
                        false,
                    );
                    ranges.push(start..builder.text.len());
                    targets.push(ClickTarget::Nick(nick.clone()));
                    builder.push("  ", &font, theme.muted, None, false, false);
                }
            }
            MessageType::Notice => {
                if !nick.is_empty() && !nick_column {
                    builder.push(
                        &format!("-{nick}- "),
                        &font,
                        theme.muted,
                        None,
                        false,
                        false,
                    );
                }
            }
            MessageType::Unhandled => {
                let command = message.command.clone().unwrap_or_default();
                builder.push(
                    &format!("[{command}] "),
                    &font,
                    theme.muted,
                    None,
                    false,
                    false,
                );
                for param in &message.params {
                    builder.push(param, &font, muted, None, false, false);
                    builder.push(" ", &font, muted, None, false, false);
                }
            }
            _ => {}
        }
    }

    let body = if status_like {
        match message.kind {
            MessageType::Topic => message.text.clone(),
            MessageType::TopicSetBy => String::new(),
            MessageType::ModeChannel => message.text.clone(),
            MessageType::ModeUser => message
                .raw_modes
                .as_ref()
                .map(value_text)
                .unwrap_or_default(),
            _ => message_body(message),
        }
    } else {
        message_body(message)
    };

    if !body.is_empty() {
        let spans = format::parse(&body);
        for span in spans {
            let color = match span.fg {
                IrcColor::Default => muted,
                color => format::color_to_rgb(color)
                    .map(|value| -> Hsla { rgb(value).into() })
                    .unwrap_or(muted),
            };
            let bg = format::color_to_rgb(span.bg).map(|value| -> Hsla { rgb(value).into() });
            let font = if span.bold {
                font.clone().bold()
            } else if span.italic {
                font.clone().italic()
            } else if span.monospace {
                mono_font.clone()
            } else {
                font.clone()
            };
            let links = format::find_links(&span.text);
            if links.is_empty() {
                builder.push_styled(
                    &span.text,
                    &font,
                    color,
                    bg,
                    span.underline,
                    span.strikethrough,
                    span.monospace,
                );
            } else {
                let mut cursor = 0;
                for link in links {
                    if link.range.start > cursor {
                        builder.push_styled(
                            &span.text[cursor..link.range.start],
                            &font,
                            color,
                            bg,
                            span.underline,
                            span.strikethrough,
                            span.monospace,
                        );
                    }
                    let start = builder.text.len();
                    let link_color = if span.fg == IrcColor::Default {
                        theme.accent
                    } else {
                        color
                    };
                    builder.push_styled(
                        &span.text[link.range.clone()],
                        &font,
                        link_color,
                        bg,
                        true,
                        span.strikethrough,
                        span.monospace,
                    );
                    ranges.push(start..builder.text.len());
                    targets.push(ClickTarget::Url(link.url));
                    cursor = link.range.end;
                }
                if cursor < span.text.len() {
                    builder.push_styled(
                        &span.text[cursor..],
                        &font,
                        color,
                        bg,
                        span.underline,
                        span.strikethrough,
                        span.monospace,
                    );
                }
            }
        }
    }

    let runs = builder
        .runs
        .into_iter()
        .map(|mut run| {
            run.font = if run.font.family.is_empty() {
                font.clone()
            } else {
                run.font
            };
            run
        })
        .collect();

    (builder.text, runs, ranges, targets)
}

fn message_body(message: &Message) -> String {
    let text = message.text.as_str();
    let reason = if text.is_empty() {
        message.reason.as_deref().unwrap_or_default()
    } else {
        text
    };
    match message.kind {
        MessageType::Join => "has joined the channel".into(),
        MessageType::Part => with_reason("has left the channel", reason),
        MessageType::Quit => with_reason("has quit", reason),
        MessageType::Nick => message
            .new_nick
            .as_ref()
            .map(|nick| format!("is now known as {nick}"))
            .unwrap_or_default(),
        MessageType::Kick => {
            let target = message
                .target
                .as_ref()
                .map(|target| target.nick.as_str())
                .unwrap_or("a user");
            with_reason(&format!("has kicked {target}"), reason)
        }
        MessageType::Mode => {
            if text.is_empty() {
                "sets mode".into()
            } else {
                format!("sets mode {text}")
            }
        }
        MessageType::ModeChannel => format!("Channel mode is {text}"),
        MessageType::ModeUser => {
            let modes = message
                .raw_modes
                .as_ref()
                .map(value_text)
                .filter(|modes| !modes.is_empty())
                .unwrap_or_else(|| text.to_string());
            format!("Your user mode is {modes}")
        }
        MessageType::Topic => {
            let from_nick = message
                .from
                .as_ref()
                .map(|from| from.nick.as_str())
                .unwrap_or_default();
            if !from_nick.is_empty() {
                if text.is_empty() {
                    "changed the topic".into()
                } else {
                    format!("changed the topic to: {text}")
                }
            } else if text.is_empty() {
                "The topic is empty".into()
            } else {
                format!("The topic is: {text}")
            }
        }
        MessageType::TopicSetBy => match message.when {
            Some(when) => format!(
                "set the topic on {}",
                when.with_timezone(&Local).format("%d %B %Y, %H:%M:%S")
            ),
            None => "set the topic".into(),
        },
        MessageType::Away => {
            if message.is_self {
                text.to_string()
            } else {
                with_reason("is away", reason)
            }
        }
        MessageType::Back => {
            if message.is_self && !text.is_empty() {
                text.to_string()
            } else {
                "is back".into()
            }
        }
        MessageType::Chghost => {
            let mut changes = Vec::new();
            if let Some(ident) = message.new_ident.as_ref().filter(|ident| !ident.is_empty()) {
                changes.push(format!("username to {ident}"));
            }
            if let Some(host) = message.new_host.as_ref().filter(|host| !host.is_empty()) {
                changes.push(format!("hostname to {host}"));
            }
            if changes.is_empty() {
                "has changed hostname".into()
            } else {
                format!("has changed {}", changes.join(" and "))
            }
        }
        MessageType::Invite => {
            let channel = message.channel.as_deref().unwrap_or_default();
            if message.invited_you {
                format!("invited you to {channel}")
            } else {
                let target = message
                    .target
                    .as_ref()
                    .map(|target| target.nick.as_str())
                    .unwrap_or("a user");
                format!("invited {target} to {channel}")
            }
        }
        MessageType::Ctcp | MessageType::CtcpRequest => message
            .ctcp_message
            .clone()
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| text.to_string()),
        MessageType::Error | MessageType::Warn => message
            .error
            .clone()
            .filter(|error| !error.is_empty())
            .unwrap_or_else(|| text.to_string()),
        MessageType::Login => "has logged in".into(),
        MessageType::Logout => "has logged out".into(),
        _ => text.to_string(),
    }
}

fn with_reason(base: &str, reason: &str) -> String {
    if reason.is_empty() {
        base.to_string()
    } else {
        format!("{base} ({reason})")
    }
}

fn value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

pub(super) fn is_status_message(kind: MessageType) -> bool {
    matches!(
        kind,
        MessageType::Away
            | MessageType::Back
            | MessageType::Chghost
            | MessageType::Ctcp
            | MessageType::CtcpRequest
            | MessageType::Error
            | MessageType::Invite
            | MessageType::Join
            | MessageType::Kick
            | MessageType::Login
            | MessageType::Logout
            | MessageType::Mode
            | MessageType::ModeChannel
            | MessageType::ModeUser
            | MessageType::MonospaceBlock
            | MessageType::Nick
            | MessageType::Note
            | MessageType::Notice
            | MessageType::Part
            | MessageType::Quit
            | MessageType::Raw
            | MessageType::Topic
            | MessageType::TopicSetBy
            | MessageType::Wallops
            | MessageType::Warn
            | MessageType::Whois
    )
}

pub(super) fn user_mode_prefix_in_message(
    user: &lounge_core::proto::UserInMessage,
    network: &NetworkState,
) -> Option<String> {
    let options = network.server_options.as_ref()?;
    if !user.mode.is_empty() {
        if let Some(symbol) = options.prefix.mode_to_symbol.get(&user.mode) {
            return Some(symbol.clone());
        }
        for prefix in &options.prefix.prefix {
            if user.mode.contains(&prefix.mode) {
                return Some(prefix.symbol.clone());
            }
        }
    }
    None
}

#[derive(Default)]
pub(super) struct RunBuilder {
    text: String,
    runs: Vec<TextRun>,
}

impl RunBuilder {
    fn push(
        &mut self,
        text: &str,
        font: &gpui::Font,
        color: Hsla,
        background: Option<Hsla>,
        underline: bool,
        strikethrough: bool,
    ) {
        self.push_styled(
            text,
            font,
            color,
            background,
            underline,
            strikethrough,
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_styled(
        &mut self,
        text: &str,
        font: &gpui::Font,
        color: Hsla,
        background: Option<Hsla>,
        underline: bool,
        strikethrough: bool,
        _monospace: bool,
    ) {
        if text.is_empty() {
            return;
        }
        self.runs.push(TextRun {
            len: text.len(),
            font: font.clone(),
            color,
            background_color: background,
            underline: underline.then_some(UnderlineStyle {
                color: Some(color),
                thickness: px(1.),
                wavy: false,
            }),
            strikethrough: strikethrough.then_some(gpui::StrikethroughStyle {
                color: Some(color),
                thickness: px(1.),
            }),
        });
        self.text.push_str(text);
    }
}

pub(super) fn image_fallback(theme: Theme, label: &'static str) -> AnyElement {
    div()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .text_color(theme.muted)
        .child(label)
        .into_any_element()
}

pub(super) fn word_range(text: &str, index: usize) -> Range<usize> {
    let index = index.min(text.len());
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_' || ch == '-';
    let mut start = index;
    while let Some(ch) = text[..start].chars().next_back() {
        if !is_word(ch) {
            break;
        }
        start -= ch.len_utf8();
    }
    let mut end = index;
    while let Some(ch) = text[end..].chars().next() {
        if !is_word(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    start..end
}

pub(super) fn select_runs(runs: Vec<TextRun>, range: Range<usize>, color: Hsla) -> Vec<TextRun> {
    if range.is_empty() {
        return runs;
    }
    let mut result = Vec::with_capacity(runs.len());
    let mut offset = 0;
    for run in runs {
        let start = offset;
        let end = offset + run.len;
        offset = end;
        if end <= range.start || start >= range.end {
            result.push(run);
            continue;
        }
        let selected_start = range.start.max(start);
        let selected_end = range.end.min(end);
        let before = selected_start - start;
        if before > 0 {
            let mut part = run.clone();
            part.len = before;
            result.push(part);
        }
        let inside = selected_end - selected_start;
        if inside > 0 {
            let mut part = run.clone();
            part.len = inside;
            part.background_color = Some(color);
            result.push(part);
        }
        let after = end - selected_end;
        if after > 0 {
            let mut part = run;
            part.len = after;
            result.push(part);
        }
    }
    result
}

pub(super) fn status_icon(kind: MessageType, theme: &Theme) -> (&'static str, Hsla) {
    match kind {
        MessageType::Join
        | MessageType::Login
        | MessageType::Logout
        | MessageType::Nick
        | MessageType::Chghost
        | MessageType::Invite => ("→", theme.success),
        MessageType::Part | MessageType::Quit | MessageType::Kick => ("←", theme.error),
        MessageType::Topic | MessageType::TopicSetBy => ("!", theme.success),
        MessageType::Mode | MessageType::ModeChannel | MessageType::ModeUser => {
            ("*", theme.success)
        }
        MessageType::Away | MessageType::Back => ("o", theme.muted),
        MessageType::Error | MessageType::Warn => ("!", theme.error),
        MessageType::Notice => ("-", theme.muted),
        MessageType::Ctcp | MessageType::CtcpRequest | MessageType::Raw => (">", theme.muted),
        _ => ("*", theme.muted),
    }
}

pub(super) fn date_label(date: chrono::NaiveDate) -> String {
    let today = Local::now().date_naive();
    if date == today {
        "Today".into()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".into()
    } else {
        date.format("%e %B %Y").to_string().trim().to_string()
    }
}

#[cfg(test)]
mod message_tests {
    use super::*;

    #[test]
    fn word_range_handles_unicode_and_boundaries() {
        let text = "hello wörld_test!";
        assert_eq!(word_range(text, 2), 0..5);
        assert_eq!(word_range(text, 7), 6..17);
        assert_eq!(word_range(text, 17), 6..17);
        assert_eq!(word_range(text, 18), 18..18);
    }

    #[test]
    fn word_range_includes_irc_nick_characters() {
        let text = "prefix [nick-name] suffix";
        assert_eq!(word_range(text, 9), 8..17);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lounge_core::proto::UserInMessage;

    fn message(kind: MessageType) -> Message {
        Message {
            kind,
            ..Default::default()
        }
    }

    #[test]
    fn topic_without_author_describes_the_topic() {
        let mut message = message(MessageType::Topic);
        message.text = "He buttons and moo boat".into();
        assert_eq!(
            message_body(&message),
            "The topic is: He buttons and moo boat"
        );
    }

    #[test]
    fn topic_with_author_is_a_status_line() {
        let mut message = message(MessageType::Topic);
        message.text = "new topic".into();
        message.from = Some(UserInMessage {
            nick: "barf".into(),
            ..Default::default()
        });
        assert_eq!(message_body(&message), "changed the topic to: new topic");
    }

    #[test]
    fn topic_set_by_is_not_blank() {
        let mut message = message(MessageType::TopicSetBy);
        message.from = Some(UserInMessage {
            nick: "Shreddy".into(),
            ..Default::default()
        });
        assert_eq!(message_body(&message), "set the topic");
    }

    #[test]
    fn join_is_not_blank() {
        assert_eq!(
            message_body(&message(MessageType::Join)),
            "has joined the channel"
        );
    }
}
