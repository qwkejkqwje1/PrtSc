#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{capture_file, encoder_test, error, run};
#[cfg(not(windows))]
pub fn error(e: &str) {
    eprintln!("{e}")
}
#[cfg(not(windows))]
pub fn run(_demo: bool) -> Result<(), String> {
    Err("PrtSc 0.5: только Windows 10/11 x64".into())
}
#[cfg(not(windows))]
pub fn capture_file(_p: &std::path::Path) -> Result<(), String> {
    Err("Захват экрана доступен только в Windows".into())
}

#[cfg(not(windows))]
pub fn encoder_test(path: &std::path::Path) -> Result<(), String> {
    if path.extension().and_then(|s| s.to_str()) != Some("gif") {
        return Err(
            "MP4 requires Windows Media Foundation; use a .gif path for headless tests.".into(),
        );
    }
    let mut g = luma_shot::video::Gif::new(path, 160, 96, 15)?;
    for n in 0..30 {
        let mut i = luma_shot::image::Image::new(160, 96)?;
        for (p, c) in i.rgba.chunks_exact_mut(4).enumerate() {
            c.copy_from_slice(&[(p % 255) as u8, (n * 8) as u8, 90, 255]);
        }
        g.frame(&i)?;
    }
    g.finish()
}
