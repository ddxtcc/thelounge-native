use super::*;
use crate::config::{FavouriteChannel, Friend};

impl LoungeApp {
    pub(super) fn render_pinned(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let friends = self.config.friends.clone();
        let favourite_networks = self.config.favourite_networks.clone();
        let favourite_channels = self.config.favourite_channels.clone();
        if friends.is_empty() && favourite_networks.is_empty() && favourite_channels.is_empty() {
            return div().into_any_element();
        }
        let mut section = div().flex().flex_col().gap_1();
        if !friends.is_empty() {
            section = section.child(
                div()
                    .px_2()
                    .pt_1()
                    .text_size(px(10.))
                    .text_color(theme.muted)
                    .child("FRIENDS"),
            );
            for friend in friends {
                let Some(network) = self
                    .state
                    .networks
                    .iter()
                    .find(|network| network.uuid == friend.network)
                else {
                    continue;
                };
                let online = self.friend_online(&friend);
                let network_name = network.name.clone();
                let click_friend = friend.clone();
                let key_friend = friend.clone();
                section = section.child(
                    div()
                        .id(ElementId::Name(
                            format!("friend-{}-{}", friend.network, friend.nick).into(),
                        ))
                        .tab_index(0)
                        .tab_stop(!self.last_overlay_open)
                        .focus(|style| style.bg(theme.border).text_color(theme.text))
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py(px(5.))
                        .rounded(px(5.))
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_friend(&click_friend, window, cx)
                        }))
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, window, cx| {
                                if activation_key(event) {
                                    this.open_friend(&key_friend, window, cx);
                                    cx.stop_propagation();
                                }
                            },
                        ))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener({
                                let friend = friend.clone();
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.pinned_menu = Some(PinnedMenu::Friend(friend.clone()));
                                    cx.notify();
                                }
                            }),
                        )
                        .child(div().w(px(14.)).flex().justify_center().child(
                            div().size(px(7.)).rounded_full().bg(if online {
                                theme.success
                            } else {
                                theme.faint
                            }),
                        ))
                        .child(div().flex_1().truncate().child(friend.nick))
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(theme.muted)
                                .truncate()
                                .child(network_name),
                        ),
                );
            }
        }
        if !favourite_networks.is_empty() || !favourite_channels.is_empty() {
            section = section.child(
                div()
                    .px_2()
                    .pt_1()
                    .text_size(px(10.))
                    .text_color(theme.muted)
                    .child("FAVORITES"),
            );
            for uuid in favourite_networks {
                let Some(network) = self
                    .state
                    .networks
                    .iter()
                    .find(|network| network.uuid == uuid)
                else {
                    continue;
                };
                let name = network.name.clone();
                let online = network.status.connected;
                let uuid_click = uuid.clone();
                let uuid_key = uuid.clone();
                section = section.child(
                    div()
                        .id(ElementId::Name(format!("favourite-network-{uuid}").into()))
                        .tab_index(0)
                        .tab_stop(!self.last_overlay_open)
                        .focus(|style| style.bg(theme.border).text_color(theme.text))
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py(px(5.))
                        .rounded(px(5.))
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_favourite_network(&uuid_click, window, cx)
                        }))
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, window, cx| {
                                if activation_key(event) {
                                    this.open_favourite_network(&uuid_key, window, cx);
                                    cx.stop_propagation();
                                }
                            },
                        ))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener({
                                let uuid = uuid.clone();
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.pinned_menu =
                                        Some(PinnedMenu::FavouriteNetwork(uuid.clone()));
                                    cx.notify();
                                }
                            }),
                        )
                        .child(div().size(px(7.)).rounded_full().bg(if online {
                            theme.success
                        } else {
                            theme.faint
                        }))
                        .child(div().flex_1().truncate().child(name)),
                );
            }
            for favourite in favourite_channels {
                let Some(network) = self
                    .state
                    .networks
                    .iter()
                    .find(|network| network.uuid == favourite.network)
                else {
                    continue;
                };
                let Some(channel) = network
                    .channels
                    .iter()
                    .find(|channel| channel.chan.name.eq_ignore_ascii_case(&favourite.name))
                else {
                    continue;
                };
                let (icon, display_name) = super::sidebar::channel_prefix(&channel.chan.name);
                let network_name = network.name.clone();
                let click_favourite = favourite.clone();
                let key_favourite = favourite.clone();
                section = section.child(
                    div()
                        .id(ElementId::Name(
                            format!("favourite-{}-{}", favourite.network, favourite.name).into(),
                        ))
                        .tab_index(0)
                        .tab_stop(!self.last_overlay_open)
                        .focus(|style| style.bg(theme.border).text_color(theme.text))
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py(px(5.))
                        .rounded(px(5.))
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .when(self.state.active == Some(channel.chan.id), |this| {
                            this.bg(theme.accent_dim)
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_favourite_channel(&click_favourite, window, cx)
                        }))
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, window, cx| {
                                if activation_key(event) {
                                    this.open_favourite_channel(&key_favourite, window, cx);
                                    cx.stop_propagation();
                                }
                            },
                        ))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener({
                                let favourite = favourite.clone();
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.pinned_menu =
                                        Some(PinnedMenu::FavouriteChannel(favourite.clone()));
                                    cx.notify();
                                }
                            }),
                        )
                        .child(
                            div()
                                .w(px(14.))
                                .text_color(if self.state.active == Some(channel.chan.id) {
                                    theme.accent
                                } else {
                                    theme.muted
                                })
                                .child(icon),
                        )
                        .child(div().flex_1().truncate().child(display_name))
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(theme.muted)
                                .truncate()
                                .child(network_name),
                        ),
                );
            }
        }
        section.into_any_element()
    }

    pub(super) fn render_pinned_menu(
        &mut self,
        menu: PinnedMenu,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match menu {
            PinnedMenu::Friend(friend) => {
                let title = friend.nick.clone();
                let mut card = self.menu_card(&title, cx);
                let open = friend.clone();
                card = card.child(self.menu_button("Open query", cx, move |this, window, cx| {
                    this.open_friend(&open, window, cx);
                    this.pinned_menu = None;
                    cx.notify();
                }));
                card = card.child(self.menu_button("Remove friend", cx, move |this, _, cx| {
                    this.toggle_friend(&friend.network, &friend.nick, cx);
                    this.pinned_menu = None;
                    cx.notify();
                }));
                card.child(self.menu_button("Cancel", cx, |this, _, cx| {
                    this.pinned_menu = None;
                    cx.notify();
                }))
                .into_any_element()
            }
            PinnedMenu::FavouriteNetwork(uuid) => {
                let title = self
                    .state
                    .networks
                    .iter()
                    .find(|network| network.uuid == uuid)
                    .map(|network| network.name.clone())
                    .unwrap_or_else(|| uuid.clone());
                let mut card = self.menu_card(&title, cx);
                let network = uuid.clone();
                card = card.child(self.menu_button(
                    "Remove from favourites",
                    cx,
                    move |this, _, cx| {
                        this.toggle_favourite_network(&network, cx);
                        this.pinned_menu = None;
                        cx.notify();
                    },
                ));
                card.child(self.menu_button("Cancel", cx, |this, _, cx| {
                    this.pinned_menu = None;
                    cx.notify();
                }))
                .into_any_element()
            }
            PinnedMenu::FavouriteChannel(favourite) => {
                let mut card = self.menu_card(&favourite.name, cx);
                let channel = favourite.clone();
                card = card.child(self.menu_button(
                    "Remove from favourites",
                    cx,
                    move |this, _, cx| {
                        this.toggle_favourite_channel(&channel.network, &channel.name, cx);
                        this.pinned_menu = None;
                        cx.notify();
                    },
                ));
                card.child(self.menu_button("Cancel", cx, |this, _, cx| {
                    this.pinned_menu = None;
                    cx.notify();
                }))
                .into_any_element()
            }
        }
    }

    fn query_channel(&self, network_uuid: &str, nick: &str) -> Option<i64> {
        self.state
            .networks
            .iter()
            .find(|network| network.uuid == network_uuid)?
            .channels
            .iter()
            .find(|channel| channel.is_query() && channel.chan.name.eq_ignore_ascii_case(nick))
            .map(|channel| channel.chan.id)
    }

    fn friend_online(&self, friend: &Friend) -> bool {
        let Some(network) = self
            .state
            .networks
            .iter()
            .find(|network| network.uuid == friend.network)
        else {
            return false;
        };
        if network.channels.iter().any(|channel| {
            channel.is_query()
                && channel.chan.name.eq_ignore_ascii_case(&friend.nick)
                && channel.chan.is_online == Some(true)
        }) {
            return true;
        }
        network.channels.iter().any(|channel| {
            channel
                .users
                .iter()
                .any(|user| user.nick.eq_ignore_ascii_case(&friend.nick))
        })
    }

    pub(super) fn is_friend(&self, network: &str, nick: &str) -> bool {
        self.config
            .friends
            .iter()
            .any(|friend| friend.network == network && friend.nick.eq_ignore_ascii_case(nick))
    }

    pub(super) fn toggle_friend(&mut self, network: &str, nick: &str, cx: &mut Context<Self>) {
        if self.is_friend(network, nick) {
            self.config.friends.retain(|friend| {
                !(friend.network == network && friend.nick.eq_ignore_ascii_case(nick))
            });
        } else {
            self.config.friends.push(Friend {
                network: network.to_string(),
                nick: nick.to_string(),
            });
            if self.query_channel(network, nick).is_none()
                && let Some(state_network) = self
                    .state
                    .networks
                    .iter()
                    .find(|state_network| state_network.uuid == network)
                && let Some(lobby) = state_network
                    .channels
                    .first()
                    .map(|channel| channel.chan.id)
                && let Some(conn) = &self.conn
            {
                conn.input(lobby, format!("/query {nick}"), None);
            }
        }
        self.config.save();
        cx.notify();
    }

    pub(super) fn is_favourite_network(&self, uuid: &str) -> bool {
        self.config
            .favourite_networks
            .iter()
            .any(|network| network == uuid)
    }

    pub(super) fn toggle_favourite_network(&mut self, uuid: &str, cx: &mut Context<Self>) {
        if self.is_favourite_network(uuid) {
            self.config
                .favourite_networks
                .retain(|network| network != uuid);
        } else {
            self.config.favourite_networks.push(uuid.to_string());
        }
        self.config.save();
        cx.notify();
    }

    pub(super) fn is_favourite_channel(&self, network: &str, name: &str) -> bool {
        self.config
            .favourite_channels
            .iter()
            .any(|channel| channel.network == network && channel.name.eq_ignore_ascii_case(name))
    }

    pub(super) fn toggle_favourite_channel(
        &mut self,
        network: &str,
        name: &str,
        cx: &mut Context<Self>,
    ) {
        if self.is_favourite_channel(network, name) {
            self.config.favourite_channels.retain(|channel| {
                !(channel.network == network && channel.name.eq_ignore_ascii_case(name))
            });
        } else {
            self.config.favourite_channels.push(FavouriteChannel {
                network: network.to_string(),
                name: name.to_string(),
            });
        }
        self.config.save();
        cx.notify();
    }

    fn open_friend(&mut self, friend: &Friend, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(channel) = self.query_channel(&friend.network, &friend.nick) {
            self.switch_channel_quiet(channel, window, cx);
            return;
        }
        if let Some(network) = self
            .state
            .networks
            .iter()
            .find(|network| network.uuid == friend.network)
            && let Some(lobby) = network.channels.first().map(|channel| channel.chan.id)
            && let Some(conn) = &self.conn
        {
            conn.input(lobby, format!("/query {}", friend.nick), None);
        }
    }

    fn open_favourite_network(&mut self, uuid: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(channel) = self
            .state
            .networks
            .iter()
            .find(|network| network.uuid == uuid)
            .and_then(|network| network.channels.first())
            .map(|channel| channel.chan.id)
        else {
            return;
        };
        self.switch_channel_quiet(channel, window, cx);
    }

    fn open_favourite_channel(
        &mut self,
        favourite: &FavouriteChannel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(channel) = self
            .state
            .networks
            .iter()
            .find(|network| network.uuid == favourite.network)
            .and_then(|network| {
                network
                    .channels
                    .iter()
                    .find(|channel| channel.chan.name.eq_ignore_ascii_case(&favourite.name))
            })
            .map(|channel| channel.chan.id)
        else {
            return;
        };
        self.switch_channel_quiet(channel, window, cx);
    }
}
