use crate::{image::Rect, ui::Layout};
// Compatibility shim for old clients: selection-attached layouts ignore docking.
// No process-global position or mode is retained.
pub fn configure(_s: &str) {}
pub fn float(_x: i32, _y: i32) {}
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
