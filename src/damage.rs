//! Conservative pixel damage geometry, shared by rendering and tests.
use crate::image::Rect;
pub fn union(a: Rect, b: Rect) -> Rect {
    if !a.valid() {
        return b;
    }
    if !b.valid() {
        return a;
    }
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect {
        x,
        y,
        w: ((a.x as i64 + a.w as i64).max(b.x as i64 + b.w as i64) - x as i64) as u32,
        h: ((a.y as i64 + a.h as i64).max(b.y as i64 + b.h as i64) - y as i64) as u32,
    }
}
pub fn expand(r: Rect, n: u32) -> Rect {
    Rect {
        x: r.x.saturating_sub(n as i32),
        y: r.y.saturating_sub(n as i32),
        w: r.w.saturating_add(n.saturating_mul(2)),
        h: r.h.saturating_add(n.saturating_mul(2)),
    }
}
