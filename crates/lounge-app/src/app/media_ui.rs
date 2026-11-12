use gstreamer as gst;
use gstreamer::prelude::*;

use super::media_audio::{
    audio_muted, audio_playing, format_clock, seek_audio, seek_audio_relative, set_audio_volume,
};
use super::media_kind::MediaKind;
use super::tooltip::Tooltip;
use super::*;

fn seek_video(video: &gpui_video_player::Video, bounds: Bounds<Pixels>, x: Pixels) {
    if bounds.size.width <= px(0.) {
        return;
    }
    let fraction = ((x - bounds.origin.x) / bounds.size.width).clamp(0., 1.) as f64;
    let duration = video.duration().as_secs_f64();
    if duration <= 0. {
        return;
    }
    let _ = video.seek(
        std::time::Duration::from_secs_f64(duration * fraction),
        true,
    );
}

fn seek_video_relative(video: &gpui_video_player::Video, delta: f64) {
    let duration = video.duration().as_secs_f64();
    if duration <= 0. {
        return;
    }
    let target = (video.position().as_secs_f64() + delta).clamp(0., duration);
    let _ = video.seek(std::time::Duration::from_secs_f64(target), true);
}

impl LoungeApp {
    pub(super) fn media_icon_button(
        &self,
        icon: &str,
        tooltip: &str,
        active: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let tooltip = tooltip.to_string();
        let color = if active { theme.accent } else { theme.muted };
        let icon = icon.to_string();
        let on_click = std::rc::Rc::new(on_click);
        let click_action = on_click.clone();
        div()
            .id(ElementId::Name(format!("media-{icon}-{tooltip}").into()))
            .tab_index(0)
            .tab_stop(!self.last_overlay_open || self.lightbox.is_some())
            .focus(|style| style.bg(theme.border).text_color(theme.text))
            .size(px(26.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .cursor_pointer()
            .when(active, |this| this.bg(theme.accent_dim))
            .when(!active, |this| this.hover(|style| style.bg(theme.hover)))
            .on_click(cx.listener(move |this, _, window, cx| {
                click_action(this, window, cx);
                cx.stop_propagation();
            }))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if activation_key(event) {
                        on_click(this, window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .tooltip(move |_, cx| cx.new(|_| Tooltip::new(tooltip.clone(), theme)).into())
            .child(
                gpui::svg()
                    .path(format!("icons/{icon}.svg"))
                    .w(px(16.))
                    .h(px(16.))
                    .text_color(color),
            )
            .into_any_element()
    }

    pub(super) fn media_speed_button(
        &self,
        video: &gpui_video_player::Video,
        key: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        const SPEEDS: [f64; 5] = [0.75, 1.0, 1.25, 1.5, 2.0];
        let theme = self.theme;
        let speed = video.speed();
        let label = if speed.fract() == 0. {
            format!("{}x", speed as u32)
        } else {
            format!("{speed}x")
        };
        let next = SPEEDS
            .iter()
            .copied()
            .find(|candidate| *candidate > speed + 0.001)
            .unwrap_or(SPEEDS[0]);
        let click_video = video.clone();
        let key_video = video.clone();
        div()
            .id(ElementId::Name(format!("media-speed-{key}").into()))
            .tab_index(0)
            .tab_stop(!self.last_overlay_open || self.lightbox.is_some())
            .focus(|style| style.bg(theme.border).text_color(theme.text))
            .min_w(px(34.))
            .h(px(26.))
            .px_1()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .cursor_pointer()
            .text_color(if speed == 1.0 {
                theme.muted
            } else {
                theme.accent
            })
            .hover(|style| style.bg(theme.hover))
            .tooltip(move |_, cx| cx.new(|_| Tooltip::new("Playback speed", theme)).into())
            .on_click(cx.listener(move |this, _, _, cx| {
                if click_video.set_speed(next).is_err() {
                    this.toast("Could not change playback speed.");
                }
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if activation_key(event) {
                    if key_video.set_speed(next).is_err() {
                        this.toast("Could not change playback speed.");
                    }
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .child(label)
            .into_any_element()
    }

    pub(super) fn toggle_media_fullscreen(&mut self, cx: &mut Context<Self>) {
        if self.media_fullscreen {
            self.media_fullscreen = false;
            cx.notify();
            return;
        }
        if let Some(inline) = self.inline_video.take() {
            self.lightbox = Some(inline.url.clone());
            self.media_video = Some((inline.url, inline.video));
        } else if let Some(inline) = self.inline_audio.take() {
            self.lightbox = Some(inline.url.clone());
            self.media_audio_url = Some(inline.url);
            self.media_audio = Some(inline.pipeline);
        }
        self.media_fullscreen = self.media_video.is_some() || self.media_audio.is_some();
        cx.notify();
    }

    pub(super) fn render_video_progress(
        &self,
        video: &gpui_video_player::Video,
        key: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let position = video.position().as_secs();
        let duration = video.duration().as_secs();
        let ratio = if duration > 0 {
            (position as f32 / duration as f32).clamp(0., 1.)
        } else {
            0.
        };
        let time = if duration > 0 {
            format!("{} / {}", format_clock(position), format_clock(duration))
        } else {
            "0:00 / --:--".to_string()
        };
        let bounds: std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>> =
            std::rc::Rc::new(std::cell::RefCell::new(None));
        let prepaint_bounds = bounds.clone();
        let click_bounds = bounds.clone();
        let move_bounds = bounds.clone();
        let click_video = video.clone();
        let move_video = video.clone();
        let key_video = video.clone();
        let label = "Seek";
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .h(px(9.))
                    .w_full()
                    .flex()
                    .items_center()
                    .on_children_prepainted(move |bounds, _, _| {
                        *prepaint_bounds.borrow_mut() = bounds.first().cloned();
                    })
                    .id(ElementId::Name(format!("media-video-seek-{key}").into()))
                    .tab_index(0)
                    .tab_stop(!self.last_overlay_open || self.lightbox.is_some())
                    .focus(|style| style.bg(theme.border).text_color(theme.text))
                    .cursor_pointer()
                    .tooltip(move |_, cx| cx.new(|_| Tooltip::new(label, theme)).into())
                    .on_key_down(cx.listener(move |_, event: &gpui::KeyDownEvent, _, cx| {
                        let key = event.keystroke.key.to_lowercase();
                        let delta = if key == "left" {
                            -5.0
                        } else if key == "right" {
                            5.0
                        } else {
                            0.
                        };
                        if delta != 0. {
                            seek_video_relative(&key_video, delta);
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |_, event: &gpui::MouseDownEvent, _, cx| {
                            let Some(bounds) = *click_bounds.borrow() else {
                                return;
                            };
                            seek_video(&click_video, bounds, event.position.x);
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(move |_, event: &gpui::MouseMoveEvent, _, cx| {
                        if event.pressed_button != Some(gpui::MouseButton::Left) {
                            return;
                        }
                        let Some(bounds) = *move_bounds.borrow() else {
                            return;
                        };
                        seek_video(&move_video, bounds, event.position.x);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .h(px(5.))
                            .w_full()
                            .rounded_full()
                            .overflow_hidden()
                            .bg(theme.border)
                            .child(
                                div()
                                    .h_full()
                                    .rounded_full()
                                    .bg(theme.accent)
                                    .w(relative(ratio)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(10.))
                    .text_color(theme.muted)
                    .child(time),
            )
            .into_any_element()
    }

    pub(super) fn render_audio_preview(
        &self,
        message_id: i64,
        url: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let play_url = url.to_string();
        let expand_url = url.to_string();
        let download_url = url.to_string();
        let file = url.rsplit('/').next().unwrap_or(url).to_string();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        gpui::svg()
                            .path("icons/music.svg")
                            .w(px(14.))
                            .h(px(14.))
                            .text_color(self.theme.accent),
                    )
                    .child(div().flex_1().truncate().child(file)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .h(px(5.))
                            .w_full()
                            .rounded_full()
                            .bg(self.theme.border),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(10.))
                            .text_color(self.theme.muted)
                            .child("0:00 / --:--")
                            .child(
                                div()
                                    .w(px(54.))
                                    .h(px(3.))
                                    .rounded_full()
                                    .bg(self.theme.border),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_1()
                            .child(self.media_icon_button(
                                "play",
                                "Play",
                                false,
                                cx,
                                move |this, _, cx| {
                                    this.play_inline_media(
                                        message_id,
                                        play_url.clone(),
                                        MediaKind::Audio,
                                        cx,
                                    );
                                },
                            ))
                            .child(self.media_icon_button(
                                if self.config.media_muted {
                                    "volume-x"
                                } else {
                                    "volume-2"
                                },
                                if self.config.media_muted {
                                    "Unmute"
                                } else {
                                    "Mute"
                                },
                                false,
                                cx,
                                |this, _, cx| {
                                    let muted = !this.config.media_muted;
                                    this.config.media_muted = muted;
                                    this.config.media_muted = muted;
                                    this.config.save();
                                    cx.notify();
                                },
                            ))
                            .child(self.media_icon_button(
                                "expand",
                                "Expand",
                                false,
                                cx,
                                move |this, _, cx| this.open_lightbox(expand_url.clone(), cx),
                            ))
                            .child(self.media_icon_button(
                                "download",
                                "Download",
                                false,
                                cx,
                                move |this, _, cx| this.download_media(download_url.clone(), cx),
                            )),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn render_audio_controls(
        &self,
        pipeline: &gst::Pipeline,
        expand_url: Option<String>,
        download_url: Option<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let media_key = pipeline.as_ptr() as usize;
        let toggle = pipeline.clone();
        let mute = pipeline.clone();
        let paused = !audio_playing(pipeline);
        let muted = audio_muted(pipeline);
        let position = pipeline
            .query_position::<gst::ClockTime>()
            .map(|time| time.seconds())
            .unwrap_or(0);
        let duration = pipeline
            .query_duration::<gst::ClockTime>()
            .map(|time| time.seconds())
            .unwrap_or(0);
        let ratio = if duration > 0 {
            (position as f64 / duration as f64).clamp(0., 1.) as f32
        } else {
            0.
        };
        let time = if duration > 0 {
            format!(
                "{} / {}",
                format_clock(position.min(duration)),
                format_clock(duration)
            )
        } else {
            "0:00 / --:--".to_string()
        };
        let volume = pipeline.property::<f64>("volume").clamp(0., 1.) as f32;
        let volume_bounds: std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>> =
            std::rc::Rc::new(std::cell::RefCell::new(None));
        let volume_prepaint = volume_bounds.clone();
        let volume_pipeline = pipeline.clone();
        let volume_move_pipeline = pipeline.clone();
        let volume_save = pipeline.clone();
        let volume_key = pipeline.clone();
        let volume_click_bounds = volume_bounds.clone();
        let volume_move_bounds = volume_bounds.clone();
        let track_bounds: std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>> =
            std::rc::Rc::new(std::cell::RefCell::new(None));
        let prepaint_bounds = track_bounds.clone();
        let seek = pipeline.clone();
        let seek_key = seek.clone();
        let seek_move = seek.clone();
        let seek_bounds = track_bounds.clone();
        let seek_move_bounds = track_bounds.clone();
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .h(px(9.))
                    .w_full()
                    .flex()
                    .items_center()
                    .on_children_prepainted(move |bounds, _, _| {
                        *prepaint_bounds.borrow_mut() = bounds.first().cloned();
                    })
                    .id(ElementId::Name(format!("media-seek-{media_key}").into()))
                    .tab_index(0)
                    .tab_stop(!self.last_overlay_open || self.lightbox.is_some())
                    .focus(|style| style.bg(theme.border).text_color(theme.text))
                    .tooltip(move |_, cx| cx.new(|_| Tooltip::new("Seek", theme)).into())
                    .on_key_down(cx.listener(move |_, event: &gpui::KeyDownEvent, _, cx| {
                        let key = event.keystroke.key.to_lowercase();
                        let delta = if key == "left" {
                            -5
                        } else if key == "right" {
                            5
                        } else {
                            0
                        };
                        if delta != 0 {
                            seek_audio_relative(&seek_key, delta);
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |_, event: &gpui::MouseDownEvent, _, cx| {
                            let Some(bounds) = *seek_bounds.borrow() else {
                                return;
                            };
                            seek_audio(&seek, bounds, event.position.x);
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(move |_, event: &gpui::MouseMoveEvent, _, cx| {
                        if event.pressed_button != Some(gpui::MouseButton::Left) {
                            return;
                        }
                        let Some(bounds) = *seek_move_bounds.borrow() else {
                            return;
                        };
                        seek_audio(&seek_move, bounds, event.position.x);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .h(px(5.))
                            .w_full()
                            .rounded_full()
                            .bg(self.theme.border)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(ratio))
                                    .rounded_full()
                                    .bg(self.theme.accent),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(10.))
                    .text_color(self.theme.muted)
                    .child(time)
                    .child(
                        div().flex().items_center().gap_1().child(
                            div()
                                .h(px(9.))
                                .w(px(54.))
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .on_children_prepainted(move |bounds, _, _| {
                                    *volume_prepaint.borrow_mut() = bounds.first().cloned();
                                })
                                .id(ElementId::Name(format!("media-volume-{media_key}").into()))
                                .tab_index(0)
                                .tab_stop(!self.last_overlay_open || self.lightbox.is_some())
                                .focus(|style| style.bg(theme.border).text_color(theme.text))
                                .tooltip(move |_, cx| {
                                    cx.new(|_| Tooltip::new("Volume", theme)).into()
                                })
                                .on_key_down(cx.listener(
                                    move |this, event: &gpui::KeyDownEvent, _, cx| {
                                        let key = event.keystroke.key.to_lowercase();
                                        let delta = if key == "left" {
                                            -0.05
                                        } else if key == "right" {
                                            0.05
                                        } else {
                                            0.
                                        };
                                        if delta != 0. {
                                            let volume = (volume_key.property::<f64>("volume")
                                                + delta)
                                                .clamp(0., 1.);
                                            volume_key.set_property("volume", volume);
                                            this.config.media_volume = volume as f32;
                                            this.config.save();
                                            cx.stop_propagation();
                                            cx.notify();
                                        }
                                    },
                                ))
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |_, event: &gpui::MouseDownEvent, _, cx| {
                                        let Some(bounds) = *volume_click_bounds.borrow() else {
                                            return;
                                        };
                                        set_audio_volume(
                                            &volume_pipeline,
                                            bounds,
                                            event.position.x,
                                        );
                                        cx.notify();
                                    }),
                                )
                                .on_mouse_move(cx.listener(
                                    move |_, event: &gpui::MouseMoveEvent, _, cx| {
                                        if event.pressed_button != Some(gpui::MouseButton::Left) {
                                            return;
                                        }
                                        let Some(bounds) = *volume_move_bounds.borrow() else {
                                            return;
                                        };
                                        set_audio_volume(
                                            &volume_move_pipeline,
                                            bounds,
                                            event.position.x,
                                        );
                                        cx.notify();
                                    },
                                ))
                                .on_mouse_up(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |this, _, _, cx| {
                                        let volume =
                                            volume_save.property::<f64>("volume").clamp(0., 1.)
                                                as f32;
                                        this.config.media_volume = volume;
                                        this.config.save();
                                        cx.notify();
                                    }),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(3.))
                                        .rounded_full()
                                        .bg(self.theme.border)
                                        .child(
                                            div()
                                                .h_full()
                                                .w(relative(volume))
                                                .rounded_full()
                                                .bg(self.theme.muted),
                                        ),
                                ),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .child(self.media_icon_button(
                        if paused { "play" } else { "pause" },
                        if paused { "Play" } else { "Pause" },
                        !paused,
                        cx,
                        move |_, _, cx| {
                            let _ = toggle.set_state(if audio_playing(&toggle) {
                                gst::State::Paused
                            } else {
                                gst::State::Playing
                            });
                            cx.notify();
                        },
                    ))
                    .child(self.media_icon_button(
                        if muted { "volume-x" } else { "volume-2" },
                        if muted { "Unmute" } else { "Mute" },
                        false,
                        cx,
                        move |this, _, cx| {
                            let muted = !audio_muted(&mute);
                            mute.set_property("mute", muted);
                            this.config.media_muted = muted;
                            this.config.save();
                            cx.notify();
                        },
                    ))
                    .when_some(expand_url, |this, url| {
                        this.child(self.media_icon_button(
                            "expand",
                            "Expand",
                            false,
                            cx,
                            move |this, _, cx| this.open_lightbox(url.clone(), cx),
                        ))
                    })
                    .when_some(download_url, |this, url| {
                        this.child(self.media_icon_button(
                            "download",
                            "Download",
                            false,
                            cx,
                            move |this, _, cx| this.download_media(url.clone(), cx),
                        ))
                    }),
            )
            .into_any_element()
    }

    pub(super) fn media_fullscreen_button(&self, cx: &mut Context<Self>) -> AnyElement {
        self.media_icon_button(
            "maximize",
            "Full screen",
            self.media_fullscreen,
            cx,
            |this, _, cx| {
                this.toggle_media_fullscreen(cx);
            },
        )
    }
}
