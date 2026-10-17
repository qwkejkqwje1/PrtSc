use luma_shot::{
    damage,
    editor::{Annotation, Point, Tool},
    image::{Image, Rect},
    render::{self, Renderer},
    text, video,
};
fn base() -> Image {
    let mut i = Image::new(800, 600).unwrap();
    for p in i.rgba.chunks_exact_mut(4) {
        p.copy_from_slice(&[255, 255, 255, 255]);
    }
    i
}
fn assert_damage(a: &Annotation) {
    let r = Renderer::new();
    let before = base();
    let mut after = before.clone();
    r.annotation(&mut after, a);
    let bounds = r.bounds(a);
    let mut changed = 0;
    for y in 0..after.height {
        for x in 0..after.width {
            if before.pixel(x, y) != after.pixel(x, y) {
                changed += 1;
                assert!(
                    bounds.contains(x as i32, y as i32),
                    "{:?} pixel {x},{y} outside {:?}",
                    a.tool,
                    bounds
                );
            }
        }
    }
    assert!(changed > 0, "{:?} must paint", a.tool);
}
#[test]
fn text_bounds_match_rendered_ink_and_multiline_hitbox() {
    assert!(text::font().is_some(), "font required for text QA");
    for width in [1., 2., 3., 8.] {
        for s in ["Wgjёж", "ЙЖЩ\nsecond\nтретий", "jÁg😀\n", "  W  "] {
            let mut a = Annotation::new(Tool::Text, Point::new(80.5, 90.5), [0, 0, 0, 255], width);
            a.text = s.into();
            assert_damage(&a);
            assert!(a.rect().h >= (s.split('\n').count() as f32 * text::size(width) * 1.4) as u32);
            if s.contains('\n') {
                assert!(a.hit(Point::new(83., 90. + text::size(width) * 1.5)));
            }
        }
    }
}
#[test]
fn empty_text_and_caret_have_visible_line_box() {
    assert!(text::bounds("", 14.).h >= 19);
    let (x, y) = text::caret("Hello\nW", 14.);
    assert!(x > 0. && (y - 19.6).abs() < 0.01);
}
#[test]
fn annotation_damage_covers_all_painted_pixels() {
    for tool in [
        Tool::Pen,
        Tool::Line,
        Tool::Arrow,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Highlight,
        Tool::Redact,
        Tool::Step,
    ] {
        for width in [1., 3., 12.] {
            for reverse in [false, true] {
                let mut a =
                    Annotation::new(tool, Point::new(120.7, 180.2), [40, 130, 220, 255], width);
                a.points[1] = if reverse {
                    Point::new(100.3, 60.8)
                } else {
                    Point::new(230.2, 110.6)
                };
                a.number = 1001;
                assert_damage(&a);
            }
        }
    }
}
#[test]
fn append_damage_patch_matches_full_render() {
    let r = Renderer::new();
    let mut annotations = vec![];
    let mut incremental = base();
    for tool in [
        Tool::Pen,
        Tool::Arrow,
        Tool::Text,
        Tool::Step,
        Tool::Highlight,
        Tool::Redact,
    ] {
        let mut a = Annotation::new(tool, Point::new(100., 120.), [230, 160, 30, 255], 3.);
        a.points[1] = Point::new(270., 280.);
        a.text = "Текст\nWgj".into();
        let mut full = incremental.clone();
        r.annotation(&mut full, &a);
        let bounds = r.bounds(&a).clamp(full.width, full.height);
        for y in bounds.y as u32..bounds.y as u32 + bounds.h {
            for x in bounds.x as u32..bounds.x as u32 + bounds.w {
                let n = ((y * full.width + x) * 4) as usize;
                incremental.rgba[n..n + 4].copy_from_slice(&full.rgba[n..n + 4]);
            }
        }
        annotations.push(a);
        assert_eq!(
            incremental.rgba,
            r.flatten(&base(), &annotations).rgba,
            "{tool:?}"
        );
    }
}
#[test]
fn multiply_highlighter_keeps_black_ink_black() {
    let r = Renderer::new();
    let mut i = base();
    i.rgba[(150 * 800 + 150) * 4..(150 * 800 + 150) * 4 + 4].copy_from_slice(&[0, 0, 0, 255]);
    let mut a = Annotation::new(
        Tool::Highlight,
        Point::new(100., 100.),
        [255, 210, 20, 255],
        3.,
    );
    a.points[1] = Point::new(200., 200.);
    r.annotation(&mut i, &a);
    assert_eq!(i.pixel(150, 150), [0, 0, 0, 255]);
    assert!(i.pixel(140, 140)[2] < 255);
    r.annotation(&mut i, &a);
    assert_eq!(i.pixel(150, 150), [0, 0, 0, 255]);
}
#[test]
fn glyph_cache_reuses_entries_and_is_bounded() {
    let r = Renderer::new();
    let mut i = base();
    r.text(&mut i, 20., 20., "QR hello", 14., [0, 0, 0, 255]);
    let first = r.glyph_cache_stats();
    r.text(&mut i, 20., 20., "QR hello", 14., [0, 0, 0, 255]);
    assert_eq!(r.glyph_cache_stats(), first);
    for size in 8..1200 {
        r.text(&mut i, 0., 0., "W", size as f32 / 10., [0, 0, 0, 255]);
    }
    let (entries, bytes) = r.glyph_cache_stats();
    assert!(entries <= 1024 && bytes <= 4 * 1024 * 1024);
}
#[test]
fn elision_preserves_unicode_and_fits_available_width() {
    let r = Renderer::new();
    for w in [0., 10., 40., 120., 300.] {
        let s = r.elide("https://пример.рф/длинный-путь😀", 14., w);
        assert!(r.text_width(&s, 14.) <= w + 0.1 || s.is_empty());
    }
}
#[test]
fn damage_union_and_expansion_cover_old_and_new() {
    let a = Rect {
        x: 20,
        y: 30,
        w: 10,
        h: 20,
    };
    let b = Rect {
        x: -10,
        y: 80,
        w: 50,
        h: 10,
    };
    let u = damage::union(a, b);
    assert_eq!(
        u,
        Rect {
            x: -10,
            y: 30,
            w: 50,
            h: 60
        }
    );
    let e = damage::expand(a, 3);
    assert_eq!(
        e,
        Rect {
            x: 17,
            y: 27,
            w: 16,
            h: 26
        }
    );
}
#[test]
fn reusable_nv12_matches_bgra_and_preserves_capacity() {
    let mut image = Image::new(32, 24).unwrap();
    for (n, p) in image.rgba.chunks_exact_mut(4).enumerate() {
        p.copy_from_slice(&[(n * 3) as u8, (n * 7) as u8, (n * 11) as u8, 255]);
    }
    let mut bgra = image.rgba.clone();
    for p in bgra.chunks_exact_mut(4) {
        p.swap(0, 2);
    }
    let expected = video::nv12(&image).unwrap();
    let mut out = Vec::with_capacity(4096);
    let ptr = out.as_ptr();
    video::nv12_bgra_into(&bgra, 32, 24, &mut out).unwrap();
    assert_eq!(out, expected);
    assert_eq!(out.as_ptr(), ptr);
    video::nv12_into(&image, &mut out).unwrap();
    assert_eq!(out, expected);
    assert_eq!(out.as_ptr(), ptr);
}
#[test]
fn nv12_rejects_invalid_input_before_allocation() {
    let mut out = vec![1, 2];
    for (w, h, n) in [(0, 0, 0), (3, 2, 24), (2, 2, 15), (u32::MAX, u32::MAX, 0)] {
        assert!(video::nv12_bgra_into(&vec![0; n], w, h, &mut out).is_err());
        assert_eq!(out, [1, 2]);
    }
}
#[test]
fn sample_clock_has_no_gaps_when_capture_is_slow() {
    let mut clock = video::SampleClock::default();
    let ticks = [0, 666666, 1900000, 2000000];
    let mut samples = vec![];
    for t in ticks {
        if let Some(s) = clock.push(t) {
            samples.push(s);
        }
    }
    samples.push(clock.finish(2800000).unwrap());
    assert_eq!(
        samples,
        [
            (0, 666666),
            (666666, 1233334),
            (1900000, 100000),
            (2000000, 800000)
        ]
    );
    assert!(clock.finish(0).is_none());
    let mut clock = video::SampleClock::default();
    assert!(clock.push(0).is_none());
    assert_eq!(clock.push(0), Some((0, 1)));
    assert_eq!(clock.finish(0), Some((1, 1)));
}
#[test]
fn resize_into_reuses_target_and_matches_wrapper() {
    let src = base();
    let mut dst = Image::new(48, 32).unwrap();
    let ptr = dst.rgba.as_ptr();
    render::resize_into(&src, &mut dst).unwrap();
    assert_eq!(dst.rgba.as_ptr(), ptr);
    assert_eq!(dst.rgba, render::resized(&src, 48, 32).unwrap().rgba);
}
#[test]
fn final_manifest_and_provided_icon_are_used() {
    let manifest = include_str!("../assets/app.manifest");
    assert!(manifest.contains("Microsoft.Windows.Common-Controls"));
    assert!(manifest.contains("1.1.0.0"));
    let icon = include_bytes!("../assets/luma.ico");
    assert_eq!(icon.len(), 31680);
    assert_eq!(&icon[..6], &[0, 0, 1, 0, 8, 0]);
}

#[test]
fn typed_toolbar_style_handles_legacy_configuration_names() {
    use luma_shot::ui::{Layout, ToolbarStyle, COLORS};
    for old in ["edge", "lightshot", "minimal"] {
        assert_eq!(ToolbarStyle::parse(old), ToolbarStyle::Edge);
    }
    assert_eq!(ToolbarStyle::parse("standard"), ToolbarStyle::Compact);
    let edge = Layout::for_style(1280, 900, None, &COLORS, ToolbarStyle::Edge);
    let compact = Layout::for_style(1280, 900, None, &COLORS, ToolbarStyle::Compact);
    assert_eq!(edge.surfaces.len(), 2);
    assert_eq!(compact.surfaces.len(), 1);
}
#[test]
fn blur_and_pixelate_are_keyboard_accessible_without_growing_toolbar() {
    use luma_shot::editor::{tool_for_key, TOOLS};
    assert_eq!(tool_for_key(b'B'), Some(Tool::Blur));
    assert_eq!(tool_for_key(b'X'), Some(Tool::Pixelate));
    assert_eq!(TOOLS.len(), 11);
    assert_eq!(tool_for_key(b'P'), Some(Tool::Pen));
}
