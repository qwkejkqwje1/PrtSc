#[path = "win_dialogs.rs"]
mod dialogs;
#[path = "win_native.rs"]
mod native;
#[path = "win_paint.rs"]
mod paint;
use dialogs::{child, history_proc, settings_proc, text_proc};
#[path = "win_text_http.rs"]
mod text_http;
#[path = "win_translate.rs"]
mod translate_ui;
#[path = "win_video.rs"]
mod video;
use luma_shot::{
    config::Config,
    editor::{Annotation, Editor, Point, Tool},
    history,
    image::{Image, Rect},
    render::Renderer,
    ui::{self, Action, Layout, View},
    util,
};
use native::wide;
use std::{
    collections::HashMap,
    mem::{size_of, zeroed},
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::mpsc::{self, Receiver, Sender},
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{LibraryLoader::*, Threading::*},
    UI::{Controls::*, HiDpi::*, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
};
static QR_REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn qr_revision() -> u64 {
    QR_REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
const TRAY: u32 = WM_APP + 1;
const DONE: u32 = WM_APP + 2;
const BEGIN: u32 = WM_APP + 3;
const CAPTURE: u32 = 10;
const SETTINGS: u32 = 11;
const HISTORY: u32 = 12;
const EXIT: u32 = 13;
const ABOUT: u32 = 14;
const LICENSES: u32 = 15;
const COPY: u32 = 101;
const SAVE: u32 = 102;
const SAVE_AS: u32 = 115;
const LENS: u32 = 103;
const RECORD: u32 = 116;
const UPLOAD: u32 = 117;
const FOLDER: u32 = 118;
const COMPAT: u32 = 119;
const REVEAL: u32 = 125;
const OCR: u32 = 105;
const TRANSLATE: u32 = 128;
const PIN: u32 = 106;
const PRINT: u32 = 107;
const QR: u32 = 111;
const SCROLL: u32 = 112;
const RGB: u32 = 113;
const HEX: u32 = 114;
const BLUR_TOOL: u32 = 126;
const PIXELATE_TOOL: u32 = 127;
#[derive(Clone)]
enum Drag {
    None,
    Selection(Point),
    RegionResize {
        before: Rect,
        edge: u8,
    },
    Draw,
    Move {
        index: usize,
        before: Annotation,
        start: Point,
    },
    Resize {
        index: usize,
        before: Annotation,
        corner: usize,
        start: Point,
    },
}
struct Session {
    cache: Option<paint::FrameCache>,
    scene_builds: u64,
    scene_appends: u64,
    dirty_pixels: u64,
    paint_count: u64,
    present_count: u64,
    qr_values: Vec<String>,
    qr_copied: bool,
    pending_surrogate: u16,
    qr_revision: u64,
    editor: Editor,
    font_ready: bool,
    renderer: Renderer,
    view: View,
    screen: Rect,
    layout: Option<Layout>,
    cursor: (i32, i32),
    drag: Drag,
    status: String,
    snaps: Vec<Rect>,
    typing: bool,
    text_replace: Option<(usize, Annotation)>,
    started: Instant,
    capture_ms: f64,
    backend: &'static str,
    painted: bool,
}
impl Session {
    fn live_damage(&self) -> Option<Rect> {
        let a = match &self.drag {
            Drag::Draw => self.editor.draft.as_ref(),
            Drag::Move { index, .. } | Drag::Resize { index, .. } => {
                self.editor.annotations.get(*index)
            }
            _ => None,
        }?;
        Some(
            luma_shot::damage::expand(self.view.rect(self.renderer.bounds(a)), 12)
                .clamp(self.view.width, self.view.height),
        )
    }
}
enum Work {
    Saved(PathBuf),
    TextTranslation {
        request: u64,
        source: String,
        target: String,
        result: Result<String, String>,
    },
    Text {
        title: String,
        text: String,
        copy: bool,
    },
    Scroll(Image, Rect),
    Qr {
        copy: bool,
        revision: u64,
        values: Vec<String>,
    },
    Uploaded(String),
    Recorded(Result<video::Report, String>),
    Silent,
}
struct Job {
    foreground: bool,
    result: Result<Work, String>,
}
struct Pin {
    image: Image,
    drag: Option<(POINT, RECT)>,
}
struct SettingsState {
    hwnd: HWND,
    fields: Vec<HWND>,
    history: HWND,
    experimental: HWND,
    sound: HWND,
    confirm: HWND,
    compat: HWND,
    video_cursor: HWND,
}
struct TextState {
    url: Option<String>,
    hwnd: HWND,
    edit: HWND,
    ocr: bool,
}
struct HistoryState {
    hwnd: HWND,
    search: HWND,
    list: HWND,
    filtered: Vec<history::Entry>,
}
struct Recording {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    hwnd: HWND,
    started: Instant,
}
#[derive(Clone)]
enum NoticeTarget {
    File(PathBuf),
    Url(String),
}
struct App {
    notice_target: std::cell::RefCell<Option<NoticeTarget>>,
    last_saved: std::cell::RefCell<Option<PathBuf>>,
    recording: Option<Recording>,
    exit_after_recording: bool,
    main: HWND,
    overlay: HWND,
    session: Option<Session>,
    config: Config,
    tx: Sender<Job>,
    rx: Receiver<Job>,
    busy: bool,
    qr_running: bool,
    pins: HashMap<isize, Pin>,
    settings: Option<SettingsState>,
    text: Option<TextState>,
    translation: Option<translate_ui::TranslationState>,
    translation_request: u64,
    translation_network_busy: bool,
    history: Option<HistoryState>,
    icon: HICON,
    window_icon: HICON,
    mutex: HANDLE,
    hotkey: bool,
    hotkey_id: i32,
}
unsafe fn app(hwnd: HWND) -> Option<&'static mut App> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    ptr.as_mut()
}
unsafe fn attach(hwnd: HWND, l: LPARAM) {
    let cs = &*(l as *const CREATESTRUCTW);
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
}
pub fn error(e: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(e).as_ptr(),
            wide("PrtSc").as_ptr(),
            if Config::load().sound_enabled {
                MB_ICONERROR
            } else {
                MB_OK
            },
        );
    }
}
pub fn capture_file(path: &Path) -> Result<(), String> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        native::capture(native::screen_rect())?.save(path)
    }
}
pub fn run(demo: bool) -> Result<(), String> {
    run_mode(demo, None)
}
/// Test fixture: shows the real privacy/translation UI without requiring WinRT OCR.
pub fn translation_preview(text: String) -> Result<(), String> {
    run_mode(true, Some(text))
}
fn run_mode(demo: bool, preview: Option<String>) -> Result<(), String> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES | ICC_LINK_CLASS,
        };
        InitCommonControlsEx(&controls);
        let mutex = CreateMutexW(null(), 0, wide("Local\\PrtSc.Instance.0.2").as_ptr());
        if mutex.is_null() {
            return Err(native::last("CreateMutex"));
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let h = FindWindowW(wide("LumaMain").as_ptr(), null());
            if !h.is_null() {
                PostMessageW(h, BEGIN, 0, 0);
            }
            CloseHandle(mutex);
            return Ok(());
        }
        let instance = GetModuleHandleW(null());
        let window_icon = native::app_icon(32)?;
        let small_icon = native::app_icon(GetSystemMetrics(SM_CXSMICON).max(16))?;
        let classes: [(&str, WNDPROC, u32); 8] = [
            ("LumaRecord", Some(record_proc), 0),
            ("LumaMain", Some(main_proc), 0),
            ("LumaOverlay", Some(overlay_proc), CS_DBLCLKS),
            ("LumaPin", Some(pin_proc), CS_DBLCLKS),
            ("LumaSettings", Some(settings_proc), 0),
            ("LumaText", Some(text_proc), 0),
            ("LumaTranslate", Some(translate_ui::translation_proc), 0),
            ("LumaHistory", Some(history_proc), CS_DBLCLKS),
        ];
        for (name, proc, style) in classes {
            let mut wc: WNDCLASSEXW = zeroed();
            wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
            wc.hInstance = instance;
            wc.hIcon = window_icon;
            wc.hIconSm = small_icon;
            wc.lpfnWndProc = proc;
            let n = wide(name);
            wc.lpszClassName = n.as_ptr();
            wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
            wc.style = style;
            wc.hbrBackground = if name == "LumaOverlay" {
                null_mut()
            } else {
                (COLOR_WINDOW + 1) as _
            };
            if RegisterClassExW(&wc) == 0 {
                return Err(native::last("Register window class"));
            }
        }
        let (tx, rx) = mpsc::channel();
        let mut state = Box::new(App {
            notice_target: std::cell::RefCell::new(None),
            last_saved: std::cell::RefCell::new(None),
            recording: None,
            exit_after_recording: false,
            main: null_mut(),
            overlay: null_mut(),
            session: None,
            config: Config::load(),
            tx,
            rx,
            busy: false,
            qr_running: false,
            pins: HashMap::new(),
            settings: None,
            text: None,
            translation: None,
            translation_request: 0,
            translation_network_busy: false,
            history: None,
            icon: small_icon,
            window_icon,
            mutex,
            hotkey: false,
            hotkey_id: 1,
        });
        luma_shot::i18n::configure(&state.config.language, &state.config.toolbar);

        let ptr = &mut *state as *mut App;
        state.main = CreateWindowExW(
            0,
            wide("LumaMain").as_ptr(),
            wide("PrtSc").as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            ptr as _,
        );
        if state.main.is_null() {
            return Err(native::last("Create tray window"));
        }
        let mut nid = state.nid();
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = TRAY;
        nid.hIcon = state.icon;
        copy_wide(&mut nid.szTip, "PrtSc · PrintScreen");
        Shell_NotifyIconW(NIM_ADD, &nid);
        let hk = luma_shot::hotkey::parse(&state.config.capture_hotkey)?;
        state.hotkey = RegisterHotKey(
            state.main,
            state.hotkey_id,
            hk.modifiers | MOD_NOREPEAT,
            hk.key,
        ) != 0;
        if !state.hotkey {
            error("Горячая клавиша занята Windows или другой программой. Выберите другую в настройках через значок PrtSc в трее.");
        }
        RegisterHotKey(state.main, 2, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, 82);
        if demo {
            state.start(true)?;
        }
        if let Some(text) = preview {
            state.close_editor();
            state.show_ocr_text("Распознанный текст", &text);
        }
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            let dialogs = [
                state.settings.as_ref().map(|s| s.hwnd),
                state.text.as_ref().map(|s| s.hwnd),
                state.translation.as_ref().map(|s| s.hwnd),
                state.history.as_ref().map(|s| s.hwnd),
            ];
            if dialogs
                .into_iter()
                .flatten()
                .any(|h| IsDialogMessageW(h, &mut msg) != 0)
            {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let nid = state.nid();
        Shell_NotifyIconW(NIM_DELETE, &nid);
        if state.hotkey {
            UnregisterHotKey(state.main, state.hotkey_id);
        }
        UnregisterHotKey(state.main, 2);
        if !state.icon.is_null() {
            DestroyIcon(state.icon);
        }
        if !state.window_icon.is_null() {
            DestroyIcon(state.window_icon);
        }
        CloseHandle(state.mutex);
        Ok(())
    }
}
fn copy_wide<const N: usize>(dst: &mut [u16; N], text: &str) {
    for (d, s) in dst.iter_mut().zip(text.encode_utf16().take(N - 1)) {
        *d = s
    }
}
impl App {
    unsafe fn nid(&self) -> NOTIFYICONDATAW {
        let mut n: NOTIFYICONDATAW = zeroed();
        n.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        n.hWnd = self.main;
        n.uID = 1;
        n
    }
    fn notify(&self, title: &str, text: &str) {
        // Keep the last actionable target across informational notifications.
        unsafe {
            let mut n = self.nid();
            n.uFlags = NIF_INFO;
            n.dwInfoFlags = NIIF_INFO
                | if self.config.sound_enabled {
                    0
                } else {
                    NIIF_NOSOUND
                };
            copy_wide(&mut n.szInfoTitle, luma_shot::i18n::tr(title));
            copy_wide(&mut n.szInfo, &luma_shot::i18n::text(text));
            Shell_NotifyIconW(NIM_MODIFY, &n);
        }
    }
    fn notify_file(&self, path: &Path) {
        *self.last_saved.borrow_mut() = Some(path.to_owned());
        self.notify(
            "Сохранено · нажмите, чтобы открыть папку",
            &path.to_string_lossy(),
        );
        *self.notice_target.borrow_mut() = Some(NoticeTarget::File(path.to_owned()));
    }
    fn open_notice(&self) {
        let target = self.notice_target.borrow().clone();
        if let Some(target) = target {
            match target {
                NoticeTarget::File(p) => {
                    if let Err(e) = native::reveal_file(&p) {
                        self.notify("PrtSc", &e);
                    }
                }
                NoticeTarget::Url(u) => native::open(Path::new(&u)),
            }
        }
    }
    fn invalidate(&self) {
        unsafe {
            if !self.overlay.is_null() {
                InvalidateRect(self.overlay, null(), 0);
            }
        }
    }
    fn invalidate_region(&self, r: Rect) {
        unsafe {
            if !self.overlay.is_null() && r.valid() {
                let r = RECT {
                    left: r.x,
                    top: r.y,
                    right: r.x + r.w as i32,
                    bottom: r.y + r.h as i32,
                };
                InvalidateRect(self.overlay, &r, 0);
            }
        }
    }
    fn fail(&mut self, e: &str) {
        if let Some(s) = &mut self.session {
            s.status = e.chars().take(110).collect();
            self.invalidate()
        } else {
            self.notify("PrtSc", e)
        }
    }
    fn start(&mut self, demo: bool) -> Result<(), String> {
        if let Some(t) = &self.translation {
            unsafe {
                SetForegroundWindow(t.hwnd);
            }
            return Ok(());
        }
        if let Some(r) = &self.recording {
            unsafe {
                SetForegroundWindow(r.hwnd);
            }
            return Err("Stop recording first: Ctrl+Shift+R".into());
        }
        if self.busy {
            return Err("Операция ещё выполняется".into());
        }
        if self.session.is_some() {
            unsafe {
                SetForegroundWindow(self.overlay);
            }
            return Ok(());
        }
        let now = Instant::now();
        unsafe {
            let screen = native::screen_rect();
            let snaps = vec![];
            let mut editor = if demo {
                ui::demo()
            } else {
                Editor::new(if self.config.window_capture {
                    native::compatible_capture(screen)?
                } else {
                    native::capture(screen)?
                })
            };
            editor.tool = Tool::Pen;
            editor.tool_colors = self.config.tool_colors.clone();
            editor.color = self.config.color_for(editor.tool);
            if demo {
                editor.selected = None
            }
            let capture_ms = now.elapsed().as_secs_f64() * 1000.;
            let view = View::fit(&editor.image, screen.w, screen.h);
            self.session = Some(Session {
                cache: None,
                scene_builds: 0,
                scene_appends: 0,
                dirty_pixels: 0,
                paint_count: 0,
                present_count: 0,
                qr_values: vec![],
                qr_copied: false,
                pending_surrogate: 0,
                qr_revision: qr_revision(),
                editor,
                font_ready: demo,
                renderer: if demo {
                    Renderer::new()
                } else {
                    Renderer::unloaded()
                },
                view,
                screen,
                layout: None,
                cursor: (-1, -1),
                drag: Drag::None,
                status: String::new(),
                snaps,
                typing: false,
                text_replace: None,
                started: now,
                capture_ms,
                backend: if demo {
                    "demo"
                } else if self.config.window_capture {
                    "BitBlt+PrintWindow (experimental)"
                } else {
                    "BitBlt"
                },
                painted: false,
            });
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                wide("LumaOverlay").as_ptr(),
                wide("PrtSc · Editor").as_ptr(),
                WS_POPUP,
                screen.x,
                screen.y,
                screen.w as i32,
                screen.h as i32,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                self as *mut _ as _,
            );
            if hwnd.is_null() {
                self.session = None;
                return Err(native::last("Create overlay"));
            }
            self.overlay = hwnd;
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetFocus(hwnd);
            UpdateWindow(hwnd);
        }
        Ok(())
    }
    fn close_editor(&mut self) {
        unsafe {
            if !self.overlay.is_null() {
                let h = self.overlay;
                self.overlay = null_mut();
                DestroyWindow(h);
            }
        }
        self.session = None;
    }
    fn commit_text(&mut self) {
        if let Some(s) = &mut self.session {
            if s.typing {
                s.typing = false;
                s.pending_surrogate = 0;
                if let Some(a) = s.editor.draft.take() {
                    if !a.text.is_empty() {
                        if let Some((index, before)) = s.text_replace.take() {
                            s.editor.replace(index, before, a)
                        } else {
                            s.editor.add(a)
                        }
                    } else if let Some((index, before)) = s.text_replace.take() {
                        s.editor.annotations[index] = before;
                    }
                }
            }
        }
    }
    fn finish_gesture(&mut self) -> bool {
        let mut changed = false;
        if let Some(s) = &mut self.session {
            changed = !matches!(s.drag, Drag::None);
            match std::mem::replace(&mut s.drag, Drag::None) {
                Drag::Draw => {
                    if let Some(d) = s.editor.draft.take() {
                        if d.rect().w > 1 || d.rect().h > 1 {
                            s.editor.add(d);
                        }
                    }
                }
                Drag::Move { index, before, .. } | Drag::Resize { index, before, .. } => {
                    if let Some(after) = s.editor.annotations.get(index).cloned() {
                        s.editor.replace(index, before, after);
                    }
                }
                _ => {}
            }
        }
        // Clear the transaction BEFORE ReleaseCapture's synchronous notification.
        unsafe {
            if !self.overlay.is_null() && GetCapture() == self.overlay {
                ReleaseCapture();
            }
        }
        if changed {
            self.invalidate();
        }
        changed
    }
    fn final_image(&mut self) -> Result<Image, String> {
        self.finish_gesture();
        self.commit_text();
        let s = self.session.as_ref().ok_or("Нет открытого снимка")?;
        let rect = s.editor.selection().ok_or("Сначала выделите область")?;
        if s.editor.annotations.is_empty() {
            return s.editor.image.crop(rect);
        }
        if s.view.scale == 1. {
            if let Some(cache) = &s.cache {
                if cache.annotations == s.editor.annotations && s.editor.draft.is_none() {
                    return cache.scene.crop(rect);
                }
            }
        }
        s.renderer
            .flatten(&s.editor.image, &s.editor.annotations)
            .crop(rect)
    }
    fn spawn<F>(&mut self, label: &str, f: F)
    where
        F: FnOnce() -> Result<Work, String> + Send + 'static,
    {
        if self.busy {
            self.fail("Операция уже выполняется");
            return;
        }
        self.busy = true;
        if let Some(s) = &mut self.session {
            s.status = label.into();
            self.invalidate()
        }
        let tx = self.tx.clone();
        let hwnd = self.main as usize;
        std::thread::spawn(move || {
            let result = f();
            let _ = tx.send(Job {
                foreground: true,
                result,
            });
            unsafe {
                PostMessageW(hwnd as HWND, DONE, 0, 0);
            }
        });
    }
    fn done(&mut self) {
        while let Ok(job) = self.rx.try_recv() {
            if job.foreground {
                self.busy = false;
            }
            match job.result {
                Err(e) => self.fail(&e),
                Ok(Work::TextTranslation {
                    request,
                    source,
                    target,
                    result,
                }) => {
                    self.translation_network_busy = false;
                    self.finish_text_translation(request, &source, &target, result);
                }
                Ok(Work::Saved(p)) => {
                    self.close_editor();
                    self.notify_file(&p);
                }
                Ok(Work::Text { title, text, copy }) => {
                    if copy {
                        let _ = native::clipboard_text(self.main, &text);
                    }
                    self.show_ocr_text(&title, &text)
                }
                Ok(Work::Scroll(image, screen)) => {
                    self.close_editor();
                    unsafe {
                        let view = View::fit(&image, screen.w, screen.h);
                        let mut editor = Editor::new(image);
                        editor.tool_colors = self.config.tool_colors.clone();
                        editor.color = self.config.color_for(Tool::Pen);
                        editor.selection = Some(editor.bounds());
                        self.session = Some(Session {
                            cache: None,
                            scene_builds: 0,
                            scene_appends: 0,
                            dirty_pixels: 0,
                            paint_count: 0,
                            present_count: 0,
                            qr_values: vec![],
                            qr_copied: false,
                            pending_surrogate: 0,
                            qr_revision: qr_revision(),
                            editor,
                            font_ready: true,
                            renderer: Renderer::new(),
                            view,
                            screen,
                            layout: None,
                            cursor: (-1, -1),
                            drag: Drag::None,
                            status: "Длинный снимок · проверьте результат".into(),
                            snaps: vec![],
                            typing: false,
                            text_replace: None,
                            started: Instant::now(),
                            capture_ms: 0.,
                            backend: "scroll/BitBlt",
                            painted: false,
                        });
                        self.overlay = CreateWindowExW(
                            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                            wide("LumaOverlay").as_ptr(),
                            wide("PrtSc · Scrolling capture").as_ptr(),
                            WS_POPUP,
                            screen.x,
                            screen.y,
                            screen.w as i32,
                            screen.h as i32,
                            null_mut(),
                            null_mut(),
                            GetModuleHandleW(null()),
                            self as *mut _ as _,
                        );
                        ShowWindow(self.overlay, SW_SHOW);
                        SetForegroundWindow(self.overlay);
                    }
                }
                Ok(Work::Uploaded(link)) => {
                    let _ = native::clipboard_text(self.main, &link);
                    self.close_editor();
                    self.show_text(luma_shot::i18n::tr("Ссылка скопирована"), &link, None);
                    self.notify("Ссылка скопирована", &link);
                    *self.notice_target.borrow_mut() = Some(NoticeTarget::Url(link));
                }
                Ok(Work::Recorded(result)) => {
                    if let Some(r) = self.recording.take() {
                        unsafe {
                            KillTimer(self.main, 5);
                            DestroyWindow(r.hwnd);
                        }
                    }
                    match result {
                        Ok(report) => {
                            let _=std::fs::write(util::data_dir().join("last-recording.json"),serde_json::json!({"frames":report.frames,"seconds":report.seconds,"width":report.width,"height":report.height}).to_string());
                            self.notify_file(&report.path);
                        }
                        Err(e) => self.show_text("PrtSc · Recording error", &e, None),
                    }
                    if self.exit_after_recording {
                        unsafe {
                            PostQuitMessage(0);
                        }
                    }
                }
                Ok(Work::Qr {
                    revision,
                    values,
                    copy,
                }) => {
                    self.qr_running = false;
                    if let Some(s) = &mut self.session {
                        if s.qr_revision == revision {
                            if s.qr_values != values {
                                s.qr_copied = false;
                            }
                            s.qr_values = values.clone();
                            if copy && !values.is_empty() {
                                match native::clipboard_text(self.main, &values.join("\r\n\r\n")) {
                                    Ok(()) => s.qr_copied = true,
                                    Err(e) => s.status = e,
                                }
                            }
                            let _ = std::fs::create_dir_all(util::data_dir());
                            let _ = std::fs::write(
                                util::data_dir().join("qr-diagnostics.json"),
                                format!("{{\"revision\":{revision},\"count\":{}}}", values.len()),
                            );
                            s.status = luma_shot::i18n::text(&if values.is_empty() {
                                "QR не найден · Enter — копировать".into()
                            } else {
                                if luma_shot::i18n::english() {
                                    format!("{} QR · click QR / Ctrl+Q to copy", values.len())
                                } else {
                                    format!("QR: {} · QR / Ctrl+Q — скопировать", values.len())
                                }
                            });
                        }
                    }
                }
                Ok(Work::Silent) => {}
            }
        }
        self.invalidate();
    }
    fn action(&mut self, a: Action) {
        if a != Action::Cancel {
            self.finish_gesture();
        }
        match a {
            Action::Tool(t) => {
                self.commit_text();
                if let Some(s) = &mut self.session {
                    s.editor.tool = t;
                    s.editor.color = self.config.color_for(t);
                    s.editor.selected = None;
                }
            }
            Action::PickColor => {
                if let Some(s) = &self.session {
                    if s.editor.tool != Tool::Redact {
                        if let Some(c) =
                            native::choose_color(self.overlay, s.editor.color, &self.config.palette)
                        {
                            self.action(Action::Color(c));
                        }
                    }
                }
            }
            Action::Cancel => self.close_editor(),
            Action::Color(c) => {
                if let Some(s) = &mut self.session {
                    let tool = s
                        .editor
                        .selected
                        .and_then(|i| s.editor.annotations.get(i))
                        .map(|a| a.tool)
                        .unwrap_or(s.editor.tool);
                    if tool == Tool::Redact {
                        return;
                    }
                    self.config.remember_color(tool, c);
                    s.editor.tool_colors = self.config.tool_colors.clone();
                    let _ = self.config.save();
                    s.editor.color = c;
                    if let Some(i) = s.editor.selected {
                        let before = s.editor.annotations[i].clone();
                        let mut after = before.clone();
                        after.color = c;
                        s.editor.replace(i, before, after)
                    }
                }
            }
            Action::Width => self.stroke(1.),
            Action::Undo => {
                self.commit_text();
                if let Some(s) = &mut self.session {
                    s.editor.undo()
                }
            }
            Action::Redo => {
                if let Some(s) = &mut self.session {
                    s.editor.redo()
                }
            }
            Action::Record => self.command(RECORD),
            Action::Upload => self.command(UPLOAD),
            Action::Qr => self.command(QR),
            Action::Pin => self.command(PIN),
            Action::Copy => self.command(COPY),
            Action::Save => self.command(SAVE),
            Action::Lens => self.command(LENS),
            Action::NewCapture => {
                self.close_editor();
                unsafe {
                    SetTimer(self.main, 3, 80, None);
                }
            }
            Action::Ocr => self.command(OCR),
            Action::Translate => self.command(TRANSLATE),
            Action::More => self.menu(true),
        }
        if matches!(
            a,
            Action::Tool(_) | Action::Color(_) | Action::Width | Action::Undo | Action::Redo
        ) {
            self.schedule_qr();
        }
        self.invalidate()
    }
    fn schedule_qr(&mut self) {
        if let Some(s) = &mut self.session {
            s.qr_revision = qr_revision();
            s.qr_values.clear();
            s.qr_copied = false;
        }
        unsafe {
            KillTimer(self.overlay, 2);
            if self.config.auto_qr && !self.overlay.is_null() {
                SetTimer(self.overlay, 2, 500, None);
            }
        }
    }
    fn stroke(&mut self, d: f32) {
        if let Some(s) = &mut self.session {
            s.editor.width = if s.editor.width + d > 12. {
                1.
            } else {
                (s.editor.width + d).max(1.)
            };
            if let Some(i) = s.editor.selected {
                let before = s.editor.annotations[i].clone();
                let mut after = before.clone();
                after.width = s.editor.width;
                s.editor.replace(i, before, after)
            }
        }
    }
    fn command(&mut self, id: u32) {
        if id == COMPAT {
            self.config.window_capture = !self.config.window_capture;
            if let Err(e) = self.config.save() {
                self.fail(&e)
            }
            return;
        }
        let action_started = Instant::now();
        if id == CAPTURE {
            if let Err(e) = self.start(false) {
                self.fail(&e)
            }
            return;
        }
        match id {
            BLUR_TOOL | PIXELATE_TOOL => {
                self.action(Action::Tool(if id == BLUR_TOOL {
                    Tool::Blur
                } else {
                    Tool::Pixelate
                }));
                return;
            }
            LICENSES => {
                self.show_text(
                    "PrtSc · Open-source licenses",
                    luma_shot::LICENSE_TEXT,
                    None,
                );
                return;
            }
            SETTINGS => {
                self.show_settings();
                return;
            }
            HISTORY => {
                self.show_history();
                return;
            }
            EXIT => {
                if self.recording.is_some() {
                    self.exit_after_recording = true;
                    self.stop_recording();
                } else {
                    unsafe {
                        PostQuitMessage(0);
                    }
                }
                return;
            }
            RECORD => {
                if let Err(e) = self.start_recording() {
                    self.fail(&e)
                }
                return;
            }
            REVEAL => {
                let p = self.last_saved.borrow().clone();
                if let Some(p) = p {
                    if let Err(e) = native::reveal_file(&p) {
                        self.fail(&e);
                    }
                }
                return;
            }
            FOLDER => {
                let path = PathBuf::from(&self.config.save_dir);
                let _ = std::fs::create_dir_all(&path);
                native::open(&path);
                return;
            }
            ABOUT => {
                let help = if luma_shot::i18n::english() {
                    "PrtSc 1.2\r\nCapture. Explain. Share.\r\nNative Rust. No clutter. Yours to keep.\r\n\r\nPrintScreen — capture · drag to select · Alt+click — window\r\nEnter / Ctrl+C — copy · Ctrl+S — save · Ctrl+Shift+S — Save as\r\nCtrl+G — Google · Ctrl+T — offline OCR · Ctrl+Q — read QR\r\nCtrl+Shift+T — translate the image in Google (paste with Ctrl+V)\r\nCtrl+Z / Ctrl+Shift+Z — undo / redo\r\nDrag an edge to resize · click outside for a new area · M — move annotations · P — pen · L — line\r\nA — arrow · R — rectangle · O — oval · H — highlight\r\nB — blur · X — pixelate (not for secrets)\r\nT — text · D — opaque redaction · N — numbered step · I — color picker\r\nClick the single color swatch; each drawing tool remembers its color. Hover icons for labels.\r\nVideo camera / Ctrl+Shift+R — record selected live area. Stop with Ctrl+Shift+R or Stop.\r\nFormat, FPS and output folder are in Settings. GIF: up to 1280px / 15 FPS. No audio.\r\nCursor recording is optional in Settings. Video does NOT include screenshot annotations/redactions.\r\nUpload arrow — selected host; links are public. Google button sends the final selection.\r\nQR is local; links are never opened automatically. OCR uses installed Windows OCR languages.\r\nOpaque redaction removes covered pixels in exports; undo and the original remain in the editor.\r\nExperimental PrintWindow is cooperative capture, NOT a guaranteed Telegram protection bypass.\r\nPerformance targets need validation on real Windows hardware."
                } else {
                    "PrtSc 1.2\r\nВыделил. Объяснил. Отправил.\r\nМастер скриншотов. Нативный Rust. Без лишнего. Навсегда.\r\n\r\nPrintScreen — снимок · мышь — выделение · Alt+щелчок — окно\r\nEnter / Ctrl+C — копировать · Ctrl+S — сохранить · Ctrl+Shift+S — сохранить как\r\nCtrl+G — Google · Ctrl+T — OCR офлайн · Ctrl+Q — прочитать QR\r\nCtrl+Shift+T — перевод картинки в Google (вставить Ctrl+V)\r\nCtrl+Z / Ctrl+Shift+Z — отменить / повторить\r\nКрай рамки — размер · вне рамки — новая область · M — перемещение пометок · P — карандаш · L — линия\r\nA — стрелка · R — прямоугольник · O — овал · H — маркер\r\nB — размытие · X — пикселизация (не для секретов)\r\nT — текст · D — скрыть данные · N — номер шага · I — пипетка\r\nНажмите на цвет: он сохраняется отдельно для каждого инструмента. Наведите на значок для пояснения.\r\nКинокамера / Ctrl+Shift+R — запись живой области. Остановка: Ctrl+Shift+R или Стоп.\r\nФормат, частота и папка — в настройках. GIF: до 1280 пикселей / 15 FPS. Без звука.\r\nКурсор в записи отключается в настройках. Видеозапись НЕ включает пометки и закрашивание из редактора снимка.\r\nСтрелка загрузки — выбранный фотохостинг; ссылки публичные. Google отправляет итоговое выделение.\r\nQR читается локально; ссылки открываются только по вашему клику. OCR использует языки Windows OCR.\r\nНепрозрачная заливка удаляет закрытые пиксели при экспорте; исходник и отмена остаются в редакторе.\r\nЭкспериментальный PrintWindow — не гарантированный обход защиты Telegram.\r\nЦелевые задержки и память требуют проверки на реальной Windows."
                };
                self.show_text(luma_shot::i18n::tr("PrtSc 1.2 · Справка"), help, None);
                return;
            }
            _ => {}
        }
        if self.busy {
            self.fail("Дождитесь завершения текущей операции");
            return;
        }
        let result = self.final_image();
        let image = match result {
            Ok(i) => i,
            Err(e) => {
                self.fail(&e);
                return;
            }
        };
        match id {
            COPY => {
                let now = Instant::now();
                match native::clipboard_image(self.main, &image) {
                    Ok(()) => {
                        let ms = now.elapsed().as_secs_f64() * 1000.;
                        self.notify(
                            "Скопировано",
                            &format!(
                                "{} × {} · Буфер обмена: {:.1} мс",
                                image.width, image.height, ms
                            ),
                        );
                        let _ = std::fs::create_dir_all(util::data_dir());
                        if let Some(s) = &self.session {
                            let metrics = serde_json::json!({"capture_backend":s.backend,"capture_ms":if s.backend=="BitBlt"{serde_json::json!(s.capture_ms)}else{serde_json::Value::Null},"clipboard_write_ms":ms,"copy_request_to_clipboard_ms":action_started.elapsed().as_secs_f64()*1000.,"note":"clipboard_write_ms excludes flatten/crop; copy_request_to_clipboard_ms includes them. capture_ms is null for synthetic demo/scroll. First CPU paint is not GPU presentation","width":image.width,"height":image.height,"scene_rebuilds":s.scene_builds,"scene_appends":s.scene_appends,"dirty_pixels":s.dirty_pixels,"paints":s.paint_count});
                            let _ = std::fs::write(
                                util::data_dir().join("last-metrics.json"),
                                serde_json::to_vec_pretty(&metrics).unwrap(),
                            );
                        }
                        self.close_editor();
                        if self.config.history_enabled {
                            let max = self.config.history_limit;
                            self.spawn(
                                "Сохраняем локальную историю",
                                move || {
                                    history::add(&image, "", max)?;
                                    Ok(Work::Silent)
                                },
                            )
                        }
                    }
                    Err(e) => self.fail(&e),
                }
            }
            SAVE | SAVE_AS => {
                let cfg = self.config.clone();
                let path = if id == SAVE_AS {
                    unsafe {
                        ShowWindow(self.overlay, SW_HIDE);
                    }
                    native::save_dialog_options(self.main, &cfg)
                } else {
                    let dir = PathBuf::from(&cfg.save_dir);
                    if let Err(e) = std::fs::create_dir_all(&dir) {
                        self.fail(&format!("Не удалось создать папку: {e}"));
                        return;
                    }
                    let (_, stamp) = util::utc_stamp(util::epoch());
                    Some(dir.join(format!(
                        "PrtSc_{stamp}_{}.{}",
                        util::unique_id(),
                        cfg.format
                    )))
                };
                if let Some(path) = path {
                    unsafe {
                        ShowWindow(self.overlay, SW_SHOW);
                        SetForegroundWindow(self.overlay);
                    }
                    self.spawn("Сохраняем", move || {
                        image.save_options(&path, cfg.jpeg_quality, &cfg.png_compression)?;
                        if cfg.history_enabled {
                            history::add(&image, "", cfg.history_limit)?
                        }
                        Ok(Work::Saved(path))
                    });
                } else {
                    unsafe {
                        ShowWindow(self.overlay, SW_SHOW);
                        SetForegroundWindow(self.overlay);
                    }
                }
            }
            LENS => {
                unsafe {
                    if self.config.confirm_google && MessageBoxW(self.overlay,wide("Передать выделенную картинку с применёнными правками в Google?\n\nПроверьте личные данные. Откроется вкладка Google с автоматической загрузкой изображения. Правила обработки данных определяет Google.").as_ptr(),wide("PrtSc · Поиск Google").as_ptr(),MB_OKCANCEL|MB_ICONINFORMATION)!=IDOK{return}
                }
                let handoff = match luma_shot::google::Handoff::from_owned(
                    image,
                    self.config.search_mode == "fast",
                ) {
                    Ok(h) => h,
                    Err(e) => {
                        self.fail(&e);
                        return;
                    }
                };
                let url = handoff.url();
                self.close_editor();
                self.spawn("Открываем Google", move || {
                    handoff.serve()?;
                    Ok(Work::Silent)
                });
                let now = Instant::now();
                if let Err(e) = native::open_browser(&url) {
                    self.fail(&e);
                }
                let metrics = serde_json::json!({"browser_dispatch_ms":now.elapsed().as_secs_f64()*1000.,"note":"Default browser dispatch, not Google result latency"});
                let _ = std::fs::write(
                    util::data_dir().join("last-browser-launch.json"),
                    metrics.to_string(),
                );
            }
            TRANSLATE => {
                if let Err(e) = self.translate_image(image) {
                    self.fail(&e);
                }
            }
            OCR => {
                let enabled = self.config.history_enabled;
                let limit = self.config.history_limit;
                self.close_editor();
                self.spawn("Распознаём текст офлайн", move || {
                    let text = native::ocr(&image)?;
                    if enabled {
                        history::add(&image, &text, limit)?
                    }
                    Ok(Work::Text {
                        title: "Текст распознан офлайн · скопирован".into(),
                        text,
                        copy: true,
                    })
                });
            }
            QR => {
                let cached = self
                    .session
                    .as_ref()
                    .map(|s| s.qr_values.clone())
                    .unwrap_or_default();
                if !cached.is_empty() {
                    match native::clipboard_text(self.main, &cached.join("\r\n\r\n")) {
                        Ok(()) => {
                            if let Some(s) = &mut self.session {
                                s.qr_copied = true;
                                s.status = luma_shot::i18n::tr(
                                    "QR скопирован · редактор остаётся открытым",
                                )
                                .into()
                            }
                        }
                        Err(e) => self.fail(&e),
                    }
                    self.invalidate();
                    return;
                }
                let revision = self.session.as_ref().map(|s| s.qr_revision).unwrap_or(0);
                self.spawn("Reading QR locally", move || {
                    let values = image.qr();
                    if values.is_empty() {
                        return Err("No QR code found in this area".into());
                    }
                    Ok(Work::Qr {
                        revision,
                        values,
                        copy: true,
                    })
                });
            }
            UPLOAD => {
                let service = self.config.upload_host.clone();
                let client = self.config.imgur_client_id.clone();
                self.spawn("Загружаем изображение", move || {
                    Ok(Work::Uploaded(native::upload(&image, &service, &client)?))
                });
            }
            PIN => {
                self.close_editor();
                self.pin(image);
            }
            PRINT => {
                self.close_editor();
                if let Err(e) = native::print_image(self.main, &image) {
                    self.fail(&e)
                }
            }
            HEX | RGB => {
                if let Some(s) = &self.session {
                    let p = s.view.point(s.cursor.0, s.cursor.1);
                    let c = s.editor.image.pixel(p.x.max(0.) as u32, p.y.max(0.) as u32);
                    let t = if id == HEX {
                        format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
                    } else {
                        format!("rgb({}, {}, {})", c[0], c[1], c[2])
                    };
                    let _ = native::clipboard_text(self.main, &t);
                    self.notify("Цвет скопирован", &t);
                }
            }
            SCROLL => {
                let s = self.session.as_ref().unwrap();
                if !s.editor.annotations.is_empty() {
                    self.fail("Сначала сделайте длинный снимок, затем добавляйте пометки");
                    return;
                }
                let r = s.editor.selection().unwrap();
                if s.view.scale != 1. {
                    self.fail("Для прокрутки начните новый снимок экрана");
                    return;
                }
                let region = Rect {
                    x: s.screen.x + r.x,
                    y: s.screen.y + r.y,
                    w: r.w,
                    h: r.h,
                };
                let screen = s.screen;
                self.close_editor();
                self.spawn(
                    "Снимок с прокруткой · Esc — остановить",
                    move || {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                        let mut image = native::capture(region)?;
                        let mut previous = image.clone();
                        unsafe {
                            let mut cursor: POINT = zeroed();
                            GetCursorPos(&mut cursor);
                            SetCursorPos(
                                region.x + region.w as i32 / 2,
                                region.y + region.h as i32 / 2,
                            );
                            for _ in 0..24 {
                                if GetAsyncKeyState(VK_ESCAPE as i32) < 0 {
                                    break;
                                }
                                mouse_event(MOUSEEVENTF_WHEEL, 0, 0, -360i32, 0);
                                std::thread::sleep(std::time::Duration::from_millis(240));
                                let next = native::capture(region)?;
                                let Some(dy) = luma_shot::stitch::offset(&previous, &next) else {
                                    break;
                                };
                                image = luma_shot::stitch::append(&image, &next, dy)?;
                                previous = next;
                            }
                            SetCursorPos(cursor.x, cursor.y);
                        }
                        Ok(Work::Scroll(image, screen))
                    },
                );
            }
            _ => {}
        }
    }
    fn menu(&mut self, editing: bool) {
        unsafe {
            let m = CreatePopupMenu();
            let mut items = if editing {
                vec![
                    (RECORD, "Начать запись MP4/GIF"),
                    (
                        UPLOAD,
                        "Загрузить на выбранный фотохостинг · ссылка публичная",
                    ),
                    (FOLDER, "Открыть папку снимков"),
                    (REVEAL, "Показать последний сохранённый файл"),
                    (SAVE_AS, "Сохранить как…"),
                    (BLUR_TOOL, "Размытие · B · не для секретов"),
                    (PIXELATE_TOOL, "Пикселизация · X · не для секретов"),
                    (PIN, "Закрепить поверх окон"),
                    (QR, "Прочитать QR локально"),
                    (
                        TRANSLATE,
                        "Перевести картинку · Ctrl+Shift+T · вставить в Google",
                    ),
                    (SCROLL, "Длинный снимок — экспериментально"),
                    (PRINT, "Печать…"),
                    (HEX, "Копировать HEX под курсором"),
                    (RGB, "Копировать RGB под курсором"),
                    (HISTORY, "История снимков…"),
                    (SETTINGS, "Настройки…"),
                    (ABOUT, "Справка и клавиши"),
                    (LICENSES, "Лицензии компонентов"),
                ]
            } else {
                vec![
                    (
                        RECORD,
                        if self.recording.is_some() {
                            "Остановить запись"
                        } else {
                            "Начать запись MP4/GIF"
                        },
                    ),
                    (FOLDER, "Открыть папку снимков"),
                    (REVEAL, "Показать последний сохранённый файл"),
                    (CAPTURE, "Новый снимок · PrtSc"),
                    (HISTORY, "История снимков…"),
                    (SETTINGS, "Настройки…"),
                    (ABOUT, "Справка и клавиши"),
                    (LICENSES, "Лицензии компонентов"),
                    (EXIT, "Выход из PrtSc"),
                ]
            };
            items.push((
                COMPAT,
                "Дополнительный захват (PrintWindow) · не гарантирует Telegram",
            ));
            for (id, label) in items {
                AppendMenuW(
                    m,
                    MF_STRING
                        | if id == RECORD
                            && self.recording.is_none()
                            && self
                                .session
                                .as_ref()
                                .and_then(|s| s.editor.selection())
                                .is_none()
                        {
                            MF_GRAYED
                        } else {
                            0
                        }
                        | if id == COMPAT && self.config.window_capture {
                            MF_CHECKED
                        } else {
                            0
                        },
                    id as usize,
                    wide(luma_shot::i18n::tr(label)).as_ptr(),
                );
            }
            let mut p: POINT = zeroed();
            GetCursorPos(&mut p);
            let owner = if editing { self.overlay } else { self.main };
            SetForegroundWindow(owner);
            let id = TrackPopupMenu(
                m,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                p.x,
                p.y,
                0,
                owner,
                null(),
            );
            DestroyMenu(m);
            if id != 0 {
                self.command(id as u32)
            }
            PostMessageW(owner, WM_NULL, 0, 0);
        }
    }
    fn pin(&mut self, image: Image) {
        unsafe {
            let screen = native::screen_rect();
            let scale = (screen.w as f64 * 0.7 / image.width as f64)
                .min(screen.h as f64 * 0.7 / image.height as f64)
                .min(1.);
            let w = (image.width as f64 * scale) as i32;
            let h = (image.height as f64 * scale) as i32;
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                wide("LumaPin").as_ptr(),
                wide("PrtSc · закреплено · двойной щелчок — закрыть").as_ptr(),
                WS_POPUP | WS_BORDER,
                screen.x + 60,
                screen.y + 60,
                w,
                h,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                self as *mut _ as _,
            );
            if !hwnd.is_null() {
                self.pins.insert(hwnd as isize, Pin { image, drag: None });
                ShowWindow(hwnd, SW_SHOW);
            }
        }
    }
}
unsafe extern "system" fn main_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        attach(hwnd, l)
    }
    if let Some(a) = app(hwnd) {
        if msg == WM_HOTKEY {
            if let Some(settings) = &a.settings {
                let field = settings.fields[14];
                if GetFocus() == field {
                    if let Ok(text) = luma_shot::hotkey::format_event(
                        (l as u32) & 0xffff,
                        ((l as u32) >> 16) & 0xffff,
                    ) {
                        SetWindowTextW(field, wide(&text).as_ptr());
                        SendMessageW(field, EM_SETSEL, 0, -1);
                    }
                    return 0;
                }
            }
        }
        match msg {
            WM_COMMAND => {
                a.command(w as u16 as u32);
                return 0;
            }
            WM_TIMER if w == 3 => {
                KillTimer(hwnd, 3);
                if let Err(e) = a.start(false) {
                    a.fail(&e)
                }
                return 0;
            }
            WM_TIMER if w == 5 => {
                if let Some(r) = &a.recording {
                    let text = format!(
                        "REC {:02}:{:02} · Ctrl+Shift+R",
                        r.started.elapsed().as_secs() / 60,
                        r.started.elapsed().as_secs() % 60
                    );
                    SetWindowTextW(GetDlgItem(r.hwnd, 901), wide(&text).as_ptr());
                }
                return 0;
            }
            WM_CLOSE => {
                a.command(EXIT);
                return 0;
            }
            WM_HOTKEY if w == 2 => {
                if a.recording.is_some() {
                    a.stop_recording()
                } else if a.session.is_some() {
                    a.command(RECORD)
                } else if let Err(e) = a.start(false) {
                    a.fail(&e)
                }
                return 0;
            }
            WM_HOTKEY if w == a.hotkey_id as usize => {
                if let Err(e) = a.start(false) {
                    a.fail(&e)
                }
                return 0;
            }
            BEGIN => {
                if let Err(e) = a.start(false) {
                    a.fail(&e)
                }
                return 0;
            }
            TRAY => {
                if l as u32 & 0xffff == NIN_BALLOONUSERCLICK {
                    a.open_notice();
                } else if l as u32 == WM_RBUTTONUP {
                    a.menu(false)
                } else if l as u32 == WM_LBUTTONDBLCLK {
                    if let Err(e) = a.start(false) {
                        a.fail(&e)
                    }
                }
                return 0;
            }
            DONE => {
                a.done();
                return 0;
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, w, l)
}
fn coords(l: LPARAM) -> (i32, i32) {
    ((l as u16 as i16) as i32, ((l >> 16) as u16 as i16) as i32)
}
unsafe fn ctrl() -> bool {
    GetKeyState(VK_CONTROL as i32) < 0
}
unsafe fn shift() -> bool {
    GetKeyState(VK_SHIFT as i32) < 0
}
unsafe fn alt() -> bool {
    GetKeyState(VK_MENU as i32) < 0
}
unsafe extern "system" fn overlay_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        attach(hwnd, l)
    }
    let Some(a) = app(hwnd) else {
        return DefWindowProcW(hwnd, msg, w, l);
    };
    match msg {
        WM_TIMER if w == 2 => {
            KillTimer(hwnd, 2);
            if a.qr_running && a.config.auto_qr {
                SetTimer(hwnd, 2, 500, None);
                return 0;
            }
            if !a.qr_running
                && a.config.auto_qr
                && a.session
                    .as_ref()
                    .map(|s| {
                        matches!(s.drag, Drag::None) && !s.typing && s.editor.selection().is_some()
                    })
                    .unwrap_or(false)
            {
                if let Ok(image) = a.final_image() {
                    let revision = a.session.as_ref().unwrap().qr_revision;
                    a.qr_running = true;
                    let tx = a.tx.clone();
                    let main = a.main as usize;
                    std::thread::spawn(move || {
                        let values = image.qr();
                        let _ = tx.send(Job {
                            foreground: false,
                            result: Ok(Work::Qr {
                                revision,
                                values,
                                copy: false,
                            }),
                        });
                        unsafe {
                            PostMessageW(main as HWND, DONE, 0, 0);
                        }
                    });
                }
            }
            return 0;
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            // Losing capture must never copy or close the editor.
            if let Some(s) = &mut a.session {
                if !matches!(s.drag, Drag::None) {
                    match std::mem::replace(&mut s.drag, Drag::None) {
                        Drag::Move { index, before, .. } | Drag::Resize { index, before, .. } => {
                            s.editor.annotations[index] = before;
                        }
                        Drag::RegionResize { before, .. } => s.editor.selection = Some(before),
                        _ => {}
                    }
                    s.editor.draft = None;
                }
            }
            a.invalidate();
            return 0;
        }
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            if let Some(s) = &mut a.session {
                let dark = dark_theme(&a.config);
                if let Err(e) = paint::paint(
                    s,
                    dc,
                    ps.rcPaint,
                    dark,
                    &a.config.palette,
                    ui::ToolbarStyle::parse(&a.config.toolbar),
                ) {
                    s.status = e;
                }
                if !s.painted {
                    s.painted = true;
                    let ms = s.started.elapsed().as_secs_f64() * 1000.;
                    let _ = ms;
                    s.status = "Enter — копировать · Esc — отмена · F1 — ещё".into();
                }
            }
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_LBUTTONDOWN => {
            KillTimer(hwnd, 2);
            let (x, y) = coords(l);
            let qr_hit = a.session.as_ref().and_then(|s| {
                let text = s.qr_values.join("\r\n\r\n");
                if text.is_empty() {
                    return None;
                }
                let rect = ui::qr_result_rect(
                    s.layout.as_ref()?,
                    s.view.rect(s.editor.selection()?),
                    s.view.width,
                    s.view.height,
                )?;
                let url = luma_shot::links::http_url(&text);
                ui::qr_result_hit(rect, x, y, url.is_some()).map(|action| (action, url))
            });
            if let Some((action, url)) = qr_hit {
                match action {
                    ui::QrResultAction::Open => {
                        if let Some(url) = url {
                            match native::open_browser(&url) {
                                Ok(()) => a.close_editor(),
                                Err(e) => a.fail(&e),
                            }
                        }
                    }
                    ui::QrResultAction::Copy => a.command(QR),
                }
                return 0;
            }
            let action = a
                .session
                .as_ref()
                .and_then(|s| s.layout.as_ref()?.hit(x, y));
            if let Some(action) = action {
                a.action(action);
                return 0;
            }
            if let Some(session) = &mut a.session {
                session.qr_revision = qr_revision();
                session.qr_values.clear();
            }
            a.commit_text();
            if alt() {
                let snaps = collect_windows();
                if let Some(s) = &mut a.session {
                    s.snaps = snaps;
                }
            }
            if let Some(s) = &mut a.session {
                s.cursor = (x, y);
                let mut p = s.view.point(x, y);
                p.x = p.x.clamp(0., s.editor.image.width as f32);
                p.y = p.y.clamp(0., s.editor.image.height as f32);
                if let Some(sel) = s.editor.selection() {
                    if !alt() {
                        if let Some(edge) =
                            luma_shot::editor::selection_edge(sel, p, 5. / s.view.scale)
                        {
                            s.drag = Drag::RegionResize { before: sel, edge };
                            s.editor.selected = None;
                            SetCapture(hwnd);
                            a.invalidate();
                            return 0;
                        }
                    }
                }
                match if s
                    .editor
                    .selection()
                    .map(|r| !r.contains(p.x as i32, p.y as i32))
                    .unwrap_or(true)
                {
                    Tool::Select
                } else {
                    s.editor.tool
                } {
                    Tool::Select => {
                        if alt() {
                            let sx = s.screen.x + p.x as i32;
                            let sy = s.screen.y + p.y as i32;
                            if let Some(r) = s
                                .snaps
                                .iter()
                                .filter(|r| r.contains(sx, sy))
                                .min_by_key(|r| r.w as u64 * r.h as u64)
                            {
                                s.editor.selection = Some(
                                    Rect {
                                        x: r.x - s.screen.x,
                                        y: r.y - s.screen.y,
                                        w: r.w,
                                        h: r.h,
                                    }
                                    .clamp(s.editor.image.width, s.editor.image.height),
                                );
                            }
                        } else {
                            s.editor.selection = None;
                            s.layout = None;
                            s.drag = Drag::Selection(p);
                        }
                    }
                    Tool::Move => {
                        s.editor.pick(p);
                        if let Some(index) = s.editor.selected {
                            let before = s.editor.annotations[index].clone();
                            let r = before.rect();
                            let corners = [
                                (r.x, r.y),
                                (r.x + r.w as i32, r.y),
                                (r.x, r.y + r.h as i32),
                                (r.x + r.w as i32, r.y + r.h as i32),
                            ];
                            if let Some(corner) = corners.iter().position(|&(x, y)| {
                                (p.x - x as f32).abs() < 12. && (p.y - y as f32).abs() < 12.
                            }) {
                                s.drag = Drag::Resize {
                                    index,
                                    before,
                                    corner,
                                    start: p,
                                };
                            } else {
                                s.drag = Drag::Move {
                                    index,
                                    before,
                                    start: p,
                                };
                            }
                        }
                    }
                    Tool::Picker => {
                        let c = s.editor.image.pixel(p.x as u32, p.y as u32);
                        let text = format!(
                            "#{:02X}{:02X}{:02X}  rgb({}, {}, {})",
                            c[0], c[1], c[2], c[0], c[1], c[2]
                        );
                        s.editor.color = c;
                        let _ = native::clipboard_text(a.main, &text);
                        s.status = format!("Copied {text}");
                    }
                    t => {
                        if s.editor.selection().is_none() {
                            s.status = "Сначала выделите область [V]".into()
                        } else {
                            let mut shape = Annotation::new(t, p, s.editor.color, s.editor.width);
                            if t == Tool::Step {
                                shape.number = s.editor.next_number();
                                s.editor.add(shape);
                                s.drag = Drag::Draw;
                            } else {
                                s.editor.draft = Some(shape);
                                if t == Tool::Text {
                                    s.typing = true;
                                    s.status =
                                        "Введите текст · Enter — готово · Shift+Enter — новая строка".into()
                                } else {
                                    s.drag = Drag::Draw;
                                }
                            }
                        }
                    }
                }
                if !matches!(s.drag, Drag::None) {
                    SetCapture(hwnd);
                }
            }
            a.invalidate();
            return 0;
        }
        WM_MOUSEMOVE => {
            let (x, y) = coords(l);
            let damage_before = a.session.as_ref().and_then(Session::live_damage);
            let hover_only = a
                .session
                .as_ref()
                .map(|s| matches!(s.drag, Drag::None) && !alt())
                .unwrap_or(false);
            if let Some(s) = &mut a.session {
                let old = s.cursor;
                let hover = |xy: (i32, i32)| s.layout.as_ref().and_then(|l| l.hit(xy.0, xy.1));
                let unchanged =
                    matches!(s.drag, Drag::None) && !alt() && hover(old) == hover((x, y));
                s.cursor = (x, y);
                let cursor_id = if s.qr_values.len() > 0
                    && s.layout
                        .as_ref()
                        .and_then(|l| {
                            s.editor.selection().and_then(|sel| {
                                ui::qr_result_rect(l, s.view.rect(sel), s.view.width, s.view.height)
                            })
                        })
                        .map(|r| r.contains(x, y))
                        .unwrap_or(false)
                {
                    IDC_HAND
                } else if s.layout.as_ref().map(|l| l.contains(x, y)).unwrap_or(false) {
                    IDC_ARROW
                } else {
                    match s.editor.selection().and_then(|r| {
                        luma_shot::editor::selection_edge(r, s.view.point(x, y), 5. / s.view.scale)
                    }) {
                        Some(1 | 2) => IDC_SIZEWE,
                        Some(4 | 8) => IDC_SIZENS,
                        Some(5 | 10) => IDC_SIZENWSE,
                        Some(_) => IDC_SIZENESW,
                        None => IDC_CROSS,
                    }
                };
                SetCursor(LoadCursorW(null_mut(), cursor_id));
                if unchanged {
                    return 0;
                }
                let mut p = s.view.point(x, y);
                p.x = p.x.clamp(0., s.editor.image.width as f32);
                p.y = p.y.clamp(0., s.editor.image.height as f32);
                match s.drag.clone() {
                    Drag::RegionResize { before, edge } => {
                        s.editor.selection = Some(luma_shot::editor::resize_selection(
                            before,
                            edge,
                            p,
                            s.editor.image.width,
                            s.editor.image.height,
                        ));
                    }
                    Drag::Selection(start) => {
                        if shift() {
                            let d = (p.x - start.x).abs().max((p.y - start.y).abs());
                            p.x = start.x + (p.x - start.x).signum() * d;
                            p.y = start.y + (p.y - start.y).signum() * d;
                        }
                        s.editor.selection = Some(
                            Rect::from_points(
                                (start.x as i32, start.y as i32),
                                (p.x as i32, p.y as i32),
                            )
                            .clamp(s.editor.image.width, s.editor.image.height),
                        );
                    }
                    Drag::Draw => {
                        if let Some(d) = &mut s.editor.draft {
                            if shift() {
                                let start = d.points[0];
                                if [Tool::Rectangle, Tool::Ellipse].contains(&d.tool) {
                                    let size = (p.x - start.x).abs().max((p.y - start.y).abs());
                                    p = Point::new(
                                        start.x + (p.x - start.x).signum() * size,
                                        start.y + (p.y - start.y).signum() * size,
                                    )
                                } else if (p.x - start.x).abs() > (p.y - start.y).abs() {
                                    p.y = start.y
                                } else {
                                    p.x = start.x
                                }
                            }
                            if d.tool == Tool::Pen {
                                let last = d.points.last().unwrap();
                                if (last.x - p.x).abs() + (last.y - p.y).abs() > 1. {
                                    d.points.push(p)
                                }
                            } else if let Some(last) = d.points.last_mut() {
                                *last = p
                            }
                        }
                    }
                    Drag::Move {
                        index,
                        before,
                        start,
                    } => {
                        let mut next = before;
                        next.translate(p.x - start.x, p.y - start.y);
                        s.editor.annotations[index] = next;
                    }
                    Drag::Resize {
                        index,
                        before,
                        corner,
                        start,
                    } => {
                        let r = before.rect();
                        let dx = p.x - start.x;
                        let dy = p.y - start.y;
                        let left = if corner % 2 == 0 {
                            r.x as f32 + dx
                        } else {
                            r.x as f32
                        };
                        let top = if corner < 2 {
                            r.y as f32 + dy
                        } else {
                            r.y as f32
                        };
                        let width = (r.w as f32 + if corner % 2 == 0 { -dx } else { dx }).max(1.);
                        let height = (r.h as f32 + if corner < 2 { -dy } else { dy }).max(1.);
                        let mut next = before.clone();
                        for q in &mut next.points {
                            q.x = left + (q.x - r.x as f32) * width / r.w.max(1) as f32;
                            q.y = top + (q.y - r.y as f32) * height / r.h.max(1) as f32;
                        }
                        if next.tool == Tool::Text {
                            next.width = (before.width * height / r.h.max(1) as f32).clamp(1., 48.)
                        }
                        s.editor.annotations[index] = next;
                    }
                    _ => {}
                }
            }
            if let (Some(before), Some(after)) = (
                damage_before,
                a.session.as_ref().and_then(Session::live_damage),
            ) {
                a.invalidate_region(luma_shot::damage::union(before, after));
            } else if hover_only {
                if let Some(s) = &a.session {
                    if let Some(l) = &s.layout {
                        let mut r = Rect::default();
                        for surface in &l.surfaces {
                            r = luma_shot::damage::union(
                                r,
                                luma_shot::damage::expand(*surface, 184),
                            );
                        }
                        a.invalidate_region(r.clamp(s.view.width, s.view.height));
                    } else {
                        a.invalidate();
                    }
                }
            } else {
                a.invalidate();
            }
            return 0;
        }
        WM_LBUTTONUP => {
            if a.finish_gesture() {
                a.schedule_qr();
            }
            a.invalidate();
            return 0;
        }
        WM_LBUTTONDBLCLK => {
            if let Some(s) = &mut a.session {
                if s.editor.tool == Tool::Move {
                    let (x, y) = coords(l);
                    s.editor.pick(s.view.point(x, y));
                    if let Some(i) = s.editor.selected {
                        let before = s.editor.annotations[i].clone();
                        if before.tool == Tool::Text {
                            s.editor.draft = Some(before.clone());
                            s.text_replace = Some((i, before));
                            s.editor.annotations[i].text.clear();
                            s.typing = true;
                            a.invalidate();
                            return 0;
                        }
                    }
                }
            }
            return overlay_proc(hwnd, WM_LBUTTONDOWN, w, l);
        }
        WM_RBUTTONUP => {
            let (x, y) = coords(l);
            if let Some(Action::PickColor) = a
                .session
                .as_ref()
                .and_then(|s| s.layout.as_ref()?.hit(x, y))
            {
                a.action(Action::PickColor);
                return 0;
            }
            a.menu(true);
            return 0;
        }
        WM_MOUSEWHEEL => {
            let delta = ((w >> 16) as u16 as i16) as i32;
            a.stroke(if delta > 0 { 1. } else { -1. });
            a.invalidate();
            return 0;
        }
        WM_CHAR => {
            if ctrl() {
                return 0;
            }
            if let Some(s) = &mut a.session {
                if s.typing {
                    if let Some(d) = &mut s.editor.draft {
                        match w as u32 {
                            8 => {
                                s.pending_surrogate = 0;
                                d.text.pop();
                            }
                            13 => {
                                if shift() {
                                    d.text.push('\n')
                                } else {
                                    a.commit_text()
                                }
                            }
                            27 => {}
                            v if v >= 32 => {
                                if let Some(c) = luma_shot::unicode::decode_input(
                                    &mut s.pending_surrogate,
                                    v as u16,
                                ) {
                                    d.text.push(c);
                                }
                            }
                            _ => {}
                        }
                        a.invalidate();
                        return 0;
                    }
                }
            }
            return 0;
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let key = w as u32;
            if key == VK_ESCAPE as u32 {
                if a.session.as_ref().map(|s| s.typing).unwrap_or(false) {
                    a.commit_text();
                    a.invalidate();
                } else {
                    a.close_editor();
                }
                return 0;
            }
            if a.session.as_ref().map(|s| s.typing).unwrap_or(false)
                && !(ctrl() && [67, 83, 71, 84].contains(&key))
            {
                if key == VK_RETURN as u32 && !shift() {
                    a.commit_text();
                    a.invalidate();
                }
                return 0;
            }
            if ctrl() {
                match key {
                    67 => a.command(COPY),
                    83 => a.command(if shift() { SAVE_AS } else { SAVE }),
                    71 => a.command(LENS),
                    81 => a.command(QR),
                    82 if shift() => a.command(RECORD),

                    84 => a.command(if shift() { TRANSLATE } else { OCR }),
                    90 => a.action(if shift() { Action::Redo } else { Action::Undo }),
                    89 => a.action(Action::Redo),
                    65 => {
                        if let Some(s) = &mut a.session {
                            s.editor.selection = Some(s.editor.bounds())
                        }
                    }
                    _ => {}
                }
            } else {
                match key {
                    13 => a.command(COPY),
                    112 => a.menu(true),
                    46 => {
                        if let Some(s) = &mut a.session {
                            s.editor.delete_selected()
                        }
                    }
                    187 | 107 => a.stroke(1.),
                    189 | 109 => a.stroke(-1.),
                    37..=40 => {
                        if let Some(s) = &mut a.session {
                            let amount = if shift() { 10. } else { 1. };
                            if let Some(i) = s.editor.selected {
                                let before = s.editor.annotations[i].clone();
                                let mut after = before.clone();
                                after.translate(
                                    if key == 37 {
                                        -amount
                                    } else if key == 39 {
                                        amount
                                    } else {
                                        0.
                                    },
                                    if key == 38 {
                                        -amount
                                    } else if key == 40 {
                                        amount
                                    } else {
                                        0.
                                    },
                                );
                                s.editor.replace(i, before, after);
                            }
                        }
                    }
                    _ => {
                        if let Some(t) = luma_shot::editor::tool_for_key(key as u8) {
                            a.action(Action::Tool(t))
                        }
                    }
                }
            }
            if key != 81 && key != 112 {
                a.schedule_qr();
            }
            a.invalidate();
            return 0;
        }
        WM_CLOSE => {
            a.close_editor();
            return 0;
        }
        WM_DISPLAYCHANGE => {
            a.close_editor();
            a.notify(
                "Экран изменился",
                "Сделайте новый снимок после изменения экрана",
            );
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, w, l)
}
unsafe fn collect_windows() -> Vec<Rect> {
    unsafe extern "system" fn child(hwnd: HWND, l: LPARAM) -> BOOL {
        let v = &mut *(l as *mut Vec<Rect>);
        if v.len() > 4000 {
            return 0;
        }
        let mut r: RECT = zeroed();
        if IsWindowVisible(hwnd) != 0
            && GetWindowRect(hwnd, &mut r) != 0
            && r.right > r.left + 5
            && r.bottom > r.top + 5
        {
            v.push(Rect {
                x: r.left,
                y: r.top,
                w: (r.right - r.left) as u32,
                h: (r.bottom - r.top) as u32,
            });
        }
        1
    }
    unsafe extern "system" fn top(hwnd: HWND, l: LPARAM) -> BOOL {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == GetCurrentProcessId() {
            return 1;
        }
        child(hwnd, l);
        if IsWindowVisible(hwnd) != 0 {
            EnumChildWindows(hwnd, Some(child), l);
        }
        1
    }
    let mut v = vec![];
    EnumWindows(Some(top), &mut v as *mut _ as isize);
    v
}
unsafe extern "system" fn pin_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        attach(hwnd, l)
    }
    if let Some(a) = app(hwnd) {
        match msg {
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = zeroed();
                let dc = BeginPaint(hwnd, &mut ps);
                if let Some(pin) = a.pins.get(&(hwnd as isize)) {
                    let mut r: RECT = zeroed();
                    GetClientRect(hwnd, &mut r);
                    native::blit(dc, &pin.image, r.right, r.bottom);
                }
                EndPaint(hwnd, &ps);
                return 0;
            }
            WM_LBUTTONDOWN => {
                if let Some(pin) = a.pins.get_mut(&(hwnd as isize)) {
                    let mut p: POINT = zeroed();
                    let mut r: RECT = zeroed();
                    GetCursorPos(&mut p);
                    GetWindowRect(hwnd, &mut r);
                    pin.drag = Some((p, r));
                    SetCapture(hwnd);
                }
                return 0;
            }
            WM_MOUSEMOVE => {
                if let Some((p, r)) = a.pins.get(&(hwnd as isize)).and_then(|p| p.drag) {
                    let mut now: POINT = zeroed();
                    GetCursorPos(&mut now);
                    SetWindowPos(
                        hwnd,
                        null_mut(),
                        r.left + now.x - p.x,
                        r.top + now.y - p.y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                return 0;
            }
            WM_LBUTTONUP => {
                ReleaseCapture();
                if let Some(p) = a.pins.get_mut(&(hwnd as isize)) {
                    p.drag = None;
                }
                return 0;
            }
            WM_LBUTTONDBLCLK | WM_RBUTTONUP | WM_CLOSE => {
                a.pins.remove(&(hwnd as isize));
                DestroyWindow(hwnd);
                return 0;
            }
            WM_KEYDOWN if w == VK_ESCAPE as usize => {
                a.pins.remove(&(hwnd as isize));
                DestroyWindow(hwnd);
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, w, l)
}

fn dark_theme(cfg: &Config) -> bool {
    if cfg.theme == "light" {
        return false;
    }
    if cfg.theme == "dark" {
        return true;
    }
    use windows_sys::Win32::System::Registry::*;
    unsafe {
        let mut value = 1u32;
        let mut length = 4u32;
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize").as_ptr(),
            wide("AppsUseLightTheme").as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut value as *mut _ as _,
            &mut length,
        );
        value == 0
    }
}

impl App {
    fn start_recording(&mut self) -> Result<(), String> {
        if self.recording.is_some() {
            self.stop_recording();
            return Ok(());
        }
        if self.busy {
            return Err("Wait for the current operation".into());
        }
        let s = self
            .session
            .as_ref()
            .ok_or("Select an area with PrintScreen first")?;
        if s.view.scale != 1. {
            return Err("Start a fresh desktop capture for video".into());
        }
        let r = s
            .editor
            .selection()
            .ok_or("Select a recording area first")?;
        let region = Rect {
            x: s.screen.x + r.x,
            y: s.screen.y + r.y,
            w: r.w,
            h: r.h,
        };
        let cfg = self.config.clone();
        std::fs::create_dir_all(&cfg.save_dir).map_err(|e| e.to_string())?;
        let path = PathBuf::from(&cfg.save_dir).join(format!(
            "PrtSc_{}.{}",
            util::unique_id(),
            cfg.video_format
        ));
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.close_editor();
        unsafe {
            let screen = native::screen_rect();
            let top = if region.y > screen.y + 100 {
                screen.y + 12
            } else {
                (screen.y + screen.h as i32 - 112).max(screen.y)
            };
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                wide("LumaRecord").as_ptr(),
                wide("PrtSc · REC").as_ptr(),
                WS_POPUP | WS_BORDER,
                screen.x + 12,
                top,
                420,
                88,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                self as *mut _ as _,
            );
            if hwnd.is_null() {
                return Err(native::last("Recording control"));
            }
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
            child(hwnd, "STATIC", "REC · Ctrl+Shift+R", 0, 12, 8, 272, 24, 901);
            child(
                hwnd,
                "BUTTON",
                if luma_shot::i18n::english() {
                    "Stop"
                } else {
                    "Стоп"
                },
                WS_TABSTOP,
                300,
                8,
                104,
                30,
                1,
            );
            child(
                hwnd,
                "STATIC",
                &format!(
                    "{} · {} × {} · {}",
                    cfg.video_format.to_uppercase(),
                    region.w,
                    region.h,
                    if luma_shot::i18n::english() {
                        "no audio"
                    } else {
                        "без звука"
                    }
                ),
                0,
                12,
                48,
                390,
                24,
                902,
            );
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetTimer(self.main, 5, 1000, None);
            self.recording = Some(Recording {
                stop: stop.clone(),
                hwnd,
                started: Instant::now(),
            });
        }
        let tx = self.tx.clone();
        let main = self.main as usize;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(120));
            let result = video::record(
                region,
                path,
                &cfg.video_format,
                cfg.video_fps,
                cfg.video_bitrate,
                cfg.video_cursor,
                stop,
            );
            let _ = tx.send(Job {
                foreground: false,
                result: Ok(Work::Recorded(result)),
            });
            unsafe {
                PostMessageW(main as HWND, DONE, 0, 0);
            }
        });
        Ok(())
    }
    fn stop_recording(&mut self) {
        if let Some(r) = &self.recording {
            r.stop.store(true, std::sync::atomic::Ordering::Relaxed);
            unsafe {
                EnableWindow(GetDlgItem(r.hwnd, 1), 0);
                SetWindowTextW(
                    GetDlgItem(r.hwnd, 901),
                    wide(if luma_shot::i18n::english() {
                        "Saving…"
                    } else {
                        "Сохраняем…"
                    })
                    .as_ptr(),
                );
            }
        }
    }
}
unsafe extern "system" fn record_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        attach(hwnd, l);
    }
    if let Some(a) = app(hwnd) {
        match msg {
            WM_CLOSE => {
                a.stop_recording();
                return 0;
            }
            WM_COMMAND if w as u16 == 1 => {
                a.stop_recording();
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, w, l)
}

pub fn encoder_test(path: &Path) -> Result<(), String> {
    let format = path.extension().and_then(|s| s.to_str()).unwrap_or("gif");
    if !["gif", "mp4"].contains(&format) {
        return Err("Encoder test output must be .gif or .mp4".into());
    }
    let mut mp4 = if format == "mp4" {
        Some(video::Mp4::new(path, 160, 96, 15, 1_000_000)?)
    } else {
        None
    };
    let mut gif = if format == "gif" {
        Some(luma_shot::video::Gif::new(path, 160, 96, 15)?)
    } else {
        None
    };
    for n in 0..30 {
        let mut image = Image::new(160, 96)?;
        for y in 0..96 {
            for x in 0..160 {
                let i = (y * 160 + x) * 4;
                image.rgba[i..i + 4].copy_from_slice(&[
                    ((x + n * 5) % 255) as u8,
                    (y * 2) as u8,
                    100,
                    255,
                ]);
            }
        }
        if let Some(g) = gif.as_mut() {
            g.frame(&image)?;
        }
        if let Some(m) = mp4.as_mut() {
            m.frame(&image, n as i64 * 10_000_000 / 15, 10_000_000 / 15)?;
        }
    }
    if let Some(g) = gif {
        g.finish()?;
    }
    if let Some(m) = mp4 {
        m.finish()?;
    }
    Ok(())
}
