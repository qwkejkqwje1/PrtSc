use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
pub fn epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn unique_id() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{n:x}-{:x}", std::process::id())
}
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("LUMASHOT_DATA_DIR") {
        return PathBuf::from(p);
    }
    #[cfg(windows)]
    {
        return PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
            .join("PrtSc");
    };
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
                    .join(".local/share")
            })
            .join("luma-shot")
    }
}
pub fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub fn percent(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char)
        } else {
            out.push_str(&format!("%{b:02X}"))
        }
    }
    out
}
pub fn utc_stamp(epoch: u64) -> (String, String) {
    let days = (epoch / 86400) as i64;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += if m <= 2 { 1 } else { 0 };
    let date = format!("{y:04}{m:02}{d:02}");
    let seconds = epoch % 86400;
    (
        date.clone(),
        format!(
            "{date}T{:02}{:02}{:02}Z",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        ),
    )
}
pub fn safe_web_url(s: &str) -> bool {
    (s.starts_with("https://") || s.starts_with("http://")) && !s.chars().any(|c| c.is_control())
}

pub fn private_dir(path: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
