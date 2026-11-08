use super::*;

impl LoungeApp {
    pub(super) fn channel_image_urls(&self) -> Vec<String> {
        let Some((_, channel)) = self.state.active_channel() else {
            return Vec::new();
        };
        let mut urls = Vec::new();
        for message in &channel.chan.messages {
            for preview in &message.previews {
                let is_image = preview.kind == "image"
                    || preview
                        .media_type
                        .as_deref()
                        .map(|kind| kind.starts_with("image/"))
                        .unwrap_or(false);
                if !is_image {
                    continue;
                }
                let candidate = preview
                    .media
                    .as_deref()
                    .filter(|url| !url.is_empty())
                    .or_else(|| {
                        preview
                            .thumb_actual_url
                            .as_deref()
                            .filter(|url| !url.is_empty())
                    })
                    .or_else(|| Some(preview.thumb.as_str()).filter(|url| !url.is_empty()))
                    .unwrap_or(&preview.link);
                urls.push(self.absolute_media_url(candidate));
            }
            for link in format::find_links(&message.text) {
                let path = link
                    .url
                    .split(['?', '#'])
                    .next()
                    .unwrap_or(&link.url)
                    .to_lowercase();
                if [".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif", ".bmp"]
                    .iter()
                    .any(|extension| path.ends_with(extension))
                {
                    urls.push(self.absolute_media_url(&link.url));
                }
            }
        }
        let mut seen = HashSet::new();
        urls.retain(|url| seen.insert(url.clone()));
        urls
    }

    pub(super) fn touch_image(&self, url: &str) {
        let url = url.to_string();
        self.image_cache_current.borrow_mut().insert(url.clone());
        let mut touched = self.image_cache_touched.borrow_mut();
        if let Some(position) = touched.iter().position(|candidate| candidate == &url) {
            touched.remove(position);
        }
        touched.push_back(url);
    }

    pub(super) fn trim_image_cache(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let limit = self.config.image_cache_limit.max(64);
        let current = self.image_cache_current.borrow().clone();
        let mut touched = self.image_cache_touched.borrow_mut();
        while touched.len() > limit {
            let Some(position) = image_eviction_position(&touched, &current) else {
                break;
            };
            let Some(url) = touched.remove(position) else {
                break;
            };
            let resource = gpui::Resource::Uri(url.into());
            self.image_cache
                .update(cx, |cache, cx| cache.remove(&resource, window, cx));
        }
        trim_memory();
    }

    pub(super) fn absolute_media_url(&self, url: &str) -> String {
        if url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("data:")
            || url.starts_with("file:")
        {
            return url.to_string();
        }
        url::Url::parse(&format!("{}/", self.config.server.trim_end_matches('/')))
            .ok()
            .and_then(|base| base.join(url).ok())
            .map(|url| url.to_string())
            .unwrap_or_else(|| url.to_string())
    }
}

fn image_eviction_position(touched: &VecDeque<String>, current: &HashSet<String>) -> Option<usize> {
    touched.iter().position(|url| !current.contains(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_eviction_skips_currently_rendered_images() {
        let touched = VecDeque::from([
            "old".to_string(),
            "visible".to_string(),
            "older".to_string(),
        ]);
        let current = HashSet::from(["visible".to_string()]);
        assert_eq!(image_eviction_position(&touched, &current), Some(0));
    }

    #[test]
    fn image_eviction_returns_none_when_all_images_are_visible() {
        let touched = VecDeque::from(["one".to_string(), "two".to_string()]);
        let current = HashSet::from(["one".to_string(), "two".to_string()]);
        assert_eq!(image_eviction_position(&touched, &current), None);
    }
}
