use image::imageops::FilterType;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use super::*;

const PREVIEW_LIMIT: usize = 64;
static DECODE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone)]
pub(super) enum PreviewState {
    Loading,
    Ready(PathBuf),
    Failed,
}

pub(super) fn load_thumbnail(url: &str) -> Result<PathBuf, String> {
    let parsed = url::Url::parse(url).map_err(|error| error.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("unsupported image URL".to_string());
    }
    let mut response = ureq::get(url)
        .header("User-Agent", "Lounge Native")
        .config()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .call()
        .map_err(|error| error.to_string())?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(25 * 1024 * 1024)
        .read_to_vec()
        .map_err(|error| error.to_string())?;
    let _decode = DECODE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|error| error.to_string())?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes.as_slice()))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|error| error.to_string())?;
    let scale = (460. / image.width() as f32)
        .min(320. / image.height() as f32)
        .min(1.);
    let width = (image.width() as f32 * scale).round().max(1.) as u32;
    let height = (image.height() as f32 * scale).round().max(1.) as u32;
    let image = image.resize_exact(width, height, FilterType::Triangle);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let path = std::env::temp_dir().join(format!("lounge-native-thumb-{:x}.png", hasher.finish()));
    image.save(&path).map_err(|error| error.to_string())?;
    Ok(path)
}

impl LoungeApp {
    fn request_preview(&self, url: &str, cx: &mut Context<Self>) -> PreviewState {
        if let Some(state) = self.image_previews.borrow().get(url).cloned() {
            return state;
        }
        if self.image_previews.borrow().len() >= PREVIEW_LIMIT {
            return PreviewState::Failed;
        }
        self.image_previews
            .borrow_mut()
            .insert(url.to_string(), PreviewState::Loading);
        let key = url.to_string();
        let target = url.to_string();
        let task = cx
            .background_executor()
            .spawn(async move { load_thumbnail(&target) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                let state = match result {
                    Ok(path) => PreviewState::Ready(path),
                    Err(_) => PreviewState::Failed,
                };
                this.image_previews.borrow_mut().insert(key.clone(), state);
                cx.notify();
            });
        })
        .detach();
        PreviewState::Loading
    }

    pub(super) fn preview_image(
        &self,
        url: &str,
        max_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.request_preview(url, cx) {
            PreviewState::Ready(path) => div()
                .w_full()
                .max_w(px(max_width))
                .child(
                    gpui::img(path)
                        .image_cache(&self.image_cache)
                        .w_full()
                        .max_h(px(320.))
                        .object_fit(gpui::ObjectFit::Contain),
                )
                .into_any_element(),
            PreviewState::Failed => {
                self.preview_placeholder(max_width, "Image preview unavailable")
            }
            PreviewState::Loading => self.preview_placeholder(max_width, "Loading image..."),
        }
    }

    pub(super) fn preview_thumbnail(
        &self,
        url: &str,
        size: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.request_preview(url, cx) {
            PreviewState::Ready(path) => div()
                .w(px(size))
                .h(px(size))
                .child(
                    gpui::img(path)
                        .image_cache(&self.image_cache)
                        .w_full()
                        .h_full()
                        .object_fit(gpui::ObjectFit::Cover),
                )
                .into_any_element(),
            PreviewState::Failed => self.preview_thumbnail_placeholder(size, "Unavailable"),
            PreviewState::Loading => self.preview_thumbnail_placeholder(size, "Loading..."),
        }
    }

    fn preview_thumbnail_placeholder(&self, size: f32, label: &str) -> AnyElement {
        div()
            .w(px(size))
            .h(px(size))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.))
            .text_color(self.theme.muted)
            .child(label.to_string())
            .into_any_element()
    }

    fn preview_placeholder(&self, max_width: f32, label: &str) -> AnyElement {
        div()
            .w_full()
            .max_w(px(max_width))
            .h(px(72.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(11.))
            .text_color(self.theme.muted)
            .child(label.to_string())
            .into_any_element()
    }

    pub(super) fn clear_image_previews(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let previews: Vec<_> = self.image_previews.borrow_mut().drain().collect();
        for (_, state) in previews {
            if let PreviewState::Ready(path) = state {
                self.image_cache.update(cx, |cache, cx| {
                    cache.remove(&gpui::Resource::Path(path.clone().into()), window, cx);
                });
                let _ = std::fs::remove_file(path);
            }
        }
        super::trim_memory();
    }
}
