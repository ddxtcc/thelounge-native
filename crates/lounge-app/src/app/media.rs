use gstreamer as gst;
use gstreamer::prelude::*;

use super::media_audio::{
    audio_event, audio_muted, audio_playing, finish_audio, format_clock, seek_audio,
    set_audio_volume, start_audio,
};
use super::media_download::download_media_file;
pub(super) use super::media_kind::{MediaKind, media_kind};
pub(super) use super::media_poster::Poster;
use super::media_poster::generate_poster;
use super::*;

pub(super) struct InlineAudio {
    pub message_id: i64,
    pub url: String,
    pub pipeline: gst::Pipeline,
}

pub(super) struct InlineMedia {
    pub message_id: i64,
    pub url: String,
    pub video: gpui_video_player::Video,
}

impl LoungeApp {
    fn start_audio_ticker(&mut self, cx: &mut Context<Self>) {
        self.audio_ticker_generation = self.audio_ticker_generation.wrapping_add(1);
        let generation = self.audio_ticker_generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(250))
                    .await;
                let active = this.update(cx, |this, cx| {
                    if this.audio_ticker_generation != generation {
                        return false;
                    }
                    let active = this.inline_audio.is_some() || this.media_audio.is_some();
                    if !active {
                        return false;
                    }
                    let changed = this.service_audio(cx);
                    if changed || this.audio_playing() {
                        cx.notify();
                    }
                    true
                });
                if !matches!(active, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    fn audio_playing(&self) -> bool {
        self.inline_audio
            .as_ref()
            .is_some_and(|audio| audio_playing(&audio.pipeline))
            || self.media_audio.as_ref().is_some_and(audio_playing)
    }

    fn service_audio(&mut self, cx: &mut Context<Self>) -> bool {
        let inline_event = self
            .inline_audio
            .as_ref()
            .and_then(|audio| audio_event(&audio.pipeline));
        let media_event = self.media_audio.as_ref().and_then(audio_event);
        let mut changed = false;
        if let Some(event) = inline_event {
            changed = true;
            match event {
                Ok(()) => {
                    if let Some(audio) = self.inline_audio.as_ref() {
                        finish_audio(&audio.pipeline);
                    }
                }
                Err(error) => {
                    if let Some(audio) = self.inline_audio.take() {
                        let _ = audio.pipeline.set_state(gst::State::Null);
                    }
                    self.toast(format!("Media playback failed: {error}"));
                }
            }
        }
        if let Some(event) = media_event {
            changed = true;
            match event {
                Ok(()) => {
                    if let Some(audio) = self.media_audio.as_ref() {
                        finish_audio(audio);
                    }
                }
                Err(error) => {
                    if let Some(audio) = self.media_audio.take() {
                        let _ = audio.set_state(gst::State::Null);
                    }
                    self.media_error = Some(error);
                }
            }
        }
        if changed {
            cx.notify();
        }
        changed
    }

    pub(super) fn download_media(&mut self, url: String, cx: &mut Context<Self>) {
        self.toast("Downloading...");
        let target = url.clone();
        let task = cx
            .background_executor()
            .spawn(async move { download_media_file(&target) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(path) => this.toast(format!("Saved {}", path.display())),
                    Err(error) => this.toast(format!("Download failed: {error}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn open_media_lightbox(&mut self, url: String, cx: &mut Context<Self>) {
        self.lightbox_index = None;
        self.lightbox = Some(url.clone());
        self.lightbox_zoom = 1.0;
        self.lightbox_pan = Point::default();
        self.lightbox_drag_last = None;
        self.media_fullscreen = false;
        self.media_video = None;
        self.media_video_loading = None;
        self.inline_video_loading = None;
        self.media_audio = None;
        self.media_error = None;
        self.media_request_generation = self.media_request_generation.wrapping_add(1);
        let generation = self.media_request_generation;
        let kind = media_kind(&url).unwrap_or(MediaKind::Video);
        let inline_audio = self.inline_audio.take().filter(|inline| inline.url == url);
        let inline_video = self.inline_video.take().filter(|inline| inline.url == url);
        if kind == MediaKind::Audio {
            if let Some(inline) = inline_audio {
                self.media_audio = Some(inline.pipeline);
            } else {
                match start_audio(&url) {
                    Ok(pipeline) => {
                        pipeline.set_property("mute", self.audio_start_muted);
                        self.media_audio = Some(pipeline);
                    }
                    Err(error) => self.media_error = Some(error),
                }
            }
            self.start_audio_ticker(cx);
            cx.notify();
            return;
        }
        if let Some(inline) = inline_video {
            self.media_video = Some((url, inline.video));
            cx.notify();
            return;
        }
        let Ok(parsed) = url::Url::parse(&url) else {
            self.media_error = Some("Invalid media URL.".to_string());
            cx.notify();
            return;
        };
        self.media_video_loading = Some(url.clone());
        let target = url.clone();
        let task = cx
            .background_executor()
            .spawn(async move { gpui_video_player::Video::new(&parsed) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.media_request_generation != generation {
                    return;
                }
                this.media_video_loading = None;
                match result {
                    Ok(video) => {
                        video.set_looping(true);
                        this.media_video = Some((target, video));
                    }
                    Err(error) => this.media_error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn clear_media(&mut self) {
        self.media_request_generation = self.media_request_generation.wrapping_add(1);
        self.media_video = None;
        self.media_video_loading = None;
        if let Some(audio) = self.media_audio.take() {
            let _ = audio.set_state(gst::State::Null);
        }
        self.media_error = None;
        self.media_fullscreen = false;
    }

    pub(super) fn clear_inline_media(&mut self) {
        self.media_request_generation = self.media_request_generation.wrapping_add(1);
        self.inline_video = None;
        self.inline_video_loading = None;
        if let Some(audio) = self.inline_audio.take() {
            let _ = audio.pipeline.set_state(gst::State::Null);
        }
    }

    pub(super) fn clear_media_posters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let posters: Vec<_> = self.media_posters.borrow_mut().drain().collect();
        for (_, poster) in posters {
            if let Some(poster) = poster {
                let path = poster.path.clone();
                self.image_cache.update(cx, |cache, cx| {
                    cache.remove(&gpui::Resource::Path(path.into()), window, cx);
                });
                let _ = std::fs::remove_file(poster.path);
            }
        }
    }

    pub(super) fn media_poster(&self, url: &str, cx: &mut Context<Self>) -> Option<Poster> {
        if let Some(path) = self.media_posters.borrow().get(url) {
            return path.clone();
        }
        if self.media_posters.borrow().len() >= 8 {
            return None;
        }
        self.media_posters
            .borrow_mut()
            .insert(url.to_string(), None);
        let target = url.to_string();
        let key = target.clone();
        let task = cx
            .background_executor()
            .spawn(async move { generate_poster(&target) });
        cx.spawn(async move |this, cx| {
            let path = task.await;
            let _ = this.update(cx, |this, cx| {
                this.media_posters.borrow_mut().insert(key.clone(), path);
                cx.notify();
            });
        })
        .detach();
        None
    }

    pub(super) fn play_inline_media(
        &mut self,
        message_id: i64,
        url: String,
        kind: MediaKind,
        cx: &mut Context<Self>,
    ) {
        if let Some(inline) = self.inline_video.as_mut()
            && inline.url == url
        {
            if inline.message_id == message_id {
                return;
            }
            inline.message_id = message_id;
            cx.notify();
            return;
        }
        if let Some((pending_message, pending_url)) = self.inline_video_loading.as_ref()
            && *pending_message == message_id
            && pending_url == &url
        {
            return;
        }
        if let Some(inline) = self.inline_audio.as_mut()
            && inline.url == url
        {
            if inline.message_id == message_id {
                return;
            }
            inline.message_id = message_id;
            cx.notify();
            return;
        }
        self.inline_video = None;
        self.inline_video_loading = None;
        self.media_video_loading = None;
        self.media_request_generation = self.media_request_generation.wrapping_add(1);
        let generation = self.media_request_generation;
        if let Some(audio) = self.inline_audio.take() {
            let _ = audio.pipeline.set_state(gst::State::Null);
        }
        if kind == MediaKind::Audio {
            match start_audio(&url) {
                Ok(pipeline) => {
                    pipeline.set_property("mute", self.audio_start_muted);
                    self.inline_audio = Some(InlineAudio {
                        message_id,
                        url,
                        pipeline,
                    });
                    self.start_audio_ticker(cx);
                }
                Err(error) => self.toast(format!("Could not play media: {error}")),
            }
            cx.notify();
            return;
        }
        let Ok(parsed) = url::Url::parse(&url) else {
            self.toast("Invalid media URL.");
            return;
        };
        self.inline_video_loading = Some((message_id, url.clone()));
        let target = url.clone();
        let task = cx
            .background_executor()
            .spawn(async move { gpui_video_player::Video::new(&parsed) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.media_request_generation != generation {
                    return;
                }
                this.inline_video_loading = None;
                match result {
                    Ok(video) => {
                        video.set_looping(true);
                        this.inline_video = Some(InlineMedia {
                            message_id,
                            url: target,
                            video,
                        });
                    }
                    Err(error) => this.toast(format!("Could not play media: {error}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn render_audio_controls(
        &self,
        pipeline: &gst::Pipeline,
        expand_url: Option<String>,
        download_url: Option<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
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
        let volume_click_bounds = volume_bounds.clone();
        let volume_move_bounds = volume_bounds.clone();
        let track_bounds: std::rc::Rc<std::cell::RefCell<Option<Bounds<Pixels>>>> =
            std::rc::Rc::new(std::cell::RefCell::new(None));
        let prepaint_bounds = track_bounds.clone();
        let seek = pipeline.clone();
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
                            this.audio_start_muted = muted;
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

    pub(super) fn render_inline_media(
        &self,
        message_id: i64,
        url: &str,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if let Some(audio) = self
            .inline_audio
            .as_ref()
            .filter(|inline| inline.message_id == message_id && inline.url == url)
        {
            let file = audio.url.rsplit('/').next().unwrap_or(url).to_string();
            let open_url = audio.url.clone();
            let download_url = audio.url.clone();
            return Some(
                div()
                    .id(ElementId::Name(format!("inline-audio-{message_id}").into()))
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
                    .child(self.render_audio_controls(
                        &audio.pipeline,
                        Some(open_url),
                        Some(download_url),
                        cx,
                    ))
                    .into_any_element(),
            );
        }
        let inline = self
            .inline_video
            .as_ref()
            .filter(|inline| inline.message_id == message_id && inline.url == url)?;
        let url = inline.url.clone();
        let video = inline.video.clone();
        let toggle = video.clone();
        let click_video = video.clone();
        let mute = video.clone();
        let open_url = url.clone();
        let download_url = url.clone();
        let paused = video.paused();
        let muted = video.muted();
        let (video_width, video_height) = video.size();
        let (display_width, display_height) = if video_width > 0 && video_height > 0 {
            let scale = (420. / video_width as f32)
                .min(320. / video_height as f32)
                .min(1.);
            (
                px(video_width as f32 * scale),
                px(video_height as f32 * scale),
            )
        } else {
            (px(440.), px(248.))
        };
        Some(
            div()
                .id(ElementId::Name(format!("inline-media-{url}").into()))
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .id(ElementId::Name(format!("inline-video-area-{url}").into()))
                        .w_full()
                        .flex()
                        .justify_center()
                        .cursor_pointer()
                        .on_click(cx.listener(move |_, _, _, cx| {
                            click_video.set_paused(!click_video.paused());
                            cx.notify();
                        }))
                        .child(
                            gpui_video_player::video(video)
                                .id(ElementId::Name(format!("inline-video-{url}").into()))
                                .buffer_capacity(30)
                                .size(display_width, display_height),
                        ),
                )
                .child(
                    div()
                        .w_full()
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
                                toggle.set_paused(!toggle.paused());
                                cx.notify();
                            },
                        ))
                        .child(self.media_icon_button(
                            if muted { "volume-x" } else { "volume-2" },
                            if muted { "Unmute" } else { "Mute" },
                            false,
                            cx,
                            move |_, _, cx| {
                                mute.set_muted(!mute.muted());
                                cx.notify();
                            },
                        ))
                        .child(self.media_icon_button(
                            "expand",
                            "Expand",
                            false,
                            cx,
                            move |this, _, cx| this.open_lightbox(open_url.clone(), cx),
                        ))
                        .child(self.media_icon_button(
                            "download",
                            "Download",
                            false,
                            cx,
                            move |this, _, cx| this.download_media(download_url.clone(), cx),
                        ))
                        .child(self.media_fullscreen_button(cx)),
                )
                .into_any_element(),
        )
    }

    pub(super) fn render_media_lightbox(
        &self,
        url: &str,
        kind: MediaKind,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let viewport = window.viewport_size();
        let (max_w, max_h) = if self.media_fullscreen {
            (viewport.width, viewport.height - px(64.))
        } else {
            (viewport.width * 0.88, viewport.height * 0.78)
        };
        if let Some(error) = &self.media_error {
            return div()
                .max_w(max_w)
                .p_4()
                .rounded(px(6.))
                .bg(theme.panel)
                .border_1()
                .border_color(theme.error)
                .text_color(theme.text)
                .child(format!("Could not play media: {error}"))
                .into_any_element();
        }
        if kind == MediaKind::Audio {
            let Some(audio) = self.media_audio.as_ref() else {
                return div()
                    .max_w(max_w)
                    .p_4()
                    .rounded(px(6.))
                    .bg(theme.panel)
                    .text_color(theme.muted)
                    .child("Loading media...")
                    .into_any_element();
            };
            let file = url.rsplit('/').next().unwrap_or(url).to_string();
            return div()
                .id("lightbox-audio")
                .w(max_w)
                .p_4()
                .rounded(px(6.))
                .bg(theme.panel)
                .border_1()
                .border_color(theme.border)
                .flex()
                .flex_col()
                .gap_3()
                .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                .child(div().text_size(px(13.)).child(file))
                .child(self.render_audio_controls(audio, None, Some(url.to_string()), cx))
                .child(
                    self.media_icon_button("x", "Close", false, cx, |this, _, cx| {
                        this.close_overlays(cx)
                    }),
                )
                .into_any_element();
        }
        let Some((_, video)) = self
            .media_video
            .as_ref()
            .filter(|(current, _)| current == url)
        else {
            return div()
                .max_w(max_w)
                .p_4()
                .rounded(px(6.))
                .bg(theme.panel)
                .text_color(theme.muted)
                .child("Loading media...")
                .into_any_element();
        };
        let video = video.clone();
        let controls_video = video.clone();
        let click_video = video.clone();
        let mute_video = video.clone();
        let download_url = url.to_string();
        let paused = video.paused();
        let muted = video.muted();
        let mut body = div()
            .id("lightbox-media")
            .max_w(max_w)
            .max_h(max_h)
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));
        body = body.child(
            div()
                .id("lightbox-video-area")
                .cursor_pointer()
                .on_click(cx.listener(move |_, _, _, cx| {
                    click_video.set_paused(!click_video.paused());
                    cx.notify();
                }))
                .child(
                    gpui_video_player::video(video)
                        .id("lightbox-video")
                        .buffer_capacity(30)
                        .size(max_w, max_h),
                ),
        );
        body = body.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .gap_3()
                .child(self.media_icon_button(
                    if paused { "play" } else { "pause" },
                    if paused { "Play" } else { "Pause" },
                    !paused,
                    cx,
                    move |_, _, cx| {
                        controls_video.set_paused(!controls_video.paused());
                        cx.notify();
                    },
                ))
                .child(self.media_icon_button(
                    if muted { "volume-x" } else { "volume-2" },
                    if muted { "Unmute" } else { "Mute" },
                    false,
                    cx,
                    move |_, _, cx| {
                        mute_video.set_muted(!mute_video.muted());
                        cx.notify();
                    },
                ))
                .child(self.media_icon_button(
                    "download",
                    "Download",
                    false,
                    cx,
                    move |this, _, cx| this.download_media(download_url.clone(), cx),
                ))
                .child(self.media_fullscreen_button(cx))
                .child(
                    self.media_icon_button("x", "Close", false, cx, |this, _, cx| {
                        this.close_overlays(cx)
                    }),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.muted)
                        .child("Click outside to close"),
                ),
        );
        body.into_any_element()
    }
}
