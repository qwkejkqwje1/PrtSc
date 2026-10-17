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
