//! Native settings, hotkey recording, text and history dialogs.
use super::*;
static CONTROL_FONT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

pub(super) unsafe fn child(
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
pub(super) unsafe fn value(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut b = vec![0u16; len + 1];
    GetWindowTextW(hwnd, b.as_mut_ptr(), b.len() as i32);
    String::from_utf16_lossy(&b[..len])
}
pub(super) unsafe fn setting_edit(
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
pub(super) unsafe fn setting_combo(
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
pub(super) unsafe fn setting_check(
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
    pub(super) fn show_settings(&mut self) {
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
    pub(super) fn settings_enable(&self) {
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
    pub(super) fn save_settings(&mut self) -> Result<(), String> {
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

        self.schedule_qr();
        self.invalidate();
        Ok(())
    }
    pub(super) fn close_settings(&mut self) {
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
    pub(super) fn show_ocr_text(&mut self, title: &str, text: &str) {
        self.show_text(title, text, None);
        if let Some(s) = &mut self.text {
            s.ocr = true;
            unsafe {
                child(
                    s.hwnd,
                    "BUTTON",
                    "Перевести",
                    WS_TABSTOP,
                    210,
                    470,
                    140,
                    32,
                    5,
                );
            }
        }
    }
    pub(super) fn show_text(&mut self, title: &str, text: &str, pending: Option<Image>) {
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
            self.text = Some(TextState {
                hwnd,
                edit,
                url,
                ocr: false,
            });
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, -1);
        }
    }
    pub(super) fn text_submit(&mut self) {
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
    pub(super) fn show_history(&mut self) {
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
    pub(super) fn filter_history(&mut self) {
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
    pub(super) fn history_open(&mut self) {
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
pub(super) unsafe fn release_snapshot_hook() {
    let h = SNAPSHOT_HOOK.swap(0, std::sync::atomic::Ordering::Relaxed);
    if h != 0 {
        UnhookWindowsHookEx(h as HHOOK);
    }
}
pub(super) unsafe extern "system" fn snapshot_record_hook(
    code: i32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
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
pub(super) unsafe extern "system" fn hotkey_proc(
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
pub(super) unsafe extern "system" fn settings_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    if msg == WM_CTLCOLORSTATIC || msg == WM_CTLCOLORBTN {
        SetBkMode(w as HDC, TRANSPARENT as i32);
        SetTextColor(w as HDC, GetSysColor(COLOR_WINDOWTEXT));
        return GetSysColorBrush(COLOR_WINDOW) as isize;
    }
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
pub(super) unsafe extern "system" fn text_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    if msg == WM_GETMINMAXINFO {
        let limits = &mut *(l as *mut MINMAXINFO);
        limits.ptMinTrackSize.x = if app(hwnd)
            .and_then(|a| a.text.as_ref())
            .map(|s| s.ocr)
            .unwrap_or(false)
        {
            700
        } else {
            600
        };
        limits.ptMinTrackSize.y = 360;
        return 0;
    }
    if msg == WM_CTLCOLORSTATIC || msg == WM_CTLCOLORBTN {
        SetBkMode(w as HDC, TRANSPARENT as i32);
        SetTextColor(w as HDC, GetSysColor(COLOR_WINDOWTEXT));
        return GetSysColorBrush(COLOR_WINDOW) as isize;
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
                    5 => {
                        if let Some(s) = &a.text {
                            if s.ocr {
                                let text = value(s.edit);
                                if let Err(e) = a.show_translation(&text) {
                                    a.fail(&e);
                                }
                            }
                        }
                    }
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
                    if !GetDlgItem(hwnd, 5).is_null() {
                        MoveWindow(
                            GetDlgItem(hwnd, 5),
                            if s.url.is_some() { 210 } else { 16 },
                            height - 50,
                            140,
                            32,
                            1,
                        );
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
pub(super) unsafe extern "system" fn history_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    if msg == WM_GETMINMAXINFO {
        let limits = &mut *(l as *mut MINMAXINFO);
        limits.ptMinTrackSize.x = 640;
        limits.ptMinTrackSize.y = 420;
        return 0;
    }
    if msg == WM_CTLCOLORSTATIC || msg == WM_CTLCOLORBTN {
        SetBkMode(w as HDC, TRANSPARENT as i32);
        SetTextColor(w as HDC, GetSysColor(COLOR_WINDOWTEXT));
        return GetSysColorBrush(COLOR_WINDOW) as isize;
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
