use std::time::Duration;

use serde_json::Value;

const RELEASES_URL: &str = "https://api.github.com/repos/ddxtcc/thelounge-native/releases/latest";

pub fn latest_version() -> Result<Option<String>, String> {
    let mut response = ureq::get(RELEASES_URL)
        .config()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "lounge-native")
        .call()
        .map_err(|error| error.to_string())?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    let value: Value = serde_json::from_str(&body).map_err(|error| error.to_string())?;
    let latest = value
        .get("tag_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| "release response has no tag".to_string())?;
    let latest = latest.trim_start_matches('v').to_string();
    if is_newer(&latest, env!("CARGO_PKG_VERSION")) {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}

fn is_newer(latest: &str, current: &str) -> bool {
    let latest = parse_version(latest);
    let current = parse_version(current);
    latest > current
}

fn parse_version(version: &str) -> Vec<u64> {
    version
        .trim_start_matches('v')
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_release_versions() {
        assert!(is_newer("0.9.1", "0.9.0"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.9.0", "0.9.0"));
        assert!(!is_newer("0.8.9", "0.9.0"));
        assert!(!is_newer("v0.9.0", "0.9.0"));
    }
}
