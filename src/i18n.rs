use std::sync::atomic::{AtomicBool, Ordering};
static EN: AtomicBool = AtomicBool::new(false);
static LIGHTSHOT: AtomicBool = AtomicBool::new(false);
pub fn configure(language: &str, toolbar: &str) {
    EN.store(language == "en", Ordering::Relaxed);
    LIGHTSHOT.store(
        toolbar == "edge" || toolbar == "lightshot" || toolbar == "minimal",
        Ordering::Relaxed,
    )
}
pub fn english() -> bool {
    EN.load(Ordering::Relaxed)
}
pub fn compact() -> bool {
    true
}
pub fn lightshot() -> bool {
    LIGHTSHOT.load(Ordering::Relaxed)
}
pub fn minimal() -> bool {
    false
}
pub fn tr(s: &str) -> &str {
    if !english() {
        return s;
    }
    match s {
        "Перевести картинку · Ctrl+Shift+T · вставить в Google" => "Translate image · Ctrl+Shift+T · paste in Google",
        "Перевод текста OCR" => "Translate OCR text",
        "Экспериментальный перевод Google без ключа. Текст отправляется только по кнопке.\nПри ошибке исходник остаётся; можно открыть переводчик в браузере." => "Experimental keyless Google translation. Text is sent only when you click Translate.\nThe original stays available on error; browser fallback is available.",
        "Исходный текст" => "Original text",
        "Перевести" => "Translate",
        "Перевод" => "Translation",
        "Открыть в браузере" => "Open in browser",
        "Копировать перевод" => "Copy translation",
        "Google обрабатывает отправленный текст онлайн. Ссылка может остаться в истории.\nДля длинного текста браузер откроется без текста в URL: вставьте его вручную." => "Google processes submitted text online. Text links may remain in browser history.\nLong text opens a text-free URL: paste it manually. Clipboard history may retain text.",
        "Дождитесь завершения запроса. Исходник можно редактировать." => "Wait for the request to finish. You can still edit the original.",
        "Добавьте текст для перевода." => "Add text to translate.",
        "Больше 5000 символов: откройте браузер. Текст не будет обрезан." => "Over 5000 characters: open the browser. Text will not be truncated.",
        "Нажмите «Перевести». Это неофициальный сервис; доступность не гарантируется." => "Click Translate. This unofficial service may become unavailable.",
        "Переводим… Исходный текст сохранён." => "Translating... Your original text is preserved.",
        "Готово. Исходный текст не изменён." => "Done. Your original text is unchanged.",
        "Перевод недоступен. Откройте в браузере." => "Translation unavailable. Open in the browser.",
        "Браузер открыт. Исходный текст остался в окне OCR." => "Browser opened. Your original text is still in the OCR window.",
        "Картинка скопирована. В Google Translate нажмите Ctrl+V или «Вставить изображение»." => "Image copied. In Google Translate, press Ctrl+V or choose Paste an image.",
        "PrtSc 1.2 · Справка" => "PrtSc 1.2 · Help",
        "Перевести текст · Ctrl+Shift+T · через браузер" => "Translate text · Ctrl+Shift+T · in browser",
        "Распознаём текст для перевода" => "Recognizing text for translation",
        "Текст не найден. Попробуйте выделить область точнее." => "No text found. Try selecting a clearer area.",
        "PrtSc · Перевод текста" => "PrtSc · Translate text",
        "Перевод выделения" => "Translate selection",
        "Текст распознан на устройстве. Картинка не отправляется.\nПроверьте и отредактируйте текст перед открытием Google Translate." => "Text was recognized on this device. The image is not sent.\nReview and edit the text before opening Google Translate.",
        "Распознанный текст" => "Recognized text",
        "Язык перевода" => "Translate to",
        "Через ссылку текст попадёт в адрес страницы и может сохраниться в истории браузера.\nДля длинного или личного текста: «Скопировать и открыть», затем вставьте его вручную.\nЭто исключает текст из URL, но не делает Google Translate офлайн-сервисом." => "Opening a text link puts its contents in the address and possibly browser history.\nFor long or private text, choose Copy and open, then paste it yourself.\nThis keeps text out of the URL; Google Translate is still an online service.",
        "Скопировать и открыть" => "Copy and open",
        "Открыть перевод" => "Open translation",
        "Можно открыть перевод. В ссылку будет включён весь текст из поля." => "Ready. The link will include all text from the field.",
        "Текст слишком длинный для ссылки. Скопируйте его и вставьте в переводчик вручную." => "Too long for a link. Copy the text and paste it into the translator yourself.",
        "Сначала добавьте текст для перевода." => "Enter some text to translate.",
        "Проверьте текст и язык. Для вставки без URL используйте копирование." => "Check the text and language. Use Copy and open to keep text out of the URL.",
        "Текст скопирован. Вставьте его в переводчик: Ctrl+V." => "Text copied. Paste it into the translator with Ctrl+V.",
        "PrtSc 1.1 · Справка" => "PrtSc 1.1 · About & help",
        "Клавиша снимка — нажмите сочетание" => {
            "Capture shortcut — press keys here"
        }
        "По краям — справа и снизу" => "Edge — right and bottom",
        "Распознать текст · Ctrl+T" => "Recognize text · Ctrl+T",
        "QR · прочитать / скопировать · Ctrl+Q" => {
            "QR · read / copy · Ctrl+Q"
        }
        "QR скопирован · редактор остаётся открытым" => {
            "QR copied · editor stays open"
        }
        "QR: нажмите, чтобы скопировать · Ctrl+клик — открыть ссылку" => {
            "QR: click to copy · Ctrl+click to open link"
        }
        "Показывать курсор в видео" => "Show mouse cursor in recordings",
        "Клавиша снимка (например Ctrl+Alt+S)" => {
            "Capture shortcut (e.g. Ctrl+Alt+S)"
        }

        "Как Lightshot — справа и снизу" => "Lightshot — right and bottom",
        "Цвет текущего инструмента — нажмите для выбора" => {
            "Current tool color — click to choose"
        }
        "Текст / QR — распознать или скопировать" => {
            "Text / QR — recognize or copy"
        }
        "Открыть последний сохранённый файл" => {
            "Reveal last saved file"
        }
        "Поиск Google: передача картинки" => "Google image transfer",
        "Быстро: JPEG, до 2048 px" => "Fast: JPEG, up to 2048 px",
        "Оригинал: PNG без уменьшения" => "Original: full-size PNG",
        "Закрыть · Esc" => "Close · Esc",

        "QR · скопировано" => "QR · copied",
        "QR: скопировать и показать · Ctrl+Q" => "QR: copy and show · Ctrl+Q",
        "Минимальная — сетка значков" => "Minimal — icon grid",
        "Дополнительный захват (PrintWindow) · не гарантирует Telegram" => {
            "Extra capture (PrintWindow) · Telegram not guaranteed"
        }
        "Панель: автоматически" => "Toolbar: automatic",
        "Панель: слева" => "Toolbar: left",
        "Панель: справа" => "Toolbar: right",
        "Панель: сверху" => "Toolbar: top",
        "Панель: снизу" => "Toolbar: bottom",
        "Закрепить снимок · поверх всех окон" => {
            "Pin screenshot · always on top"
        }
        "Открыть ссылку" => "Open link",
        "Сохранено · нажмите, чтобы открыть папку" => {
            "Saved · click to open folder"
        }

        "PrtSc 1.0 · Справка" => "PrtSc 1.0 · Help",
        "Размытие · B · не для секретов" => "Blur · B · not for secrets",
        "Пикселизация · X · не для секретов" => {
            "Pixelate · X · not for secrets"
        }
        "Область" => "Select area",
        "Перемещение" => "Move objects",
        "Карандаш" => "Pen",
        "Линия" => "Line",
        "Стрелка" => "Arrow",
        "Прямоугольник" => "Rectangle",
        "Овал" => "Ellipse",
        "Маркер" => "Highlight",
        "Текст" => "Text",
        "Скрыть данные" => "Redact",
        "Номер шага" => "Numbered step",
        "Пипетка" => "Color picker",
        "Копия" => "Copy",
        "Сохранить" => "Save",
        "Ещё" => "More",
        "Видео" => "Record",
        "Загрузить" => "Upload",
        "Настройки…" => "Settings…",
        "История снимков…" => "Local history…",
        "Папка данных" => "Data folder",
        "Отмена" => "Cancel",
        "Закрыть" => "Close",
        "Копировать текст" => "Copy text",
        "PrtSc · Настройки" => "PrtSc · Settings",
        "Тема панели" => "Panel theme",
        "Тёмная" => "Dark",
        "Светлая" => "Light",
        "Как в Windows" => "Windows theme",
        "Папка снимков" => "Output folder",
        "Выбрать…" => "Browse…",
        "Формат сохранения" => "Image format",
        "PNG — без потерь" => "PNG — lossless",
        "JPEG — с потерями" => "JPEG — lossy",
        "WebP — без потерь" => "WebP — lossless",
        "Качество JPEG (1–100)" => "JPEG quality (1–100)",
        "Только для JPEG" => "JPEG only",
        "Сжатие PNG" => "PNG compression",
        "Быстрое" => "Fast",
        "Обычное" => "Default",
        "Максимальное (медленнее)" => "Best (slower)",
        "Лимит истории" => "History limit",
        "Язык / Language" => "Language",
        "Вид панели" => "Toolbar layout",
        "Обычная — с подписями" => "Standard — labelled",
        "Компактная — значки" => "Compact — icons",
        "Формат видео" => "Recording format",
        "Частота кадров" => "Frame rate",
        "Фотохостинг" => "Image host",
        "Звуки уведомлений" => "Notification sounds",
        "Подтверждать отправку в Google" => "Confirm sending to Google",
        "Авто-QR после выделения — локально, без открытия ссылок" => {
            "Auto-QR after selection — local; never opens links"
        }
        "Хранить локальную историю снимков" => {
            "Keep local screenshot history"
        }
        "Хранить локальную историю снимков (по умолчанию выключено)" => {
            "Keep local history (off by default)"
        }
        "PrtSc · Всё локально, пока вы сами не выберете Google" => {
            "PrtSc · Local until you explicitly choose Google or Upload"
        }
        "Копировать · Enter / Ctrl+C" => "Copy · Enter / Ctrl+C",
        "Сохранить · Ctrl+S · Сохранить как: Ctrl+Shift+S" => {
            "Save · Ctrl+S · Save as: Ctrl+Shift+S"
        }
        "Поиск по картинке · Ctrl+G · передача в Google" => {
            "Image search · Ctrl+G · sends selection to Google"
        }
        "Распознать текст офлайн · Ctrl+T" => {
            "Offline text recognition · Ctrl+T"
        }
        "Другие действия · F1" => "More actions · F1",
        "Записать область в MP4/GIF · Ctrl+Shift+R" => {
            "Record area to MP4/GIF · Ctrl+Shift+R"
        }
        "Загрузить на выбранный фотохостинг · ссылка публичная" => {
            "Upload to selected image host · public link"
        }
        "Показать результат QR · Ctrl+Q" => "Show QR result · Ctrl+Q",
        "Цвет · правая кнопка: изменить цвет палитры" => {
            "Color · right-click to edit palette"
        }
        "Толщина · колесо / + / −" => "Stroke width · wheel / + / −",
        "Отменить · Ctrl+Z" => "Undo · Ctrl+Z",
        "Повторить · Ctrl+Shift+Z" => "Redo · Ctrl+Shift+Z",
        "Сохранить как…" => "Save as…",
        "Закрепить поверх окон" => "Pin on top",
        "Прочитать QR локально" => "Read QR locally",
        "Длинный снимок — экспериментально" => {
            "Scrolling capture — experimental"
        }
        "Печать…" => "Print…",
        "Копировать HEX под курсором" => "Copy HEX at cursor",
        "Копировать RGB под курсором" => "Copy RGB at cursor",
        "Справка и клавиши" => "Help and shortcuts",
        "Лицензии компонентов" => "Component licences",
        "Новый снимок · PrtSc" => "New screenshot · PrtSc",
        "Выход из PrtSc" => "Exit PrtSc",
        "Начать запись MP4/GIF" => "Start MP4/GIF recording",
        "Остановить запись" => "Stop recording",
        "Открыть папку снимков" => "Open output folder",
        "Выделите область мышью · Alt + щелчок — окно · Esc — отмена" => {
            "Drag to select · Alt+click: window · Esc: cancel"
        }
        "Отпустите кнопку — появятся инструменты" => {
            "Release mouse to show the tools"
        }
        "Enter — копировать · Esc — отмена · F1 — ещё" => {
            "Enter: copy · Esc: cancel · F1: more"
        }
        "Enter — копировать · Esc — отмена · Alt — лупа" => {
            "Enter: copy · Esc: cancel · Alt: magnify"
        }
        "Сохранено" => "Saved",
        "Скопировано" => "Copied",
        "Сохраняем" => "Saving",
        "Открываем Google" => "Opening Google",
        "Распознаём текст офлайн" => "Recognizing text locally",
        "Текст распознан офлайн · скопирован" => {
            "Offline OCR · copied to clipboard"
        }
        "В этой области QR не найден" => "No QR code found in this area",
        "QR · текст · ссылки не открываются автоматически" => {
            "QR text · links are never opened automatically"
        }
        "Загружаем изображение" => "Uploading image",
        "Ссылка скопирована" => "Link copied",
        "QR не найден · Enter — копировать" => "No QR found · Enter: copy",
        "Авто-QR после выделения" => "Auto-QR after selection",
        "Захват активного окна (PrintWindow)" => {
            "Active-window capture (PrintWindow)"
        }
        "Запись выбранной области без аудио.\nСтоп: Ctrl+Shift+R или кнопка Стоп." => {
            "Record the selected area without audio.\nStop: Ctrl+Shift+R or the Stop button."
        }
        "Загрузка только по кнопке. Ссылки публичные.\nНе отправляйте секреты на фотохостинг." => {
            "Uploads are explicit; image links are public.\nDo not upload confidential information."
        }
        "Поиск по локальной истории и распознанному тексту" => {
            "Search local history and recognized text"
        }
        "История пуста. Включите её в настройках. Текст добавляется при запуске OCR." => {
            "History is empty. Enable it in Settings. Text is indexed when OCR is run."
        }
        "Открыть выбранный" => "Open selected",
        "Удалить всю историю…" => "Delete all history…",
        "Удалить всю локальную историю снимков и распознанный текст?" => {
            "Permanently delete all local history and OCR text?"
        }
        "Очистить историю" => "Clear history",
        "Проверьте настройки" => "Check settings",
        "PrtSc 0.4 · Справка" => "PrtSc 0.4 · Help",
        "Введите текст · Enter — готово · Shift+Enter — новая строка" => {
            "Type text · Enter: done · Shift+Enter: new line"
        }
        "Сначала выделите область" => "Select an area first",
        "Нет открытого снимка" => "No screenshot is open",
        "Цвет скопирован" => "Color copied",
        "Битрейт MP4 (Мбит/с)" => "MP4 bitrate (Mbps)",
        "Imgur Client ID (не пароль)" => "Imgur Client ID (not password)",
        "Ctrl+S — сохранить · Ctrl+Shift+S — сохранить как" => {
            "Ctrl+S — save · Ctrl+Shift+S — Save as"
        }
        "Сохраняем локальную историю" => "Saving local history",
        "Прочитать / скопировать QR · Ctrl+Q" => "Read / copy QR · Ctrl+Q",
        "PrtSc 0.5 · Справка" => "PrtSc 0.5 · Help",
        "Lightshot — справа и снизу" => "Lightshot — right and bottom",
        "Показать последний сохранённый файл" => {
            "Reveal last saved file"
        }
        "Передача в Google" => "Google image transfer",
        "Быстро — JPEG, до 2048 px" => "Fast — JPEG, up to 2048 px",
        "Оригинал — PNG без потерь" => "Original — lossless PNG",
        _ => s,
    }
}
pub fn text(s: &str) -> String {
    let translated = tr(s);
    if translated != s {
        return translated.into();
    }
    if !english() {
        return s.into();
    }
    if let Some(v) = s.strip_prefix("QR найден: ") {
        return format!(
            "QR found: {}",
            v.replace("F1 → Прочитать QR", "F1 → Read QR")
        );
    }
    s.into()
}
