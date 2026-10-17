//! Local OCR preview and explicit browser handoff. No network call in this module.
use super::*;
use luma_shot::translate::{self, UrlError};
pub(super) struct TranslationState {
    pub(super) hwnd: HWND,
    edit: HWND,
    language: HWND,
}
fn tr(s: &str) -> &str {
    luma_shot::i18n::tr(s)
}
impl App {
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
                (GetSystemMetrics(SM_CYSCREEN) - 640).max(0) / 2,
                800,
                640,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                self as *mut _ as _,
            );
            if hwnd.is_null() {
                return Err(native::last("Create translation preview"));
            }
            child(hwnd, "STATIC", "Перевод выделения", 0, 20, 16, 744, 24, 0);
            child(hwnd,"STATIC","Текст распознан на устройстве. Картинка не отправляется.\nПроверьте и отредактируйте текст перед открытием Google Translate.",0,20,48,744,44,0);
            child(hwnd, "STATIC", "Распознанный текст", 0, 20, 98, 744, 22, 0);
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
                124,
                744,
                250,
                100,
            );
            SendMessageW(edit, EM_SETLIMITTEXT, 1_200_000, 0);
            child(hwnd, "STATIC", "Язык перевода", 0, 20, 389, 160, 24, 0);
            let language = child(
                hwnd,
                "COMBOBOX",
                "",
                WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                190,
                384,
                260,
                240,
                101,
            );
            let labels = if luma_shot::i18n::english() {
                [
                    "Russian",
                    "English",
                    "Chinese (Simplified)",
                    "German",
                    "French",
                    "Spanish",
                    "Japanese",
                    "Korean",
                    "Ukrainian",
                ]
            } else {
                [
                    "Русский",
                    "English",
                    "简体中文",
                    "Deutsch",
                    "Français",
                    "Español",
                    "日本語",
                    "한국어",
                    "Українська",
                ]
            };
            for label in labels {
                SendMessageW(language, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
            let selected = translate::TARGETS
                .iter()
                .position(|t| *t == self.config.translation_target)
                .unwrap_or(0);
            SendMessageW(language, CB_SETCURSEL, selected, 0);
            child(hwnd,"STATIC","Через ссылку текст попадёт в адрес страницы и может сохраниться в истории браузера.\nДля длинного или личного текста: «Скопировать и открыть», затем вставьте его вручную.\nЭто исключает текст из URL, но не делает Google Translate офлайн-сервисом.",0,20,428,744,58,0);
            child(hwnd, "STATIC", "", 0, 20, 493, 744, 30, 102);
            child(
                hwnd,
                "BUTTON",
                "Скопировать и открыть",
                WS_TABSTOP,
                20,
                538,
                258,
                34,
                3,
            );
            child(
                hwnd,
                "BUTTON",
                "Открыть перевод",
                WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
                430,
                538,
                200,
                34,
                1,
            );
            child(hwnd, "BUTTON", "Отмена", WS_TABSTOP, 644, 538, 120, 34, 2);
            self.translation = Some(TranslationState {
                hwnd,
                edit,
                language,
            });
            self.refresh_translation();
            if !self.overlay.is_null() {
                ShowWindow(self.overlay, SW_HIDE);
            }
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, 0);
            Ok(())
        }
    }
    fn translation_values(&self) -> Option<(String, &'static str)> {
        let state = self.translation.as_ref()?;
        unsafe {
            let i = SendMessageW(state.language, CB_GETCURSEL, 0, 0);
            let target = translate::TARGETS
                .get(i.max(0) as usize)
                .copied()
                .unwrap_or("ru");
            Some((dialogs::value(state.edit), target))
        }
    }
    fn refresh_translation(&self) {
        if let (Some(state), Some((text, target))) = (&self.translation, self.translation_values())
        {
            let result = translate::text_url(&text, target);
            let message=match result {
                Ok(_) => "Можно открыть перевод. В ссылку будет включён весь текст из поля.",
                Err(UrlError::TooLong) => "Текст слишком длинный для ссылки. Скопируйте его и вставьте в переводчик вручную.",
                Err(UrlError::Empty) => "Сначала добавьте текст для перевода.",
                Err(_) => "Проверьте текст и язык. Для вставки без URL используйте копирование.",
            };
            unsafe {
                EnableWindow(GetDlgItem(state.hwnd, 1), result.is_ok() as i32);
                EnableWindow(GetDlgItem(state.hwnd, 3), (!text.trim().is_empty()) as i32);
                SetWindowTextW(GetDlgItem(state.hwnd, 102), wide(tr(message)).as_ptr());
            }
        }
    }
    fn submit_translation(&mut self, copy_only_url: bool) {
        let Some((text, target)) = self.translation_values() else {
            return;
        };
        if text.trim().is_empty() {
            return;
        }
        let url = if copy_only_url {
            translate::landing_url(target)
        } else {
            translate::text_url(&text, target)
        };
        let Ok(url) = url else {
            self.refresh_translation();
            return;
        };
        let result = (|| {
            if copy_only_url {
                native::clipboard_text(self.main, &text)?;
            }
            native::open_browser(&url)
        })();
        if let Err(e) = result {
            if let Some(t) = &self.translation {
                unsafe {
                    SetWindowTextW(GetDlgItem(t.hwnd, 102), wide(&e).as_ptr());
                }
            }
            return;
        }
        self.config.translation_target = target.into();
        let _ = self.config.save(); // Only language preference; never recognized text or URL.
        self.close_editor();
        self.close_translation();
        if copy_only_url {
            self.notify(
                "PrtSc",
                tr("Текст скопирован. Вставьте его в переводчик: Ctrl+V."),
            );
        }
    }
    pub(super) fn close_translation(&mut self) {
        if let Some(t) = self.translation.take() {
            unsafe {
                DestroyWindow(t.hwnd);
            }
        }
        if !self.overlay.is_null() {
            unsafe {
                ShowWindow(self.overlay, SW_SHOW);
                SetForegroundWindow(self.overlay);
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
                    1 => a.submit_translation(false),
                    2 => a.close_translation(),
                    3 => a.submit_translation(true),
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
