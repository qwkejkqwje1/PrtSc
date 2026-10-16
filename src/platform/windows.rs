#[path = "win_native.rs"]
mod native;
#[path = "win_paint.rs"]
mod paint;
#[path = "win_video.rs"]
mod video;
use luma_shot::{
    config::Config,
    editor::{Annotation, Editor, Point, Tool, TOOLS},
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
const PIN: u32 = 106;
const PRINT: u32 = 107;
const QR: u32 = 111;
const SCROLL: u32 = 112;
const RGB: u32 = 113;
const HEX: u32 = 114;
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
enum Work {
    Saved(PathBuf),
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
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
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
        let classes: [(&str, WNDPROC, u32); 7] = [
            ("LumaRecord", Some(record_proc), 0),
            ("LumaMain", Some(main_proc), 0),
            ("LumaOverlay", Some(overlay_proc), CS_DBLCLKS),
            ("LumaPin", Some(pin_proc), CS_DBLCLKS),
            ("LumaSettings", Some(settings_proc), 0),
            ("LumaText", Some(text_proc), 0),
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
            history: None,
            icon: small_icon,
            window_icon,
            mutex,
            hotkey: false,
            hotkey_id: 1,
        });
        luma_shot::i18n::configure(&state.config.language, &state.config.toolbar);
        luma_shot::dock::configure(&state.config.toolbar_dock);
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
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            let dialogs = [
                state.settings.as_ref().map(|s| s.hwnd),
                state.text.as_ref().map(|s| s.hwnd),
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
    fn fail(&mut self, e: &str) {
        if let Some(s) = &mut self.session {
            s.status = e.chars().take(110).collect();
            self.invalidate()
        } else {
            self.notify("PrtSc", e)
        }
    }
    fn start(&mut self, demo: bool) -> Result<(), String> {
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
                    Renderer { font: None }
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
    fn final_image(&mut self) -> Result<Image, String> {
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
                Ok(Work::Saved(p)) => {
                    self.close_editor();
                    self.notify_file(&p);
                }
                Ok(Work::Text { title, text, copy }) => {
                    if copy {
                        let _ = native::clipboard_text(self.main, &text);
                    }
                    self.show_text(&title, &text, None)
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
                    "PrintScreen — capture · drag to select · Alt+click — window\r\nEnter / Ctrl+C — copy · Ctrl+S — save · Ctrl+Shift+S — Save as\r\nCtrl+G — Google · Ctrl+T — offline OCR · Ctrl+Q — read QR\r\nCtrl+Z / Ctrl+Shift+Z — undo / redo\r\nDrag an edge to resize · click outside for a new area · M — move annotations · P — pen · L — line\r\nA — arrow · R — rectangle · O — oval · H — highlight\r\nT — text · D — opaque redaction · N — numbered step · I — color picker\r\nClick the single color swatch; each drawing tool remembers its color. Hover icons for labels.\r\nVideo camera / Ctrl+Shift+R — record selected live area. Stop with Ctrl+Shift+R or Stop.\r\nFormat, FPS and output folder are in Settings. GIF: up to 1280px / 15 FPS. No audio.\r\nCursor recording is optional in Settings. Video does NOT include screenshot annotations/redactions.\r\nUpload arrow — selected host; links are public. Google button sends the final selection.\r\nQR is local; links are never opened automatically. OCR uses installed Windows OCR languages.\r\nOpaque redaction removes covered pixels in exports; undo and the original remain in the editor.\r\nExperimental PrintWindow is cooperative capture, NOT a guaranteed Telegram protection bypass.\r\nPerformance targets need validation on real Windows hardware."
                } else {
                    "PrintScreen — снимок · мышь — выделение · Alt+щелчок — окно\r\nEnter / Ctrl+C — копировать · Ctrl+S — сохранить · Ctrl+Shift+S — сохранить как\r\nCtrl+G — Google · Ctrl+T — OCR офлайн · Ctrl+Q — прочитать QR\r\nCtrl+Z / Ctrl+Shift+Z — отменить / повторить\r\nКрай рамки — размер · вне рамки — новая область · M — перемещение пометок · P — карандаш · L — линия\r\nA — стрелка · R — прямоугольник · O — овал · H — маркер\r\nT — текст · D — скрыть данные · N — номер шага · I — пипетка\r\nНажмите на цвет: он сохраняется отдельно для каждого инструмента. Наведите на значок для пояснения.\r\nКинокамера / Ctrl+Shift+R — запись живой области. Остановка: Ctrl+Shift+R или Стоп.\r\nФормат, частота и папка — в настройках. GIF: до 1280 пикселей / 15 FPS. Без звука.\r\nКурсор в записи отключается в настройках. Видеозапись НЕ включает пометки и закрашивание из редактора снимка.\r\nСтрелка загрузки — выбранный фотохостинг; ссылки публичные. Google отправляет итоговое выделение.\r\nQR читается локально, без открытия ссылок. OCR использует языки Windows OCR.\r\nНепрозрачная заливка удаляет закрытые пиксели при экспорте; исходник и отмена остаются в редакторе.\r\nЭкспериментальный PrintWindow — не гарантированный обход защиты Telegram.\r\nЦелевые задержки и память требуют проверки на реальной Windows."
                };
                self.show_text(luma_shot::i18n::tr("PrtSc 0.8 · Справка"), help, None);
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
                            let metrics = serde_json::json!({"capture_backend":s.backend,"capture_ms":if s.backend=="BitBlt"{serde_json::json!(s.capture_ms)}else{serde_json::Value::Null},"clipboard_write_ms":ms,"copy_request_to_clipboard_ms":action_started.elapsed().as_secs_f64()*1000.,"note":"clipboard_write_ms excludes flatten/crop; copy_request_to_clipboard_ms includes them. capture_ms is null for synthetic demo/scroll. First CPU paint is not GPU presentation","width":image.width,"height":image.height,"scene_rebuilds":s.scene_builds,"paints":s.paint_count});
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
                    (PIN, "Закрепить поверх окон"),
                    (QR, "Прочитать QR локально"),
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
                if let Err(e) = paint::paint(s, dc, ps.rcPaint, dark, &a.config.palette) {
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
            a.invalidate();
            return 0;
        }
        WM_LBUTTONUP => {
            let mut changed = false;
            if let Some(s) = &mut a.session {
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
                        let after = s.editor.annotations[index].clone();
                        s.editor.replace(index, before, after);
                    }
                    _ => {}
                }
            }
            ReleaseCapture();
            if changed {
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
                a.close_editor();
                return 0;
            }
            if a.session.as_ref().map(|s| s.typing).unwrap_or(false) {
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

                    84 => a.command(OCR),
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
                        if let Some(t) = TOOLS
                            .iter()
                            .find(|t| t.key().as_bytes().first().copied() == Some(key as u8))
                        {
                            a.action(Action::Tool(*t))
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
static CONTROL_FONT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

unsafe fn child(
    parent: HWND,
    class: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: usize,
) -> HWND {
    let c = CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(if class == "EDIT" {
            text
        } else {
            luma_shot::i18n::tr(text)
        })
        .as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        x,
        y,
        w,
        h,
        parent,
        id as HMENU,
        GetModuleHandleW(null()),
        null(),
    );
    SendMessageW(
        c,
        WM_SETFONT,
        *CONTROL_FONT.get_or_init(|| {
            CreateFontW(
                -15,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                wide("Segoe UI").as_ptr(),
            ) as usize
        }),
        1,
    );
    c
}
unsafe fn value(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut b = vec![0u16; len + 1];
    GetWindowTextW(hwnd, b.as_mut_ptr(), b.len() as i32);
    String::from_utf16_lossy(&b[..len])
}
unsafe fn setting_edit(
    hwnd: HWND,
    label: &str,
    x: i32,
    y: i32,
    w: i32,
    id: usize,
    text: &str,
) -> HWND {
    child(hwnd, "STATIC", label, 0, x, y, w, 22, 0);
    child(
        hwnd,
        "EDIT",
        text,
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        x,
        y + 26,
        w,
        28,
        id,
    )
}
unsafe fn setting_combo(
    hwnd: HWND,
    label: &str,
    x: i32,
    y: i32,
    w: i32,
    id: usize,
    items: &[&str],
    index: usize,
) -> HWND {
    child(hwnd, "STATIC", label, 0, x, y, w, 22, 0);
    let h = child(
        hwnd,
        "COMBOBOX",
        "",
        WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
        x,
        y + 26,
        w,
        200,
        id,
    );
    for text in items {
        SendMessageW(
            h,
            CB_ADDSTRING,
            0,
            wide(luma_shot::i18n::tr(text)).as_ptr() as isize,
        );
    }
    SendMessageW(h, CB_SETCURSEL, index, 0);
    h
}
unsafe fn setting_check(
    hwnd: HWND,
    label: &str,
    x: i32,
    y: i32,
    w: i32,
    id: usize,
    checked: bool,
) -> HWND {
    let h = child(
        hwnd,
        "BUTTON",
        label,
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        x,
        y,
        w,
        28,
        id,
    );
    SendMessageW(
        h,
        BM_SETCHECK,
        if checked { BST_CHECKED } else { BST_UNCHECKED } as usize,
        0,
    );
    h
}
impl App {
    fn show_settings(&mut self) {
        unsafe {
            if let Some(s) = &self.settings {
                SetForegroundWindow(s.hwnd);
                return;
            }
            if !self.overlay.is_null() {
                ShowWindow(self.overlay, SW_HIDE);
            }
            let hwnd = CreateWindowExW(
                WS_EX_APPWINDOW,
                wide("LumaSettings").as_ptr(),
                wide(luma_shot::i18n::tr("PrtSc · Настройки")).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                820,
                800,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                self as *mut _ as _,
            );
            let c = &self.config;
            let mut fields = vec![];
            child(
                hwnd,
                "STATIC",
                "PrtSc · Settings / Настройки",
                0,
                20,
                16,
                770,
                25,
                0,
            );
            let language = setting_combo(
                hwnd,
                "Язык / Language",
                20,
                54,
                350,
                106,
                &["Русский", "English"],
                if c.language == "en" { 1 } else { 0 },
            );
            let theme = setting_combo(
                hwnd,
                "Тема панели",
                20,
                120,
                350,
                100,
                &["Тёмная", "Светлая", "Как в Windows"],
                match c.theme.as_str() {
                    "light" => 1,
                    "system" => 2,
                    _ => 0,
                },
            );
            let toolbar = setting_combo(
                hwnd,
                "Вид панели",
                20,
                186,
                350,
                107,
                &["Компактная — значки", "По краям — справа и снизу"],
                if ["edge", "lightshot"].contains(&c.toolbar.as_str()) {
                    1
                } else {
                    0
                },
            );
            child(hwnd, "STATIC", "Папка снимков", 0, 20, 252, 350, 22, 0);
            let dir = child(
                hwnd,
                "EDIT",
                &c.save_dir,
                WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                20,
                278,
                258,
                28,
                101,
            );
            child(
                hwnd,
                "BUTTON",
                "Выбрать…",
                WS_TABSTOP,
                286,
                278,
                84,
                28,
                302,
            );
            let format = setting_combo(
                hwnd,
                "Формат сохранения",
                20,
                322,
                350,
                102,
                &["PNG — без потерь", "JPEG — с потерями", "WebP — без потерь"],
                match c.format.as_str() {
                    "jpg" => 1,
                    "webp" => 2,
                    _ => 0,
                },
            );
            child(
                hwnd,
                "STATIC",
                "Качество JPEG (1–100)",
                0,
                20,
                388,
                245,
                22,
                0,
            );
            let quality = child(
                hwnd,
                "EDIT",
                &c.jpeg_quality.to_string(),
                WS_BORDER | WS_TABSTOP | ES_NUMBER as u32,
                286,
                384,
                84,
                28,
                103,
            );
            let compression = setting_combo(
                hwnd,
                "Сжатие PNG",
                20,
                428,
                350,
                104,
                &["Быстрое", "Обычное", "Максимальное (медленнее)"],
                match c.png_compression.as_str() {
                    "default" => 1,
                    "best" => 2,
                    _ => 0,
                },
            );
            child(hwnd, "STATIC", "Лимит истории", 0, 20, 498, 245, 22, 0);
            let history_limit = child(
                hwnd,
                "EDIT",
                &c.history_limit.to_string(),
                WS_BORDER | WS_TABSTOP | ES_NUMBER as u32,
                286,
                494,
                84,
                28,
                105,
            );
            let history = setting_check(
                hwnd,
                "Хранить локальную историю снимков",
                20,
                536,
                350,
                300,
                c.history_enabled,
            );
            let auto_qr = setting_check(
                hwnd,
                "Авто-QR после выделения",
                20,
                572,
                350,
                301,
                c.auto_qr,
            );
            let sound = setting_check(
                hwnd,
                "Звуки уведомлений",
                20,
                608,
                350,
                304,
                c.sound_enabled,
            );
            let video = setting_combo(
                hwnd,
                "Формат видео",
                410,
                54,
                370,
                108,
                &["MP4 / H.264", "GIF (≤1280 px, ≤15 FPS)"],
                if c.video_format == "gif" { 1 } else { 0 },
            );
            let fps = setting_combo(
                hwnd,
                "Частота кадров",
                410,
                120,
                370,
                109,
                &["10 FPS", "15 FPS", "24 FPS", "30 FPS"],
                match c.video_fps {
                    10 => 0,
                    24 => 2,
                    30 => 3,
                    _ => 1,
                },
            );
            child(
                hwnd,
                "STATIC",
                "Битрейт MP4 (Мбит/с)",
                0,
                410,
                192,
                250,
                22,
                0,
            );
            let bitrate = child(
                hwnd,
                "EDIT",
                &(c.video_bitrate / 1_000_000).to_string(),
                WS_BORDER | WS_TABSTOP | ES_NUMBER as u32,
                690,
                188,
                90,
                28,
                110,
            );
            child(
                hwnd,
                "STATIC",
                "Запись выбранной области без аудио.\nСтоп: Ctrl+Shift+R или кнопка Стоп.",
                0,
                410,
                238,
                370,
                52,
                0,
            );
            let host = setting_combo(
                hwnd,
                "Фотохостинг",
                410,
                322,
                370,
                111,
                &["Litterbox — 1h", "Catbox — permanent", "Imgur — Client ID"],
                match c.upload_host.as_str() {
                    "catbox" => 1,
                    "imgur" => 2,
                    _ => 0,
                },
            );
            child(
                hwnd,
                "STATIC",
                "Imgur Client ID (не пароль)",
                0,
                410,
                388,
                370,
                22,
                0,
            );
            let client = child(
                hwnd,
                "EDIT",
                &c.imgur_client_id,
                WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                410,
                414,
                370,
                28,
                112,
            );
            child(hwnd,"STATIC","Загрузка только по кнопке. Ссылки публичные.\nНе отправляйте секреты на фотохостинг.",0,410,464,370,52,0);
            let video_cursor = setting_check(
                hwnd,
                "Показывать курсор в видео",
                410,
                290,
                370,
                307,
                c.video_cursor,
            );
            let hotkey = setting_edit(
                hwnd,
                "Клавиша снимка — нажмите сочетание",
                20,
                646,
                350,
                114,
                &c.capture_hotkey,
            );
            SendMessageW(hotkey, EM_SETREADONLY, 1, 0);
            SetWindowSubclass(hotkey, Some(hotkey_proc), 1, 0);
            let confirm = setting_check(
                hwnd,
                "Подтверждать отправку в Google",
                410,
                600,
                370,
                305,
                c.confirm_google,
            );
            let search_mode = setting_combo(
                hwnd,
                "Передача в Google",
                410,
                536,
                370,
                113,
                &["Быстро — JPEG, до 2048 px", "Оригинал — PNG без потерь"],
                if c.search_mode == "original" { 1 } else { 0 },
            );
            child(
                hwnd,
                "BUTTON",
                "Папка данных",
                WS_TABSTOP,
                20,
                718,
                150,
                32,
                303,
            );
            child(
                hwnd,
                "BUTTON",
                "Сохранить",
                BS_DEFPUSHBUTTON as u32 | WS_TABSTOP,
                540,
                718,
                116,
                32,
                1,
            );
            child(hwnd, "BUTTON", "Отмена", WS_TABSTOP, 664, 718, 116, 32, 2);
            fields.extend([
                theme,
                dir,
                format,
                quality,
                compression,
                history_limit,
                language,
                toolbar,
                video,
                fps,
                bitrate,
                host,
                client,
                search_mode,
                hotkey,
            ]);
            let compat = setting_check(
                hwnd,
                "Захват активного окна (PrintWindow)",
                410,
                632,
                370,
                306,
                c.window_capture,
            );
            self.settings = Some(SettingsState {
                hwnd,
                fields,
                history,
                experimental: auto_qr,
                sound,
                confirm,
                compat,
                video_cursor,
            });
            self.settings_enable();
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        }
    }
    fn settings_enable(&self) {
        unsafe {
            if let Some(s) = &self.settings {
                let f = SendMessageW(s.fields[2], CB_GETCURSEL, 0, 0);
                EnableWindow(s.fields[3], if f == 1 { 1 } else { 0 });
                EnableWindow(s.fields[4], if f == 0 { 1 } else { 0 });
                EnableWindow(
                    s.fields[5],
                    if SendMessageW(s.history, BM_GETCHECK, 0, 0) == BST_CHECKED as isize {
                        1
                    } else {
                        0
                    },
                );
                EnableWindow(
                    s.fields[10],
                    if SendMessageW(s.fields[8], CB_GETCURSEL, 0, 0) == 0 {
                        1
                    } else {
                        0
                    },
                );
                EnableWindow(
                    s.fields[12],
                    if SendMessageW(s.fields[11], CB_GETCURSEL, 0, 0) == 2 {
                        1
                    } else {
                        0
                    },
                );
            }
        }
    }
    fn save_settings(&mut self) -> Result<(), String> {
        let s = self.settings.as_ref().ok_or("Settings closed")?;
        let mut c = self.config.clone();
        unsafe {
            let selected = |i: usize| SendMessageW(s.fields[i], CB_GETCURSEL, 0, 0) as usize;
            c.theme = ["dark", "light", "system"]
                .get(selected(0))
                .ok_or("Invalid theme")?
                .to_string();
            c.save_dir = value(s.fields[1]);
            c.format = ["png", "jpg", "webp"]
                .get(selected(2))
                .ok_or("Invalid image format")?
                .to_string();
            c.jpeg_quality = value(s.fields[3]).parse().unwrap_or(c.jpeg_quality);
            c.png_compression = ["fast", "default", "best"]
                .get(selected(4))
                .ok_or("Invalid compression")?
                .to_string();
            c.history_limit = value(s.fields[5]).parse().unwrap_or(c.history_limit);
            c.language = ["ru", "en"]
                .get(selected(6))
                .ok_or("Invalid language")?
                .to_string();
            c.toolbar = ["compact", "edge"]
                .get(selected(7))
                .ok_or("Invalid toolbar")?
                .to_string();
            c.video_format = ["mp4", "gif"]
                .get(selected(8))
                .ok_or("Invalid video format")?
                .to_string();
            c.video_fps = *[10, 15, 24, 30].get(selected(9)).ok_or("Invalid FPS")?;
            c.video_bitrate = value(s.fields[10])
                .parse::<u32>()
                .map_err(|_| "Invalid bitrate")?
                .checked_mul(1_000_000)
                .ok_or("Invalid bitrate")?;
            c.upload_host = ["litterbox", "catbox", "imgur"]
                .get(selected(11))
                .ok_or("Invalid image host")?
                .to_string();
            c.imgur_client_id = value(s.fields[12]).trim().into();
            c.search_mode = if selected(13) == 1 {
                "original"
            } else {
                "fast"
            }
            .into();
            c.history_enabled = SendMessageW(s.history, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
            c.auto_qr = SendMessageW(s.experimental, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
            c.sound_enabled = SendMessageW(s.sound, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
            c.window_capture = SendMessageW(s.compat, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
            c.capture_hotkey = value(s.fields[14]).trim().to_string();
            c.video_cursor =
                SendMessageW(s.video_cursor, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
            c.toolbar_dock = "auto".into();
            c.confirm_google = SendMessageW(s.confirm, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
        }
        c.validate()?;
        std::fs::create_dir_all(&c.save_dir).map_err(|e| e.to_string())?;
        let changed = luma_shot::hotkey::parse(&c.capture_hotkey)?
            != luma_shot::hotkey::parse(&self.config.capture_hotkey)?
            || !self.hotkey;
        let next_id = if self.hotkey_id == 1 { 3 } else { 1 };
        if changed {
            let hk = luma_shot::hotkey::parse(&c.capture_hotkey)?;
            if unsafe { RegisterHotKey(self.main, next_id, hk.modifiers | MOD_NOREPEAT, hk.key) }
                == 0
            {
                return Err(
                    "Сочетание занято Windows или другой программой. Прежняя клавиша сохранена."
                        .into(),
                );
            }
        }
        if let Err(e) = c.save() {
            if changed {
                unsafe {
                    UnregisterHotKey(self.main, next_id);
                }
            }
            return Err(e);
        }
        if changed {
            unsafe {
                if self.hotkey {
                    UnregisterHotKey(self.main, self.hotkey_id);
                }
            }
            self.hotkey_id = next_id;
            self.hotkey = true;
        }
        self.config = c;
        luma_shot::i18n::configure(&self.config.language, &self.config.toolbar);
        luma_shot::dock::configure(&self.config.toolbar_dock);
        self.schedule_qr();
        self.invalidate();
        Ok(())
    }
    fn close_settings(&mut self) {
        if let Some(s) = self.settings.take() {
            unsafe {
                DestroyWindow(s.hwnd);
                if !self.overlay.is_null() {
                    ShowWindow(self.overlay, SW_SHOW);
                    SetForegroundWindow(self.overlay);
                }
            }
        }
    }
    fn show_text(&mut self, title: &str, text: &str, pending: Option<Image>) {
        unsafe {
            if let Some(old) = self.text.take() {
                DestroyWindow(old.hwnd);
            }
            let ask = false;
            let _ = pending;
            let hwnd = CreateWindowExW(
                WS_EX_APPWINDOW,
                wide("LumaText").as_ptr(),
                wide(luma_shot::i18n::tr(title)).as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                800,
                560,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                self as *mut _ as _,
            );
            let url = luma_shot::links::http_url(text);
            let edit = child(
                hwnd,
                "EDIT",
                text,
                WS_BORDER
                    | WS_VSCROLL
                    | (ES_MULTILINE as u32)
                    | (ES_AUTOVSCROLL as u32)
                    | (ES_WANTRETURN as u32)
                    | WS_TABSTOP
                    | if ask { 0 } else { ES_READONLY as u32 },
                16,
                16,
                750,
                if url.is_some() { 410 } else { 440 },
                100,
            );
            SendMessageW(edit, EM_SETLIMITTEXT, 1200000, 0);
            child(
                hwnd,
                "BUTTON",
                if ask {
                    "Копировать текст"
                } else {
                    "Копировать текст"
                },
                (BS_DEFPUSHBUTTON as u32) | WS_TABSTOP,
                500,
                470,
                144,
                32,
                1,
            );
            child(hwnd, "BUTTON", "Закрыть", WS_TABSTOP, 654, 470, 100, 32, 2);

            if let Some(ref link) = url {
                let init = INITCOMMONCONTROLSEX {
                    dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
                    dwICC: ICC_LINK_CLASS,
                };
                InitCommonControlsEx(&init);
                let escaped = link.replace('&', "&amp;");
                child(
                    hwnd,
                    "SysLink",
                    &format!("<a href=\"{escaped}\">{escaped}</a>"),
                    WS_TABSTOP,
                    16,
                    438,
                    750,
                    24,
                    4,
                );

                child(
                    hwnd,
                    "BUTTON",
                    "Открыть ссылку",
                    WS_TABSTOP,
                    16,
                    470,
                    180,
                    32,
                    3,
                );
            }
            self.text = Some(TextState { hwnd, edit, url });
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, -1);
        }
    }
    fn text_submit(&mut self) {
        if let Some(s) = &self.text {
            let text = unsafe { value(s.edit) };
            if let Err(e) = native::clipboard_text(self.main, &text) {
                self.fail(&e)
            } else {
                unsafe {
                    SetWindowTextW(
                        GetDlgItem(s.hwnd, 1),
                        wide(luma_shot::i18n::tr("Скопировано")).as_ptr(),
                    );
                }
            }
        }
    }
    fn show_history(&mut self) {
        unsafe {
            if let Some(s) = &self.history {
                SetForegroundWindow(s.hwnd);
                return;
            }
            let hwnd = CreateWindowExW(
                WS_EX_APPWINDOW,
                wide("LumaHistory").as_ptr(),
                wide("PrtSc · Local history").as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                820,
                570,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                self as *mut _ as _,
            );
            child(
                hwnd,
                "STATIC",
                "Поиск по локальной истории и распознанному тексту",
                0,
                18,
                16,
                760,
                24,
                0,
            );
            let search = child(
                hwnd,
                "EDIT",
                "",
                WS_BORDER | (ES_AUTOHSCROLL as u32) | WS_TABSTOP,
                18,
                46,
                764,
                30,
                201,
            );
            let list = child(
                hwnd,
                "LISTBOX",
                "",
                WS_BORDER
                    | WS_VSCROLL
                    | (LBS_NOTIFY as u32)
                    | (LBS_NOINTEGRALHEIGHT as u32)
                    | WS_TABSTOP,
                18,
                90,
                764,
                364,
                202,
            );
            child(
                hwnd,
                "BUTTON",
                "Открыть выбранный",
                (BS_DEFPUSHBUTTON as u32) | WS_TABSTOP,
                18,
                474,
                144,
                32,
                1,
            );
            child(
                hwnd,
                "BUTTON",
                "Удалить всю историю…",
                WS_TABSTOP,
                176,
                474,
                180,
                32,
                3,
            );
            child(hwnd, "BUTTON", "Закрыть", WS_TABSTOP, 682, 474, 100, 32, 2);
            self.history = Some(HistoryState {
                hwnd,
                search,
                list,
                filtered: vec![],
            });
            self.filter_history();
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetFocus(search);
        }
    }
    fn filter_history(&mut self) {
        if let Some(s) = &mut self.history {
            unsafe {
                let q = value(s.search);
                let entries = history::load();
                s.filtered = history::search(&entries, &q).into_iter().cloned().collect();
                SendMessageW(s.list, LB_RESETCONTENT, 0, 0);
                for e in &s.filtered {
                    let (date, stamp) = util::utc_stamp(e.created);
                    let preview = e
                        .text
                        .replace(['\r', '\n'], " ")
                        .chars()
                        .take(76)
                        .collect::<String>();
                    let label = format!(
                        "{}-{}-{} {}:{}   {} × {}   {}",
                        &date[..4],
                        &date[4..6],
                        &date[6..],
                        &stamp[9..11],
                        &stamp[11..13],
                        e.width,
                        e.height,
                        preview
                    );
                    SendMessageW(s.list, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                if s.filtered.is_empty() {
                    SendMessageW(s.list,LB_ADDSTRING,0,wide(luma_shot::i18n::tr("История пуста. Включите её в настройках. Текст добавляется при запуске OCR.")).as_ptr() as isize);
                }
            }
        }
    }
    fn history_open(&mut self) {
        if let Some(s) = &self.history {
            let i = unsafe { SendMessageW(s.list, LB_GETCURSEL, 0, 0) };
            if i >= 0 {
                if let Some(e) = s.filtered.get(i as usize) {
                    if let Ok(p) = history::path(e) {
                        native::open(&p)
                    }
                }
            }
        }
    }
}
const RECORD_SNAPSHOT: u32 = WM_APP + 44;
static SNAPSHOT_HOOK: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
unsafe fn release_snapshot_hook() {
    let h = SNAPSHOT_HOOK.swap(0, std::sync::atomic::Ordering::Relaxed);
    if h != 0 {
        UnhookWindowsHookEx(h as HHOOK);
    }
}
unsafe extern "system" fn snapshot_record_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let event = &*(l as *const KBDLLHOOKSTRUCT);
        if event.vkCode == VK_SNAPSHOT as u32 {
            let field = GetFocus();
            if !field.is_null()
                && GetDlgCtrlID(field) == 114
                && GetParent(field) == FindWindowW(wide("LumaSettings").as_ptr(), null())
            {
                let modifiers = (if GetAsyncKeyState(VK_CONTROL as i32) < 0 {
                    MOD_CONTROL
                } else {
                    0
                }) | (if GetAsyncKeyState(VK_SHIFT as i32) < 0 {
                    MOD_SHIFT
                } else {
                    0
                }) | (if GetAsyncKeyState(VK_MENU as i32) < 0 {
                    MOD_ALT
                } else {
                    0
                }) | (if GetAsyncKeyState(VK_LWIN as i32) < 0
                    || GetAsyncKeyState(VK_RWIN as i32) < 0
                {
                    MOD_WIN
                } else {
                    0
                });
                PostMessageW(field, RECORD_SNAPSHOT, modifiers as usize, 0);
                return 1;
            }
        }
    }
    CallNextHookEx(null_mut(), code, w, l)
}
unsafe extern "system" fn hotkey_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_GETDLGCODE => {
            // Let Tab/Shift+Tab remain normal dialog navigation.
            if w == VK_TAB as usize {
                return 0;
            }
            return (DLGC_WANTALLKEYS | DLGC_WANTCHARS | DLGC_WANTARROWS) as isize;
        }
        WM_SETFOCUS => {
            SendMessageW(hwnd, EM_SETSEL, 0, -1);
            release_snapshot_hook();
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(snapshot_record_hook),
                GetModuleHandleW(null()),
                0,
            );
            SNAPSHOT_HOOK.store(hook as isize, std::sync::atomic::Ordering::Relaxed);
        }
        WM_KILLFOCUS => release_snapshot_hook(),
        RECORD_SNAPSHOT => {
            if let Ok(text) = luma_shot::hotkey::format_event(w as u32, VK_SNAPSHOT as u32) {
                SetWindowTextW(hwnd, wide(&text).as_ptr());
                SendMessageW(hwnd, EM_SETSEL, 0, -1);
            }
            return 0;
        }
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
            if (msg == WM_KEYUP || msg == WM_SYSKEYUP) && w != VK_SNAPSHOT as usize {
                return 0;
            }
            let key = w as u32;
            let modifiers = (if ctrl() { MOD_CONTROL } else { 0 })
                | (if shift() { MOD_SHIFT } else { 0 })
                | (if alt() { MOD_ALT } else { 0 })
                | (if GetKeyState(VK_LWIN as i32) < 0 || GetKeyState(VK_RWIN as i32) < 0 {
                    MOD_WIN
                } else {
                    0
                });
            if key == VK_ESCAPE as u32 {
                if let Some(a) = app(GetParent(hwnd)) {
                    SetWindowTextW(hwnd, wide(&a.config.capture_hotkey).as_ptr());
                }
                SetFocus(GetDlgItem(GetParent(hwnd), 1));
                return 0;
            }
            if key == VK_RETURN as u32 && modifiers == 0 {
                SetFocus(GetDlgItem(GetParent(hwnd), 1));
                return 0;
            }
            if let Ok(text) = luma_shot::hotkey::format_event(modifiers, key) {
                SetWindowTextW(hwnd, wide(&text).as_ptr());
                SendMessageW(hwnd, EM_SETSEL, 0, -1);
            }
            return 0;
        }
        WM_CHAR | WM_SYSCHAR => return 0,
        WM_NCDESTROY => {
            release_snapshot_hook();
            RemoveWindowSubclass(hwnd, Some(hotkey_proc), 1);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, w, l)
}
unsafe extern "system" fn settings_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        attach(hwnd, l)
    }
    if let Some(a) = app(hwnd) {
        match msg {
            WM_COMMAND => {
                match w as u16 {
                    1 => match a.save_settings() {
                        Ok(()) => a.close_settings(),
                        Err(e) => {
                            MessageBoxW(
                                hwnd,
                                wide(&e).as_ptr(),
                                wide(luma_shot::i18n::tr("Проверьте настройки")).as_ptr(),
                                if a.config.sound_enabled {
                                    MB_ICONERROR
                                } else {
                                    MB_OK
                                },
                            );
                        }
                    },
                    2 => a.close_settings(),
                    302 => {
                        if let Some(path) = native::choose_folder(hwnd) {
                            if let Some(s) = &a.settings {
                                SetWindowTextW(s.fields[1], wide(&path.to_string_lossy()).as_ptr());
                            }
                        }
                    }
                    303 => {
                        let dir = util::data_dir();
                        let _ = std::fs::create_dir_all(&dir);
                        native::open(&dir);
                    }
                    102 | 300 | 108 | 111 => a.settings_enable(),
                    _ => {}
                }
                return 0;
            }
            WM_CLOSE => {
                a.close_settings();
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, w, l)
}
unsafe extern "system" fn text_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_GETMINMAXINFO {
        let limits = &mut *(l as *mut MINMAXINFO);
        limits.ptMinTrackSize.x = 600;
        limits.ptMinTrackSize.y = 360;
        return 0;
    }
    if msg == WM_NCCREATE {
        attach(hwnd, l)
    }
    if let Some(a) = app(hwnd) {
        match msg {
            WM_NOTIFY => {
                let n = &*(l as *const NMHDR);
                if n.idFrom == 4 && (n.code == NM_CLICK || n.code == NM_RETURN) {
                    if let Some(url) = a.text.as_ref().and_then(|s| s.url.as_ref()) {
                        native::open(Path::new(url));
                    }
                    return 0;
                }
            }

            WM_COMMAND => {
                match w as u16 {
                    1 => a.text_submit(),
                    3 => {
                        if let Some(url) = a.text.as_ref().and_then(|s| s.url.as_ref()) {
                            native::open(Path::new(url));
                        }
                    }
                    2 => {
                        a.text = None;
                        DestroyWindow(hwnd);
                    }
                    _ => {}
                }
                return 0;
            }
            WM_CLOSE => {
                a.text = None;
                DestroyWindow(hwnd);
                return 0;
            }
            WM_SIZE => {
                if let Some(s) = &a.text {
                    let (width, height) = (l as u16 as i32, ((l >> 16) as u16) as i32);
                    MoveWindow(
                        s.edit,
                        16,
                        16,
                        (width - 32).max(100),
                        (height - if s.url.is_some() { 112 } else { 82 }).max(60),
                        1,
                    );
                    if !GetDlgItem(hwnd, 4).is_null() {
                        MoveWindow(
                            GetDlgItem(hwnd, 4),
                            16,
                            height - 90,
                            (width - 32).max(100),
                            24,
                            1,
                        );
                    }
                    if !GetDlgItem(hwnd, 3).is_null() {
                        MoveWindow(GetDlgItem(hwnd, 3), 16, height - 50, 180, 32, 1);
                    }
                    let b = GetDlgItem(hwnd, 1);
                    MoveWindow(b, (width - 300).max(16), height - 50, 144, 32, 1);
                    let b = GetDlgItem(hwnd, 2);
                    MoveWindow(b, (width - 146).max(170), height - 50, 130, 32, 1);
                }
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, w, l)
}
unsafe extern "system" fn history_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_GETMINMAXINFO {
        let limits = &mut *(l as *mut MINMAXINFO);
        limits.ptMinTrackSize.x = 640;
        limits.ptMinTrackSize.y = 420;
        return 0;
    }
    if msg == WM_NCCREATE {
        attach(hwnd, l)
    }
    if let Some(a) = app(hwnd) {
        match msg {
            WM_COMMAND => {
                let id = w as u16;
                let event = (w >> 16) as u16;
                match id {
                    1 => a.history_open(),
                    2 => {
                        a.history = None;
                        DestroyWindow(hwnd);
                    }
                    3 => {
                        if MessageBoxW(
                            hwnd,
                            wide(luma_shot::i18n::tr(
                                "Удалить всю локальную историю снимков и распознанный текст?",
                            ))
                            .as_ptr(),
                            wide(luma_shot::i18n::tr("Очистить историю")).as_ptr(),
                            MB_YESNO | MB_ICONWARNING,
                        ) == IDYES
                        {
                            if let Err(e) = history::clear() {
                                a.fail(&e)
                            }
                            a.filter_history();
                        }
                    }
                    201 if event == EN_CHANGE as u16 => a.filter_history(),
                    202 if event == LBN_DBLCLK as u16 => a.history_open(),
                    _ => {}
                }
                return 0;
            }
            WM_CLOSE => {
                a.history = None;
                DestroyWindow(hwnd);
                return 0;
            }
            WM_SIZE => {
                if let Some(s) = &a.history {
                    let (width, height) = (l as u16 as i32, ((l >> 16) as u16) as i32);
                    MoveWindow(s.search, 18, 46, (width - 36).max(100), 30, 1);
                    MoveWindow(
                        s.list,
                        18,
                        90,
                        (width - 36).max(100),
                        (height - 162).max(100),
                        1,
                    );
                    for (id, x, w) in [
                        (1, 18, 170),
                        (3, 198, 180),
                        (2, (width - 118).max(370), 100),
                    ] {
                        MoveWindow(GetDlgItem(hwnd, id), x, height - 52, w, 32, 1);
                    }
                }
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
