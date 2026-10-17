use crate::{
    editor::{Annotation, Point, Tool},
    image::{Image, Rect},
};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, PixmapMut, Stroke, Transform};
type Glyph = (fontdue::Metrics, std::rc::Rc<[u8]>);
pub struct Renderer {
    pub font: Option<&'static fontdue::Font>,
    glyphs: std::cell::RefCell<std::collections::HashMap<(char, u32), Glyph>>,
    glyph_bytes: std::cell::Cell<usize>,
}
impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
impl Renderer {
    pub fn unloaded() -> Self {
        Self {
            font: None,
            glyphs: Default::default(),
            glyph_bytes: Default::default(),
        }
    }
    pub fn new() -> Self {
        Self {
            font: crate::text::font(),
            ..Self::unloaded()
        }
    }
    fn glyph(&self, ch: char, size: f32) -> Option<Glyph> {
        let font = self.font?;
        let key = (ch, size.to_bits());
        if let Some(g) = self.glyphs.borrow().get(&key) {
            return Some(g.clone());
        }
        let (m, b) = font.rasterize(ch, size);
        let bytes = b.len();
        let g = (m, std::rc::Rc::<[u8]>::from(b));
        // QR/text input must not grow an unbounded glyph cache.
        if bytes <= 4 * 1024 * 1024 {
            let mut cache = self.glyphs.borrow_mut();
            if cache.len() >= 1024 || self.glyph_bytes.get() + bytes > 4 * 1024 * 1024 {
                cache.clear();
                self.glyph_bytes.set(0);
            }
            cache.insert(key, g.clone());
            self.glyph_bytes.set(self.glyph_bytes.get() + bytes);
        }
        Some(g)
    }
    pub fn glyph_cache_stats(&self) -> (usize, usize) {
        (self.glyphs.borrow().len(), self.glyph_bytes.get())
    }
    pub fn bounds(&self, a: &Annotation) -> Rect {
        let r = a.rect();
        let pad = match a.tool {
            Tool::Text => 2.,
            Tool::Arrow => (a.width * 4.5).max(14.) + a.width,
            Tool::Step => 4.,
            Tool::Redact | Tool::Highlight | Tool::Blur | Tool::Pixelate => 0.,
            _ => a.width * 0.5 + 3.,
        };
        let mut result = crate::damage::expand(r, pad.ceil() as u32);
        if a.tool == Tool::Step && !a.points.is_empty() {
            let t = a.number.to_string();
            let w = self.text_width(&t, 18.);
            let rr = crate::text::bounds(&t, 18.);
            let p = a.points[0];
            result = crate::damage::union(
                result,
                Rect {
                    x: (p.x - w / 2.).floor() as i32 + rr.x,
                    y: (p.y - 9.).floor() as i32 + rr.y,
                    w: rr.w,
                    h: rr.h,
                },
            );
        }
        result
    }
    pub fn text(&self, img: &mut Image, x: f32, y: f32, text: &str, size: f32, color: [u8; 4]) {
        if self.font.is_none() {
            return;
        }
        let mut px = x;
        let mut py = y;
        for ch in text.chars() {
            if ch == '\n' {
                px = x;
                py += size * 1.4;
                continue;
            }
            let Some((m, b)) = self.glyph(ch, size) else {
                continue;
            };
            for gy in 0..m.height {
                for gx in 0..m.width {
                    let xx = px as i32 + m.xmin + gx as i32;
                    let yy = (py + size * 0.8) as i32 - m.ymin - m.height as i32 + gy as i32;
                    if xx >= 0 && yy >= 0 && xx < img.width as i32 && yy < img.height as i32 {
                        let alpha = b[gy * m.width + gx] as u32 * color[3] as u32 / 255;
                        let i = (yy as usize * img.width as usize + xx as usize) * 4;
                        for (k, &c) in color[..3].iter().enumerate() {
                            img.rgba[i + k] = ((c as u32 * alpha
                                + img.rgba[i + k] as u32 * (255 - alpha))
                                / 255) as u8;
                        }
                        img.rgba[i + 3] = 255
                    }
                }
            }
            px += m.advance_width;
        }
    }
    pub fn text_width(&self, text: &str, size: f32) -> f32 {
        self.font
            .as_ref()
            .map(|f| text.chars().map(|c| f.metrics(c, size).advance_width).sum())
            .unwrap_or(text.len() as f32 * size * 0.5)
    }
    pub fn elide(&self, text: &str, size: f32, max_width: f32) -> String {
        if self.text_width(text, size) <= max_width {
            return text.into();
        }
        let mut result = String::new();
        let mut width = 0.;
        let ellipsis = self.text_width("…", size);
        if ellipsis > max_width {
            return result;
        }
        for c in text.chars() {
            let cw = self
                .font
                .map(|f| f.metrics(c, size).advance_width)
                .unwrap_or(size);
            if width + cw + ellipsis > max_width {
                break;
            }
            width += cw;
            result.push(c);
        }
        result.push('…');
        result
    }
    pub fn flatten(&self, base: &Image, shapes: &[Annotation]) -> Image {
        let mut out = base.clone();
        for a in shapes {
            self.annotation(&mut out, a)
        }
        out
    }
    pub fn annotation(&self, out: &mut Image, a: &Annotation) {
        if a.points.is_empty() {
            return;
        }
        let p = a.points[0];
        let q = *a.points.last().unwrap();
        let r = a.rect().clamp(out.width, out.height);
        let color = a.color;
        let line = |o: &mut Image, p: Point, q: Point, c: [u8; 4], w: f32| {
            let mut b = PathBuilder::new();
            b.move_to(p.x, p.y);
            b.line_to(q.x, q.y);
            if let Some(path) = b.finish() {
                let mut paint = Paint::default();
                paint.set_color_rgba8(c[0], c[1], c[2], c[3]);
                paint.anti_alias = true;
                let stroke = Stroke {
                    width: w,
                    line_cap: tiny_skia::LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Stroke::default()
                };
                if let Some(mut pm) = PixmapMut::from_bytes(&mut o.rgba, o.width, o.height) {
                    pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None)
                }
            }
        };
        match a.tool {
            Tool::Pen => {
                let mut b = PathBuilder::new();
                b.move_to(p.x, p.y);
                for p in a.points.iter().skip(1) {
                    b.line_to(p.x, p.y)
                }
                if let Some(path) = b.finish() {
                    let mut paint = Paint::default();
                    paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
                    paint.anti_alias = true;
                    let stroke = Stroke {
                        width: a.width,
                        line_cap: tiny_skia::LineCap::Round,
                        line_join: tiny_skia::LineJoin::Round,
                        ..Stroke::default()
                    };
                    if let Some(mut pm) =
                        PixmapMut::from_bytes(&mut out.rgba, out.width, out.height)
                    {
                        pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None)
                    }
                }
            }
            Tool::Line | Tool::Arrow => {
                line(out, p, q, color, a.width);
                if a.tool == Tool::Arrow {
                    let angle = (q.y - p.y).atan2(q.x - p.x);
                    let len = (a.width * 4.5).max(14.);
                    for da in [-0.48f32, 0.48] {
                        line(
                            out,
                            q,
                            Point::new(
                                q.x - len * (angle + da).cos(),
                                q.y - len * (angle + da).sin(),
                            ),
                            color,
                            a.width,
                        );
                    }
                }
            }
            Tool::Rectangle => stroke_rect(out, r, color, a.width),
            Tool::Ellipse => ellipse(out, r, color, a.width, false),
            Tool::Highlight => {
                let mut c = color;
                c[3] = 110;
                multiply_rect(out, r, c)
            }
            Tool::Redact => fill_rect(out, r, [24, 24, 27, 255]),
            Tool::Pixelate => pixelate(out, r, (a.width * 4.).max(8.) as u32),
            Tool::Blur => blur(out, r, (a.width * 3.).clamp(3., 36.) as u32),
            Tool::Text => self.text(out, p.x, p.y, &a.text, (a.width * 7.).max(14.), color),
            Tool::Step => {
                let r = Rect {
                    x: p.x as i32 - 17,
                    y: p.y as i32 - 17,
                    w: 34,
                    h: 34,
                };
                ellipse(out, r, color, 1., true);
                let s = a.number.to_string();
                let w = self.text_width(&s, 18.);
                self.text(out, p.x - w / 2., p.y - 9., &s, 18., contrast_ink(color));
            }
            _ => {}
        }
    }
}
/// Multiply highlighter preserves black text and uses the same opaque-image contract as fill_rect.
pub fn multiply_rect(img: &mut Image, r: Rect, c: [u8; 4]) {
    let r = r.clamp(img.width, img.height);
    if !r.valid() {
        return;
    }
    let Some(rr) = tiny_skia::Rect::from_xywh(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
    else {
        return;
    };
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    p.blend_mode = tiny_skia::BlendMode::Multiply;
    if let Some(mut pm) = PixmapMut::from_bytes(&mut img.rgba, img.width, img.height) {
        pm.fill_rect(rr, &p, Transform::identity(), None);
    }
}
pub fn fill_rect(img: &mut Image, r: Rect, c: [u8; 4]) {
    let r = r.clamp(img.width, img.height);
    if !r.valid() {
        return;
    }
    let Some(rr) = tiny_skia::Rect::from_xywh(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
    else {
        return;
    };
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    if let Some(mut pm) = PixmapMut::from_bytes(&mut img.rgba, img.width, img.height) {
        pm.fill_rect(rr, &p, Transform::identity(), None)
    }
}
pub fn stroke_rect(img: &mut Image, r: Rect, c: [u8; 4], width: f32) {
    if !r.valid() {
        return;
    }
    let Some(rr) = tiny_skia::Rect::from_xywh(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
    else {
        return;
    };
    let path = PathBuilder::from_rect(rr);
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    p.anti_alias = true;
    if let Some(mut pm) = PixmapMut::from_bytes(&mut img.rgba, img.width, img.height) {
        pm.stroke_path(
            &path,
            &p,
            &Stroke {
                width,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        )
    }
}
pub fn round_rect(img: &mut Image, r: Rect, rad: f32, c: [u8; 4]) {
    if r.w == 0 || r.h == 0 {
        return;
    }
    let x = r.x as f32;
    let y = r.y as f32;
    let w = r.w as f32;
    let h = r.h as f32;
    let rad = rad.min(w / 2.).min(h / 2.);
    let k = 0.5522848;
    let mut b = PathBuilder::new();
    b.move_to(x + rad, y);
    b.line_to(x + w - rad, y);
    b.cubic_to(
        x + w - rad + rad * k,
        y,
        x + w,
        y + rad - rad * k,
        x + w,
        y + rad,
    );
    b.line_to(x + w, y + h - rad);
    b.cubic_to(
        x + w,
        y + h - rad + rad * k,
        x + w - rad + rad * k,
        y + h,
        x + w - rad,
        y + h,
    );
    b.line_to(x + rad, y + h);
    b.cubic_to(
        x + rad - rad * k,
        y + h,
        x,
        y + h - rad + rad * k,
        x,
        y + h - rad,
    );
    b.line_to(x, y + rad);
    b.cubic_to(x, y + rad - rad * k, x + rad - rad * k, y, x + rad, y);
    b.close();
    if let Some(path) = b.finish() {
        let mut p = Paint::default();
        p.set_color_rgba8(c[0], c[1], c[2], c[3]);
        p.anti_alias = true;
        if let Some(mut pm) = PixmapMut::from_bytes(&mut img.rgba, img.width, img.height) {
            pm.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None)
        }
    }
}
pub fn ellipse(img: &mut Image, r: Rect, c: [u8; 4], width: f32, fill: bool) {
    let Some(rr) = tiny_skia::Rect::from_xywh(r.x as f32, r.y as f32, r.w as f32, r.h as f32)
    else {
        return;
    };
    let Some(path) = PathBuilder::from_oval(rr) else {
        return;
    };
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    p.anti_alias = true;
    if let Some(mut pm) = PixmapMut::from_bytes(&mut img.rgba, img.width, img.height) {
        if fill {
            pm.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None)
        } else {
            pm.stroke_path(
                &path,
                &p,
                &Stroke {
                    width,
                    ..Stroke::default()
                },
                Transform::identity(),
                None,
            )
        }
    }
}
fn pixelate(img: &mut Image, r: Rect, block: u32) {
    let r = r.clamp(img.width, img.height);
    if !r.valid() {
        return;
    }
    for y in (r.y as u32..r.y as u32 + r.h).step_by(block as usize) {
        for x in (r.x as u32..r.x as u32 + r.w).step_by(block as usize) {
            let w = block.min(r.x as u32 + r.w - x);
            let h = block.min(r.y as u32 + r.h - y);
            let mut sum = [0u64; 3];
            for yy in y..y + h {
                for xx in x..x + w {
                    let p = img.pixel(xx, yy);
                    for k in 0..3 {
                        sum[k] += p[k] as u64
                    }
                }
            }
            fill_rect(
                img,
                Rect {
                    x: x as i32,
                    y: y as i32,
                    w,
                    h,
                },
                [
                    (sum[0] / (w * h) as u64) as u8,
                    (sum[1] / (w * h) as u64) as u8,
                    (sum[2] / (w * h) as u64) as u8,
                    255,
                ],
            );
        }
    }
}
fn blur(img: &mut Image, r: Rect, rad: u32) {
    let r = r.clamp(img.width, img.height);
    if !r.valid() {
        return;
    }
    let Ok(mut temp) = img.crop(r) else { return };
    let original = temp.rgba.clone();
    for y in 0..r.h {
        let mut sums = [0u64; 3];
        let mut left = 0;
        let mut right = 0;
        for x in 0..r.w {
            let end = (x + rad + 1).min(r.w);
            while right < end {
                let i = (y as usize * r.w as usize + right as usize) * 4;
                for k in 0..3 {
                    sums[k] += original[i + k] as u64
                }
                right += 1
            }
            let begin = x.saturating_sub(rad);
            while left < begin {
                let i = (y as usize * r.w as usize + left as usize) * 4;
                for k in 0..3 {
                    sums[k] -= original[i + k] as u64
                }
                left += 1
            }
            let i = (y as usize * r.w as usize + x as usize) * 4;
            for k in 0..3 {
                temp.rgba[i + k] = (sums[k] / (right - left) as u64) as u8
            }
        }
    }
    for x in 0..r.w {
        let mut sums = [0u64; 3];
        let mut top = 0;
        let mut bottom = 0;
        for y in 0..r.h {
            let end = (y + rad + 1).min(r.h);
            while bottom < end {
                let p = temp.pixel(x, bottom);
                for k in 0..3 {
                    sums[k] += p[k] as u64
                }
                bottom += 1
            }
            let begin = y.saturating_sub(rad);
            while top < begin {
                let p = temp.pixel(x, top);
                for k in 0..3 {
                    sums[k] -= p[k] as u64
                }
                top += 1
            }
            let i =
                ((r.y as u32 + y) as usize * img.width as usize + (r.x as u32 + x) as usize) * 4;
            for k in 0..3 {
                img.rgba[i + k] = (sums[k] / (bottom - top) as u64) as u8
            }
        }
    }
}
pub fn resized(src: &Image, w: u32, h: u32) -> Result<Image, String> {
    let mut out = Image::new(w, h)?;
    resize_into(src, &mut out)?;
    Ok(out)
}
pub fn resize_into(src: &Image, out: &mut Image) -> Result<(), String> {
    let (w, h) = (out.width, out.height);
    let pix = tiny_skia::PixmapRef::from_bytes(&src.rgba, src.width, src.height)
        .ok_or("Invalid source")?;
    let mut pm = PixmapMut::from_bytes(&mut out.rgba, w, h).ok_or("Invalid target")?;
    pm.fill(Color::from_rgba8(20, 22, 26, 255));
    pm.draw_pixmap(
        0,
        0,
        pix,
        &tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
        },
        Transform::from_scale(w as f32 / src.width as f32, h as f32 / src.height as f32),
        None,
    );
    Ok(())
}

/// Select black or white text with WCAG AA contrast on an opaque swatch.
pub fn contrast_ink(c: [u8; 4]) -> [u8; 4] {
    let component = |v: u8| {
        let v = v as f64 / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let l = 0.2126 * component(c[0]) + 0.7152 * component(c[1]) + 0.0722 * component(c[2]);
    if l > 0.179 {
        [0, 0, 0, 255]
    } else {
        [255, 255, 255, 255]
    }
}
