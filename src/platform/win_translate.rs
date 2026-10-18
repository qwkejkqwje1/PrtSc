//! Explicit text translation, separate from screenshot translation.
use super::*;
use luma_shot::translate;
pub(super) struct TranslationState {
    pub(super) hwnd: HWND,
    edit: HWND,
    language: HWND,
    request: u64,
}
fn tr(s: &str) -> &str {
    luma_shot::i18n::tr(s)
}
impl App {
    pub(super) fn translate_image(&mut self, image: Image) -> Result<(), String> {
        let target = if translate::TARGETS.contains(&self.config.translation_target.as_str()) {
            self.config.translation_target.as_str()
        } else {
            "ru"
        };
        let url = translate::image_url(target).map_err(str::to_string)?;
        native::clipboard_image(self.main, &image)?;
        // Browsers forbid cross-origin file-input injection. Do not synthesize global
        // Ctrl+V or redirect an ordinary Lens search and claim it is a translation.
        native::open_browser(&url)?;
        self.close_editor();
        self.notify("PrtSc", tr("Картинка скопирована. В Google Translate нажмите Ctrl+V или «Вставить изображение»."));
        Ok(())
    }
    pub(super) fn show_translation(&mut self, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err(tr("Текст не найден. Попробуйте выделить область точнее.").into());
        }
        unsafe {
            if let Some(t) = &self.translation {
                SetForegroundWindow(t.hwnd);
                return Ok(());
            }
            let hwnd = CreateWindowExW(
                WS_EX_APPWINDOW,
                wide("LumaTranslate").as_ptr(),
                wide(tr("PrtSc · Перевод текста")).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                (GetSystemMetrics(SM_CXSCREEN) - 800).max(0) / 2,
                (GetSystemMetrics(SM_CYSCREEN) - 680).max(0) / 2,
                800,
                680,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                self as *mut _ as _,
            );
            if hwnd.is_null() {
                return Err(native::last("Create translation preview"));
            }
            child(hwnd, "STATIC", "Перевод текста OCR", 0, 20, 12, 744, 22, 0);
            child(hwnd,"STATIC","Экспериментальный перевод Google без ключа. Текст отправляется только по кнопке.\nПри ошибке исходник остаётся; можно открыть переводчик в браузере.",0,20,38,744,40,0);
            child(hwnd, "STATIC", "Исходный текст", 0, 20, 84, 744, 20, 0);
            let edit = child(
                hwnd,
                "EDIT",
                text,
                WS_BORDER
                    | WS_VSCROLL
                    | ES_MULTILINE as u32
                    | ES_AUTOVSCROLL as u32
                    | ES_WANTRETURN as u32
                    | WS_TABSTOP,
                20,
                108,
                744,
                140,
                100,
            );
            SendMessageW(edit, EM_SETLIMITTEXT, 1_200_000, 0);
            child(hwnd, "STATIC", "Язык перевода", 0, 20, 260, 160, 24, 0);
            let language = child(
                hwnd,
                "COMBOBOX",
                "",
                WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                180,
                256,
                220,
                240,
                101,
            );
            let labels = [
                "Русский / Russian",
                "English",
                "Chinese (Simplified)",
                "Deutsch",
                "Français",
                "Español",
                "Japanese",
                "Korean",
                "Українська",
            ];
            for label in labels {
                SendMessageW(language, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
            let selected = translate::TARGETS
                .iter()
                .position(|t| *t == self.config.translation_target)
                .unwrap_or(0);
            SendMessageW(language, CB_SETCURSEL, selected, 0);
            child(
                hwnd,
                "BUTTON",
                "Перевести",
                WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
                574,
                254,
                190,
                32,
                1,
            );
            child(hwnd, "STATIC", "Перевод", 0, 20, 296, 744, 20, 0);
            child(
                hwnd,
                "EDIT",
                "",
                WS_BORDER
                    | WS_VSCROLL
                    | ES_MULTILINE as u32
                    | ES_AUTOVSCROLL as u32
                    | ES_READONLY as u32
                    | WS_TABSTOP,
                20,
                320,
                744,
                154,
                103,
            );
            child(hwnd, "STATIC", "", 0, 20, 484, 744, 42, 102);
            child(hwnd,"STATIC","Google обрабатывает отправленный текст онлайн. Ссылка может остаться в истории.\nДля длинного текста браузер откроется без текста в URL: вставьте его вручную.",0,20,534,744,38,0);
            child(
                hwnd,
                "BUTTON",
                "Открыть в браузере",
                WS_TABSTOP,
                20,
                590,
                230,
                32,
                3,
            );
            child(
                hwnd,
                "BUTTON",
                "Копировать перевод",
                WS_TABSTOP,
                270,
                590,
                230,
                32,
                4,
            );
            child(hwnd, "BUTTON", "Закрыть", WS_TABSTOP, 644, 590, 120, 32, 2);
            self.translation = Some(TranslationState {
                hwnd,
                edit,
                language,
                request: 0,
            });
            self.refresh_translation();
            if let Some(original) = &self.text {
                ShowWindow(original.hwnd, SW_HIDE);
            }
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, 0);
            Ok(())
        }
    }
    fn translation_values(&self) -> Option<(String, &'static str)> {
        let s = self.translation.as_ref()?;
        unsafe {
            let i = SendMessageW(s.language, CB_GETCURSEL, 0, 0);
            Some((
                dialogs::value(s.edit),
                translate::TARGETS
                    .get(i.max(0) as usize)
                    .copied()
                    .unwrap_or("ru"),
            ))
        }
    }
    fn translation_status(&self, message: &str) {
        if let Some(s) = &self.translation {
            unsafe {
                SetWindowTextW(GetDlgItem(s.hwnd, 102), wide(message).as_ptr());
            }
        }
    }
    fn refresh_translation(&mut self) {
        let values = self.translation_values();
        if let (Some(s), Some((text, target))) = (&mut self.translation, values) {
            // Editing while a request runs invalidates its result without losing the source.
            s.request = 0;
            unsafe {
                SetWindowTextW(GetDlgItem(s.hwnd, 103), wide("").as_ptr());
                EnableWindow(GetDlgItem(s.hwnd, 4), 0);
                EnableWindow(
                    GetDlgItem(s.hwnd, 1),
                    (!self.translation_network_busy
                        && translate::request_body(&text, target).is_ok())
                        as i32,
                );
                EnableWindow(GetDlgItem(s.hwnd, 3), (!text.trim().is_empty()) as i32);
            }
            let msg = if self.translation_network_busy {
                "Дождитесь завершения запроса. Исходник можно редактировать."
            } else if text.trim().is_empty() {
                "Добавьте текст для перевода."
            } else if text.chars().count() > translate::MAX_TEXT_CHARS {
                "Больше 5000 символов: откройте браузер. Текст не будет обрезан."
            } else {
                "Нажмите «Перевести». Это неофициальный сервис; доступность не гарантируется."
            };
            self.translation_status(tr(msg));
        }
    }
    fn start_text_translation(&mut self) {
        if self.translation_network_busy {
            return;
        }
        let Some((source, target)) = self.translation_values() else {
            return;
        };
        if let Err(e) = translate::request_body(&source, target) {
            self.translation_status(e);
            return;
        }
        self.translation_request = self.translation_request.wrapping_add(1).max(1);
        let request = self.translation_request;
        let Some(s) = &mut self.translation else {
            return;
        };
        s.request = request;
        self.translation_network_busy = true;
        unsafe {
            EnableWindow(GetDlgItem(s.hwnd, 1), 0);
            EnableWindow(GetDlgItem(s.hwnd, 4), 0);
            SetWindowTextW(GetDlgItem(s.hwnd, 103), wide("").as_ptr());
        }
        self.translation_status(tr("Переводим… Исходный текст сохранён."));
        let tx = self.tx.clone();
        let hwnd = self.main as usize;
        let target = target.to_string();
        std::thread::spawn(move || {
            let result = text_http::translate(&source, &target);
            let _ = tx.send(Job {
                foreground: false,
                result: Ok(Work::TextTranslation {
                    request,
                    source,
                    target,
                    result,
                }),
            });
            unsafe {
                PostMessageW(hwnd as HWND, DONE, 0, 0);
            }
        });
    }
    pub(super) fn finish_text_translation(
        &mut self,
        request: u64,
        source: &str,
        target: &str,
        result: Result<String, String>,
    ) {
        let current = self.translation_values();
        let valid = self
            .translation
            .as_ref()
            .zip(current.as_ref())
            .map(|(state, (text, lang))| {
                translate::result_matches(request, state.request, source, target, text, lang)
            })
            .unwrap_or(false);
        if !valid {
            self.refresh_translation();
            return;
        }
        let Some(s) = &self.translation else { return };
        unsafe {
            EnableWindow(GetDlgItem(s.hwnd, 1), 1);
        }
        match result {
            Ok(text) => {
                unsafe {
                    SetWindowTextW(GetDlgItem(s.hwnd, 103), wide(&text).as_ptr());
                    EnableWindow(GetDlgItem(s.hwnd, 4), 1);
                }
                self.config.translation_target = target.into();
                let _ = self.config.save();
                self.translation_status(tr("Готово. Исходный текст не изменён."));
            }
            Err(e) => self.translation_status(&format!(
                "{} {e}",
                tr("Перевод недоступен. Откройте в браузере.")
            )),
        }
    }
    fn translation_browser(&mut self) {
        let Some((text, target)) = self.translation_values() else {
            return;
        };
        if text.trim().is_empty() {
            return;
        }
        let (url, paste) = match translate::text_url(&text, target) {
            Ok(u) => (u, false),
            Err(_) => match translate::landing_url(target) {
                Ok(u) => (u, true),
                Err(_) => return,
            },
        };
        let result = (|| {
            if paste {
                native::clipboard_text(self.main, &text)?;
            }
            native::open_browser(&url)
        })();
        match result {
            Ok(()) => self.translation_status(tr(if paste {
                "Текст скопирован. Вставьте его в переводчик: Ctrl+V."
            } else {
                "Браузер открыт. Исходный текст остался в окне OCR."
            })),
            Err(e) => self.translation_status(&e),
        }
    }
    pub(super) fn close_translation(&mut self) {
        if let Some(s) = self.translation.take() {
            unsafe {
                DestroyWindow(s.hwnd);
            }
        }
        if let Some(s) = &self.text {
            unsafe {
                ShowWindow(s.hwnd, SW_SHOW);
                SetForegroundWindow(s.hwnd);
            }
        }
    }
}
pub(super) unsafe extern "system" fn translation_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        attach(hwnd, l);
    }
    if msg == WM_CTLCOLORSTATIC || msg == WM_CTLCOLORBTN {
        SetBkMode(w as HDC, TRANSPARENT as i32);
        SetTextColor(w as HDC, GetSysColor(COLOR_WINDOWTEXT));
        return GetSysColorBrush(COLOR_WINDOW) as isize;
    }
    if let Some(a) = app(hwnd) {
        match msg {
            WM_COMMAND => {
                let id = w as u16;
                let notification = (w >> 16) as u16;
                match id {
                    1 => a.start_text_translation(),
                    2 => a.close_translation(),
                    3 => a.translation_browser(),
                    4 => {
                        if let Some(s) = &a.translation {
                            let text = dialogs::value(GetDlgItem(s.hwnd, 103));
                            if let Err(e) = native::clipboard_text(a.main, &text) {
                                a.translation_status(&e);
                            }
                        }
                    }
                    100 if notification == EN_CHANGE as u16 => a.refresh_translation(),
                    101 if notification == CBN_SELCHANGE as u16 => a.refresh_translation(),
                    _ => {}
                }
                return 0;
            }
            WM_CLOSE => {
                a.close_translation();
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, w, l)
}
