use gstreamer as gst;
use gstreamer::prelude::*;

use super::media_audio::{audio_event, audio_playing, finish_audio, seek_audio_to, start_audio};
use super::media_download::download_media_file;
pub(super) use super::media_kind::{MediaKind, media_kind};
pub(super) use super::media_poster::Poster;
pub(crate) use super::media_poster::extract_poster;
use super::media_poster::generate_poster_external;
use super::*;

const MEDIA_STATE_LIMIT: usize = 64;

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

#[cfg(target_os = "macos")]
pub(super) fn configure_gstreamer() {
    use std::path::PathBuf;

    let Some(exe) = std::env::current_exe().ok() else {
        return;
    };
    let Some(root) = exe.parent().map(PathBuf::from) else {
        return;
    };
    let framework = root
        .join("..")
        .join("Frameworks")
        .join("GStreamer.framework")
        .join("Versions")
        .join("1.0");
    let plugins = framework.join("lib").join("gstreamer-1.0");
    if plugins.is_dir() {
        unsafe {
            std::env::set_var("GST_PLUGIN_SYSTEM_PATH", &plugins);
            std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", &plugins);
            std::env::set_var("GST_PLUGIN_PATH", &plugins);
            std::env::set_var("GST_PLUGIN_PATH_1_0", &plugins);
        }
    }
    let scanner = root.join("gst-plugin-scanner");
    if scanner.is_file() {
        unsafe {
            std::env::set_var("GST_PLUGIN_SCANNER", &scanner);
            std::env::set_var("GST_PLUGIN_SCANNER_1_0", &scanner);
        }
    }
    let gio = framework.join("lib").join("gio").join("modules");
    if gio.is_dir() {
        unsafe {
            std::env::set_var("GIO_EXTRA_MODULES", &gio);
        }
    }
}

#[cfg(target_os = "windows")]
pub(super) fn configure_gstreamer() {
    use std::path::PathBuf;

    let mut roots = Vec::new();
    if let Some(exe) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
    {
        roots.push(exe);
    }
    for name in [
        "GSTREAMER_1_0_ROOT_MSVC_X86_64",
        "GSTREAMER_ROOT",
        "GSTREAMER_1_0_ROOT_MINGW_X86_64",
    ] {
        if let Some(root) = std::env::var_os(name) {
            roots.push(PathBuf::from(root));
        }
    }
    roots.extend([
        PathBuf::from(r"C:\gstreamer\1.0\msvc_x86_64"),
        PathBuf::from(r"C:\Program Files\gstreamer\1.0\msvc_x86_64"),
        PathBuf::from(r"C:\Program Files (x86)\gstreamer\1.0\msvc_x86_64"),
    ]);
    let Some(root) = roots.into_iter().find(|root| {
        root.join("lib").join("gstreamer-1.0").is_dir()
            || root.join("bin").join("gstreamer-1.0-0.dll").is_file()
            || root.join("bin").join("gstreamer-1.0.dll").is_file()
    }) else {
        return;
    };
    let plugins = root.join("lib").join("gstreamer-1.0");
    if plugins.is_dir() {
        unsafe {
            std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", &plugins);
            std::env::set_var("GST_PLUGIN_PATH_1_0", &plugins);
        }
    }
    for relative in [
        r"libexec\gst-plugin-scanner.exe",
        r"bin\gst-plugin-scanner.exe",
        r"libexec\gstreamer-1.0\gst-plugin-scanner.exe",
    ] {
        let scanner = root.join(relative);
        if scanner.is_file() {
            unsafe {
                std::env::set_var("GST_PLUGIN_SCANNER", &scanner);
                std::env::set_var("GST_PLUGIN_SCANNER_1_0", &scanner);
            }
            break;
        }
    }
    let gio = root.join("lib").join("gio").join("modules");
    if gio.is_dir() {
        unsafe {
            std::env::set_var("GIO_EXTRA_MODULES", &gio);
        }
    }
    let mut paths = vec![root.join("bin")];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    if let Ok(path) = std::env::join_paths(paths) {
        unsafe {
            std::env::set_var("PATH", path);
        }
    }
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
                        self.media_positions.remove(&audio.url);
                        self.media_failures.remove(&audio.url);
                        finish_audio(&audio.pipeline);
                    }
                }
                Err(error) => {
                    if let Some(audio) = self.inline_audio.take() {
                        self.remember_failure(audio.url.clone(), error.clone());
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
                        if let Some(url) = self.media_audio_url.as_deref() {
                            self.media_positions.remove(url);
                            self.media_failures.remove(url);
                        }
                        finish_audio(audio);
                    }
                }
                Err(error) => {
                    if let Some(audio) = self.media_audio.take() {
                        if let Some(url) = self.media_audio_url.take() {
                            self.remember_failure(url, error.clone());
                        }
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
        self.media_audio_url = None;
        self.media_error = None;
        self.media_request_generation = self.media_request_generation.wrapping_add(1);
        let generation = self.media_request_generation;
        let kind = media_kind(&url).unwrap_or(MediaKind::Video);
        self.media_failures.remove(&url);
        if let Some(audio) = self.inline_audio.take() {
            self.remember_audio_position(&audio.url, &audio.pipeline);
            if audio.url == url {
                self.media_audio = Some(audio.pipeline);
                self.media_audio_url = Some(audio.url);
            } else {
                let _ = audio.pipeline.set_state(gst::State::Null);
            }
        }
        if let Some(video) = self.inline_video.take() {
            self.remember_video_position(&video.url, &video.video);
            if video.url == url {
                self.media_video = Some((video.url, video.video));
            }
        }
        if kind == MediaKind::Audio {
            if self.media_audio.is_none() {
                match start_audio(&url) {
                    Ok(pipeline) => {
                        pipeline.set_property("mute", self.config.media_muted);
                        pipeline.set_property(
                            "volume",
                            f64::from(self.config.media_volume.clamp(0., 1.)),
                        );
                        self.resume_audio(&url, &pipeline);
                        self.media_audio = Some(pipeline);
                        self.media_audio_url = Some(url);
                    }
                    Err(error) => self.media_error = Some(error),
                }
            }
            self.start_audio_ticker(cx);
            cx.notify();
            return;
        }
        if self.media_video.is_some() {
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
                        video.set_muted(this.config.media_muted);
                        video.set_volume(f64::from(this.config.media_volume.clamp(0., 1.)));
                        this.resume_video(&target, &video);
                        this.media_video = Some((target, video));
                        this.start_video_ticker(cx);
                    }
                    Err(error) => this.media_error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn start_video_ticker(&mut self, cx: &mut Context<Self>) {
        self.video_ticker_generation = self.video_ticker_generation.wrapping_add(1);
        let generation = self.video_ticker_generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(250))
                    .await;
                let keep = this.update(cx, |this, cx| {
                    if this.video_ticker_generation != generation {
                        return false;
                    }
                    let Some(video) = this
                        .inline_video
                        .as_ref()
                        .map(|inline| &inline.video)
                        .or_else(|| this.media_video.as_ref().map(|(_, video)| video))
                    else {
                        return false;
                    };
                    if !video.paused() {
                        cx.notify();
                    }
                    true
                });
                if !matches!(keep, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    fn remember_audio_position(&mut self, url: &str, pipeline: &gst::Pipeline) {
        if let Some(position) = pipeline.query_position::<gst::ClockTime>() {
            self.remember_position(url, position.seconds());
        }
    }

    fn remember_video_position(&mut self, url: &str, video: &gpui_video_player::Video) {
        self.remember_position(url, video.position().as_secs());
    }

    fn remember_position(&mut self, url: &str, position: u64) {
        if position > 0
            && (self.media_positions.len() < MEDIA_STATE_LIMIT
                || self.media_positions.contains_key(url))
        {
            self.media_positions.insert(url.to_string(), position);
        } else {
            self.media_positions.remove(url);
        }
        self.save_media_positions();
    }

    fn save_media_positions(&mut self) {
        if self.config.remember_media_positions {
            self.config.media_positions = self.media_positions.clone();
        } else {
            self.config.media_positions.clear();
        }
        self.config.save();
    }

    pub(super) fn persist_media_positions(&mut self) {
        if let Some((url, video)) = self.media_video.as_ref() {
            let url = url.clone();
            let position = video.position().as_secs();
            self.remember_position(&url, position);
        }
        if let Some(audio) = self.media_audio.as_ref()
            && let Some(url) = self.media_audio_url.clone()
        {
            let position = audio
                .query_position::<gst::ClockTime>()
                .map(|position| position.seconds())
                .unwrap_or(0);
            self.remember_position(&url, position);
        }
        if let Some(video) = self.inline_video.as_ref() {
            let url = video.url.clone();
            let position = video.video.position().as_secs();
            self.remember_position(&url, position);
        }
        if let Some(audio) = self.inline_audio.as_ref() {
            let url = audio.url.clone();
            let position = audio
                .pipeline
                .query_position::<gst::ClockTime>()
                .map(|position| position.seconds())
                .unwrap_or(0);
            self.remember_position(&url, position);
        }
        self.save_media_positions();
    }

    fn remember_failure(&mut self, url: String, error: String) {
        if self.media_failures.len() < MEDIA_STATE_LIMIT || self.media_failures.contains_key(&url) {
            self.media_failures.insert(url, error);
        }
    }

    pub(super) fn clear_media_session(&mut self) {
        self.media_failures.clear();
    }

    fn resume_audio(&self, url: &str, pipeline: &gst::Pipeline) {
        if let Some(position) = self.media_positions.get(url) {
            seek_audio_to(pipeline, *position);
        }
    }

    fn resume_video(&self, url: &str, video: &gpui_video_player::Video) {
        if let Some(position) = self.media_positions.get(url) {
            let _ = video.seek(std::time::Duration::from_secs(*position), true);
        }
    }

    pub(super) fn clear_media(&mut self) {
        self.media_request_generation = self.media_request_generation.wrapping_add(1);
        if let Some((url, video)) = self.media_video.take() {
            self.remember_video_position(&url, &video);
        }
        if let Some(audio) = self.media_audio.take() {
            if let Some(url) = self.media_audio_url.take() {
                self.remember_audio_position(&url, &audio);
            }
            let _ = audio.set_state(gst::State::Null);
        }
        self.media_video_loading = None;
        self.inline_video_loading = None;
        self.media_error = None;
        self.media_fullscreen = false;
        super::trim_memory();
    }

    pub(super) fn clear_inline_media(&mut self) {
        self.media_request_generation = self.media_request_generation.wrapping_add(1);
        if let Some(video) = self.inline_video.take() {
            self.remember_video_position(&video.url, &video.video);
        }
        if let Some(audio) = self.inline_audio.take() {
            self.remember_audio_position(&audio.url, &audio.pipeline);
            let _ = audio.pipeline.set_state(gst::State::Null);
        }
        self.inline_video_loading = None;
        super::trim_memory();
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
        super::trim_memory();
    }

    pub(super) fn media_poster(&self, url: &str, cx: &mut Context<Self>) -> Option<Poster> {
        if !self.config.video_thumbnails {
            return None;
        }
        if let Some(path) = self.media_posters.borrow().get(url) {
            return path.clone();
        }
        if self.media_posters.borrow().len() >= 4 {
            return None;
        }
        self.media_posters
            .borrow_mut()
            .insert(url.to_string(), None);
        let target = url.to_string();
        let key = target.clone();
        let task = cx
            .background_executor()
            .spawn(async move { generate_poster_external(&target) });
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
        self.media_failures.remove(&url);
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
        if let Some(video) = self.inline_video.take() {
            self.remember_video_position(&video.url, &video.video);
        }
        if let Some(audio) = self.inline_audio.take() {
            self.remember_audio_position(&audio.url, &audio.pipeline);
            let _ = audio.pipeline.set_state(gst::State::Null);
        }
        self.inline_video_loading = None;
        self.media_video_loading = None;
        self.media_request_generation = self.media_request_generation.wrapping_add(1);
        let generation = self.media_request_generation;
        if kind == MediaKind::Audio {
            match start_audio(&url) {
                Ok(pipeline) => {
                    pipeline.set_property("mute", self.config.media_muted);
                    pipeline
                        .set_property("volume", f64::from(self.config.media_volume.clamp(0., 1.)));
                    self.resume_audio(&url, &pipeline);
                    self.media_failures.remove(&url);
                    self.inline_audio = Some(InlineAudio {
                        message_id,
                        url,
                        pipeline,
                    });
                    self.start_audio_ticker(cx);
                }
                Err(error) => {
                    self.remember_failure(url.clone(), error.clone());
                    self.toast(format!("Could not play media: {error}"));
                }
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
                        video.set_muted(this.config.media_muted);
                        video.set_volume(f64::from(this.config.media_volume.clamp(0., 1.)));
                        this.resume_video(&target, &video);
                        this.inline_video = Some(InlineMedia {
                            message_id,
                            url: target,
                            video,
                        });
                        this.start_video_ticker(cx);
                    }
                    Err(error) => {
                        this.media_failures
                            .insert(target.clone(), error.to_string());
                        this.toast(format!("Could not play media: {error}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn render_inline_media(
        &self,
        message_id: i64,
        url: &str,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if let Some(error) = self.media_failures.get(url) {
            let retry_url = url.to_string();
            let kind = media_kind(url).unwrap_or(MediaKind::Video);
            return Some(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .rounded(px(5.))
                    .bg(self.theme.panel)
                    .border_1()
                    .border_color(self.theme.error)
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(self.theme.error)
                            .child("Could not play media."),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(self.theme.muted)
                            .child(error.clone()),
                    )
                    .child(self.media_icon_button(
                        "play",
                        "Try again",
                        false,
                        cx,
                        move |this, _, cx| {
                            this.play_inline_media(message_id, retry_url.clone(), kind, cx)
                        },
                    ))
                    .into_any_element(),
            );
        }
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
        let progress_video = video.clone();
        let toggle = video.clone();
        let click_video = video.clone();
        let mute = video.clone();
        let open_url = url.clone();
        let download_url = url.clone();
        let paused = video.paused();
        let muted = video.muted();
        let (video_width, video_height) = video.size();
        let (display_width, display_height) = if video_width > 0 && video_height > 0 {
            let scale = (MEDIA_PREVIEW_MAX_WIDTH / video_width as f32)
                .min(MEDIA_PREVIEW_MAX_HEIGHT / video_height as f32)
                .min(1.);
            (
                px(video_width as f32 * scale),
                px(video_height as f32 * scale),
            )
        } else {
            (px(MEDIA_PREVIEW_MAX_WIDTH), px(MEDIA_PREVIEW_MAX_HEIGHT))
        };
        let speed_button = self
            .config
            .features
            .video_speed
            .then(|| self.media_speed_button(&progress_video, &url, cx));
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
                                .buffer_capacity(4)
                                .size(display_width, display_height),
                        ),
                )
                .child(self.render_video_progress(&progress_video, &url, cx))
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
                        .when_some(speed_button, |this, button| this.child(button))
                        .child(self.media_icon_button(
                            if muted { "volume-x" } else { "volume-2" },
                            if muted { "Unmute" } else { "Mute" },
                            false,
                            cx,
                            move |this, _, cx| {
                                let muted = !mute.muted();
                                mute.set_muted(muted);
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
        let progress_video = video.clone();
        let controls_video = video.clone();
        let click_video = video.clone();
        let mute_video = video.clone();
        let download_url = url.to_string();
        let paused = video.paused();
        let muted = video.muted();
        let speed_button = self
            .config
            .features
            .video_speed
            .then(|| self.media_speed_button(&progress_video, "lightbox", cx));
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
                        .buffer_capacity(4)
                        .size(max_w, max_h),
                ),
        );
        body = body
            .child(div().w_full().child(self.render_video_progress(
                &progress_video,
                "lightbox",
                cx,
            )))
            .child(
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
                    .when_some(speed_button, |this, button| this.child(button))
                    .child(self.media_icon_button(
                        if muted { "volume-x" } else { "volume-2" },
                        if muted { "Unmute" } else { "Mute" },
                        false,
                        cx,
                        move |this, _, cx| {
                            let muted = !mute_video.muted();
                            mute_video.set_muted(muted);
                            this.config.media_muted = muted;
                            this.config.save();
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
