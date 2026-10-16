use luma_shot::{
    config::Config,
    editor::{self, Editor, Point, Tool, TOOLS},
    hotkey,
    image::{Image, Rect},
    ui::{self, Action},
};
#[test]
fn pen_default_order_and_dedicated_qr() {
    assert_eq!(Editor::new(Image::new(10, 10).unwrap()).tool, Tool::Pen);
    assert_eq!(TOOLS[0], Tool::Pen);
    assert!(!TOOLS.contains(&Tool::Select));
    let i = TOOLS.iter().position(|t| *t == Tool::Picker).unwrap();
    assert_eq!(TOOLS[i + 1], Tool::Move);
    for style in ["edge", "compact"] {
        luma_shot::i18n::configure("ru", style);
        let l = ui::Layout::new(
            1280,
            900,
            Some(Rect {
                x: 100,
                y: 100,
                w: 800,
                h: 500,
            }),
        );
        let actions: Vec<_> = l
            .buttons
            .iter()
            .map(|b| b.action)
            .filter(|a| {
                !matches!(
                    a,
                    Action::Tool(_)
                        | Action::PickColor
                        | Action::Width
                        | Action::Undo
                        | Action::Redo
                )
            })
            .collect();
        assert_eq!(actions[0], Action::Lens);
        assert!(actions.contains(&Action::Qr));
        assert!(actions.contains(&Action::Ocr));
        assert!(!l
            .buttons
            .iter()
            .any(|b| b.action == Action::Tool(Tool::Select)));
    }
}
#[test]
fn all_eight_selection_edges_resize_without_scaling_annotations() {
    let r = Rect {
        x: 100,
        y: 100,
        w: 200,
        h: 150,
    };
    for (x, y, edge) in [
        (100., 150., 1),
        (300., 150., 2),
        (200., 100., 4),
        (200., 250., 8),
        (100., 100., 5),
        (300., 100., 6),
        (100., 250., 9),
        (300., 250., 10),
    ] {
        assert_eq!(editor::selection_edge(r, Point::new(x, y), 5.), Some(edge));
        let p = Point::new(x + 12., y + 8.);
        let n = editor::resize_selection(r, edge, p, 1280, 900);
        assert!(n.valid());
        assert_ne!(n, r);
    }
    assert_eq!(editor::selection_edge(r, Point::new(200., 160.), 5.), None);
    let n = editor::resize_selection(r, 2 | 8, Point::new(9999., 9999.), 1280, 900);
    assert_eq!(
        n,
        Rect {
            x: 100,
            y: 100,
            w: 1180,
            h: 800
        }
    );
    let n = editor::resize_selection(r, 1 | 4, Point::new(9999., 9999.), 1280, 900);
    assert_eq!((n.w, n.h), (1, 1));
}
#[test]
fn compact_dimensions_and_qr_rows_do_not_cover_panels() {
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
            let b = ui::size_badge(r);
            assert_eq!(b.height, 22);
            assert!(b.width <= 116);
            let Some(q) = ui::qr_result_rect(&l, r, 1280, 900) else {
                assert!(r.w < 104 || r.h < 32);
                continue;
            };
            assert!(!l.surfaces.iter().any(|p| ui::overlap(*p, q)));
            assert!(q.x >= 0 && q.y >= 0 && q.x + q.w as i32 <= 1280 && q.y + q.h as i32 <= 900);
        }
    }
}
#[test]
fn custom_hotkeys_validate_and_persist() {
    for (s, m, k) in [
        ("PrintScreen", 0, 44),
        ("Ctrl+Alt+S", 3, 83),
        ("shift + f9", 4, 120),
        ("Win+Shift+4", 12, 52),
        ("A", 0, 65),
    ] {
        assert_eq!(
            hotkey::parse(s).unwrap(),
            hotkey::Hotkey {
                modifiers: m,
                key: k
            }
        );
    }
    for s in [
        "",
        "Ctrl",
        "Ctrl+Ctrl+A",
        "Ctrl+A+B",
        "Ctrl+Shift+R",
        "F12",
        "F25",
        "Delete+Something",
    ] {
        assert!(hotkey::parse(s).is_err(), "{s}");
    }
    let mut c = Config::default();
    c.capture_hotkey = "Ctrl+Alt+S".into();
    c.video_cursor = false;
    assert!(c.validate().is_ok());
    let c: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(c.capture_hotkey, "Ctrl+Alt+S");
    assert!(!c.video_cursor);
}
#[test]
fn tool_icons_retain_distinct_colors_including_active_tool() {
    luma_shot::i18n::configure("ru", "edge");
    let mut e = ui::demo();
    e.tool = Tool::Pen;
    e.tool_colors.insert("P".into(), [201, 40, 89, 255]);
    e.tool_colors.insert("A".into(), [40, 190, 100, 255]);
    let l = ui::Layout::new(1280, 800, e.selection());
    let r = luma_shot::render::Renderer::new();
    let panels = ui::panel_images(&r, &e, &l, true, (0, 0), 1);
    for rgb in [[201, 40, 89, 255], [40, 190, 100, 255]] {
        assert!(panels
            .iter()
            .any(|(i, _)| i.rgba.chunks_exact(4).any(|p| p == rgb)));
    }
}
