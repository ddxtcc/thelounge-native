use std::path::PathBuf;

pub(super) fn download_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DOWNLOAD_DIR") {
        let dir = PathBuf::from(dir);
        if dir.is_absolute() {
            return Some(dir);
        }
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join("Downloads"))
}

pub(super) fn download_name(url: &str) -> String {
    let raw = url::Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed
                .path_segments()
                .and_then(|mut segments| segments.next_back().map(str::to_string))
        })
        .filter(|name| !name.is_empty() && name != "." && name != "..")
        .unwrap_or_else(|| "media".to_string());
    raw.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn open_download_file(url: &str) -> Result<(PathBuf, std::fs::File), String> {
    let dir = download_dir().ok_or_else(|| "Could not find a download directory.".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let name = download_name(url);
    let path = PathBuf::from(&name);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("media")
        .to_string();
    let extension = path.extension().and_then(|extension| extension.to_str());
    for attempt in 0..1000 {
        let filename = if attempt == 0 {
            name.clone()
        } else if let Some(extension) = extension {
            format!("{stem}-{attempt}.{extension}")
        } else {
            format!("{stem}-{attempt}")
        };
        let candidate = dir.join(filename);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Could not choose a download filename.".to_string())
}

pub(super) fn download_media_file(url: &str) -> Result<PathBuf, String> {
    let mut response = ureq::get(url)
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(600)))
        .build()
        .call()
        .map_err(|error| error.to_string())?;
    let (path, mut file) = open_download_file(url)?;
    let mut reader = response.body_mut().as_reader();
    if let Err(error) = std::io::copy(&mut reader, &mut file) {
        drop(file);
        let _ = std::fs::remove_file(&path);
        return Err(error.to_string());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_names_follow_urls() {
        assert_eq!(
            download_name("https://example.com/a/level-bg-2.mp3?x=1"),
            "level-bg-2.mp3"
        );
        assert_eq!(
            download_name("https://example.com/a/a%20b.mp3"),
            "a_20b.mp3"
        );
        assert_eq!(download_name("https://example.com/"), "media");
    }
}
