use super::*;

impl LoungeApp {
    pub(super) fn prompt_for_upload(&mut self, cx: &mut Context<Self>) {
        if !self
            .state
            .configuration
            .as_ref()
            .map(|config| config.file_upload)
            .unwrap_or(false)
        {
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
            return;
        };
        let display_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        self.toast(format!("Uploading {}...", display_name));
        let task = cx
            .background_executor()
            .spawn(async move { lounge_core::upload::upload_file(&server, &token, &path) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.uploads_in_flight = this.uploads_in_flight.saturating_sub(1);
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
    }

    pub(super) fn upload_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        for path in paths {
            if path.is_file() {
                self.queue_upload(path, cx);
            }
        }
    }
}
