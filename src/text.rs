//! One font and one layout rule for text rendering, hit testing and damage bounds.
use crate::image::Rect;
use std::sync::OnceLock;
static FONT: OnceLock<Option<fontdue::Font>> = OnceLock::new();
pub fn font() -> Option<&'static fontdue::Font> {
    FONT.get_or_init(|| {
        let mut paths = Vec::new();
        if let Ok(w) = std::env::var("WINDIR") {
            for n in ["segoeui.ttf", "arial.ttf", "tahoma.ttf"] {
                paths.push(format!("{w}/Fonts/{n}"));
            }
        }
        paths.extend(
            [
                "/usr/share/fonts/truetype/msttcorefonts/Arial.ttf",
                "/usr/share/fonts/msttcore/arial.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
                "/System/Library/Fonts/Supplemental/Arial.ttf",
            ]
            .map(String::from),
        );
        paths.into_iter().find_map(|p| {
            std::fs::read(p)
                .ok()
                .and_then(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).ok())
        })
    })
    .as_ref()
}
pub fn size(width: f32) -> f32 {
    (width * 7.).max(14.)
}
pub fn caret(text: &str, size: f32) -> (f32, f32) {
    let (mut x, mut y) = (0., 0.);
    for c in text.chars() {
        if c == '\n' {
            x = 0.;
            y += size * 1.4;
        } else {
            x += font()
                .map(|f| f.metrics(c, size).advance_width)
                .unwrap_or(size);
        }
    }
    (x, y)
}
pub fn bounds(text: &str, size: f32) -> Rect {
    let (mut x, mut y) = (0f32, 0f32);
    let (mut left, mut top, mut right, mut bottom) = (0f32, 0f32, 1f32, size * 1.4);
    for c in text.chars() {
        if c == '\n' {
            x = 0.;
            y += size * 1.4;
            bottom = bottom.max(y + size * 1.4);
            continue;
        }
        if let Some(f) = font() {
            let m = f.metrics(c, size);
            let xx = x.floor() + m.xmin as f32;
            let yy = (y + size * 0.8).floor() - m.ymin as f32 - m.height as f32;
            left = left.min(xx);
            top = top.min(yy);
            right = right.max(xx + m.width as f32);
            bottom = bottom.max(yy + m.height as f32);
            x += m.advance_width;
        } else {
            x += size;
        }
        right = right.max(x);
    }
    Rect {
        x: left.floor() as i32,
        y: top.floor() as i32,
        w: (right.ceil() - left.floor()).max(1.) as u32,
        h: (bottom.ceil() - top.floor()).max(1.) as u32,
    }
}
