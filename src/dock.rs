use crate::{image::Rect, ui::Layout};
use std::sync::atomic::{AtomicI32, AtomicU8, Ordering};
static MODE: AtomicU8 = AtomicU8::new(0);
static X: AtomicI32 = AtomicI32::new(8);
static Y: AtomicI32 = AtomicI32::new(36);
pub fn configure(s: &str) {
    MODE.store(
        match s {
            "left" => 1,
            "right" => 2,
            "top" => 3,
            "bottom" => 4,
            "floating" => 5,
            _ => 0,
        },
        Ordering::Relaxed,
    );
}
pub fn float(x: i32, y: i32) {
    X.store(x, Ordering::Relaxed);
    Y.store(y, Ordering::Relaxed);
    MODE.store(5, Ordering::Relaxed);
}
pub fn snap(p: Rect, w: u32, h: u32) -> &'static str {
    if p.x <= 32 {
        "left"
    } else if p.x + p.w as i32 >= w as i32 - 32 {
        "right"
    } else if p.y <= 64 {
        "top"
    } else if p.y + p.h as i32 >= h as i32 - 32 {
        "bottom"
    } else {
        "floating"
    }
}
pub fn apply(_l: &mut Layout, _w: u32, _h: u32) {
    // 0.6: both styles are attached to the selection; old side preferences ignored.
}
