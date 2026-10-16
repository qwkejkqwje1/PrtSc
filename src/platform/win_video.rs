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
struct Runtime {
    plat: HMODULE,
    read: HMODULE,
    shutdown: unsafe extern "system" fn() -> HRESULT,
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
        let start: unsafe extern "system" fn(u32, u32) -> HRESULT = std::mem::transmute(
            GetProcAddress(plat, b"MFStartup\0".as_ptr()).ok_or("MFStartup missing")?,
        );
        let shutdown = std::mem::transmute(
            GetProcAddress(plat, b"MFShutdown\0".as_ptr()).ok_or("MFShutdown missing")?,
        );
        let r = Self {
            plat,
            read,
            shutdown,
            com: true,
        };
        start(MF_VERSION, 0).ok().map_err(|e| e.to_string())?;
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
            let _ = (self.shutdown)();
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
    runtime: Runtime,
}
impl Mp4 {
    pub fn new(path: &Path, w: u32, h: u32, fps: u32, bitrate: u32) -> Result<Self, String> {
        unsafe {
            let runtime = Runtime::new()?;
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
                runtime,
            })
        }
    }
    pub fn frame(&mut self, image: &Image, time: i64, duration: i64) -> Result<(), String> {
        if (image.width, image.height) != (self.width, self.height) {
            return Err("Video frame size changed".into());
        }
        unsafe {
            let create: unsafe extern "system" fn(u32, *mut Ptr) -> HRESULT = std::mem::transmute(
                GetProcAddress(self.runtime.plat, b"MFCreateMemoryBuffer\0".as_ptr())
                    .ok_or("MF buffer factory missing")?,
            );
            let mut p = null_mut();
            let nv12 = luma_shot::video::nv12(image)?;
            create(nv12.len() as u32, &mut p)
                .ok()
                .map_err(|e| e.to_string())?;
            let buffer = IMFMediaBuffer::from_raw(p);
            let mut pixels = null_mut();
            buffer
                .Lock(&mut pixels, None, None)
                .map_err(|e| e.to_string())?;
            let dst = std::slice::from_raw_parts_mut(pixels, nv12.len());
            dst.copy_from_slice(&nv12);
            buffer.Unlock().map_err(|e| e.to_string())?;
            buffer
                .SetCurrentLength(dst.len() as u32)
                .map_err(|e| e.to_string())?;
            let sample: IMFSample = self.runtime.create(b"MFCreateSample\0")?;
            sample
                .AddBuffer(&buffer)
                .and_then(|_| sample.SetSampleTime(time))
                .and_then(|_| sample.SetSampleDuration(duration))
                .and_then(|_| self.writer.WriteSample(self.stream, &sample))
                .map_err(|e| e.to_string())
        }
    }
    pub fn finish(self) -> Result<(), String> {
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
        let mut last_tick = 0i64;
        while !stop.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(3600) {
            let mut image = native::capture_frame(region, include_cursor)?;
            if scale < 1. {
                image = luma_shot::render::resized(&image, width, height)?;
            }
            let tick = (started.elapsed().as_secs_f64() * 10_000_000.) as i64;
            if let Some(g) = gif_writer.as_mut() {
                let delay = if frames == 0 {
                    100 / fps as i64
                } else {
                    (tick - last_tick) / 100_000
                };
                g.frame_with_delay(&image, delay.clamp(1, 65535) as u16)?;
                last_tick = tick;
            }
            if let Some(m) = mp4.as_mut() {
                m.frame(&image, tick.max(last_tick), 10_000_000 / fps as i64)?;
                last_tick = tick + 1;
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
        if let Some(g) = gif_writer {
            g.finish()?;
        }
        if let Some(m) = mp4 {
            m.finish()?;
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
