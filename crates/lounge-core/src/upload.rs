use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use ureq::unversioned::multipart::{Form, Part};

#[derive(Clone)]
pub struct UploadProgress {
    sent: Arc<AtomicU64>,
    total: u64,
    cancelled: Arc<AtomicBool>,
}

impl UploadProgress {
    pub fn new(total: u64) -> Self {
        Self {
            sent: Arc::new(AtomicU64::new(0)),
            total,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn sent(&self) -> u64 {
        self.sent.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

struct ProgressReader {
    file: File,
    progress: UploadProgress,
}

impl Read for ProgressReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.progress.is_cancelled() {
            return Err(io::Error::other("upload cancelled"));
        }
        let read = loop {
            match self.file.read(buffer) {
                Ok(read) => break read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        };
        self.progress.sent.fetch_add(read as u64, Ordering::Relaxed);
        Ok(read)
    }
}

pub fn upload_file(
    base_url: &str,
    token: &str,
    path: &Path,
    progress: UploadProgress,
) -> Result<String, String> {
    let base_url = base_url.trim_end_matches('/');
    let endpoint = format!("{base_url}/uploads/new/{token}");
    let file = File::open(path).map_err(|error| format!("could not open file: {error}"))?;
    let display_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    let part = Part::owned_reader(ProgressReader {
        file,
        progress: progress.clone(),
    })
    .file_name(&display_name)
    .mime_str("application/octet-stream")
    .map_err(|error| format!("invalid file type: {error}"))?;
    let form = Form::new().part("file", part);

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_send_body(Some(Duration::from_secs(300)))
        .timeout_recv_body(Some(Duration::from_secs(30)))
        .build()
        .new_agent();
    let mut response = agent.post(&endpoint).send(form).map_err(|error| {
        format!(
            "upload failed: {}",
            error.to_string().replace(token, "[redacted]")
        )
    })?;

    let body: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| format!("invalid upload response: {error}"))?;

    if let Some(error) = body.get("error").and_then(|error| error.as_str()) {
        return Err(error.to_string());
    }

    let url = body
        .get("url")
        .and_then(|url| url.as_str())
        .ok_or_else(|| "upload response did not include a URL".to_string())?;

    let base = url::Url::parse(&format!("{base_url}/"))
        .map_err(|error| format!("invalid server URL: {error}"))?;
    base.join(url)
        .map(|url| url.to_string())
        .map_err(|error| format!("invalid upload URL: {error}"))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    use super::*;

    fn read_chunked(stream: &mut std::net::TcpStream, initial: &[u8]) -> Vec<u8> {
        let mut buffer = initial.to_vec();
        let mut body = Vec::new();
        loop {
            let line_end = loop {
                if let Some(position) = buffer.windows(2).position(|window| window == b"\r\n") {
                    break position;
                }
                let mut chunk = [0u8; 8192];
                let read = stream.read(&mut chunk).expect("chunk size read");
                if read == 0 {
                    return body;
                }
                buffer.extend_from_slice(&chunk[..read]);
            };
            let size_line = String::from_utf8_lossy(&buffer[..line_end]).to_string();
            let size = usize::from_str_radix(size_line.trim(), 16).expect("hex chunk size");
            buffer.drain(..line_end + 2);
            if size == 0 {
                return body;
            }
            while buffer.len() < size + 2 {
                let mut chunk = [0u8; 8192];
                let read = stream.read(&mut chunk).expect("chunk body");
                if read == 0 {
                    return body;
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
            body.extend_from_slice(&buffer[..size]);
            buffer.drain(..size + 2);
        }
    }

    #[test]
    fn cancellation_uses_a_non_retryable_error_kind() {
        let path =
            std::env::temp_dir().join(format!("lounge-cancel-test-{}.bin", std::process::id()));
        std::fs::write(&path, b"payload").expect("temp file");
        let progress = UploadProgress::new(7);
        progress.cancel();
        let mut reader = ProgressReader {
            file: File::open(&path).expect("open"),
            progress,
        };
        let mut buffer = [0u8; 7];
        let error = reader.read(&mut buffer).expect_err("cancelled");
        assert_ne!(error.kind(), io::ErrorKind::Interrupted);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn streams_multipart_upload_and_reports_progress() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let (body_tx, body_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut headers = Vec::new();
            let header_end = loop {
                let mut byte = [0u8; 1];
                stream.read_exact(&mut byte).expect("header");
                headers.push(byte[0]);
                if headers.ends_with(b"\r\n\r\n") {
                    break headers.len();
                }
            };
            let request = String::from_utf8_lossy(&headers[..header_end]).to_string();
            let mut rest = headers[header_end..].to_vec();
            let chunked = request
                .to_lowercase()
                .contains("transfer-encoding: chunked");
            let body = if chunked {
                read_chunked(&mut stream, &rest)
            } else {
                let length = request
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while rest.len() < length {
                    let mut chunk = [0u8; 8192];
                    let read = stream.read(&mut chunk).expect("body");
                    if read == 0 {
                        break;
                    }
                    rest.extend_from_slice(&chunk[..read]);
                }
                rest
            };
            body_tx.send(body).expect("body channel");
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 27\r\nconnection: close\r\n\r\n{\"url\":\"/uploads/file.zip\"}",
                )
                .expect("response");
        });

        let path =
            std::env::temp_dir().join(format!("lounge-upload-test-{}.bin", std::process::id()));
        std::fs::write(&path, b"upload payload").expect("temp file");
        let progress = UploadProgress::new(14);
        let result = upload_file(
            &format!("http://{address}"),
            "test-token",
            &path,
            progress.clone(),
        );
        let body = body_rx.recv().expect("body");
        server.join().expect("server");
        let _ = std::fs::remove_file(&path);

        assert!(result.is_ok(), "{result:?}");
        assert!(body.windows(14).any(|window| window == b"upload payload"));
        assert_eq!(progress.sent(), 14);
    }
}
