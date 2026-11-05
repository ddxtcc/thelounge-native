use std::path::Path;

use ureq::unversioned::multipart::Form;

pub fn upload_file(base_url: &str, token: &str, path: &Path) -> Result<String, String> {
    let base_url = base_url.trim_end_matches('/');
    let endpoint = format!("{base_url}/uploads/new/{token}");
    let form = Form::new()
        .file("file", path)
        .map_err(|error| format!("could not open file: {error}"))?;

    let mut response = ureq::post(&endpoint)
        .send(form)
        .map_err(|error| format!("upload failed: {error}"))?;

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
