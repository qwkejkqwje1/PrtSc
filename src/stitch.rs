use crate::image::{Image, MAX_PIXELS};
/// Estimate scroll distance using overlap, ignoring the first/last 24 pixels
/// (common sticky bars). Returns None for duplicate or unmatchable frames.
pub fn offset(old: &Image, new: &Image) -> Option<u32> {
    if old.width != new.width || old.height != new.height || old.height < 96 {
        return None;
    }
    let w = old.width;
    let h = old.height;
    let mut best = (f64::MAX, 0);
    let mut zero = 0u64;
    let mut zn = 0u64;
    for y in (24..h - 24).step_by(13) {
        for x in (8..w.saturating_sub(8)).step_by((w / 37).max(1) as usize) {
            let a = old.pixel(x, y);
            let b = new.pixel(x, y);
            zero += a[..3]
                .iter()
                .zip(&b)
                .map(|(a, b)| a.abs_diff(*b) as u64)
                .sum::<u64>();
            zn += 3
        }
    }
    if zn == 0 || zero as f64 / (zn as f64) < 0.7 {
        return None;
    }
    for dy in 8..h - 48 {
        let mut sum = 0u64;
        let mut n = 0;
        for y in (24..(h - dy - 24).min(260)).step_by(11) {
            for x in (8..w.saturating_sub(8)).step_by((w / 37).max(1) as usize) {
                let a = old.pixel(x, y + dy);
                let b = new.pixel(x, y);
                sum += a[..3]
                    .iter()
                    .zip(&b)
                    .map(|(a, b)| a.abs_diff(*b) as u64)
                    .sum::<u64>();
                n += 3
            }
        }
        if n > 0 {
            let score = sum as f64 / n as f64;
            if score < best.0 {
                best = (score, dy)
            }
        }
    }
    if best.0 < 12.0 {
        Some(best.1)
    } else {
        None
    }
}
pub fn append(tall: &Image, new: &Image, dy: u32) -> Result<Image, String> {
    if tall.width != new.width || dy == 0 || dy > new.height {
        return Err("Invalid scrolling frame".into());
    }
    let h = tall
        .height
        .checked_add(dy)
        .ok_or("Scrolling image too tall")?;
    if h as u64 * tall.width as u64 > MAX_PIXELS {
        return Err("Scrolling image exceeds 120 MP".into());
    }
    let mut out = Image::new(tall.width, h)?;
    out.rgba[..tall.rgba.len()].copy_from_slice(&tall.rgba);
    let start = (new.height - dy) as usize * new.width as usize * 4;
    out.rgba[tall.rgba.len()..].copy_from_slice(&new.rgba[start..]);
    Ok(out)
}
