pub fn http_url(text: &str) -> Option<String> {
    let s = text.trim();
    let rest = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))?;
    if s.len() > 4096
        || s.chars()
            .any(|c| c.is_whitespace() || c.is_control() || "<>\"\\".contains(c))
    {
        return None;
    }
    let host = rest.split(['/', '?', '#']).next()?;
    if host.is_empty() || host.contains('@') {
        return None;
    }
    Some(s.to_owned())
}
