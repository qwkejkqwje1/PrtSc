use crate::util;
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub language: String,
    pub window_capture: bool,
    pub toolbar: String,
    pub toolbar_dock: String,
    pub tool_colors: std::collections::BTreeMap<String, [u8; 4]>,
    pub search_mode: String,
    pub capture_hotkey: String,
    pub translation_target: String,
    pub video_cursor: bool,
    pub imgur_client_id: String,
    pub save_dir: String,
    pub format: String,
    pub jpeg_quality: u8,
    pub png_compression: String,
    pub auto_qr: bool,
    pub sound_enabled: bool,
    pub confirm_google: bool,
    pub video_format: String,
    pub video_fps: u32,
    pub video_bitrate: u32,
    pub upload_host: String,
    pub palette: [[u8; 4]; 7],
    pub history_enabled: bool,
    pub history_limit: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "ru".into(),
            window_capture: false,
            toolbar: "edge".into(),
            toolbar_dock: "auto".into(),
            tool_colors: Default::default(),
            search_mode: "fast".into(),
            capture_hotkey: "PrintScreen".into(),
            translation_target: "ru".into(),
            video_cursor: true,
            imgur_client_id: String::new(),
            save_dir: std::env::var_os("USERPROFILE")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| ".".into())
                .join("Pictures")
                .join("PrtSc")
                .to_string_lossy()
                .into(),
            format: "png".into(),
            jpeg_quality: 92,
            png_compression: "fast".into(),
            auto_qr: true,
            sound_enabled: false,
            confirm_google: false,
            video_format: "mp4".into(),
            video_fps: 15,
            video_bitrate: 6_000_000,
            upload_host: "litterbox".into(),
            palette: crate::ui::COLORS,
            history_enabled: false,
            history_limit: 200,
        }
    }
}
impl Config {
    pub fn color_for(&self, tool: crate::editor::Tool) -> [u8; 4] {
        if tool == crate::editor::Tool::Redact {
            return [24, 24, 27, 255];
        }
        self.tool_colors.get(tool.key()).copied().unwrap_or(
            if tool == crate::editor::Tool::Highlight {
                [240, 177, 47, 255]
            } else {
                [39, 131, 222, 255]
            },
        )
    }
    pub fn remember_color(&mut self, tool: crate::editor::Tool, mut color: [u8; 4]) {
        if tool != crate::editor::Tool::Redact {
            color[3] = 255;
            self.tool_colors.insert(tool.key().into(), color);
        }
    }
    pub fn load() -> Self {
        let mut config: Self = std::fs::read(util::data_dir().join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default();
        if config.toolbar == "standard" {
            config.toolbar = "compact".into();
        }
        if config.toolbar == "minimal" || config.toolbar == "lightshot" {
            config.toolbar = "edge".into();
        }
        config.toolbar_dock = "auto".into();
        config
    }
    pub fn validate(&self) -> Result<(), String> {
        crate::hotkey::parse(&self.capture_hotkey)?;
        if !crate::translate::TARGETS.contains(&self.translation_target.as_str()) {
            return Err("Invalid translation target".into());
        }
        if !["ru", "en"].contains(&self.language.as_str()) {
            return Err("Language: ru / en".into());
        }
        if !["standard", "compact", "minimal", "lightshot", "edge"].contains(&self.toolbar.as_str())
        {
            return Err("Toolbar: compact / edge".into());
        }
        if !["auto", "left", "right", "top", "bottom", "floating"]
            .contains(&self.toolbar_dock.as_str())
        {
            return Err("Invalid toolbar dock".into());
        }
        if !["fast", "original"].contains(&self.search_mode.as_str()) {
            return Err("Invalid search mode".into());
        }
        if !["dark", "light", "system"].contains(&self.theme.as_str()) {
            return Err("Некорректная тема".into());
        }
        if !(1..=10_000).contains(&self.history_limit) {
            return Err("Лимит истории: от 1 до 10000".into());
        }
        if !["png", "jpg", "webp"].contains(&self.format.as_str()) {
            return Err("Выберите PNG, JPEG или WebP".into());
        }
        if !(1..=100).contains(&self.jpeg_quality) {
            return Err("Качество JPEG: от 1 до 100".into());
        }
        if !["fast", "default", "best"].contains(&self.png_compression.as_str()) {
            return Err("Некорректное сжатие PNG".into());
        }
        if self.save_dir.trim().is_empty() {
            return Err("Укажите папку сохранения".into());
        }
        if !["mp4", "gif"].contains(&self.video_format.as_str()) {
            return Err("Формат видео: MP4 или GIF".into());
        }
        if ![10, 15, 24, 30].contains(&self.video_fps) {
            return Err("Частота видео: 10, 15, 24 или 30 кадров/с".into());
        }
        if !(1_000_000..=40_000_000).contains(&self.video_bitrate) {
            return Err("Битрейт видео: 1–40 Мбит/с".into());
        }
        if !["litterbox", "catbox", "imgur"].contains(&self.upload_host.as_str()) {
            return Err("Выберите Litterbox, Catbox или Imgur".into());
        }
        Ok(())
    }
    pub fn save(&self) -> Result<(), String> {
        self.validate()?;
        let dir = util::data_dir();
        util::private_dir(&dir)?;
        atomic_write(
            &dir.join("settings.json"),
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension(format!("{}.tmp", util::unique_id()));
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let a: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let ok = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                a.as_ptr(),
                b.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            let _ = std::fs::remove_file(&tmp);
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    }
}
