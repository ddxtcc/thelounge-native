use gstreamer as gst;
use gstreamer::glib::object::Cast;
use gstreamer::prelude::*;
use image::{ImageBuffer, Rgba};
use std::hash::{Hash, Hasher};
use yuv::{YuvBiPlanarImage, YuvConversionMode, YuvRange, YuvStandardMatrix, yuv_nv12_to_bgra};

use super::tooltip::Tooltip;
use super::*;

#[derive(Clone)]
pub(super) struct Poster {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MediaKind {
    Video,
    Audio,
}

pub(super) fn media_kind(url: &str) -> Option<MediaKind> {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();
    let extension = path.rsplit('.').next()?;
    match extension {
        "mp4" | "m4v" | "webm" | "mkv" | "mov" => Some(MediaKind::Video),
        "mp3" | "m4a" | "aac" | "ogg" | "oga" | "opus" | "wav" | "flac" => Some(MediaKind::Audio),
        _ => None,
    }
}

fn start_audio(url: &str) -> Result<gst::Pipeline, String> {
    gst::init().map_err(|error| error.to_string())?;
    let uri = url.replace('"', "%22");
    let pipeline = gst::parse::launch(&format!("playbin uri=\"{uri}\""))
        .map_err(|error| error.to_string())?
        .downcast::<gst::Pipeline>()
        .map_err(|_| "audio pipeline was not a playbin".to_string())?;
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|error| error.to_string())?;
    Ok(pipeline)
}

fn audio_playing(pipeline: &gst::Pipeline) -> bool {
    pipeline.state(gst::ClockTime::ZERO).1 == gst::State::Playing
}

fn audio_muted(pipeline: &gst::Pipeline) -> bool {
    pipeline.property::<bool>("mute")
}

fn generate_poster(url: &str) -> Option<Poster> {
    let parsed = url::Url::parse(url).ok()?;
    let video = gpui_video_player::Video::new(&parsed).ok()?;
    video.set_muted(true);
    let started = std::time::Instant::now();
    let (yuv, width, height) = loop {
        if let Some(frame) = video.current_frame_data()
            && frame.1 > 0
            && frame.2 > 0
        {
            break frame;
        }
        if started.elapsed() > std::time::Duration::from_secs(3) {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let width_usize = width as usize;
    let height_usize = height as usize;
    let y_size = width_usize * height_usize;
    let uv_size = y_size / 2;
    if yuv.len() < y_size + uv_size {
        return None;
    }
    let image = YuvBiPlanarImage {
        y_plane: &yuv[..y_size],
        y_stride: width,
        uv_plane: &yuv[y_size..y_size + uv_size],
        uv_stride: width,
        width,
        height,
    };
    let mut bgra = vec![0u8; width_usize * height_usize * 4];
    let mut converted = false;
    for (range, matrix) in [
        (YuvRange::Full, YuvStandardMatrix::Bt709),
        (YuvRange::Limited, YuvStandardMatrix::Bt709),
        (YuvRange::Limited, YuvStandardMatrix::Bt601),
    ] {
        if yuv_nv12_to_bgra(
            &image,
            &mut bgra,
            width * 4,
            range,
            matrix,
            YuvConversionMode::Balanced,
        )
        .is_ok()
        {
            converted = true;
            break;
        }
    }
    if !converted {
        return None;
    }
    let mut rgba = vec![0u8; bgra.len()];
    for (source, target) in bgra
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rgba.as_chunks_mut::<4>().0)
    {
        target[0] = source[2];
        target[1] = source[1];
        target[2] = source[0];
        target[3] = source[3];
    }
    let buffer: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, rgba)?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let path = std::env::temp_dir().join(format!("lounge-native-poster-{:x}.png", hasher.finish()));
    buffer.save(&path).ok()?;
    Some(Poster {
        path,
        width,
        height,
    })
}

impl LoungeApp {
    pub(super) fn open_media_lightbox(&mut self, url: String, cx: &mut Context<Self>) {
        self.lightbox_index = None;
        self.lightbox = Some(url.clone());
        self.lightbox_zoom = 1.0;
        self.lightbox_pan = Point::default();
        self.lightbox_drag_last = None;
        self.media_fullscreen = false;
        self.media_video = None;
        self.media_audio = None;
        self.media_error = None;
        let kind = media_kind(&url).unwrap_or(MediaKind::Video);
        let inline_audio = self.inline_audio.take().filter(|inline| inline.url == url);
        let inline_video = self.inline_video.take().filter(|inline| inline.url == url);
        if kind == MediaKind::Audio {
            if let Some(inline) = inline_audio {
                self.media_audio = Some(inline.pipeline);
            } else {
                match start_audio(&url) {
                    Ok(pipeline) => self.media_audio = Some(pipeline),
                    Err(error) => self.media_error = Some(error),
                }
            }
            cx.notify();
            return;
        }
        if let Some(inline) = inline_video {
            self.media_video = Some((url, inline.video));
            cx.notify();
            return;
        }
        let parsed = match url::Url::parse(&url) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.media_error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        match gpui_video_player::Video::new(&parsed) {
            Ok(video) => {
                video.set_looping(true);
                self.media_video = Some((url, video));
            }
            Err(error) => self.media_error = Some(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn clear_media(&mut self) {
        self.media_video = None;
        if let Some(audio) = self.media_audio.take() {
            let _ = audio.set_state(gst::State::Null);
        }
        self.media_error = None;
        self.media_fullscreen = false;
    }

    pub(super) fn clear_inline_media(&mut self) {
        self.inline_video = None;
        if let Some(audio) = self.inline_audio.take() {
            let _ = audio.pipeline.set_state(gst::State::Null);
        }
    }

    pub(super) fn clear_media_posters(&mut self) {
        for (_, poster) in self.media_posters.borrow_mut().drain() {
            if let Some(poster) = poster {
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
        if let Some(audio) = self.inline_audio.take() {
            let _ = audio.pipeline.set_state(gst::State::Null);
        }
        if kind == MediaKind::Audio {
            match start_audio(&url) {
                Ok(pipeline) => {
                    self.inline_audio = Some(InlineAudio {
                        message_id,
                        url,
                        pipeline,
                    });
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
        match gpui_video_player::Video::new(&parsed) {
            Ok(video) => {
                video.set_looping(true);
                self.inline_video = Some(InlineMedia {
                    message_id,
                    url,
                    video,
                });
            }
            Err(error) => self.toast(format!("Could not play media: {error}")),
        }
        cx.notify();
    }

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
        div()
            .id(ElementId::Name(format!("media-{icon}-{tooltip}").into()))
            .size(px(26.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .cursor_pointer()
            .when(active, |this| this.bg(theme.accent_dim))
            .when(!active, |this| this.hover(|style| style.bg(theme.hover)))
            .on_click(cx.listener(move |this, _, window, cx| {
                on_click(this, window, cx);
                cx.stop_propagation();
            }))
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

    fn toggle_media_fullscreen(&mut self, cx: &mut Context<Self>) {
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
            self.media_audio = Some(inline.pipeline);
        }
        self.media_fullscreen = self.media_video.is_some() || self.media_audio.is_some();
        cx.notify();
    }

    fn media_fullscreen_button(&self, cx: &mut Context<Self>) -> AnyElement {
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

    fn render_audio_controls(
        &self,
        pipeline: &gst::Pipeline,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let toggle = pipeline.clone();
        let mute = pipeline.clone();
        let paused = !audio_playing(pipeline);
        let muted = audio_muted(pipeline);
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
                move |_, _, cx| {
                    mute.set_property("mute", !audio_muted(&mute));
                    cx.notify();
                },
            ))
            .child(self.media_fullscreen_button(cx))
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
                    .child(self.render_audio_controls(&audio.pipeline, cx))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_center()
                            .gap_1()
                            .child(self.media_icon_button(
                                "expand",
                                "Expand",
                                false,
                                cx,
                                move |this, _, cx| this.open_lightbox(open_url.clone(), cx),
                            ))
                            .child(self.media_icon_button(
                                "x",
                                "Close",
                                false,
                                cx,
                                |this, _, cx| {
                                    this.clear_inline_media();
                                    cx.notify();
                                },
                            )),
                    )
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
                        .child(self.media_fullscreen_button(cx))
                        .child(
                            self.media_icon_button("x", "Close", false, cx, |this, _, cx| {
                                this.clear_inline_media();
                                cx.notify();
                            }),
                        ),
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
                .child(self.render_audio_controls(audio, cx))
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
        if kind == MediaKind::Video {
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
        } else {
            body = body.child(
                div()
                    .w(max_w)
                    .py_12()
                    .rounded(px(6.))
                    .bg(theme.panel)
                    .border_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.text)
                    .child("Audio"),
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_direct_media_urls() {
        assert!(matches!(
            media_kind("https://example.com/a.webm?x=1"),
            Some(MediaKind::Video)
        ));
        assert!(matches!(
            media_kind("https://example.com/a.mp3"),
            Some(MediaKind::Audio)
        ));
        assert_eq!(media_kind("https://example.com/a.png"), None);
        assert_eq!(media_kind("https://example.com/a"), None);
    }
}
