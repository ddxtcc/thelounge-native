use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum NavSection {
    Main,
    Networks,
    Channels,
    Users,
}

impl LoungeApp {
    fn navigation_available(&self) -> bool {
        self.route == Route::Chat && !self.last_overlay_open
    }

    fn sync_nav_network(&mut self) {
        if let Some(index) = self.state.active_network().and_then(|active| {
            self.state
                .networks
                .iter()
                .position(|n| n.uuid == active.uuid)
        }) {
            self.nav_network = index;
        }
    }

    fn focus_navigation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.nav_mode = true;
        self.pending_focus = false;
        self.focus_request = None;
        window.focus(&self.root_focus);
        cx.notify();
    }

    fn navigation_sections(&self) -> Vec<NavSection> {
        let mut sections = vec![NavSection::Main];
        if self.narrow || self.config.show_sidebar {
            sections.push(NavSection::Networks);
            sections.push(NavSection::Channels);
        }
        if self
            .state
            .active_channel()
            .is_some_and(|(_, channel)| channel.is_channel())
            && (self.narrow || self.config.show_user_list)
        {
            sections.push(NavSection::Users);
        }
        sections
    }

    fn visible_users(&self, cx: &mut Context<Self>) -> Vec<String> {
        let filter = self.inputs.users_filter.read(cx).content().to_lowercase();
        self.state
            .active_channel()
            .map(|(network, channel)| {
                let mut users: Vec<_> = channel
                    .users
                    .iter()
                    .filter(|user| {
                        filter.is_empty()
                            || user.nick.to_lowercase().contains(&filter)
                            || user.away.to_lowercase().contains(&filter)
                            || (user.is_bot && "bot".contains(&filter))
                    })
                    .collect();
                if self.config.sort_users_by_mode {
                    super::sidebar::sort_users(&mut users, network);
                }
                users.into_iter().map(|user| user.nick.clone()).collect()
            })
            .unwrap_or_default()
    }

    fn set_section_overlays(&mut self) {
        if !self.narrow {
            return;
        }
        self.sidebar_overlay = matches!(
            self.nav_section,
            NavSection::Networks | NavSection::Channels
        );
        self.users_overlay = self.nav_section == NavSection::Users;
    }

    fn cycle_navigation(&mut self, delta: i32, window: &mut Window, cx: &mut Context<Self>) {
        let sections = self.navigation_sections();
        let current = sections
            .iter()
            .position(|section| *section == self.nav_section)
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(sections.len() as i32) as usize;
        self.nav_section = sections[next];
        self.sync_nav_network();
        if self.nav_section == NavSection::Users {
            self.nav_user = 0;
        }
        self.set_section_overlays();
        self.focus_navigation(window, cx);
    }

    fn move_nav_user(&mut self, delta: i32, cx: &mut Context<Self>) -> bool {
        let users = self.visible_users(cx);
        if users.is_empty() {
            return false;
        }
        let next = (self.nav_user as i32 + delta).rem_euclid(users.len() as i32) as usize;
        self.nav_user = next;
        cx.notify();
        true
    }

    fn move_nav_network(
        &mut self,
        delta: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let len = self.state.networks.len();
        if len == 0 {
            return false;
        }
        let next = self.nav_network as i32 + delta;
        if next < 0 || next >= len as i32 {
            if delta > 0 {
                self.nav_section = NavSection::Channels;
                self.sync_nav_network();
                self.focus_navigation(window, cx);
                return true;
            }
            return false;
        }
        self.nav_network = next as usize;
        let Some(channel) = self.state.networks[next as usize].channels.first() else {
            self.focus_navigation(window, cx);
            return true;
        };
        let channel = channel.chan.id;
        self.switch_channel_quiet(channel, window, cx);
        self.pending_focus = false;
        self.sync_nav_network();
        self.focus_navigation(window, cx);
        true
    }

    fn move_nav_channel(
        &mut self,
        delta: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(network) = self.state.networks.get(self.nav_network) else {
            return false;
        };
        let ids: Vec<i64> = network
            .channels
            .iter()
            .map(|channel| channel.chan.id)
            .collect();
        if ids.is_empty() {
            return false;
        }
        let current = self
            .state
            .active
            .and_then(|active| ids.iter().position(|id| *id == active))
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(ids.len() as i32) as usize;
        self.switch_channel_quiet(ids[next], window, cx);
        self.pending_focus = false;
        self.focus_navigation(window, cx);
        true
    }

    fn toggle_network_collapsed(
        &mut self,
        expand: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(network) = self.state.networks.get(self.nav_network) else {
            return false;
        };
        let uuid = network.uuid.clone();
        if expand {
            self.collapsed_networks.remove(&uuid);
        } else {
            self.collapsed_networks.insert(uuid);
        }
        self.focus_navigation(window, cx);
        true
    }

    fn cycle_navigable_channel(
        &mut self,
        delta: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let ids: Vec<i64> = self
            .state
            .networks
            .iter()
            .flat_map(|network| network.channels.iter())
            .filter(|channel| channel.chan.special.as_deref() != Some("list_channels"))
            .map(|channel| channel.chan.id)
            .collect();
        if ids.is_empty() {
            return false;
        }
        let current = self
            .state
            .active
            .and_then(|active| ids.iter().position(|id| *id == active))
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(ids.len() as i32) as usize;
        self.switch_channel_quiet(ids[next], window, cx);
        self.pending_focus = false;
        self.focus_navigation(window, cx);
        true
    }

    pub(super) fn handle_overlay_key(
        &mut self,
        key: &str,
        shift: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.last_overlay_open {
            return false;
        }
        let key = key.to_lowercase();
        if self.settings_open {
            let tabs = [
                SettingsTab::Appearance,
                SettingsTab::Messages,
                SettingsTab::General,
                SettingsTab::Server,
                SettingsTab::About,
            ];
            let current = tabs
                .iter()
                .position(|tab| *tab == self.settings_tab)
                .unwrap_or(0) as i32;
            match key.as_str() {
                "tab" => {
                    let delta = if shift { -1 } else { 1 };
                    self.settings_tab =
                        tabs[(current + delta).rem_euclid(tabs.len() as i32) as usize];
                    self.settings_refocus = true;
                    cx.notify();
                    return true;
                }
                "left" | "h" => {
                    self.settings_tab = tabs[(current - 1).rem_euclid(tabs.len() as i32) as usize];
                    self.settings_refocus = true;
                    cx.notify();
                    return true;
                }
                "right" | "l" => {
                    self.settings_tab = tabs[(current + 1).rem_euclid(tabs.len() as i32) as usize];
                    self.settings_refocus = true;
                    cx.notify();
                    return true;
                }
                "up" | "k" => {
                    window.focus_prev();
                    return true;
                }
                "down" | "j" => {
                    window.focus_next();
                    return true;
                }
                _ => {}
            }
            return false;
        }
        if self.lightbox.is_some() {
            match key.as_str() {
                "left" | "h" => {
                    self.move_lightbox(-1, cx);
                    return true;
                }
                "right" | "l" => {
                    self.move_lightbox(1, cx);
                    return true;
                }
                _ => return false,
            }
        }
        if self.palette.is_some()
            || self.mentions_open
            || self.emoji_picker_open
            || self.search_open
        {
            return false;
        }
        match key.as_str() {
            "up" | "k" | "left" | "h" => {
                window.focus_prev();
                true
            }
            "down" | "j" | "right" | "l" => {
                window.focus_next();
                true
            }
            _ => false,
        }
    }

    fn jump_nav_channel(&mut self, end: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(network) = self.state.networks.get(self.nav_network) else {
            return false;
        };
        let Some(channel) = network.channels.get(if end {
            network.channels.len().saturating_sub(1)
        } else {
            0
        }) else {
            return false;
        };
        let id = channel.chan.id;
        self.switch_channel_quiet(id, window, cx);
        self.pending_focus = false;
        self.focus_navigation(window, cx);
        true
    }

    fn keyboard_menu_position(window: &Window) -> Point<Pixels> {
        let position = window.mouse_position();
        let viewport = window.viewport_size();
        if position.x < px(0.)
            || position.y < px(0.)
            || position.x > viewport.width
            || position.y > viewport.height
        {
            gpui::point(px(320.), px(160.))
        } else {
            position
        }
    }

    fn move_message_selection(&mut self, delta: i32, cx: &mut Context<Self>) -> bool {
        let Some((_, channel)) = self.state.active_channel() else {
            return false;
        };
        let messages = &channel.chan.messages;
        if messages.is_empty() {
            return false;
        }
        let current = self
            .focused_message
            .and_then(|id| messages.iter().position(|message| message.id == id));
        let target = match current {
            Some(index) => (index as i32 + delta).clamp(0, messages.len() as i32 - 1) as usize,
            None => messages.len() - 1,
        };
        let id = messages[target].id;
        self.focused_message = Some(id);
        self.pending_focus_message = Some(id);
        self.messages_at_bottom = false;
        cx.notify();
        true
    }

    fn open_selected_message_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.focused_message else {
            return false;
        };
        let Some((_, channel)) = self.state.active_channel() else {
            return false;
        };
        let Some(message) = channel
            .chan
            .messages
            .iter()
            .find(|message| message.id == id)
            .cloned()
        else {
            return false;
        };
        self.message_menu = Some(MessageMenu {
            position: Self::keyboard_menu_position(window),
            channel_id: channel.chan.id,
            message,
        });
        cx.notify();
        true
    }

    fn open_selected_context_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match self.nav_section {
            NavSection::Networks => {
                let Some(network) = self.state.networks.get(self.nav_network) else {
                    return false;
                };
                let uuid = network.uuid.clone();
                self.open_network_menu(&uuid, cx);
                true
            }
            NavSection::Channels => {
                let Some((network, channel)) = self.state.active_channel() else {
                    return false;
                };
                let menu = ChannelMenu {
                    chan_id: channel.chan.id,
                    network_uuid: network.uuid.clone(),
                    name: channel.chan.name.clone(),
                    is_query: channel.is_query(),
                    muted: channel.chan.muted,
                    position: Self::keyboard_menu_position(window),
                };
                self.open_channel_menu(menu, cx);
                true
            }
            NavSection::Users => {
                let Some((network, channel)) = self.state.active_channel() else {
                    return false;
                };
                let network_uuid = network.uuid.clone();
                let chan_id = channel.chan.id;
                let users = self.visible_users(cx);
                let Some(nick) = users
                    .get(self.nav_user.min(users.len().saturating_sub(1)))
                    .cloned()
                else {
                    return false;
                };
                self.open_user_menu(&nick, &network_uuid, chan_id, window.mouse_position(), cx);
                true
            }
            NavSection::Main => self.open_selected_message_menu(window, cx),
        }
    }

    pub(super) fn handle_navigation_key(
        &mut self,
        key: &str,
        shift: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.navigation_available() {
            return false;
        }
        let key = key.to_lowercase();
        if key == "tab" {
            self.cycle_navigation(if shift { -1 } else { 1 }, window, cx);
            return true;
        }
        if window.focused(cx).as_ref() != Some(&self.root_focus) {
            return false;
        }
        if ((key == "f10" && shift) || matches!(key.as_str(), "menu" | "contextmenu" | "apps"))
            && self.open_selected_context_menu(window, cx)
        {
            return true;
        }
        if key == "escape" {
            let had_focus = self.nav_mode
                || self.focused_message.is_some()
                || self.nav_section != NavSection::Main
                || self.sidebar_overlay
                || self.users_overlay;
            if had_focus {
                self.nav_mode = false;
                self.nav_section = NavSection::Main;
                self.focused_message = None;
                self.pending_focus_message = None;
                self.sidebar_overlay = false;
                self.users_overlay = false;
                self.focus_composer(window, cx);
                cx.notify();
                return true;
            }
            return false;
        }
        if !self.nav_mode {
            return false;
        }
        match self.nav_section {
            NavSection::Main => match key.as_str() {
                "up" | "k" => self.move_message_selection(-1, cx),
                "down" | "j" => self.move_message_selection(1, cx),
                "home" => {
                    self.scroll_messages_home(cx);
                    true
                }
                "end" => {
                    self.scroll_messages_end(cx);
                    true
                }
                "left" | "h" => self.cycle_navigable_channel(-1, window, cx),
                "right" | "l" => self.cycle_navigable_channel(1, window, cx),
                "enter" => {
                    if self.open_selected_message_menu(window, cx) {
                        true
                    } else {
                        self.nav_mode = false;
                        self.focus_composer(window, cx);
                        true
                    }
                }
                _ => false,
            },
            NavSection::Networks => match key.as_str() {
                "up" | "k" => self.move_nav_network(-1, window, cx),
                "down" | "j" => self.move_nav_network(1, window, cx),
                "home" => self.move_nav_network(-(self.nav_network as i32), window, cx),
                "end" => {
                    let last = self.state.networks.len().saturating_sub(1) as i32;
                    self.move_nav_network(last - self.nav_network as i32, window, cx)
                }
                "left" | "h" => self.toggle_network_collapsed(false, window, cx),
                "right" | "l" => self.toggle_network_collapsed(true, window, cx),
                "enter" => {
                    self.nav_section = NavSection::Channels;
                    self.sync_nav_network();
                    self.focus_navigation(window, cx);
                    true
                }
                _ => false,
            },
            NavSection::Channels => match key.as_str() {
                "up" | "k" => self.move_nav_channel(-1, window, cx),
                "down" | "j" => self.move_nav_channel(1, window, cx),
                "home" => self.jump_nav_channel(false, window, cx),
                "end" => self.jump_nav_channel(true, window, cx),
                "left" | "h" => self.move_nav_network(-1, window, cx),
                "right" | "l" => self.move_nav_network(1, window, cx),
                "enter" => {
                    self.nav_mode = false;
                    self.focus_composer(window, cx);
                    true
                }
                _ => false,
            },
            NavSection::Users => match key.as_str() {
                "up" | "k" => self.move_nav_user(-1, cx),
                "down" | "j" => self.move_nav_user(1, cx),
                "home" => {
                    let users = self.visible_users(cx);
                    if users.is_empty() {
                        false
                    } else {
                        self.nav_user = 0;
                        cx.notify();
                        true
                    }
                }
                "end" => {
                    let users = self.visible_users(cx);
                    if users.is_empty() {
                        false
                    } else {
                        self.nav_user = users.len() - 1;
                        cx.notify();
                        true
                    }
                }
                "enter" => {
                    let users = self.visible_users(cx);
                    let nick = users
                        .get(self.nav_user.min(users.len().saturating_sub(1)))
                        .cloned();
                    if let Some(nick) = nick {
                        self.nav_mode = false;
                        if self.narrow {
                            self.users_overlay = false;
                        }
                        self.insert_nick(&nick, cx);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
        }
    }
}
