use luma_shot::{
    config::Config,
    image::Rect,
    translate::{self, UrlError},
    ui::{self, Action},
};
fn decoded(url: &str) -> String {
    let t = url
        .split("&text=")
        .nth(1)
        .unwrap()
        .split("&op=")
        .next()
        .unwrap();
    let mut bytes = Vec::new();
    let b = t.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            bytes.push(u8::from_str_radix(&t[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            bytes.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(bytes).unwrap()
}
#[test]
fn unicode_and_query_characters_round_trip() {
    for text in [
        "Hello world",
        "Привет, мир!",
        "你好，世界 🐱",
        "first\nsecond\tthird",
        "a&tl=xx#?%+<script>\"'",
    ] {
        let url = translate::text_url(text, "en").unwrap();
        assert_eq!(decoded(&url), text);
        assert_eq!(url.matches("&tl=").count(), 1);
        assert!(url.starts_with("https://translate.google.com/?sl=auto&tl=en&text="));
        assert!(!url.contains('#'));
        assert!(url.is_ascii());
    }
}
#[test]
fn invalid_text_and_targets_are_rejected() {
    for text in ["", " \r\n\t "] {
        assert_eq!(translate::text_url(text, "ru"), Err(UrlError::Empty));
    }
    for text in ["x\0y", "x\u{1b}y"] {
        assert_eq!(
            translate::text_url(text, "ru"),
            Err(UrlError::ControlCharacter)
        );
    }
    for target in ["", "en&text=evil", "https://evil.test", "xx"] {
        assert_eq!(translate::landing_url(target), Err(UrlError::InvalidTarget));
        assert_eq!(
            translate::text_url("hello", target),
            Err(UrlError::InvalidTarget)
        );
    }
}
#[test]
fn exact_url_boundary_never_truncates() {
    let overhead = translate::text_url("x", "ru").unwrap().len() - 1;
    let text = "a".repeat(translate::MAX_URL_BYTES - overhead);
    let url = translate::text_url(&text, "ru").unwrap();
    assert_eq!(url.len(), translate::MAX_URL_BYTES);
    assert_eq!(decoded(&url), text);
    assert_eq!(
        translate::text_url(&(text + "a"), "ru"),
        Err(UrlError::TooLong)
    );
    assert_eq!(
        translate::text_url(&"中".repeat(250), "ru"),
        Err(UrlError::TooLong)
    );
}
#[test]
fn manual_paste_url_has_no_text() {
    for target in translate::TARGETS {
        let url = translate::landing_url(target).unwrap();
        assert!(!url.contains("text="));
        assert!(!url.contains("secret"));
        assert!(url.len() < 100);
    }
}
#[test]
fn old_settings_migrate_without_text_storage() {
    let mut c: Config =
        serde_json::from_str(r#"{"theme":"light","capture_hotkey":"PrintScreen"}"#).unwrap();
    assert_eq!(c.translation_target, "ru");
    c.translation_target = "en&text=bad".into();
    assert!(c.validate().is_err());
    for target in translate::TARGETS {
        c.translation_target = target.into();
        c.validate().unwrap();
        assert_eq!(c.translation_target, target);
    }
}
#[test]
fn translation_is_immediately_after_search() {
    for style in ["edge", "compact"] {
        luma_shot::i18n::configure("en", style);
        for r in [
            Rect {
                x: 188,
                y: 177,
                w: 888,
                h: 436,
            },
            Rect {
                x: 0,
                y: 0,
                w: 1280,
                h: 900,
            },
            Rect {
                x: 1160,
                y: 700,
                w: 110,
                h: 180,
            },
        ] {
            let l = ui::Layout::new(1280, 900, Some(r));
            let lens = l
                .buttons
                .iter()
                .position(|b| b.action == Action::Lens)
                .unwrap();
            assert_eq!(l.buttons[lens + 1].action, Action::Translate);
            let actions: Vec<_> = l
                .buttons
                .iter()
                .filter(|b| {
                    matches!(
                        b.action,
                        Action::Lens | Action::Translate | Action::Copy | Action::Ocr | Action::Qr
                    )
                })
                .collect();
            for (i, a) in actions.iter().enumerate() {
                for b in actions.iter().skip(i + 1) {
                    assert!(!ui::overlap(a.rect, b.rect));
                }
            }
        }
    }
}
