//! Streaming GIF output; frames are encoded immediately, not accumulated in RAM.
use crate::image::Image;
use std::{fs::File, io::BufWriter, path::Path};
pub struct Gif {
    encoder: gif::Encoder<BufWriter<File>>,
    width: u32,
    height: u32,
    fps: u32,
    frames: u64,
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
        let mut rgba = image.rgba.clone();
        let mut frame =
            gif::Frame::from_rgba_speed(self.width as u16, self.height as u16, &mut rgba, 20);
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
    let (w, h) = (image.width as usize, image.height as usize);
    if w % 2 != 0 || h % 2 != 0 {
        return Err("NV12 needs even dimensions".into());
    }
    let mut out = vec![0u8; w * h * 3 / 2];
    for y in 0..h {
        for x in 0..w {
            let p = image.pixel(x as u32, y as u32);
            let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
            out[y * w + x] = (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16).clamp(16, 235) as u8;
        }
    }
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let (mut r, mut g, mut b) = (0, 0, 0);
            for yy in 0..2 {
                for xx in 0..2 {
                    let p = image.pixel((x + xx) as u32, (y + yy) as u32);
                    r += p[0] as i32;
                    g += p[1] as i32;
                    b += p[2] as i32;
                }
            }
            r /= 4;
            g /= 4;
            b /= 4;
            let i = w * h + y / 2 * w + x;
            out[i] = (((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128).clamp(16, 240) as u8;
            out[i + 1] = (((112 * r - 94 * g - 18 * b + 128) >> 8) + 128).clamp(16, 240) as u8;
        }
    }
    Ok(out)
}
