use luma_shot::{
    dock,
    image::{Image, Rect},
    links,
    ui::{self, Action},
    video,
};
#[test]
fn top_down_nv12_has_bright_top_and_dark_bottom() {
    let mut i = Image::new(4, 4).unwrap();
    for y in 0..4 {
        for x in 0..4 {
            let n = (y * 4 + x) * 4;
            let c = if y < 2 { 255 } else { 0 };
            i.rgba[n..n + 4].copy_from_slice(&[c, c, c, 255]);
        }
    }
    let f = video::nv12(&i).unwrap();
    assert_eq!(f.len(), 24);
    assert_eq!(&f[..8], &[235; 8]);
    assert_eq!(&f[8..16], &[16; 8]);
    assert_eq!(&f[16..], &[128; 8]);
    assert!(video::nv12(&Image::new(3, 4).unwrap()).is_err());
}
#[test]
fn nv12_chroma_is_uv_not_vu() {
    let mut i = Image::new(2, 2).unwrap();
    for p in i.rgba.chunks_exact_mut(4) {
        p.copy_from_slice(&[255, 0, 0, 255]);
    }
    let f = video::nv12(&i).unwrap();
    assert_eq!(f, [82, 82, 82, 82, 90, 240]);
}
#[test]
fn links_only_explicit_http_urls() {
    for x in ["https://example.com/qr?id=1", "http://example.org/a"] {
        assert_eq!(links::http_url(x).as_deref(), Some(x));
    }
    for x in [
        "javascript:alert(1)",
        "file:///C:/secret",
        "https://example.com\nsecond",
        "https://user:pass@example.com/",
        "https://",
        "https://example.com\\evil",
    ] {
        assert!(links::http_url(x).is_none(), "{x}");
    }
}
#[test]
fn icon_layouts_and_docking_keep_all_actions_and_hitboxes() {
    for style in ["compact", "lightshot"] {
        luma_shot::i18n::configure("en", style);
        for side in ["auto", "left", "right", "top", "bottom"] {
            dock::configure(side);
            let mut l = ui::Layout::new(
                1280,
                900,
                Some(Rect {
                    x: 200,
                    y: 100,
                    w: 600,
                    h: 400,
                }),
            );
            dock::apply(&mut l, 1280, 900);
            for action in [
                Action::Pin,
                Action::Upload,
                Action::Copy,
                Action::Save,
                Action::Lens,
                Action::Ocr,
                Action::Record,
                Action::PickColor,
                Action::More,
                Action::Undo,
                Action::Redo,
            ] {
                assert!(l.buttons.iter().any(|b| b.action == action));
            }
            for b in &l.buttons {
                assert!(l.panel.contains(b.rect.x, b.rect.y));
                assert!(l.panel.contains(
                    b.rect.x + b.rect.w as i32 - 1,
                    b.rect.y + b.rect.h as i32 - 1
                ));
                assert_eq!(
                    l.hit(
                        b.rect.x + b.rect.w as i32 / 2,
                        b.rect.y + b.rect.h as i32 / 2
                    ),
                    Some(b.action)
                );
            }
            for (i, a) in l.buttons.iter().enumerate() {
                for b in &l.buttons[i + 1..] {
                    let overlap = a.rect.x < b.rect.x + b.rect.w as i32
                        && a.rect.x + a.rect.w as i32 > b.rect.x
                        && a.rect.y < b.rect.y + b.rect.h as i32
                        && a.rect.y + a.rect.h as i32 > b.rect.y;
                    assert!(!overlap, "{:?} {:?}", a.action, b.action);
                }
            }
        }
    }
    dock::configure("auto");
    luma_shot::i18n::configure("ru", "compact");
}
