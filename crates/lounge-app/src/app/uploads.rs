use super::*;

impl LoungeApp {
    pub(super) fn paste_image_from_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else {
            self.toast("Clipboard is empty.");
            return;
        };
        let Some(image) = item.entries().iter().find_map(|entry| match entry {
            gpui::ClipboardEntry::Image(image) => Some(image),
            gpui::ClipboardEntry::String(_) => None,
        }) else {
            self.toast("Clipboard does not contain an image.");
            return;
        };
        let extension = match image.format() {
            gpui::ImageFormat::Png => "png",
            gpui::ImageFormat::Jpeg => "jpg",
            gpui::ImageFormat::Webp => "webp",
            gpui::ImageFormat::Gif => "gif",
            gpui::ImageFormat::Bmp => "bmp",
            gpui::ImageFormat::Tiff => "tiff",
            gpui::ImageFormat::Svg => "svg",
        };
        let temp = match tempfile::Builder::new()
            .prefix("lounge-native-clipboard-")
            .suffix(&format!(".{extension}"))
            .tempfile()
        {
            Ok(temp) => temp,
            Err(error) => {
                self.toast(format!("Could not save clipboard image: {error}"));
                return;
            }
        };
        if let Err(error) = std::fs::write(temp.path(), image.bytes()) {
            self.toast(format!("Could not save clipboard image: {error}"));
            return;
        }
        let path = match temp.keep() {
            Ok((_, path)) => path,
            Err(error) => {
                self.toast(format!("Could not save clipboard image: {}", error.error));
                return;
            }
        };
        self.queue_upload(path, cx);
    }

    fn uploads_enabled(&self) -> bool {
        self.state
            .configuration
            .as_ref()
            .map(|config| config.file_upload)
            .unwrap_or(false)
    }

    pub(super) fn reset_uploads(&mut self) {
        if let Some(active) = self.active_upload.take() {
            active.progress.cancel();
        }
        self.pending_uploads.clear();
        self.uploads_in_flight = 0;
    }

    pub(super) fn prompt_for_upload(&mut self, cx: &mut Context<Self>) {
        if !self.uploads_enabled() {
            self.toast("File uploads are disabled on this server.");
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Upload file".into()),
        });
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    let _ = this.update(cx, |this, cx| this.queue_upload(path, cx));
                }
            }
            Ok(Ok(None)) => {}
            Ok(Err(error)) => {
                let _ = this.update(cx, |this, _| {
                    this.toast(format!("File picker failed: {error}"))
                });
            }
            Err(_) => {}
        })
        .detach();
    }

    pub(super) fn queue_upload(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.uploads_enabled() {
            self.toast("File uploads are disabled on this server.");
            return;
        }
        let Some(conn) = &self.conn else {
            self.toast("Not connected.");
            return;
        };
        let max_size = self
            .state
            .configuration
            .as_ref()
            .and_then(|config| config.file_upload_max_file_size)
            .unwrap_or(0);
        if max_size > 0
            && let Ok(metadata) = std::fs::metadata(&path)
            && metadata.len() > max_size
        {
            self.toast(format!(
                "{} is larger than the server limit",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
            return;
        }
        self.pending_uploads.push(path);
        if self.uploads_in_flight == 0 {
            conn.emit("upload:auth", None);
            self.uploads_in_flight = 1;
        }
        cx.notify();
    }

    pub(super) fn upload_next(&mut self, token: String, cx: &mut Context<Self>) {
        let Some(path) = self.pending_uploads.first().cloned() else {
            self.uploads_in_flight = 0;
            return;
        };
        self.pending_uploads.remove(0);
        let Some(server) = self
            .state
            .configuration
            .as_ref()
            .map(|_| self.config.server.clone())
        else {
            self.uploads_in_flight = 0;
            self.active_upload = None;
            self.toast("Upload settings are unavailable.");
            return;
        };
        let display_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let total = std::fs::metadata(&path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let progress = lounge_core::upload::UploadProgress::new(total);
        self.active_upload = Some(ActiveUpload {
            name: display_name,
            progress: progress.clone(),
        });
        let task_progress = progress.clone();
        let cleanup = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("lounge-native-clipboard-"))
            .then(|| path.clone());
        let task = cx.background_executor().spawn(async move {
            lounge_core::upload::upload_file(&server, &token, &path, task_progress)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            if let Some(path) = cleanup {
                let _ = std::fs::remove_file(path);
            }
            let _ = this.update(cx, |this, cx| {
                this.uploads_in_flight = this.uploads_in_flight.saturating_sub(1);
                this.active_upload = None;
                match result {
                    Ok(url) => {
                        let content = this.inputs.composer.read(cx).content().to_string();
                        let separator = if content.is_empty() || content.ends_with(' ') {
                            ""
                        } else {
                            " "
                        };
                        this.inputs.composer.update(cx, |input, cx| {
                            input.set_content(format!("{content}{separator}{url} "), cx)
                        });
                    }
                    Err(_error) if progress.is_cancelled() => {
                        this.toast("Upload cancelled.");
                    }
                    Err(error) => this.toast(error),
                }
                if !this.pending_uploads.is_empty()
                    && let Some(conn) = &this.conn
                {
                    this.uploads_in_flight += 1;
                    conn.emit("upload:auth", None);
                }
                cx.notify();
            });
        })
        .detach();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
                let active = this.update(cx, |this, cx| {
                    if this.active_upload.is_some() {
                        cx.notify();
                        true
                    } else {
                        false
                    }
                });
                if !matches!(active, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn upload_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        for path in paths {
            if path.is_file() {
                self.queue_upload(path, cx);
            }
        }
    }
}
