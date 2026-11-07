use super::*;

impl LoungeApp {
    pub(super) fn open_palette(&mut self, mode: PaletteMode, cx: &mut Context<Self>) {
        self.close_overlays(cx);
        self.settings_open = false;
        self.security_open = false;
        self.font_picker = None;
        self.palette = Some(mode);
        self.palette_selection = 0;
        self.palette_scroll = gpui::ScrollHandle::new();
        self.inputs.palette.update(cx, |input, cx| {
            input.set_placeholder(match mode {
                PaletteMode::Commands => "Type a command...",
                PaletteMode::Channels => "Switch to channel...",
                PaletteMode::Themes => "Choose a theme...",
                PaletteMode::Alias => "alias = command label",
                PaletteMode::Users(action) => action.prompt(),
            });
            input.set_content("", cx);
        });
        self.focus_request = Some(InputRole::Palette);
        cx.notify();
    }

    pub(super) fn palette_entries(&self, cx: &mut Context<Self>) -> Vec<(String, PaletteEntry)> {
        let query = self.inputs.palette.read(cx).content().to_lowercase();
        match self.palette {
            Some(PaletteMode::Commands) => {
                let mut definitions = command_defs().to_vec();
                definitions.sort_by_key(|(_, command)| command.section());
                let favorite_commands: Vec<PaletteCommand> = self
                    .config
                    .favorite_commands
                    .iter()
                    .filter_map(|favorite| {
                        definitions
                            .iter()
                            .find(|(label, _)| label.eq_ignore_ascii_case(favorite))
                            .map(|(_, command)| *command)
                    })
                    .collect();
                let mut entries = Vec::new();
                if query.is_empty() {
                    for command in &favorite_commands {
                        if let Some((label, _)) = definitions
                            .iter()
                            .find(|(_, candidate)| candidate == command)
                        {
                            entries.push((label.to_string(), PaletteEntry::Favorite(*command)));
                        }
                    }
                    for command in &self.recent_commands {
                        if favorite_commands.contains(command) {
                            continue;
                        }
                        if let Some((label, _)) = definitions
                            .iter()
                            .find(|(_, candidate)| candidate == command)
                        {
                            entries.push((label.to_string(), PaletteEntry::Recent(*command)));
                        }
                    }
                }
                for (label, command) in definitions {
                    if query.is_empty()
                        && (self.recent_commands.contains(&command)
                            || favorite_commands.contains(&command))
                    {
                        continue;
                    }
                    if label.to_lowercase().contains(&query) {
                        entries.push((label.to_string(), PaletteEntry::Command(command)));
                    }
                }
                if !query.is_empty() {
                    for (alias, target) in &self.config.command_aliases {
                        if !alias.to_lowercase().contains(&query) {
                            continue;
                        }
                        if let Some((label, command)) = command_defs()
                            .iter()
                            .find(|(label, _)| label.eq_ignore_ascii_case(target))
                        {
                            entries.push((
                                format!("{alias} → {label}"),
                                PaletteEntry::Command(*command),
                            ));
                        }
                    }
                }
                entries
            }
            Some(PaletteMode::Channels) => {
                let mut entries = Vec::new();
                for network in &self.state.networks {
                    for channel in &network.channels {
                        let name = if channel.is_lobby() {
                            network.name.clone()
                        } else {
                            channel.chan.name.clone()
                        };
                        let label = format!("{} / {}", network.name, name);
                        if label.to_lowercase().contains(&query) {
                            entries.push((label, PaletteEntry::Channel(channel.chan.id)));
                        }
                    }
                }
                entries
            }
            Some(PaletteMode::Themes) => {
                let mut entries = Vec::new();
                for (id, label) in crate::theme::THEMES {
                    if label.to_lowercase().contains(&query) {
                        entries.push((
                            label.to_string(),
                            PaletteEntry::Theme(id.to_string(), label.to_string()),
                        ));
                    }
                }
                if self.config.custom_theme.is_some() && "custom".contains(&query) {
                    entries.push((
                        "Custom".to_string(),
                        PaletteEntry::Theme("custom".to_string(), "Custom".to_string()),
                    ));
                }
                entries
            }
            Some(PaletteMode::Alias) => Vec::new(),
            Some(PaletteMode::Users(action)) => {
                let Some((network, channel)) = self.state.active_channel() else {
                    return Vec::new();
                };
                let mut entries = Vec::new();
                for user in &channel.users {
                    if action.destructive() && user.nick.eq_ignore_ascii_case(&network.nick) {
                        continue;
                    }
                    let prefix =
                        super::sidebar::user_mode_prefix(user, network).unwrap_or_default();
                    let label = format!("{prefix}{}", user.nick);
                    if label.to_lowercase().contains(&query) {
                        entries.push((
                            label,
                            PaletteEntry::User {
                                action,
                                nick: user.nick.clone(),
                            },
                        ));
                    }
                }
                entries
            }
            None => Vec::new(),
        }
    }

    pub(super) fn move_palette(&mut self, delta: i32, cx: &mut Context<Self>) {
        let len = self.palette_entries(cx).len();
        if len == 0 {
            return;
        }
        let current = self.palette_selection.min(len - 1) as i32;
        self.palette_selection = (current + delta).rem_euclid(len as i32) as usize;
        self.palette_scroll.scroll_to_item(self.palette_selection);
        cx.notify();
    }

    pub(super) fn run_palette_selection(&mut self, cx: &mut Context<Self>) {
        let entries = self.palette_entries(cx);
        let Some((_, entry)) = entries
            .get(self.palette_selection.min(entries.len().saturating_sub(1)))
            .cloned()
        else {
            return;
        };
        self.run_palette_entry(entry, cx);
    }

    pub(super) fn run_palette_entry(&mut self, entry: PaletteEntry, cx: &mut Context<Self>) {
        if let Some(command) = match &entry {
            PaletteEntry::Command(command)
            | PaletteEntry::Recent(command)
            | PaletteEntry::Favorite(command) => Some(*command),
            _ => None,
        } {
            self.remember_palette_command(command);
        }
        self.palette = None;
        self.palette_selection = 0;
        match entry {
            PaletteEntry::User { action, nick } => {
                self.run_palette_user_action(action, &nick, cx);
            }
            PaletteEntry::Command(PaletteCommand::UserAction(action))
            | PaletteEntry::Recent(PaletteCommand::UserAction(action))
            | PaletteEntry::Favorite(PaletteCommand::UserAction(action)) => {
                self.open_palette(PaletteMode::Users(action), cx);
            }
            PaletteEntry::Theme(id, label) => {
                self.config.theme = id;
                self.apply_theme(cx);
                self.config.save();
                self.toast(format!("Theme: {label}."));
                cx.notify();
            }
            PaletteEntry::Channel(id) => {
                self.pending_channel_switch = Some(id);
                cx.notify();
            }
            PaletteEntry::Command(command)
            | PaletteEntry::Recent(command)
            | PaletteEntry::Favorite(command) => match command {
                PaletteCommand::Settings => self.open_settings(cx),
                PaletteCommand::SettingsTab(tab) => {
                    self.settings_tab = tab;
                    self.settings_open = true;
                    cx.notify();
                }
                PaletteCommand::ToggleTray => {
                    self.set_tray_enabled(!self.config.show_tray_icon, cx);
                }
                PaletteCommand::ToggleGlobalHotkey => {
                    self.set_global_hotkey_enabled(!self.config.enable_global_hotkey, cx);
                }
                PaletteCommand::Security => {
                    self.settings_open = false;
                    self.security_open = true;
                    if let Some(conn) = &self.conn {
                        conn.emit("sessions:get", None);
                    }
                    cx.notify();
                }
                PaletteCommand::SwitchChannel => self.open_palette(PaletteMode::Channels, cx),
                PaletteCommand::NextChannel => self.move_palette_channel(1, cx),
                PaletteCommand::PreviousChannel => self.move_palette_channel(-1, cx),
                PaletteCommand::Join => {
                    self.inputs
                        .composer
                        .update(cx, |input, cx| input.set_content("/join ", cx));
                    self.focus_request = Some(InputRole::Composer);
                    cx.notify();
                }
                PaletteCommand::UploadFile => self.prompt_for_upload(cx),
                PaletteCommand::Search => self.open_search(cx),
                PaletteCommand::Mentions => self.open_mentions(cx),
                PaletteCommand::Emoji => self.open_emoji_picker(cx),
                PaletteCommand::Theme => self.open_palette(PaletteMode::Themes, cx),
                PaletteCommand::AddAlias => self.open_palette(PaletteMode::Alias, cx),
                PaletteCommand::Shortcuts => {
                    self.shortcuts_open = true;
                    cx.notify();
                }
                PaletteCommand::ChooseUiFont => self.open_font_picker(FontTarget::Ui, cx),
                PaletteCommand::ChooseMonoFont => self.open_font_picker(FontTarget::Mono, cx),
                PaletteCommand::ImportTheme => self.import_theme(cx),
                PaletteCommand::ExportTheme => self.export_theme(cx),
                PaletteCommand::ToggleSidebar => {
                    if self.narrow {
                        self.sidebar_overlay = !self.sidebar_overlay;
                        self.users_overlay = false;
                    } else {
                        self.config.show_sidebar = !self.config.show_sidebar;
                        self.config.save();
                    }
                    cx.notify();
                }
                PaletteCommand::ToggleUsers => {
                    if self.narrow {
                        self.users_overlay = !self.users_overlay;
                        self.sidebar_overlay = false;
                    } else {
                        self.config.show_user_list = !self.config.show_user_list;
                        self.config.save();
                    }
                    cx.notify();
                }
                PaletteCommand::ToggleInlineImages => {
                    self.config.show_inline_images = !self.config.show_inline_images;
                    self.config.save();
                    cx.notify();
                }
                PaletteCommand::TogglePreviews => {
                    self.config.show_previews = !self.config.show_previews;
                    self.config.save();
                    cx.notify();
                }
                PaletteCommand::ToggleTimestamps => {
                    self.config.show_timestamps = !self.config.show_timestamps;
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::Toggle24Hour => {
                    self.config.use_12h_clock = !self.config.use_12h_clock;
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::ToggleSeconds => {
                    self.config.show_seconds = !self.config.show_seconds;
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::ToggleColoredNicks => {
                    self.config.colored_nicks = !self.config.colored_nicks;
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::ToggleNotifications => {
                    self.config.notifications = !self.config.notifications;
                    self.config.save();
                    cx.notify();
                }
                PaletteCommand::ToggleIconButtons => {
                    self.config.icon_buttons = !self.config.icon_buttons;
                    self.config.save();
                    cx.notify();
                }
                PaletteCommand::ToggleAlignedText => {
                    self.config.nick_column = !self.config.nick_column;
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::CycleStatusMessages => {
                    self.config.status_messages = match self.config.status_messages {
                        StatusMessages::Condensed => StatusMessages::Shown,
                        StatusMessages::Shown => StatusMessages::Hidden,
                        StatusMessages::Hidden => StatusMessages::Condensed,
                    };
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::FontSizeUp => {
                    self.config.font_size = (self.config.font_size + 1.0).min(24.0);
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::FontSizeDown => {
                    self.config.font_size = (self.config.font_size - 1.0).max(11.0);
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::LineHeightUp => {
                    self.config.line_height = (self.config.line_height + 0.05).min(2.2);
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::LineHeightDown => {
                    self.config.line_height = (self.config.line_height - 0.05).max(1.0);
                    self.apply_theme(cx);
                    self.config.save();
                }
                PaletteCommand::MessageSpacingUp => {
                    self.config.message_spacing = (self.config.message_spacing + 1.0).min(8.0);
                    self.config.save();
                    cx.notify();
                }
                PaletteCommand::MessageSpacingDown => {
                    self.config.message_spacing = (self.config.message_spacing - 1.0).max(0.0);
                    self.config.save();
                    cx.notify();
                }
                PaletteCommand::SettingsHighlights => {
                    self.open_settings(cx);
                    self.focus_request = Some(InputRole::SettingHighlights);
                    cx.notify();
                }
                PaletteCommand::SettingsHighlightExceptions => {
                    self.open_settings(cx);
                    self.focus_request = Some(InputRole::SettingHighlightExceptions);
                    cx.notify();
                }
                PaletteCommand::SettingsAwayMessage => {
                    self.open_settings(cx);
                    self.focus_request = Some(InputRole::SettingAwayMessage);
                    cx.notify();
                }
                PaletteCommand::MoveChannelUp => {
                    if let Some(target) = self.state.active
                        && let Some(uuid) = self.state.network_uuid_for_chan(target)
                    {
                        self.move_channel(&uuid, target, -1, cx);
                    }
                }
                PaletteCommand::MoveChannelDown => {
                    if let Some(target) = self.state.active
                        && let Some(uuid) = self.state.network_uuid_for_chan(target)
                    {
                        self.move_channel(&uuid, target, 1, cx);
                    }
                }
                PaletteCommand::AddNetwork => self.open_network_modal(cx),
                PaletteCommand::EditNetwork => {
                    if let Some(uuid) = self.active_network_uuid() {
                        self.begin_edit_network(&uuid, cx);
                    }
                }
                PaletteCommand::ReconnectNetwork => {
                    if let Some(uuid) = self.active_network_uuid() {
                        self.send_network_command(&uuid, "/connect", cx);
                    }
                }
                PaletteCommand::DisconnectNetwork => {
                    if let Some(uuid) = self.active_network_uuid() {
                        self.send_network_command(&uuid, "/disconnect", cx);
                    }
                }
                PaletteCommand::RemoveNetwork => {
                    if let Some(uuid) = self.active_network_uuid() {
                        self.network_menu = None;
                        self.confirm_remove_network = Some(uuid);
                        cx.notify();
                    }
                }
                PaletteCommand::MoveNetworkUp => {
                    if let Some(uuid) = self.active_network_uuid() {
                        self.move_network(&uuid, -1, cx);
                    }
                }
                PaletteCommand::MoveNetworkDown => {
                    if let Some(uuid) = self.active_network_uuid() {
                        self.move_network(&uuid, 1, cx);
                    }
                }
                PaletteCommand::LeaveChannel => {
                    if let Some(target) = self.state.active {
                        let is_query = self
                            .state
                            .find_channel(target)
                            .map(|(_, channel)| channel.is_query())
                            .unwrap_or(false);
                        if let Some(conn) = &self.conn {
                            conn.input(target, if is_query { "/close" } else { "/part" }, None);
                        }
                        cx.notify();
                    }
                }
                PaletteCommand::ToggleMute => {
                    if let Some(target) = self.state.active {
                        let muted = self
                            .state
                            .find_channel(target)
                            .map(|(_, channel)| channel.chan.muted)
                            .unwrap_or(false);
                        if let Some(conn) = &self.conn {
                            conn.emit(
                                "mute:change",
                                Some(serde_json::json!({
                                    "target": target,
                                    "setMutedTo": !muted,
                                })),
                            );
                        }
                        if let Some(channel) = self.state.find_channel_mut(target) {
                            channel.chan.muted = !muted;
                        }
                        cx.notify();
                    }
                }
                PaletteCommand::ClearChannelHistory => {
                    if let Some(target) = self.state.active {
                        if let Some(conn) = &self.conn {
                            conn.emit("history:clear", Some(serde_json::json!({"target": target})));
                        }
                        self.toast("Cleared channel history.");
                    }
                    cx.notify();
                }
                PaletteCommand::EditTopic => {
                    self.inputs
                        .composer
                        .update(cx, |input, cx| input.set_content("/topic ", cx));
                    self.focus_request = Some(InputRole::Composer);
                    cx.notify();
                }
                PaletteCommand::LoadOlder => {
                    let Some(target) = self.state.active else {
                        return;
                    };
                    if let Some(last_id) = self.state.mark_history_requested(target)
                        && let Some(conn) = &self.conn
                    {
                        self.scroll_after_more = true;
                        conn.more(
                            target,
                            last_id,
                            self.config.status_messages != StatusMessages::Shown,
                        );
                        cx.notify();
                    }
                }
                PaletteCommand::JumpToBottom => self.schedule_scroll_bottom(cx),
                PaletteCommand::ClearComposer => {
                    self.inputs.composer.update(cx, |input, cx| input.clear(cx));
                    self.focus_request = Some(InputRole::Composer);
                    cx.notify();
                }
                PaletteCommand::PasteImage => self.paste_image_from_clipboard(cx),
                PaletteCommand::CancelReply => {
                    self.reply_to = None;
                    cx.notify();
                }
                PaletteCommand::CopySelection => {
                    if !self.copy_text_selection(cx) {
                        self.toast("No text selected.");
                    }
                }
                PaletteCommand::SignOut => self.sign_out(cx),
                PaletteCommand::UserAction(_) => {}
            },
        }
    }

    fn move_palette_channel(&mut self, delta: isize, cx: &mut Context<Self>) {
        let ids: Vec<i64> = self
            .state
            .networks
            .iter()
            .flat_map(|network| network.channels.iter().map(|channel| channel.chan.id))
            .collect();
        if ids.is_empty() {
            return;
        }
        let current = self
            .state
            .active
            .and_then(|id| ids.iter().position(|candidate| *candidate == id))
            .unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(ids.len() as isize) as usize;
        self.pending_channel_switch = Some(ids[next]);
        cx.notify();
    }

    pub(super) fn command_is_favorite(&self, command: PaletteCommand) -> bool {
        self.command_label(command).is_some_and(|label| {
            self.config
                .favorite_commands
                .iter()
                .any(|favorite| favorite.eq_ignore_ascii_case(label))
        })
    }

    pub(super) fn toggle_command_favorite(
        &mut self,
        command: PaletteCommand,
        cx: &mut Context<Self>,
    ) {
        let Some(label) = self.command_label(command) else {
            return;
        };
        if let Some(position) = self
            .config
            .favorite_commands
            .iter()
            .position(|favorite| favorite.eq_ignore_ascii_case(label))
        {
            self.config.favorite_commands.remove(position);
        } else {
            self.config.favorite_commands.push(label.to_string());
        }
        self.config.save();
        cx.notify();
    }

    fn command_label(&self, command: PaletteCommand) -> Option<&'static str> {
        command_defs()
            .iter()
            .find(|(_, candidate)| *candidate == command)
            .map(|(label, _)| *label)
    }

    pub(super) fn save_command_alias(&mut self, cx: &mut Context<Self>) {
        let input = self.inputs.palette.read(cx).content().trim().to_string();
        let Some((alias, target)) = input.split_once('=') else {
            self.toast("Use format: alias = command label");
            return;
        };
        let alias = alias.trim().to_lowercase();
        let target = target.trim();
        if alias.is_empty() || target.is_empty() {
            self.toast("Use format: alias = command label");
            return;
        }
        let Some((label, _)) = command_defs()
            .iter()
            .find(|(label, _)| label.eq_ignore_ascii_case(target))
        else {
            self.toast("No command with that label.");
            return;
        };
        self.config
            .command_aliases
            .insert(alias.clone(), label.to_string());
        self.config.save();
        self.palette = None;
        self.toast(format!("Alias {alias} saved."));
        self.pending_focus = true;
        cx.notify();
    }

    fn remember_palette_command(&mut self, command: PaletteCommand) {
        self.recent_commands.retain(|recent| *recent != command);
        self.recent_commands.insert(0, command);
        self.recent_commands.truncate(6);
    }

    fn active_network_uuid(&self) -> Option<String> {
        self.state
            .active
            .and_then(|id| self.state.network_uuid_for_chan(id))
    }

    fn run_palette_user_action(
        &mut self,
        action: PaletteUserAction,
        nick: &str,
        cx: &mut Context<Self>,
    ) {
        if action == PaletteUserAction::Mention {
            self.insert_nick(nick, cx);
            cx.notify();
            return;
        }
        let Some(target) = self.state.active else {
            return;
        };
        if let Some(conn) = &self.conn {
            conn.input(target, format!("/{} {nick}", action.command()), None);
        }
        cx.notify();
    }
}
fn command_defs() -> &'static [(&'static str, PaletteCommand)] {
    &[
        ("Open settings", PaletteCommand::Settings),
        (
            "Settings: Appearance",
            PaletteCommand::SettingsTab(SettingsTab::Appearance),
        ),
        (
            "Settings: Messages",
            PaletteCommand::SettingsTab(SettingsTab::Messages),
        ),
        (
            "Settings: General",
            PaletteCommand::SettingsTab(SettingsTab::General),
        ),
        (
            "Settings: Server",
            PaletteCommand::SettingsTab(SettingsTab::Server),
        ),
        (
            "Settings: About",
            PaletteCommand::SettingsTab(SettingsTab::About),
        ),
        ("Toggle tray icon", PaletteCommand::ToggleTray),
        ("Toggle global shortcut", PaletteCommand::ToggleGlobalHotkey),
        ("Add command alias", PaletteCommand::AddAlias),
        ("Keyboard shortcuts", PaletteCommand::Shortcuts),
        ("Security & sessions", PaletteCommand::Security),
        ("Switch channel", PaletteCommand::SwitchChannel),
        ("Next channel", PaletteCommand::NextChannel),
        ("Previous channel", PaletteCommand::PreviousChannel),
        ("Join a channel", PaletteCommand::Join),
        ("Upload a file", PaletteCommand::UploadFile),
        (
            "Mention user",
            PaletteCommand::UserAction(PaletteUserAction::Mention),
        ),
        (
            "Open query",
            PaletteCommand::UserAction(PaletteUserAction::Query),
        ),
        (
            "Whois user",
            PaletteCommand::UserAction(PaletteUserAction::Whois),
        ),
        ("Give op", PaletteCommand::UserAction(PaletteUserAction::Op)),
        (
            "Remove op",
            PaletteCommand::UserAction(PaletteUserAction::Deop),
        ),
        (
            "Give voice",
            PaletteCommand::UserAction(PaletteUserAction::Voice),
        ),
        (
            "Remove voice",
            PaletteCommand::UserAction(PaletteUserAction::Devoice),
        ),
        (
            "Kick user",
            PaletteCommand::UserAction(PaletteUserAction::Kick),
        ),
        (
            "Ban user",
            PaletteCommand::UserAction(PaletteUserAction::Ban),
        ),
        ("Search messages", PaletteCommand::Search),
        ("Recent mentions", PaletteCommand::Mentions),
        ("Emoji picker", PaletteCommand::Emoji),
        ("Choose theme", PaletteCommand::Theme),
        ("Choose message font", PaletteCommand::ChooseUiFont),
        ("Choose monospace font", PaletteCommand::ChooseMonoFont),
        ("Import custom theme", PaletteCommand::ImportTheme),
        ("Export current theme", PaletteCommand::ExportTheme),
        ("Toggle sidebar", PaletteCommand::ToggleSidebar),
        ("Toggle user list", PaletteCommand::ToggleUsers),
        ("Toggle inline images", PaletteCommand::ToggleInlineImages),
        ("Toggle link previews", PaletteCommand::TogglePreviews),
        ("Toggle timestamps", PaletteCommand::ToggleTimestamps),
        ("Toggle 24-hour clock", PaletteCommand::Toggle24Hour),
        ("Toggle seconds", PaletteCommand::ToggleSeconds),
        (
            "Toggle colored nicknames",
            PaletteCommand::ToggleColoredNicks,
        ),
        (
            "Toggle desktop notifications",
            PaletteCommand::ToggleNotifications,
        ),
        ("Toggle icon buttons", PaletteCommand::ToggleIconButtons),
        (
            "Toggle aligned message text",
            PaletteCommand::ToggleAlignedText,
        ),
        ("Cycle status messages", PaletteCommand::CycleStatusMessages),
        ("Increase font size", PaletteCommand::FontSizeUp),
        ("Decrease font size", PaletteCommand::FontSizeDown),
        ("Increase line height", PaletteCommand::LineHeightUp),
        ("Decrease line height", PaletteCommand::LineHeightDown),
        ("Increase message spacing", PaletteCommand::MessageSpacingUp),
        (
            "Decrease message spacing",
            PaletteCommand::MessageSpacingDown,
        ),
        ("Edit highlight words", PaletteCommand::SettingsHighlights),
        (
            "Edit highlight exceptions",
            PaletteCommand::SettingsHighlightExceptions,
        ),
        ("Edit away message", PaletteCommand::SettingsAwayMessage),
        ("Move current channel up", PaletteCommand::MoveChannelUp),
        ("Move current channel down", PaletteCommand::MoveChannelDown),
        ("Add network", PaletteCommand::AddNetwork),
        ("Edit current network", PaletteCommand::EditNetwork),
        (
            "Reconnect current network",
            PaletteCommand::ReconnectNetwork,
        ),
        (
            "Disconnect current network",
            PaletteCommand::DisconnectNetwork,
        ),
        ("Remove current network", PaletteCommand::RemoveNetwork),
        ("Move current network up", PaletteCommand::MoveNetworkUp),
        ("Move current network down", PaletteCommand::MoveNetworkDown),
        ("Leave current channel", PaletteCommand::LeaveChannel),
        ("Mute/unmute current channel", PaletteCommand::ToggleMute),
        (
            "Clear current channel history",
            PaletteCommand::ClearChannelHistory,
        ),
        ("Edit current channel topic", PaletteCommand::EditTopic),
        ("Load older messages", PaletteCommand::LoadOlder),
        ("Jump to bottom", PaletteCommand::JumpToBottom),
        ("Clear composer", PaletteCommand::ClearComposer),
        ("Paste image from clipboard", PaletteCommand::PasteImage),
        ("Cancel reply", PaletteCommand::CancelReply),
        ("Copy selected text", PaletteCommand::CopySelection),
        ("Sign out", PaletteCommand::SignOut),
    ]
}
