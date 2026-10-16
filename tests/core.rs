use base64::{engine::general_purpose::STANDARD, Engine};
use luma_shot::{
    config::Config,
    editor::{Annotation, Editor, Point, Tool},
    image::{Image, Rect, MAX_PIXELS},
    render::Renderer,
    stitch, ui, util,
};
use std::path::PathBuf;
fn rect(x: i32, y: i32, w: u32, h: u32) -> Rect {
    Rect { x, y, w, h }
}
fn temp(ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!("luma-test-{}.{}", util::unique_id(), ext))
}
fn shape(t: Tool, a: (f32, f32), b: (f32, f32)) -> Annotation {
    let mut s = Annotation::new(t, Point::new(a.0, a.1), [39, 131, 222, 255], 3.);
    s.points.push(Point::new(b.0, b.1));
    s
}
#[test]
fn rect_reverse_drag() {
    assert_eq!(Rect::from_points((30, 40), (10, 15)), rect(10, 15, 20, 25));
}
#[test]
fn rect_clips_negative_origin() {
    assert_eq!(rect(-5, -7, 20, 30).clamp(10, 12), rect(0, 0, 10, 12));
}
#[test]
fn rect_outside_is_empty() {
    assert!(!rect(300, 300, 20, 20).clamp(100, 100).valid());
}
#[test]
fn rect_boundary_is_half_open() {
    let r = rect(10, 20, 30, 40);
    assert!(r.contains(10, 20));
    assert!(r.contains(39, 59));
    assert!(!r.contains(40, 60));
}
#[test]
fn rect_extreme_coordinates_do_not_overflow() {
    assert!(!rect(i32::MAX, i32::MIN, u32::MAX, u32::MAX).contains(0, 0));
}
#[test]
fn invalid_dimensions_rejected() {
    assert!(Image::new(0, 4).is_err());
    assert!(Image::new(u32::MAX, u32::MAX).is_err());
    assert!(Image::new(MAX_PIXELS as u32, 2).is_err());
}
#[test]
fn crop_correct_pixels() {
    let mut i = Image::new(4, 3).unwrap();
    for (n, p) in i.rgba.chunks_exact_mut(4).enumerate() {
        p[0] = n as u8;
    }
    let c = i.crop(rect(1, 1, 2, 2)).unwrap();
    assert_eq!((c.width, c.height), (2, 2));
    assert_eq!(c.pixel(0, 0)[0], 5);
    assert_eq!(c.pixel(1, 1)[0], 10);
}
#[test]
fn empty_crop_rejected() {
    assert!(Image::new(4, 4).unwrap().crop(rect(20, 20, 2, 2)).is_err());
}
#[test]
fn png_roundtrip() {
    let mut i = Image::new(21, 13).unwrap();
    i.rgba[0..4].copy_from_slice(&[2, 19, 80, 255]);
    let p = temp("png");
    i.save(&p).unwrap();
    assert_eq!(Image::load_png(&p).unwrap(), i);
    let _ = std::fs::remove_file(p);
}
#[test]
fn jpeg_and_webp_are_real_encoded_files() {
    let i = Image::new(37, 29).unwrap();
    for (ext, magic) in [("jpg", &b"\xff\xd8"[..]), ("webp", &b"RIFF"[..])] {
        let p = temp(ext);
        i.save(&p).unwrap();
        let b = std::fs::read(&p).unwrap();
        assert!(b.starts_with(magic));
        let _ = std::fs::remove_file(p);
    }
}
#[test]
fn unknown_export_format_rejected() {
    let p = temp("exe");
    assert!(Image::new(1, 1).unwrap().save(&p).is_err());
    let _ = std::fs::remove_file(p);
}
#[test]
fn add_undo_redo_preserves_shape() {
    let mut e = Editor::new(Image::new(100, 100).unwrap());
    let a = shape(Tool::Arrow, (1., 2.), (30., 40.));
    e.add(a.clone());
    e.undo();
    assert!(e.annotations.is_empty());
    e.redo();
    assert_eq!(e.annotations, vec![a]);
}
#[test]
fn new_edit_invalidates_redo() {
    let mut e = Editor::new(Image::new(10, 10).unwrap());
    e.add(shape(Tool::Rectangle, (0., 0.), (4., 4.)));
    e.undo();
    e.add(shape(Tool::Ellipse, (1., 1.), (3., 3.)));
    e.redo();
    assert_eq!(e.annotations.len(), 1);
    assert_eq!(e.annotations[0].tool, Tool::Ellipse);
}
#[test]
fn delete_undo_retains_z_order() {
    let mut e = Editor::new(Image::new(10, 10).unwrap());
    for t in [Tool::Rectangle, Tool::Ellipse, Tool::Arrow] {
        e.add(shape(t, (1., 1.), (5., 5.)))
    }
    let before = e.annotations.clone();
    e.selected = Some(1);
    e.delete_selected();
    e.undo();
    assert_eq!(e.annotations, before);
    e.redo();
    assert_eq!(e.annotations.len(), 2);
}
#[test]
fn moving_is_undoable() {
    let mut e = Editor::new(Image::new(50, 50).unwrap());
    let a = shape(Tool::Text, (5., 5.), (15., 20.));
    e.add(a.clone());
    let mut b = a.clone();
    b.translate(10., -3.);
    e.replace(0, a.clone(), b.clone());
    e.undo();
    assert_eq!(e.annotations[0], a);
    e.redo();
    assert_eq!(e.annotations[0], b);
}
#[test]
fn undo_is_not_capped_at_small_count() {
    let mut e = Editor::new(Image::new(10, 10).unwrap());
    for _ in 0..1500 {
        e.add(shape(Tool::Line, (0., 0.), (4., 4.)))
    }
    for _ in 0..1500 {
        e.undo()
    }
    assert!(e.annotations.is_empty());
    for _ in 0..1500 {
        e.redo()
    }
    assert_eq!(e.annotations.len(), 1500);
}
#[test]
fn selection_is_clipped_to_image() {
    let mut e = Editor::new(Image::new(20, 20).unwrap());
    e.selection = Some(rect(-5, -5, 30, 40));
    assert_eq!(e.selection(), Some(rect(0, 0, 20, 20)));
}
#[test]
fn pick_topmost_shape() {
    let mut e = Editor::new(Image::new(50, 50).unwrap());
    e.add(shape(Tool::Rectangle, (5., 5.), (20., 20.)));
    e.add(shape(Tool::Arrow, (5., 5.), (20., 20.)));
    e.pick(Point::new(10., 10.));
    assert_eq!(e.selected, Some(1));
}
#[test]
fn redaction_is_burned_into_export() {
    let mut i = Image::new(64, 64).unwrap();
    i.rgba
        .chunks_exact_mut(4)
        .for_each(|p| p.copy_from_slice(&[244, 40, 91, 255]));
    let s = shape(Tool::Redact, (8., 8.), (40., 40.));
    let r = Renderer::new().flatten(&i, &[s]);
    assert_eq!(r.pixel(20, 20), [24, 24, 27, 255]);
    assert_eq!(r.pixel(2, 2), [244, 40, 91, 255]);
    assert_eq!(i.pixel(20, 20), [244, 40, 91, 255]);
}
#[test]
fn blur_and_pixelate_only_touch_their_rect() {
    let mut i = Image::new(40, 40).unwrap();
    for (y, row) in i.rgba.chunks_exact_mut(40 * 4).enumerate() {
        for (x, p) in row.chunks_exact_mut(4).enumerate() {
            p[0] = (x * 5) as u8;
            p[1] = (y * 5) as u8;
        }
    }
    for t in [Tool::Blur, Tool::Pixelate] {
        let out = Renderer::new().flatten(&i, &[shape(t, (10., 10.), (30., 30.))]);
        assert_eq!(out.pixel(0, 0), i.pixel(0, 0));
        assert_eq!(out.pixel(39, 39), i.pixel(39, 39));
        assert_ne!(out.pixel(10, 10), i.pixel(10, 10));
    }
}
#[test]
fn all_annotation_tools_render_without_panicking() {
    let mut i = Image::new(300, 300).unwrap();
    let r = Renderer::new();
    for t in [
        Tool::Pen,
        Tool::Line,
        Tool::Arrow,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Highlight,
        Tool::Text,
        Tool::Blur,
        Tool::Pixelate,
        Tool::Redact,
        Tool::Step,
    ] {
        let mut a = shape(t, (20., 20.), (110., 100.));
        a.text = "Привет / Hello".into();
        r.annotation(&mut i, &a);
    }
    assert!(i.validate());
}
#[test]
fn step_numbers_follow_existing_maximum() {
    let mut e = Editor::new(Image::new(10, 10).unwrap());
    let mut a = shape(Tool::Step, (1., 1.), (1., 1.));
    a.number = 7;
    e.add(a);
    assert_eq!(e.next_number(), 8);
}
#[test]
fn toolbar_stays_inside_desktop() {
    for (w, h) in [(1280, 800), (1920, 1080), (3840, 2160), (800, 600)] {
        for selection in [
            None,
            Some(rect(0, 0, 400, 300)),
            Some(rect(w as i32 - 50, h as i32 - 50, 40, 40)),
        ] {
            let l = ui::Layout::new(w, h, selection);
            assert!(l.panel.x >= 0 && l.panel.y >= 0);
            assert!(l.panel.x + l.panel.w as i32 <= w as i32);
            assert!(l.panel.y + l.panel.h as i32 <= h as i32);
            for b in l.buttons {
                assert!(l.panel.contains(b.rect.x, b.rect.y));
            }
        }
    }
}
#[test]
fn qr_decodes_local_fixture() {
    let i = Image::load_png(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/qr-fixture.png"
    )))
    .unwrap();
    assert_eq!(i.qr(), vec!["LUMA-QR-TEST-2026"]);
}
#[test]
fn qr_blank_image_has_no_results() {
    assert!(Image::new(100, 100).unwrap().qr().is_empty());
}
#[test]
fn safe_link_scheme_is_limited() {
    assert!(util::safe_web_url("https://example.org/x"));
    assert!(!util::safe_web_url("javascript:alert(1)"));
    assert!(!util::safe_web_url("file:///etc/passwd"));
    assert!(!util::safe_web_url("https://a/\n"));
}
#[test]
fn utc_conversion_and_encoding() {
    assert_eq!(
        util::utc_stamp(0),
        ("19700101".into(), "19700101T000000Z".into())
    );
    assert_eq!(util::utc_stamp(1440938160).1, "20150830T123600Z");
    assert_eq!(util::percent("a b/я"), "a%20b%2F%D1%8F");
}
#[test]
fn config_rejects_invalid_export_and_values() {
    let mut c = Config::default();
    assert!(c.validate().is_ok());
    c.format = "exe".into();
    assert!(c.validate().is_err());
    c.format = "png".into();
    c.history_limit = 0;
    assert!(c.validate().is_err());
}
#[test]
fn no_network_is_needed_to_render_ui() {
    let e = ui::demo();
    let r = Renderer::new();
    for dark in [true, false] {
        let (img, layout) = ui::overlay(
            &r,
            &e,
            ui::View::fit(&e.image, 1280, 800),
            dark,
            (0, 0),
            "",
            false,
        );
        assert!(img.validate());
        assert_eq!(
            layout
                .buttons
                .iter()
                .filter(|b| matches!(b.action, ui::Action::Tool(_)))
                .count(),
            11
        );
    }
}
fn scroll_pattern(w: u32, h: u32) -> Image {
    let mut i = Image::new(w, h).unwrap();
    for y in 0..h {
        for x in 0..w {
            let n = y.wrapping_mul(7919) ^ x.wrapping_mul(104729) ^ y.wrapping_mul(x);
            let p = (y as usize * w as usize + x as usize) * 4;
            i.rgba[p..p + 4].copy_from_slice(&[
                (n % 251) as u8,
                ((n / 13) % 253) as u8,
                ((n / 177) % 255) as u8,
                255,
            ]);
        }
    }
    i
}
#[test]
fn scrolling_overlap_is_detected() {
    let base = scroll_pattern(128, 560);
    let a = base.crop(rect(0, 0, 128, 360)).unwrap();
    let b = base.crop(rect(0, 120, 128, 360)).unwrap();
    assert_eq!(stitch::offset(&a, &b), Some(120));
    let out = stitch::append(&a, &b, 120).unwrap();
    assert_eq!(out, base.crop(rect(0, 0, 128, 480)).unwrap());
}
#[test]
fn identical_scroll_frames_stop() {
    let i = scroll_pattern(128, 360);
    assert_eq!(stitch::offset(&i, &i), None);
}
#[test]
fn scrolling_mismatched_dimensions_rejected() {
    assert!(stitch::append(&Image::new(3, 3).unwrap(), &Image::new(4, 3).unwrap(), 1).is_err());
}
#[test]
fn history_ids_do_not_allow_path_traversal() {
    let e = luma_shot::history::Entry {
        id: "../../secret".into(),
        created: 0,
        width: 1,
        height: 1,
        text: "".into(),
        note: "".into(),
    };
    assert!(luma_shot::history::path(&e).is_err());
}
#[test]
fn history_search_is_case_insensitive_and_ocr_aware() {
    let entries = vec![luma_shot::history::Entry {
        id: "abc".into(),
        created: 0,
        width: 1920,
        height: 1080,
        text: "Ошибка подключения Database".into(),
        note: "".into(),
    }];
    assert_eq!(
        luma_shot::history::search(&entries, "ОШИБКА database").len(),
        1
    );
    assert!(luma_shot::history::search(&entries, "missing").is_empty());
}
#[test]
fn no_secrets_in_serialized_config() {
    let c = Config::default();
    let j = serde_json::to_string(&c).unwrap();
    assert!(!j.contains("api_key"));
    assert!(!j.contains("bearer_token"));
    assert!(!j.contains("secret_access_key"));
}
#[test]
fn step_label_ink_has_aa_contrast_for_palette() {
    let lum = |c: [u8; 4]| {
        let f = |x: u8| {
            let v = x as f64 / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2])
    };
    for c in ui::COLORS {
        let ink = luma_shot::render::contrast_ink(c);
        let a = lum(c);
        let b = lum(ink);
        assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5);
    }
}
