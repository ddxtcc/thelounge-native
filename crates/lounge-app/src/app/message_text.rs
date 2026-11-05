use super::*;

pub(super) fn build_message_text(
    message: &Message,
    network: &NetworkState,
    channel: &ChannelState,
    config: &AppConfig,
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

    let status_like = matches!(
        message.kind,
        MessageType::Join
            | MessageType::Part
            | MessageType::Quit
            | MessageType::Nick
            | MessageType::Kick
            | MessageType::Mode
            | MessageType::Topic
            | MessageType::TopicSetBy
            | MessageType::Away
            | MessageType::Back
            | MessageType::Chghost
            | MessageType::Notice
            | MessageType::Error
            | MessageType::Warn
            | MessageType::ModeChannel
            | MessageType::ModeUser
            | MessageType::Raw
            | MessageType::Whois
            | MessageType::Wallops
            | MessageType::MonospaceBlock
            | MessageType::Note
    );
    let muted = if status_like {
        theme.muted
    } else if message.is_self {
        theme.self_text
    } else {
        theme.text
    };
    let nick_color = if config.colored_nicks && !nick.is_empty() {
        rgb(format::nick_color(&nick)).into()
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

    match message.kind {
        MessageType::Action => {
            builder.push("* ", &font, theme.muted, None, false, false);
            if !nick.is_empty() && !config.nick_column {
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
            if !nick.is_empty() && !config.nick_column {
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
            if !nick.is_empty() && !config.nick_column {
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
        _ => {
            if !nick.is_empty() && !config.nick_column {
                let start = builder.text.len();
                builder.push(&format!("-{nick}- "), &font, muted, None, false, false);
                ranges.push(start..builder.text.len());
                targets.push(ClickTarget::Nick(nick.clone()));
            }
        }
    }

    let mut body = message.text.clone();
    if body.is_empty() {
        body = match message.kind {
            MessageType::Nick => message
                .new_nick
                .as_ref()
                .map(|nick| format!("is now known as {nick}"))
                .unwrap_or_default(),
            MessageType::Kick => message.reason.clone().unwrap_or_default(),
            MessageType::Quit | MessageType::Part => message.reason.clone().unwrap_or_default(),
            MessageType::Error | MessageType::Warn => message
                .error
                .clone()
                .unwrap_or_else(|| message.text.clone()),
            _ => String::new(),
        };
    }

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
