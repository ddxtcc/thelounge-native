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
            .map(|(_, channel)| {
                channel
                    .users
                    .iter()
                    .filter(|user| {
                        filter.is_empty()
                            || user.nick.to_lowercase().contains(&filter)
                            || user.away.to_lowercase().contains(&filter)
                            || (user.is_bot && "bot".contains(&filter))
                    })
                    .map(|user| user.nick.clone())
                    .collect()
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
        if key == "escape" {
            if self.nav_section != NavSection::Main {
                self.nav_section = NavSection::Main;
                self.sidebar_overlay = false;
                self.users_overlay = false;
                self.focus_navigation(window, cx);
                return true;
            }
            return false;
        }
        match self.nav_section {
            NavSection::Main => match key.as_str() {
                "up" | "k" => {
                    self.scroll_messages_by(px(48.), cx);
                    true
                }
                "down" | "j" => {
                    self.scroll_messages_by(px(-48.), cx);
                    true
                }
                "left" | "h" => self.cycle_navigable_channel(-1, window, cx),
                "right" | "l" => self.cycle_navigable_channel(1, window, cx),
                "enter" => {
                    self.nav_mode = false;
                    self.focus_composer(window, cx);
                    true
                }
                _ => false,
            },
            NavSection::Networks => match key.as_str() {
                "up" | "k" => self.move_nav_network(-1, window, cx),
                "down" | "j" => self.move_nav_network(1, window, cx),
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
