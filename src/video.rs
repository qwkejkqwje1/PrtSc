//! Streaming GIF output; frames are encoded immediately, not accumulated in RAM.
use crate::image::Image;
use std::{fs::File, io::BufWriter, path::Path};
pub struct Gif {
    encoder: gif::Encoder<BufWriter<File>>,
    width: u32,
    height: u32,
    fps: u32,
    frames: u64,
    scratch: Vec<u8>,
}
impl Gif {
    pub fn new(path: &Path, width: u32, height: u32, fps: u32) -> Result<Self, String> {
        if width == 0 || height == 0 || width > 65535 || height > 65535 || fps == 0 {
            return Err("Invalid recording dimensions or FPS".into());
        }
        let file = BufWriter::new(File::create(path).map_err(|e| e.to_string())?);
        let mut encoder =
            gif::Encoder::new(file, width as u16, height as u16, &[]).map_err(|e| e.to_string())?;
        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            encoder,
            width,
            height,
            fps,
            frames: 0,
            scratch: Vec::new(),
        })
    }
    pub fn frame(&mut self, image: &Image) -> Result<(), String> {
        if (image.width, image.height) != (self.width, self.height) {
            return Err("Video frame size changed".into());
        }
        self.frame_with_delay(
            image,
            (((self.frames + 1) * 100 / self.fps as u64) - (self.frames * 100 / self.fps as u64))
                .max(1) as u16,
        )
    }
    pub fn frame_with_delay(&mut self, image: &Image, delay: u16) -> Result<(), String> {
        if (image.width, image.height) != (self.width, self.height) {
            return Err("Video frame size changed".into());
        }
        self.scratch.clear();
        self.scratch.extend_from_slice(&image.rgba);
        let mut frame = gif::Frame::from_rgba_speed(
            self.width as u16,
            self.height as u16,
            &mut self.scratch,
            20,
        );
        frame.delay = delay.max(1);
        frame.dispose = gif::DisposalMethod::Keep;
        self.encoder
            .write_frame(&frame)
            .map_err(|e| e.to_string())?;
        self.frames += 1;
        Ok(())
    }
    pub fn finish(self) -> Result<(), String> {
        use std::io::Write;
        self.encoder
            .into_inner()
            .map_err(|e| e.to_string())?
            .flush()
            .map_err(|e| e.to_string())
    }
}

/// Top-down NV12 (limited-range BT.601). Even dimensions; no DIB row inversion.
pub fn nv12(image: &Image) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    nv12_into(image, &mut out)?;
    Ok(out)
}
pub fn nv12_into(image: &Image, out: &mut Vec<u8>) -> Result<(), String> {
    convert_nv12(&image.rgba, image.width, image.height, false, out)
}
/// Direct conversion from the persistent Windows capture DIB, without RGBA staging.
pub fn nv12_bgra_into(bytes: &[u8], w: u32, h: u32, out: &mut Vec<u8>) -> Result<(), String> {
    convert_nv12(bytes, w, h, true, out)
}
fn convert_nv12(
    bytes: &[u8],
    width: u32,
    height: u32,
    bgra: bool,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    let pixels = width as u64 * height as u64;
    if width == 0
        || height == 0
        || width % 2 != 0
        || height % 2 != 0
        || pixels > crate::image::MAX_PIXELS
        || pixels * 4 != bytes.len() as u64
    {
        return Err("NV12 needs valid pixels and nonzero even dimensions".into());
    }
    let (w, h) = (width as usize, height as usize);
    out.resize(w * h * 3 / 2, 0);
    let (ri, bi) = if bgra { (2, 0) } else { (0, 2) };
    // Each source 2x2 block produces four Y values and one interleaved UV pair.
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let (mut sr, mut sg, mut sb) = (0, 0, 0);
            for yy in 0..2 {
                for xx in 0..2 {
                    let i = ((y + yy) * w + x + xx) * 4;
                    let (r, g, b) = (
                        bytes[i + ri] as i32,
                        bytes[i + 1] as i32,
                        bytes[i + bi] as i32,
                    );
                    out[(y + yy) * w + x + xx] =
                        (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16).clamp(16, 235) as u8;
                    sr += r;
                    sg += g;
                    sb += b;
                }
            }
            let (r, g, b) = (sr / 4, sg / 4, sb / 4);
            let i = w * h + y / 2 * w + x;
            out[i] = (((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128).clamp(16, 240) as u8;
            out[i + 1] = (((112 * r - 94 * g - 18 * b + 128) >> 8) + 128).clamp(16, 240) as u8;
        }
    }
    Ok(())
}

/// One-frame lookahead: a sample lasts until the next capture, including slow frames.
#[derive(Default)]
pub struct SampleClock {
    pending: Option<i64>,
}
impl SampleClock {
    pub fn push(&mut self, tick: i64) -> Option<(i64, i64)> {
        let tick = tick.max(self.pending.map(|p| p + 1).unwrap_or(0));
        self.pending.replace(tick).map(|p| (p, tick - p))
    }
    pub fn finish(&mut self, end: i64) -> Option<(i64, i64)> {
        self.pending.take().map(|p| (p, (end - p).max(1)))
    }
}
