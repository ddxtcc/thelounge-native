use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum NavSection {
    Main,
    Networks,
    Channels,
}

const SECTIONS: [NavSection; 3] = [NavSection::Main, NavSection::Networks, NavSection::Channels];

impl LoungeApp {
    fn navigation_available(&self) -> bool {
        self.route == Route::Chat
            && !self.last_overlay_open
            && !self.sidebar_overlay
            && !self.users_overlay
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
        self.pending_focus = false;
        self.focus_request = None;
        window.focus(&self.root_focus);
        cx.notify();
    }

    fn cycle_navigation(&mut self, delta: i32, window: &mut Window, cx: &mut Context<Self>) {
        let current = SECTIONS
            .iter()
            .position(|section| *section == self.nav_section)
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(SECTIONS.len() as i32) as usize;
        self.nav_section = SECTIONS[next];
        self.sync_nav_network();
        self.focus_navigation(window, cx);
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
        let next = (self.nav_network as i32 + delta).rem_euclid(len as i32) as usize;
        self.nav_network = next;
        let Some(channel) = self.state.networks[next].channels.first() else {
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
                "left" | "h" => {
                    self.cycle_channel(-1, window, cx);
                    self.focus_navigation(window, cx);
                    true
                }
                "right" | "l" => {
                    self.cycle_channel(1, window, cx);
                    self.focus_navigation(window, cx);
                    true
                }
                "enter" => {
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
                    self.focus_composer(window, cx);
                    true
                }
                _ => false,
            },
        }
    }
}
