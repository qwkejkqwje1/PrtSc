use crate::{config, image::Image, util};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub created: u64,
    pub width: u32,
    pub height: u32,
    pub text: String,
    pub note: String,
}
pub fn load() -> Vec<Entry> {
    std::fs::read(util::data_dir().join("history.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
pub fn path(e: &Entry) -> Result<PathBuf, String> {
    if e.id.is_empty() || !e.id.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-') {
        return Err("Unsafe history ID".into());
    }
    Ok(util::data_dir()
        .join("history")
        .join(format!("{}.png", e.id)))
}
pub fn add(image: &Image, text: &str, limit: usize) -> Result<(), String> {
    let e = Entry {
        id: util::unique_id(),
        created: util::epoch(),
        width: image.width,
        height: image.height,
        text: text.into(),
        note: String::new(),
    };
    util::private_dir(&util::data_dir())?;
    util::private_dir(&util::data_dir().join("history"))?;
    image.save(&path(&e)?)?;
    let mut entries = load();
    entries.insert(0, e);
    while entries.len() > limit {
        if let Some(e) = entries.pop() {
            if let Ok(p) = path(&e) {
                let _ = std::fs::remove_file(p);
            }
        }
    }
    config::atomic_write(
        &util::data_dir().join("history.json"),
        &serde_json::to_vec_pretty(&entries).map_err(|e| e.to_string())?,
    )
}
pub fn search<'a>(entries: &'a [Entry], query: &str) -> Vec<&'a Entry> {
    let q = query.to_lowercase();
    entries
        .iter()
        .filter(|e| {
            q.split_whitespace().all(|word| {
                format!("{} {} {} {}x{}", e.id, e.text, e.note, e.width, e.height)
                    .to_lowercase()
                    .contains(word)
            })
        })
        .collect()
}
pub fn clear() -> Result<(), String> {
    for e in load() {
        if let Ok(p) = path(&e) {
            let _ = std::fs::remove_file(p);
        }
    }
    config::atomic_write(&util::data_dir().join("history.json"), b"[]")
}
