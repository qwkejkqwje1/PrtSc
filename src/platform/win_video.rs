//! Native MP4/H.264 through dynamically loaded Windows Media Foundation.
use super::native;
use luma_shot::image::{Image, Rect};
use std::{
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use windows::core::{Interface, HRESULT};
use windows::Win32::Media::MediaFoundation::*;
use windows_sys::Win32::{
    Foundation::*,
    System::{Com::*, LibraryLoader::*},
};
type Ptr = *mut std::ffi::c_void;
type BufferFactory = unsafe extern "system" fn(u32, *mut Ptr) -> HRESULT;
type SampleFactory = unsafe extern "system" fn(*mut Ptr) -> HRESULT;
struct Runtime {
    plat: HMODULE,
    read: HMODULE,
    shutdown: Option<unsafe extern "system" fn() -> HRESULT>,
    started: bool,
    com: bool,
}
impl Runtime {
    unsafe fn new() -> Result<Self, String> {
        let hr = CoInitializeEx(null(), COINIT_MULTITHREADED as u32);
        if hr < 0 {
            return Err(format!("COM: 0x{hr:08x}"));
        }
        let plat = LoadLibraryW(native::wide("mfplat.dll").as_ptr());
        let read = LoadLibraryW(native::wide("mfreadwrite.dll").as_ptr());
        if plat.is_null() || read.is_null() {
            if !plat.is_null() {
                FreeLibrary(plat);
            }
            if !read.is_null() {
                FreeLibrary(read);
            }
            CoUninitialize();
            return Err("MP4 needs Windows Media Feature Pack. Select GIF instead.".into());
        }
        // RAII starts before symbol lookup, so missing factories cannot leak COM/DLLs.
        let mut r = Self {
            plat,
            read,
            shutdown: None,
            started: false,
            com: true,
        };
        let start: unsafe extern "system" fn(u32, u32) -> HRESULT = std::mem::transmute(
            GetProcAddress(plat, b"MFStartup\0".as_ptr()).ok_or("MFStartup missing")?,
        );
        r.shutdown = Some(std::mem::transmute(
            GetProcAddress(plat, b"MFShutdown\0".as_ptr()).ok_or("MFShutdown missing")?,
        ));
        start(MF_VERSION, 0).ok().map_err(|e| e.to_string())?;
        r.started = true;
        Ok(r)
    }
    unsafe fn create<T: Interface>(&self, name: &[u8]) -> Result<T, String> {
        let f: unsafe extern "system" fn(*mut Ptr) -> HRESULT = std::mem::transmute(
            GetProcAddress(self.plat, name.as_ptr()).ok_or("MF factory unavailable")?,
        );
        let mut p = null_mut();
        f(&mut p).ok().map_err(|e| e.to_string())?;
        if p.is_null() {
            return Err("MF factory returned null".into());
        }
        Ok(T::from_raw(p))
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            if self.started {
                if let Some(shutdown) = self.shutdown {
                    let _ = shutdown();
                }
            }
            FreeLibrary(self.read);
            FreeLibrary(self.plat);
            if self.com {
                CoUninitialize();
            }
        }
    }
}
pub struct Mp4 {
    writer: IMFSinkWriter,
    stream: u32,
    width: u32,
    height: u32,
    _runtime: Runtime,
    buffer_factory: BufferFactory,
    sample_factory: SampleFactory,
    nv12: Vec<u8>,
    clock: luma_shot::video::SampleClock,
    nominal_duration: i64,
}
impl Mp4 {
    pub fn new(path: &Path, w: u32, h: u32, fps: u32, bitrate: u32) -> Result<Self, String> {
        if w == 0
            || h == 0
            || w % 2 != 0
            || h % 2 != 0
            || fps == 0
            || w as u64 * h as u64 > luma_shot::image::MAX_PIXELS
        {
            return Err("Invalid MP4 dimensions or FPS".into());
        }
        unsafe {
            let runtime = Runtime::new()?;
            let buffer_factory = std::mem::transmute(
                GetProcAddress(runtime.plat, b"MFCreateMemoryBuffer\0".as_ptr())
                    .ok_or("MF buffer factory missing")?,
            );
            let sample_factory = std::mem::transmute(
                GetProcAddress(runtime.plat, b"MFCreateSample\0".as_ptr())
                    .ok_or("MF sample factory missing")?,
            );
            let create: unsafe extern "system" fn(*const u16, Ptr, Ptr, *mut Ptr) -> HRESULT =
                std::mem::transmute(
                    GetProcAddress(runtime.read, b"MFCreateSinkWriterFromURL\0".as_ptr())
                        .ok_or("MP4 writer unavailable")?,
                );
            let mut p = null_mut();
            let name = native::wide(&path.to_string_lossy());
            create(name.as_ptr(), null_mut(), null_mut(), &mut p)
                .ok()
                .map_err(|e| format!("MP4 container creation: {e}. Select GIF if Windows Media Foundation is unavailable."))?;
            if p.is_null() {
                return Err("MP4 sink writer unavailable; select GIF.".into());
            }
            let writer = IMFSinkWriter::from_raw(p);
            let output: IMFMediaType = runtime.create(b"MFCreateMediaType\0")?;
            let build = || -> windows::core::Result<u32> {
                output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
                output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
                output.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
                output.SetUINT32(&MF_MT_INTERLACE_MODE, 2)?;
                output.SetUINT64(&MF_MT_FRAME_SIZE, (w as u64) << 32 | h as u64)?;
                output.SetUINT64(&MF_MT_FRAME_RATE, (fps as u64) << 32 | 1)?;
                output.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, 1u64 << 32 | 1)?;
                writer.AddStream(&output)
            };
            let stream = build().map_err(|e| e.to_string())?;
            let input: IMFMediaType = runtime.create(b"MFCreateMediaType\0")?;
            let setup = || -> windows::core::Result<()> {
                input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
                input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
                input.SetUINT32(&MF_MT_INTERLACE_MODE, 2)?;
                input.SetUINT64(&MF_MT_FRAME_SIZE, (w as u64) << 32 | h as u64)?;
                input.SetUINT64(&MF_MT_FRAME_RATE, (fps as u64) << 32 | 1)?;
                input.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, 1u64 << 32 | 1)?;
                input.SetUINT32(&MF_MT_DEFAULT_STRIDE, w)?;
                input.SetUINT32(&MF_MT_YUV_MATRIX, 2)?;
                input.SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, 2)?;
                writer.SetInputMediaType(stream, &input, None::<&IMFAttributes>)?;
                writer.BeginWriting()
            };
            setup().map_err(|e| {
                format!("MP4 encoder: {e}. Select GIF if Media Feature Pack is unavailable.")
            })?;
            Ok(Self {
                writer,
                stream,
                width: w,
                height: h,
                _runtime: runtime,
                buffer_factory,
                sample_factory,
                nv12: Vec::new(),
                clock: Default::default(),
                nominal_duration: 10_000_000 / fps as i64,
            })
        }
    }
    pub fn frame(&mut self, image: &Image, time: i64, duration: i64) -> Result<(), String> {
        if (image.width, image.height) != (self.width, self.height) {
            return Err("Video frame size changed".into());
        }
        luma_shot::video::nv12_into(image, &mut self.nv12)?;
        self.write_pending(time, duration)
    }
    pub fn push_bgra(&mut self, bytes: &[u8], tick: i64) -> Result<(), String> {
        if let Some((time, duration)) = self.clock.push(tick) {
            self.write_pending(time, duration)?;
        }
        luma_shot::video::nv12_bgra_into(bytes, self.width, self.height, &mut self.nv12)
    }
    fn write_pending(&self, time: i64, duration: i64) -> Result<(), String> {
        if time < 0 || duration <= 0 {
            return Err("Invalid sample timestamp".into());
        }
        unsafe {
            let mut p = null_mut();
            (self.buffer_factory)(self.nv12.len() as u32, &mut p)
                .ok()
                .map_err(|e| e.to_string())?;
            if p.is_null() {
                return Err("MF buffer factory returned null".into());
            }
            let buffer = IMFMediaBuffer::from_raw(p);
            let mut pixels = null_mut();
            buffer
                .Lock(&mut pixels, None, None)
                .map_err(|e| e.to_string())?;
            if pixels.is_null() {
                let _ = buffer.Unlock();
                return Err("MF buffer pixels unavailable".into());
            }
            std::ptr::copy_nonoverlapping(self.nv12.as_ptr(), pixels, self.nv12.len());
            buffer.Unlock().map_err(|e| e.to_string())?;
            buffer
                .SetCurrentLength(self.nv12.len() as u32)
                .map_err(|e| e.to_string())?;
            let mut p = null_mut();
            (self.sample_factory)(&mut p)
                .ok()
                .map_err(|e| e.to_string())?;
            if p.is_null() {
                return Err("MF sample factory returned null".into());
            }
            let sample = IMFSample::from_raw(p);
            sample
                .AddBuffer(&buffer)
                .and_then(|_| sample.SetSampleTime(time))
                .and_then(|_| sample.SetSampleDuration(duration))
                .and_then(|_| self.writer.WriteSample(self.stream, &sample))
                .map_err(|e| e.to_string())
        }
    }
    pub fn finish_at(mut self, end: i64) -> Result<(), String> {
        if let Some((time, duration)) = self.clock.finish(end) {
            self.write_pending(time, duration)?;
        }
        unsafe { self.writer.Finalize().map_err(|e| e.to_string()) }
    }
    pub fn finish(mut self) -> Result<(), String> {
        if let Some((time, _)) = self.clock.finish(0) {
            self.write_pending(time, self.nominal_duration)?;
        }
        unsafe { self.writer.Finalize().map_err(|e| e.to_string()) }
    }
}

pub struct Report {
    pub path: PathBuf,
    pub frames: u64,
    pub seconds: f64,
    pub width: u32,
    pub height: u32,
}
pub fn record(
    region: Rect,
    path: PathBuf,
    format: &str,
    fps: u32,
    bitrate: u32,
    include_cursor: bool,
    stop: Arc<AtomicBool>,
) -> Result<Report, String> {
    if fps == 0 {
        return Err("Recording FPS must be positive".into());
    }
    let tmp = path.with_file_name(format!(
        "{}.partial.{}",
        path.file_stem().unwrap_or_default().to_string_lossy(),
        format
    ));
    let result = (|| {
        let gif = format == "gif";
        let mut region = region;
        if !gif {
            region.w -= region.w % 2;
            region.h -= region.h % 2;
        }
        if region.w < 2 || region.h < 2 {
            return Err("Recording area is too small".into());
        }
        let scale = if gif {
            (1280.0 / region.w.max(region.h) as f32).min(1.)
        } else {
            1.
        };
        let width = (region.w as f32 * scale).max(1.) as u32;
        let height = (region.h as f32 * scale).max(1.) as u32;
        let fps = if gif { fps.min(15) } else { fps };
        let mut gif_writer = if gif {
            Some(luma_shot::video::Gif::new(&tmp, width, height, fps)?)
        } else {
            None
        };
        let mut mp4 = if !gif {
            Some(Mp4::new(&tmp, width, height, fps, bitrate)?)
        } else {
            None
        };
        let period = Duration::from_secs_f64(1. / fps as f64);
        let started = Instant::now();
        let mut frames = 0u64;
        let mut next = started;
        let mut capturer = native::ScreenCapturer::new(region)?;
        // GIF needs RGBA for quantization, MP4 goes directly from BGRA to NV12.
        let mut image = if gif {
            Some(Image::new(region.w, region.h)?)
        } else {
            None
        };
        let mut pending = if gif {
            Some(Image::new(width, height)?)
        } else {
            None
        };
        let mut scaled = if gif && scale < 1. {
            Some(Image::new(width, height)?)
        } else {
            None
        };
        let mut clock = luma_shot::video::SampleClock::default();
        let mut origin = None;
        let mut gif_remainder = 0i64;
        let mut gif_frame =
            |g: &mut luma_shot::video::Gif, frame: &Image, duration: i64| -> Result<(), String> {
                let total = duration + gif_remainder;
                let delay = (total / 100_000).clamp(1, 65535);
                gif_remainder = total - delay * 100_000;
                g.frame_with_delay(frame, delay as u16)
            };
        while !stop.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(3600) {
            if let Some(current) = image.as_mut() {
                capturer.capture_into(current, include_cursor)?;
            }
            let bgra = if !gif {
                Some(capturer.capture_bgra(include_cursor)?)
            } else {
                None
            };
            let now_tick = (started.elapsed().as_secs_f64() * 10_000_000.) as i64;
            let tick = now_tick - *origin.get_or_insert(now_tick);
            if let Some(g) = gif_writer.as_mut() {
                if let Some((_, duration)) = clock.push(tick) {
                    gif_frame(g, pending.as_ref().unwrap(), duration)?;
                }
                let current = image.as_mut().unwrap();
                if let Some(scaled) = scaled.as_mut() {
                    luma_shot::render::resize_into(current, scaled)?;
                    std::mem::swap(scaled, pending.as_mut().unwrap());
                } else {
                    std::mem::swap(current, pending.as_mut().unwrap());
                }
            }
            if let Some(m) = mp4.as_mut() {
                m.push_bgra(bgra.unwrap(), tick)?;
            }
            frames += 1;
            next += period;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now)
            } else {
                next = now;
            }
        }
        if frames == 0 {
            return Err("Recording cancelled before the first frame".into());
        }
        let end = ((started.elapsed().as_secs_f64() * 10_000_000.) as i64 - origin.unwrap()).max(1);
        if let Some(mut g) = gif_writer {
            if let Some((_, duration)) = clock.finish(end) {
                gif_frame(&mut g, pending.as_ref().unwrap(), duration)?;
            }
            g.finish()?;
        }
        if let Some(m) = mp4 {
            m.finish_at(end)?;
        }
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        Ok(Report {
            path: path.clone(),
            frames,
            seconds: started.elapsed().as_secs_f64(),
            width,
            height,
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}
