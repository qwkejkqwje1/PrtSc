//! Browser-only text translation. No image upload, API key or network client.
//! URLs are prepared only after an explicit click in the preview dialog.
pub const MAX_URL_BYTES: usize = 1800;
pub const TARGETS: [&str; 9] = ["ru", "en", "zh-CN", "de", "fr", "es", "ja", "ko", "uk"];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UrlError {
    Empty,
    TooLong,
    InvalidTarget,
    ControlCharacter,
}
pub fn landing_url(target: &str) -> Result<String, UrlError> {
    if !TARGETS.contains(&target) {
        return Err(UrlError::InvalidTarget);
    }
    Ok(format!(
        "https://translate.google.com/?sl=auto&tl={target}&op=translate"
    ))
}
pub fn text_url(text: &str, target: &str) -> Result<String, UrlError> {
    landing_url(target)?;
    let text = text.trim();
    if text.is_empty() {
        return Err(UrlError::Empty);
    }
    if text
        .chars()
        .any(|c| c.is_control() && !['\n', '\r', '\t'].contains(&c))
    {
        return Err(UrlError::ControlCharacter);
    }
    let mut url = format!("https://translate.google.com/?sl=auto&tl={target}&text=");
    const HEX: &[u8] = b"0123456789ABCDEF";
    let suffix = "&op=translate";
    for b in text.bytes() {
        let unreserved = b.is_ascii_alphanumeric() || b"-._~".contains(&b);
        if url.len() + if unreserved { 1 } else { 3 } + suffix.len() > MAX_URL_BYTES {
            return Err(UrlError::TooLong);
        }
        if unreserved {
            url.push(b as char);
        } else {
            url.push('%');
            url.push(HEX[(b >> 4) as usize] as char);
            url.push(HEX[(b & 15) as usize] as char);
        }
    }
    url.push_str(suffix);
    Ok(url)
}

/// Experimental, undocumented text endpoint. Only called after explicit user action.
pub const MAX_TEXT_CHARS: usize = 5_000;
pub const MAX_RESPONSE_BYTES: usize = 262_144;
pub fn encoded(text: &str) -> String {
    let mut out = String::new();
    const HEX: &[u8] = b"0123456789ABCDEF";
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char)
        } else {
            out.push('%');
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 15) as usize] as char);
        }
    }
    out
}
pub fn request_body(text: &str, target: &str) -> Result<Vec<u8>, &'static str> {
    if !TARGETS.contains(&target) {
        return Err("Invalid translation target");
    }
    if text.trim().is_empty() {
        return Err("No text to translate");
    }
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err("Text exceeds 5000 characters; use the browser. Nothing was truncated.");
    }
    if text
        .chars()
        .any(|c| c.is_control() && !['\r', '\n', '\t'].contains(&c))
    {
        return Err("Unsupported control character in text");
    }
    Ok(format!("client=gtx&sl=auto&tl={target}&dt=t&q={}", encoded(text)).into_bytes())
}
pub fn parse_response(bytes: &[u8]) -> Result<String, &'static str> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err("Translation response exceeds limit");
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| "Google returned an unexpected response; use the browser.")?;
    let segments = value
        .get(0)
        .and_then(|v| v.as_array())
        .filter(|v| !v.is_empty())
        .ok_or("Google returned no translation; use the browser.")?;
    let mut result = String::new();
    for segment in segments {
        let text = segment
            .as_array()
            .and_then(|s| s.first())
            .and_then(|v| v.as_str())
            .ok_or("Google changed its response format; use the browser.")?;
        if result.len() + text.len() > 100_000 {
            return Err("Translated text exceeds limit");
        }
        result.push_str(text);
    }
    if result.trim().is_empty() {
        return Err("Google returned no translation; use the browser.");
    }
    Ok(result)
}

/// Official browser image UI; the selected screenshot is supplied through clipboard.
pub fn image_url(target: &str) -> Result<String, &'static str> {
    if !TARGETS.contains(&target) {
        return Err("Invalid translation target");
    }
    Ok(format!(
        "https://translate.google.com/?sl=auto&tl={target}&op=images"
    ))
}

/// Ignore replies from a closed/reopened dialog or a changed source/target.
pub fn result_matches(
    request: u64,
    active: u64,
    source: &str,
    target: &str,
    current_source: &str,
    current_target: &str,
) -> bool {
    request != 0 && request == active && source == current_source && target == current_target
}
