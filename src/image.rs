use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

pub const MAX_PIXELS: u64 = 120_000_000;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}
impl Rect {
    pub fn from_points(a: (i32, i32), b: (i32, i32)) -> Self {
        Self {
            x: a.0.min(b.0),
            y: a.1.min(b.1),
            w: a.0.abs_diff(b.0),
            h: a.1.abs_diff(b.1),
        }
    }
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x
            && y >= self.y
            && (x as i64) < self.x as i64 + self.w as i64
            && (y as i64) < self.y as i64 + self.h as i64
    }
    pub fn clamp(self, width: u32, height: u32) -> Self {
        let x = (self.x as i64).clamp(0, width as i64);
        let y = (self.y as i64).clamp(0, height as i64);
        let r = (self.x as i64 + self.w as i64).clamp(x, width as i64);
        let b = (self.y as i64 + self.h as i64).clamp(y, height as i64);
        Self {
            x: x as i32,
            y: y as i32,
            w: (r - x) as u32,
            h: (b - y) as u32,
        }
    }
    pub fn valid(self) -> bool {
        self.w > 0 && self.h > 0
    }
}
impl Image {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let n = width as u64 * height as u64;
        if width == 0 || height == 0 || n > MAX_PIXELS {
            return Err("Invalid image dimensions or image exceeds 120 MP".into());
        }
        Ok(Self {
            width,
            height,
            rgba: vec![255; n as usize * 4],
        })
    }
    pub fn validate(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.width as u64 * self.height as u64 <= MAX_PIXELS
            && self.rgba.len() == self.width as usize * self.height as usize * 4
    }
    pub fn crop(&self, r: Rect) -> Result<Self, String> {
        let r = r.clamp(self.width, self.height);
        let mut out = Self::new(r.w, r.h)?;
        for y in 0..r.h as usize {
            let s = ((y + r.y as usize) * self.width as usize + r.x as usize) * 4;
            let d = y * r.w as usize * 4;
            out.rgba[d..d + r.w as usize * 4].copy_from_slice(&self.rgba[s..s + r.w as usize * 4]);
        }
        Ok(out)
    }
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y.min(self.height - 1) as usize * self.width as usize
            + x.min(self.width - 1) as usize)
            * 4;
        self.rgba[i..i + 4].try_into().unwrap()
    }
    pub fn png(&self) -> Result<Vec<u8>, String> {
        self.png_with_compression("fast")
    }
    pub fn png_with_compression(&self, compression: &str) -> Result<Vec<u8>, String> {
        let mut out = vec![];
        {
            let mut enc = png::Encoder::new(&mut out, self.width, self.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.set_compression(match compression {
                "best" => png::Compression::Best,
                "default" => png::Compression::Default,
                _ => png::Compression::Fast,
            });
            let mut w = enc.write_header().map_err(|e| e.to_string())?;
            w.write_image_data(&self.rgba).map_err(|e| e.to_string())?;
        }
        Ok(out)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.save_options(path, 92, "fast")
    }
    pub fn save_options(&self, path: &Path, quality: u8, compression: &str) -> Result<(), String> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("png")
            .to_ascii_lowercase();
        if !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
            return Err("Поддерживаются PNG, JPEG и WebP".into());
        }
        let f = File::create(path).map_err(|e| e.to_string())?;
        let mut out = BufWriter::new(f);
        match ext.as_str() {
            "png" => out
                .write_all(&self.png_with_compression(compression)?)
                .map_err(|e| e.to_string())?,
            "jpg" | "jpeg" => {
                if self.width > u16::MAX as u32 || self.height > u16::MAX as u32 {
                    return Err("JPEG dimensions exceed 65535".into());
                }
                let rgb: Vec<u8> = self
                    .rgba
                    .chunks_exact(4)
                    .flat_map(|p| [p[0], p[1], p[2]])
                    .collect();
                jpeg_encoder::Encoder::new(&mut out, quality.clamp(1, 100))
                    .encode(
                        &rgb,
                        self.width as u16,
                        self.height as u16,
                        jpeg_encoder::ColorType::Rgb,
                    )
                    .map_err(|e| e.to_string())?
            }
            "webp" => image_webp::WebPEncoder::new(&mut out)
                .encode(
                    &self.rgba,
                    self.width,
                    self.height,
                    image_webp::ColorType::Rgba8,
                )
                .map_err(|e| e.to_string())?,
            _ => return Err("Supported formats: PNG, JPEG, WebP".into()),
        };
        out.flush().map_err(|e| e.to_string())
    }
    pub fn load_png(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let mut d = png::Decoder::new(file);
        d.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut r = d.read_info().map_err(|e| e.to_string())?;
        let info = r.info();
        if info.width as u64 * info.height as u64 > MAX_PIXELS {
            return Err("Image exceeds size limit".into());
        }
        let mut b = vec![0; r.output_buffer_size()];
        let i = r.next_frame(&mut b).map_err(|e| e.to_string())?;
        let mut image = Image::new(i.width, i.height)?;
        let src = &b[..i.buffer_size()];
        for (p, v) in image
            .rgba
            .chunks_exact_mut(4)
            .zip(src.chunks_exact(match i.color_type {
                png::ColorType::Rgba => 4,
                png::ColorType::Rgb => 3,
                png::ColorType::GrayscaleAlpha => 2,
                png::ColorType::Grayscale => 1,
                _ => return Err("Unsupported PNG color type".into()),
            }))
        {
            match i.color_type {
                png::ColorType::Rgba => p.copy_from_slice(v),
                png::ColorType::Rgb => p[..3].copy_from_slice(v),
                png::ColorType::Grayscale => p[..3].fill(v[0]),
                png::ColorType::GrayscaleAlpha => {
                    p[..3].fill(v[0]);
                    p[3] = v[1]
                }
                _ => {}
            }
        }
        Ok(image)
    }
    pub fn qr(&self) -> Vec<String> {
        fn decode(image: &Image, invert: bool) -> Vec<String> {
            let border = 24usize;
            let width = image.width as usize + border * 2;
            let height = image.height as usize + border * 2;
            let mut gray = vec![255u8; width * height];
            for y in 0..image.height as usize {
                for x in 0..image.width as usize {
                    let i = (y * image.width as usize + x) * 4;
                    let p = &image.rgba[i..i + 4];
                    let v = ((p[0] as u32 * 77 + p[1] as u32 * 150 + p[2] as u32 * 29) >> 8) as u8;
                    gray[(y + border) * width + x + border] = if invert { 255 - v } else { v };
                }
            }
            let mut q = quircs::Quirc::default();
            let mut found = vec![];
            for code in q.identify(width, height, &gray).flatten() {
                if let Ok(data) = code.decode() {
                    let text = String::from_utf8_lossy(&data.payload).to_string();
                    if !found.contains(&text) {
                        found.push(text);
                    }
                }
            }
            found
        }
        let found = decode(self, false);
        if !found.is_empty() {
            return found;
        }
        let found = decode(self, true);
        if !found.is_empty() {
            return found;
        }
        if self.width.max(self.height) < 900 {
            if let Ok(larger) = crate::render::resized(self, self.width * 2, self.height * 2) {
                return decode(&larger, false);
            }
        }
        vec![]
    }
}
