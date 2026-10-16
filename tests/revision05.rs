use luma_shot::{
    config::Config,
    editor::{Tool, TOOLS},
    image::{Image, Rect},
    ui::{self, Action},
};
#[test]
fn attachment_edges_single_swatch_and_independent_qr() {
    luma_shot::i18n::configure("ru", "lightshot");
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
            x: 1200,
            y: 830,
            w: 80,
            h: 70,
        },
        Rect {
            x: 1,
            y: 1,
            w: 10,
            h: 10,
        },
        Rect {
            x: 400,
            y: 400,
            w: 5,
            h: 5,
        },
    ] {
        let l = ui::Layout::new(1280, 900, Some(r));
        assert_eq!(l.surfaces.len(), 2);
        assert!(
            !ui::overlap(l.surfaces[0], l.surfaces[1]),
            "{:?}",
            l.surfaces
        );
        assert_eq!(
            l.buttons
                .iter()
                .filter(|b| b.action == Action::PickColor)
                .count(),
            1
        );
        assert!(!l
            .buttons
            .iter()
            .any(|b| matches!(b.action, Action::Color(_))));
        assert!(l.buttons.iter().any(|b| b.action == Action::Qr));
        for t in TOOLS {
            assert!(l.buttons.iter().any(|b| b.action == Action::Tool(t)));
        }
        for b in &l.buttons {
            assert!(l.surfaces.iter().any(|s| s.contains(b.rect.x, b.rect.y)
                && s.contains(
                    b.rect.x + b.rect.w as i32 - 1,
                    b.rect.y + b.rect.h as i32 - 1
                )));
            assert!(
                b.rect.x >= 0
                    && b.rect.y >= 0
                    && b.rect.x + b.rect.w as i32 <= 1280
                    && b.rect.y + b.rect.h as i32 <= 900
            );
        }
        let badge = ui::size_badge(r);
        let (x, y) = ui::badge_position(r, &badge, 1280, 900);
        let badge_rect = Rect {
            x,
            y,
            w: badge.width,
            h: badge.height,
        };
        assert!(
            !l.surfaces.iter().any(|p| ui::overlap(*p, badge_rect)),
            "Size badge hidden by toolbar for {r:?}"
        );
        assert!(
            x >= 0 && y >= 0 && x + badge.width as i32 <= 1280 && y + badge.height as i32 <= 900
        );
    }
    let r = Rect {
        x: 188,
        y: 177,
        w: 888,
        h: 436,
    };
    let l = ui::Layout::new(1280, 900, Some(r));
    assert_eq!(l.surfaces[0].x, 1080);
    assert_eq!(l.surfaces[1].y, 617);
    assert!(!l.contains(500, 400)); // the area between rails is the canvas, not a draggable toolbar
    luma_shot::i18n::configure("ru", "compact");
    for r in [
        Rect {
            x: 1,
            y: 1,
            w: 10,
            h: 10,
        },
        Rect {
            x: 1160,
            y: 700,
            w: 110,
            h: 180,
        },
    ] {
        let l = ui::Layout::new(1280, 900, Some(r));
        let badge = ui::size_badge(r);
        let (x, y) = ui::badge_position(r, &badge, 1280, 900);
        assert!(!ui::overlap(
            l.panel,
            Rect {
                x,
                y,
                w: badge.width,
                h: badge.height
            }
        ));
    }
}
#[test]
fn per_tool_colors_roundtrip_and_redaction_always_opaque() {
    let mut c = Config::default();
    c.remember_color(Tool::Arrow, [210, 45, 90, 1]);
    c.remember_color(Tool::Pen, [12, 50, 80, 255]);
    c.remember_color(Tool::Redact, [255, 255, 255, 0]);
    let c: Config = serde_json::from_slice(&serde_json::to_vec(&c).unwrap()).unwrap();
    assert_eq!(c.color_for(Tool::Arrow), [210, 45, 90, 255]);
    assert_eq!(c.color_for(Tool::Pen), [12, 50, 80, 255]);
    assert_eq!(c.color_for(Tool::Redact), [24, 24, 27, 255]);
    assert!(c.validate().is_ok());
}
#[test]
fn fast_google_payload_is_bounded_jpeg_original_is_exact_png() {
    let p = luma_shot::google::encode(Image::new(3840, 2160).unwrap(), true).unwrap();
    assert_eq!((p.width, p.height), (2048, 1152));
    assert_eq!(&p.bytes[..2], &[255, 216]);
    let i = Image::new(4, 5).unwrap();
    let p = luma_shot::google::encode(i.clone(), false).unwrap();
    assert_eq!(p.bytes, i.png().unwrap());
    assert_eq!((p.width, p.height), (4, 5));
}
