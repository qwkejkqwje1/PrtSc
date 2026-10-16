use luma_shot::{
    hotkey,
    image::Rect,
    ui::{self, Action},
};
#[test]
fn qr_row_is_inside_selection_and_close_to_qr_button() {
    for style in ["edge", "compact"] {
        luma_shot::i18n::configure("ru", style);
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
            Rect {
                x: 0,
                y: 0,
                w: 10,
                h: 10,
            },
        ] {
            let l = ui::Layout::new(1280, 900, Some(r));
            let q = ui::qr_result_rect(&l, r, 1280, 900);
            if r.w < 104 || r.h < 32 {
                assert!(q.is_none());
                continue;
            }
            let q = q.expect("row fits inside normal region");
            assert!(q.x >= r.x && q.y >= r.y);
            assert!(q.x + q.w as i32 <= r.x + r.w as i32 && q.y + q.h as i32 <= r.y + r.h as i32);
            assert!(!l.surfaces.iter().any(|p| ui::overlap(*p, q)));
            let button = l
                .buttons
                .iter()
                .find(|b| b.action == Action::Qr)
                .unwrap()
                .rect;
            assert!((button.y - q.y).abs() < 200);
        }
    }
}
#[test]
fn snapshot_vk_is_supported_with_or_without_modifiers() {
    for m in 0..16 {
        let s = hotkey::format_event(m, 0x2c).unwrap();
        assert!(s.ends_with("PrintScreen"));
        assert_eq!(hotkey::parse(&s).unwrap().key, 0x2c);
    }
}
