use luma_shot::image::{Image, Rect};
use std::{
    mem::{size_of, zeroed},
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};
use windows_sys::Win32::Storage::Xps::*;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{DataExchange::*, Memory::*},
    UI::{Controls::Dialogs::*, WindowsAndMessaging::*},
};
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn last(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}
pub unsafe fn screen_rect() -> Rect {
    Rect {
        x: GetSystemMetrics(SM_XVIRTUALSCREEN),
        y: GetSystemMetrics(SM_YVIRTUALSCREEN),
        w: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1) as u32,
        h: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1) as u32,
    }
}
pub fn capture(r: Rect) -> Result<Image, String> {
    capture_frame(r, false)
}
/// Owns the screen DC and one top-down DIB for the whole recording.
/// Returned BGRA pixels are borrowed until the next capture, never cached by callers.
pub struct ScreenCapturer {
    region: Rect,
    source: HDC,
    raster: Raster,
    bits: *const u8,
}
impl ScreenCapturer {
    pub fn new(region: Rect) -> Result<Self, String> {
        let raster = Raster::empty(region.w, region.h)?;
        unsafe {
            let mut bm: BITMAP = zeroed();
            if GetObjectW(
                raster.bitmap,
                size_of::<BITMAP>() as i32,
                &mut bm as *mut _ as _,
            ) == 0
                || bm.bmBits.is_null()
            {
                return Err(last("Capture DIB pixels"));
            }
            let source = GetDC(null_mut());
            if source.is_null() {
                return Err(last("GetDC"));
            }
            Ok(Self {
                region,
                source,
                raster,
                bits: bm.bmBits as *const u8,
            })
        }
    }
    pub fn capture_bgra(&mut self, include_cursor: bool) -> Result<&[u8], String> {
        unsafe {
            let r = self.region;
            let dc = self.raster.dc;
            if BitBlt(
                dc,
                0,
                0,
                r.w as i32,
                r.h as i32,
                self.source,
                r.x,
                r.y,
                SRCCOPY | CAPTUREBLT,
            ) == 0
            {
                return Err(last("BitBlt"));
            }
            if include_cursor {
                let mut ci: CURSORINFO = zeroed();
                ci.cbSize = size_of::<CURSORINFO>() as u32;
                if GetCursorInfo(&mut ci) != 0 && ci.flags & CURSOR_SHOWING != 0 {
                    let mut info: ICONINFO = zeroed();
                    let mut cursor = ci.hCursor;
                    let mut valid = GetIconInfo(cursor, &mut info);
                    if valid == 0 {
                        // A foreign cursor handle can be unavailable (e.g. Wine). Preserve
                        // pointer motion with the standard arrow instead of silently omitting it.
                        cursor = LoadCursorW(null_mut(), IDC_ARROW);
                        valid = GetIconInfo(cursor, &mut info);
                    }
                    if valid != 0 {
                        DrawIconEx(
                            dc,
                            ci.ptScreenPos.x - r.x - info.xHotspot as i32,
                            ci.ptScreenPos.y - r.y - info.yHotspot as i32,
                            cursor,
                            0,
                            0,
                            0,
                            null_mut(),
                            DI_NORMAL,
                        );
                        if !info.hbmMask.is_null() {
                            DeleteObject(info.hbmMask);
                        }
                        if !info.hbmColor.is_null() {
                            DeleteObject(info.hbmColor);
                        }
                    }
                }
            }
            GdiFlush();
            Ok(std::slice::from_raw_parts(
                self.bits,
                r.w as usize * r.h as usize * 4,
            ))
        }
    }
    pub fn capture_into(&mut self, image: &mut Image, include_cursor: bool) -> Result<(), String> {
        if (image.width, image.height) != (self.region.w, self.region.h) {
            return Err("Capture dimensions changed".into());
        }
        let bytes = self.capture_bgra(include_cursor)?;
        for (dst, src) in image.rgba.chunks_exact_mut(4).zip(bytes.chunks_exact(4)) {
            dst.copy_from_slice(&[src[2], src[1], src[0], 255]);
        }
        Ok(())
    }
}
impl Drop for ScreenCapturer {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(null_mut(), self.source);
        }
    }
}
pub fn capture_frame(r: Rect, include_cursor: bool) -> Result<Image, String> {
    let mut capturer = ScreenCapturer::new(r)?;
    let mut image = Image::new(r.w, r.h)?;
    capturer.capture_into(&mut image, include_cursor)?;
    Ok(image)
}
pub unsafe fn blit(hdc: HDC, image: &Image, w: i32, h: i32) {
    let mut bmi: BITMAPINFO = zeroed();
    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = image.width as i32;
    bmi.bmiHeader.biHeight = -(image.height as i32);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;
    let mut bgra = image.rgba.clone();
    for p in bgra.chunks_exact_mut(4) {
        p.swap(0, 2)
    }
    SetStretchBltMode(hdc, HALFTONE);
    StretchDIBits(
        hdc,
        0,
        0,
        w,
        h,
        0,
        0,
        image.width as i32,
        image.height as i32,
        bgra.as_ptr() as _,
        &bmi,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}
fn clipboard_set(owner: HWND, format: u32, data: &[u8]) -> Result<(), String> {
    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE, data.len());
        if h.is_null() {
            return Err(last("GlobalAlloc"));
        }
        let p = GlobalLock(h);
        if p.is_null() {
            GlobalFree(h);
            return Err(last("GlobalLock"));
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), p as *mut u8, data.len());
        GlobalUnlock(h);
        let mut opened = false;
        for _ in 0..6 {
            if OpenClipboard(owner) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
        if !opened {
            GlobalFree(h);
            return Err("Буфер обмена занят; повторите попытку".into());
        }
        if EmptyClipboard() == 0 {
            CloseClipboard();
            GlobalFree(h);
            return Err(last("EmptyClipboard"));
        }
        let ok = SetClipboardData(format, h);
        CloseClipboard();
        if ok.is_null() {
            GlobalFree(h);
            Err(last("SetClipboardData"))
        } else {
            Ok(())
        }
    }
}
// Fill CF_DIB directly in its final movable allocation: no full-frame staging Vec.
pub fn clipboard_image(owner: HWND, image: &Image) -> Result<(), String> {
    unsafe {
        let mut header: BITMAPINFOHEADER = zeroed();
        header.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        header.biWidth = image.width as i32;
        header.biHeight = -(image.height as i32);
        header.biPlanes = 1;
        header.biBitCount = 32;
        header.biCompression = BI_RGB;
        header.biSizeImage = image.rgba.len() as u32;
        let len = size_of::<BITMAPINFOHEADER>() + image.rgba.len();
        let h = GlobalAlloc(GMEM_MOVEABLE, len);
        if h.is_null() {
            return Err(last("GlobalAlloc"));
        }
        let p = GlobalLock(h) as *mut u8;
        if p.is_null() {
            GlobalFree(h);
            return Err(last("GlobalLock"));
        }
        std::ptr::copy_nonoverlapping(
            &header as *const _ as *const u8,
            p,
            size_of::<BITMAPINFOHEADER>(),
        );
        let pixels =
            std::slice::from_raw_parts_mut(p.add(size_of::<BITMAPINFOHEADER>()), image.rgba.len());
        for (dst, src) in pixels.chunks_exact_mut(4).zip(image.rgba.chunks_exact(4)) {
            dst.copy_from_slice(&[src[2], src[1], src[0], 255]);
        }
        GlobalUnlock(h);
        let mut opened = false;
        for _ in 0..6 {
            if OpenClipboard(owner) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
        if !opened {
            GlobalFree(h);
            return Err("Буфер обмена занят; повторите попытку".into());
        }
        if EmptyClipboard() == 0 {
            CloseClipboard();
            GlobalFree(h);
            return Err(last("EmptyClipboard"));
        }
        let ok = SetClipboardData(8, h);
        CloseClipboard();
        if ok.is_null() {
            GlobalFree(h);
            Err(last("SetClipboardData"))
        } else {
            Ok(())
        }
    }
}
pub fn clipboard_text(owner: HWND, text: &str) -> Result<(), String> {
    let data = wide(text);
    let bytes: Vec<u8> = data.iter().flat_map(|u| u.to_le_bytes()).collect();
    clipboard_set(owner, 13, &bytes)
}
pub fn save_dialog_options(owner: HWND, cfg: &luma_shot::config::Config) -> Option<PathBuf> {
    unsafe {
        let mut path = [0u16; 32768];
        let default = wide("PrtSc");
        let dir = wide(&cfg.save_dir);
        path[..default.len()].copy_from_slice(&default);
        let filter = wide(
            "PNG без потерь (*.png)\0*.png\0JPEG (*.jpg)\0*.jpg\0WebP без потерь (*.webp)\0*.webp\0",
        );
        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = owner;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = path.as_mut_ptr();
        ofn.nMaxFile = path.len() as u32;
        ofn.nFilterIndex = match cfg.format.as_str() {
            "jpg" => 2,
            "webp" => 3,
            _ => 1,
        };
        ofn.lpstrInitialDir = dir.as_ptr();
        ofn.Flags = OFN_OVERWRITEPROMPT | OFN_EXPLORER | OFN_NOCHANGEDIR;
        let title = wide("Сохранить снимок PrtSc");
        ofn.lpstrTitle = title.as_ptr();
        if GetSaveFileNameW(&mut ofn) == 0 {
            return None;
        }
        let len = path.iter().position(|c| *c == 0).unwrap_or(path.len());
        let mut p = PathBuf::from(String::from_utf16_lossy(&path[..len]));
        if p.extension().is_none() {
            p.set_extension(match ofn.nFilterIndex {
                2 => "jpg",
                3 => "webp",
                _ => "png",
            });
        }
        Some(p)
    }
}
pub fn ocr(image: &Image) -> Result<String, String> {
    use windows::{
        Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::Streams::DataWriter,
    };
    #[link(name = "runtimeobject")]
    extern "system" {
        fn RoInitialize(kind: u32) -> i32;
        fn RoUninitialize();
    }
    struct Runtime;
    impl Drop for Runtime {
        fn drop(&mut self) {
            unsafe { RoUninitialize() }
        }
    }
    let hr = unsafe { RoInitialize(1) };
    if hr < 0 {
        return Err(format!("Среда Windows OCR недоступна: 0x{hr:08X}"));
    }
    let _runtime = Runtime;
    let result = (|| -> windows::core::Result<String> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        let max = OcrEngine::MaxImageDimension()?;
        let scaled;
        let image = if image.width > max || image.height > max {
            let s = (max as f64 / image.width as f64).min(max as f64 / image.height as f64);
            scaled = luma_shot::render::resized(
                image,
                (image.width as f64 * s) as u32,
                (image.height as f64 * s) as u32,
            )
            .map_err(|_| {
                windows::core::Error::from_hresult(windows::core::HRESULT(0x80070057u32 as i32))
            })?;
            &scaled
        } else {
            image
        };
        let bitmap = SoftwareBitmap::Create(
            BitmapPixelFormat::Bgra8,
            image.width as i32,
            image.height as i32,
        )?;
        let mut bgra = image.rgba.clone();
        for p in bgra.chunks_exact_mut(4) {
            p.swap(0, 2)
        }
        let writer = DataWriter::new()?;
        writer.WriteBytes(&bgra)?;
        bitmap.CopyFromBuffer(&writer.DetachBuffer()?)?;
        let result = engine.RecognizeAsync(&bitmap)?.get()?;
        Ok(result.Text()?.to_string())
    })();
    result
        .map_err(|e| format!("Для OCR нужна Windows 10/11 и установленный язык распознавания. {e}"))
}
pub fn print_image(owner: HWND, image: &Image) -> Result<(), String> {
    unsafe {
        let mut pd: PRINTDLGW = zeroed();
        pd.lStructSize = size_of::<PRINTDLGW>() as u32;
        pd.hwndOwner = owner;
        pd.Flags = PD_RETURNDC | PD_NOPAGENUMS | PD_NOSELECTION;
        if PrintDlgW(&mut pd) == 0 {
            return Ok(());
        }
        let dc = pd.hDC;
        if dc.is_null() {
            return Err("Принтер не выбран".into());
        }
        let name = wide("Снимок PrtSc");
        let mut doc: DOCINFOW = zeroed();
        doc.cbSize = size_of::<DOCINFOW>() as i32;
        doc.lpszDocName = name.as_ptr();
        if StartDocW(dc, &doc) <= 0 {
            DeleteDC(dc);
            return Err(last("Start print job"));
        }
        StartPage(dc);
        let w = GetDeviceCaps(dc, HORZRES as i32);
        let h = GetDeviceCaps(dc, VERTRES as i32);
        let s = (w as f64 / image.width as f64).min(h as f64 / image.height as f64);
        blit(
            dc,
            image,
            (image.width as f64 * s) as i32,
            (image.height as f64 * s) as i32,
        );
        EndPage(dc);
        EndDoc(dc);
        DeleteDC(dc);
        if !pd.hDevMode.is_null() {
            GlobalFree(pd.hDevMode);
        }
        if !pd.hDevNames.is_null() {
            GlobalFree(pd.hDevNames);
        }
        Ok(())
    }
}
pub fn open(path: &Path) {
    unsafe {
        let w = wide(&path.to_string_lossy());
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            null_mut(),
            wide("open").as_ptr(),
            w.as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        );
    }
}

pub struct Raster {
    pub dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    pub width: i32,
    pub height: i32,
}
impl Raster {
    /// A persistent top-down DIB. No unnecessary CPU copy on allocation.
    pub fn empty(width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 || width as u64 * height as u64 > luma_shot::image::MAX_PIXELS
        {
            return Err("Invalid raster dimensions".into());
        }
        unsafe {
            let dc = CreateCompatibleDC(null_mut());
            if dc.is_null() {
                return Err(last("CreateCompatibleDC"));
            }
            let mut bmi: BITMAPINFO = zeroed();
            bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width as i32;
            bmi.bmiHeader.biHeight = -(height as i32);
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            let mut bits = null_mut();
            let bitmap = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if bitmap.is_null() {
                DeleteDC(dc);
                return Err(last("CreateDIBSection"));
            }
            let old = SelectObject(dc, bitmap);
            Ok(Self {
                dc,
                bitmap,
                old,
                width: width as i32,
                height: height as i32,
            })
        }
    }
    pub fn new(image: &Image, dim: bool) -> Result<Self, String> {
        let mut raster = Self::empty(image.width, image.height)?;
        raster.update_region(
            image,
            Rect {
                x: 0,
                y: 0,
                w: image.width,
                h: image.height,
            },
            dim,
        )?;
        Ok(raster)
    }
    pub fn update_region(&mut self, image: &Image, r: Rect, dim: bool) -> Result<(), String> {
        if image.width != self.width as u32 || image.height != self.height as u32 {
            return Err("Raster dimensions changed".into());
        }
        let r = r.clamp(image.width, image.height);
        if !r.valid() {
            return Ok(());
        }
        unsafe {
            // CPU writes must not race pending GDI reads of this DIB.
            GdiFlush();
            let mut bm: BITMAP = zeroed();
            if GetObjectW(
                self.bitmap,
                size_of::<BITMAP>() as i32,
                &mut bm as *mut _ as _,
            ) == 0
                || bm.bmBits.is_null()
            {
                return Err(last("DIB pixels"));
            }
            let pixels = std::slice::from_raw_parts_mut(bm.bmBits as *mut u8, image.rgba.len());
            let f = if dim { 40u16 } else { 100 };
            for y in r.y as usize..r.y as usize + r.h as usize {
                for x in r.x as usize..r.x as usize + r.w as usize {
                    let i = (y * image.width as usize + x) * 4;
                    let p = &image.rgba[i..i + 4];
                    pixels[i..i + 4].copy_from_slice(&[
                        (p[2] as u16 * f / 100) as u8,
                        (p[1] as u16 * f / 100) as u8,
                        (p[0] as u16 * f / 100) as u8,
                        255,
                    ]);
                }
            }
        }
        Ok(())
    }
    pub unsafe fn paint(&self, dc: HDC, r: RECT) {
        BitBlt(
            dc,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            self.dc,
            r.left,
            r.top,
            SRCCOPY,
        );
    }
}
impl Drop for Raster {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

pub fn choose_color(owner: HWND, initial: [u8; 4], palette: &[[u8; 4]; 7]) -> Option<[u8; 4]> {
    unsafe {
        let mut custom = [0u32; 16];
        for (d, c) in custom.iter_mut().zip(palette) {
            *d = c[0] as u32 | (c[1] as u32) << 8 | (c[2] as u32) << 16;
        }
        let mut c: CHOOSECOLORW = zeroed();
        c.lStructSize = size_of::<CHOOSECOLORW>() as u32;
        c.hwndOwner = owner;
        c.rgbResult = initial[0] as u32 | (initial[1] as u32) << 8 | (initial[2] as u32) << 16;
        c.lpCustColors = custom.as_mut_ptr();
        c.Flags = CC_FULLOPEN | CC_RGBINIT;
        if ChooseColorW(&mut c) == 0 {
            None
        } else {
            Some([
                c.rgbResult as u8,
                (c.rgbResult >> 8) as u8,
                (c.rgbResult >> 16) as u8,
                255,
            ])
        }
    }
}
pub fn choose_folder(owner: HWND) -> Option<PathBuf> {
    unsafe {
        use windows_sys::Win32::UI::Shell::*;
        let mut title = [0u16; 260];
        let mut b: BROWSEINFOW = zeroed();
        b.hwndOwner = owner;
        b.pszDisplayName = title.as_mut_ptr();
        let caption = wide("Папка сохранения снимков PrtSc");
        b.lpszTitle = caption.as_ptr();
        b.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
        let id = SHBrowseForFolderW(&b);
        if id.is_null() {
            return None;
        }
        let mut path = [0u16; 32768];
        let ok = SHGetPathFromIDListW(id, path.as_mut_ptr());
        windows_sys::Win32::System::Com::CoTaskMemFree(id as _);
        if ok == 0 {
            None
        } else {
            let n = path.iter().position(|c| *c == 0).unwrap_or(path.len());
            Some(PathBuf::from(String::from_utf16_lossy(&path[..n])))
        }
    }
}

pub fn upload(image: &Image, service: &str, client_id: &str) -> Result<String, String> {
    use windows_sys::Win32::Networking::WinHttp::*;
    struct Handle(*mut std::ffi::c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                if !self.0.is_null() {
                    WinHttpCloseHandle(self.0);
                }
            }
        }
    }
    unsafe {
        let r = luma_shot::hosting::request(image, service)?;
        if service == "imgur"
            && (client_id.is_empty() || !client_id.bytes().all(|c| c.is_ascii_alphanumeric()))
        {
            return Err("Imgur: enter your application's Client ID in Settings (not your account password).".into());
        }
        let session = Handle(WinHttpOpen(
            wide(concat!("PrtSc/", env!("CARGO_PKG_VERSION"))).as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            null(),
            null(),
            0,
        ));
        if session.0.is_null() {
            return Err(last("WinHttpOpen"));
        }
        WinHttpSetTimeouts(session.0, 10000, 10000, 30000, 30000);
        let conn = Handle(WinHttpConnect(session.0, wide(r.host).as_ptr(), 443, 0));
        if conn.0.is_null() {
            return Err(last("WinHttpConnect"));
        }
        let request = Handle(WinHttpOpenRequest(
            conn.0,
            wide("POST").as_ptr(),
            wide(r.path).as_ptr(),
            null(),
            null(),
            null(),
            WINHTTP_FLAG_SECURE,
        ));
        if request.0.is_null() {
            return Err(last("WinHttpOpenRequest"));
        }
        let policy = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
        WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_REDIRECT_POLICY,
            &policy as *const _ as _,
            std::mem::size_of_val(&policy) as u32,
        );
        let mut headers = format!("Content-Type: {}\r\n", r.content_type);
        if service == "imgur" {
            headers.push_str(&format!("Authorization: Client-ID {client_id}\r\n"));
        }
        let headers = wide(&headers);
        if WinHttpSendRequest(
            request.0,
            headers.as_ptr(),
            headers.len() as u32 - 1,
            r.body.as_ptr() as _,
            r.body.len() as u32,
            r.body.len() as u32,
            0,
        ) == 0
        {
            return Err(last("Upload send"));
        }
        if WinHttpReceiveResponse(request.0, null_mut()) == 0 {
            return Err(last("Upload response"));
        }
        let mut status = 0u32;
        let mut n = 4u32;
        if WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            null(),
            &mut status as *mut _ as _,
            &mut n,
            null_mut(),
        ) == 0
        {
            return Err(last("HTTP status"));
        }
        if !(200..300).contains(&status) {
            return Err(format!(
                "Image host returned HTTP {status}; nothing was marked as uploaded."
            ));
        }
        let mut response = vec![];
        let mut buf = [0u8; 8192];
        loop {
            let mut n = 0;
            if WinHttpReadData(request.0, buf.as_mut_ptr() as _, buf.len() as u32, &mut n) == 0 {
                return Err(last("Read upload response"));
            }
            if n == 0 {
                break;
            }
            response.extend_from_slice(&buf[..n as usize]);
            if response.len() > 1_000_000 {
                return Err("Host response exceeds limit".into());
            }
        }
        luma_shot::hosting::link(&response, service)
    }
}

/// Optional cooperative window rendering; never modifies another process.
/// The active app may ignore PrintWindow or return a protected/blank frame.
pub fn compatible_capture(region: Rect) -> Result<Image, String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static IN_FLIGHT: AtomicBool = AtomicBool::new(false);
    let mut desktop = capture(region)?;
    let hwnd = unsafe { GetForegroundWindow() } as usize;
    if hwnd == 0 || IN_FLIGHT.swap(true, Ordering::AcqRel) {
        return Ok(desktop);
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| unsafe {
            let hwnd = hwnd as HWND;
            let mut r: RECT = zeroed();
            if GetWindowRect(hwnd, &mut r) == 0 {
                return Err(last("Window bounds"));
            }
            let w = (r.right - r.left).max(0) as u32;
            let h = (r.bottom - r.top).max(0) as u32;
            if w == 0 || h == 0 || w as u64 * h as u64 > 40_000_000 {
                return Err("Invalid window size".into());
            }
            let mut image = Image::new(w, h)?;
            let raster = Raster::new(&image, false)?;
            if windows_sys::Win32::Storage::Xps::PrintWindow(hwnd, raster.dc, 2) == 0 {
                return Err("Window does not support PrintWindow".into());
            }
            let mut bmi: BITMAPINFO = zeroed();
            bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = w as i32;
            bmi.bmiHeader.biHeight = -(h as i32);
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            // Read the DIB's selected bitmap through a second DC after deselection.
            SelectObject(raster.dc, raster.old);
            let ok = GetDIBits(
                raster.dc,
                raster.bitmap,
                0,
                h,
                image.rgba.as_mut_ptr() as _,
                &mut bmi,
                DIB_RGB_COLORS,
            );
            SelectObject(raster.dc, raster.bitmap);
            if ok == 0 {
                return Err(last("PrintWindow pixels"));
            }
            for p in image.rgba.chunks_exact_mut(4) {
                p.swap(0, 2);
                p[3] = 255;
            }
            let first = image.pixel(0, 0);
            if image.rgba.chunks_exact(4).all(|p| p == first) {
                return Err("PrintWindow returned a uniform/blank frame".into());
            }
            Ok((image, r.left, r.top))
        })();
        IN_FLIGHT.store(false, Ordering::Release);
        let _ = tx.send(result);
    });
    if let Ok(Ok((image, x, y))) = rx.recv_timeout(std::time::Duration::from_millis(200)) {
        let left = x.max(region.x);
        let top = y.max(region.y);
        let right = (x + image.width as i32).min(region.x + region.w as i32);
        let bottom = (y + image.height as i32).min(region.y + region.h as i32);
        for yy in top..bottom {
            let sx = (left - x) as usize;
            let sy = (yy - y) as usize;
            let dx = (left - region.x) as usize;
            let dy = (yy - region.y) as usize;
            let n = (right - left).max(0) as usize * 4;
            if n > 0 {
                desktop.rgba
                    [(dy * region.w as usize + dx) * 4..(dy * region.w as usize + dx) * 4 + n]
                    .copy_from_slice(
                        &image.rgba[(sy * image.width as usize + sx) * 4
                            ..(sy * image.width as usize + sx) * 4 + n],
                    );
            }
        }
    }
    Ok(desktop)
}

/// Shell PIDLs avoid Explorer command-line quoting bugs (spaces, commas, Unicode).
pub fn reveal_file(path: &Path) -> Result<(), String> {
    use windows_sys::Win32::{System::Com::*, UI::Shell::*};
    let absolute = std::fs::canonicalize(path).map_err(|e| format!("Saved file: {e}"))?;
    let text = absolute.to_string_lossy();
    let text = if let Some(p) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{p}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    };
    unsafe {
        let hr = CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32);
        if hr < 0 && hr != 0x80010106u32 as i32 {
            return Err(format!("Shell COM: 0x{:08X}", hr as u32));
        }
        let mut pidl = null_mut();
        let parsed = SHParseDisplayName(wide(&text).as_ptr(), null_mut(), &mut pidl, 0, null_mut());
        let result = if parsed >= 0 {
            let result = SHOpenFolderAndSelectItems(pidl, 0, null(), 0);
            CoTaskMemFree(pidl as _);
            result
        } else {
            parsed
        };
        if hr >= 0 {
            CoUninitialize();
        }
        if result >= 0 {
            Ok(())
        } else {
            // Explorer's /select grammar: the single quoted path follows the comma.
            let parameters = wide(&format!("/select,\"{text}\""));
            let launched = ShellExecuteW(
                null_mut(),
                wide("open").as_ptr(),
                wide("explorer.exe").as_ptr(),
                parameters.as_ptr(),
                null(),
                SW_SHOWNORMAL,
            ) as isize;
            if launched > 32 {
                Ok(())
            } else {
                Err(format!(
                    "Select saved file: shell error {launched}, HRESULT 0x{:08X}",
                    result as u32
                ))
            }
        }
    }
}
/// Standard Windows URL association dispatch (same path as earlier builds).
pub fn open_browser(url: &str) -> Result<(), String> {
    unsafe {
        let rc = windows_sys::Win32::UI::Shell::ShellExecuteW(
            null_mut(),
            wide("open").as_ptr(),
            wide(url).as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        ) as isize;
        if rc <= 32 {
            Err(format!("Open default browser: {rc}"))
        } else {
            Ok(())
        }
    }
}

pub fn app_icon(size: i32) -> Result<HICON, String> {
    unsafe {
        let h = LoadImageW(
            windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(null()),
            1usize as *const u16,
            IMAGE_ICON,
            size,
            size,
            LR_DEFAULTCOLOR,
        ) as HICON;
        if !h.is_null() {
            return Ok(h);
        }
        let ico = include_bytes!("../../assets/luma.ico");
        let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
        let mut best = None;
        for i in 0..count {
            let e = 6 + i * 16;
            let w = if ico[e] == 0 { 256 } else { ico[e] as i32 };
            let distance = (w - size).abs();
            if best.map(|(d, _)| distance < d).unwrap_or(true) {
                best = Some((distance, e));
            }
        }
        let (_, e) = best.ok_or("Application icon has no sizes")?;
        let length = u32::from_le_bytes(ico[e + 8..e + 12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(ico[e + 12..e + 16].try_into().unwrap()) as usize;
        let bytes = ico
            .get(offset..offset + length)
            .ok_or("Invalid application icon")?;
        let h = CreateIconFromResourceEx(
            bytes.as_ptr(),
            bytes.len() as u32,
            1,
            0x00030000,
            size,
            size,
            LR_DEFAULTCOLOR,
        );
        if h.is_null() {
            Err(last("Load tiger icon"))
        } else {
            Ok(h)
        }
    }
}
