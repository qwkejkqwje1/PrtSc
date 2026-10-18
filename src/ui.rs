use crate::{
    editor::{Annotation, Editor, Point, Tool, TOOLS},
    image::{Image, Rect},
    render::{self, Renderer},
};
/// Typed layout choice; old names are accepted only at the configuration boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarStyle {
    Edge,
    Compact,
}
impl ToolbarStyle {
    pub fn parse(value: &str) -> Self {
        match value {
            "edge" | "lightshot" | "minimal" => Self::Edge,
            _ => Self::Compact,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    NewCapture,
    Record,
    Upload,
    Qr,
    Pin,
    PickColor,
    Cancel,
    Tool(Tool),
    Color([u8; 4]),
    Width,
    Undo,
    Redo,
    Copy,
    Save,
    Lens,
    Translate,
    Ocr,
    More,
}
#[derive(Clone)]
pub struct Button {
    pub rect: Rect,
    pub action: Action,
    pub label: String,
    pub hint: String,
}
pub struct Layout {
    pub panel: Rect,
    pub buttons: Vec<Button>,
    pub surfaces: Vec<Rect>,
}
pub const COLORS: [[u8; 4]; 7] = [
    [39, 131, 222, 255],
    [236, 86, 76, 255],
    [240, 177, 47, 255],
    [67, 173, 115, 255],
    [161, 118, 220, 255],
    [255, 255, 255, 255],
    [24, 24, 27, 255],
];
impl Layout {
    pub fn new(w: u32, h: u32, selection: Option<Rect>) -> Self {
        Self::with_palette(w, h, selection, &COLORS)
    }
    pub fn with_palette(w: u32, h: u32, selection: Option<Rect>, colors: &[[u8; 4]; 7]) -> Self {
        let style = if crate::i18n::lightshot() {
            ToolbarStyle::Edge
        } else {
            ToolbarStyle::Compact
        };
        Self::for_style(w, h, selection, colors, style)
    }
    pub fn for_style(
        w: u32,
        h: u32,
        selection: Option<Rect>,
        _colors: &[[u8; 4]; 7],
        style: ToolbarStyle,
    ) -> Self {
        let split = style == ToolbarStyle::Edge;
        let selection = selection.unwrap_or(Rect {
            x: 32,
            y: 32,
            w: w.saturating_sub(96).max(1),
            h: h.saturating_sub(96).max(1),
        });
        let mut buttons = vec![];
        let mut add = |rect: Rect, action: Action, hint: &str| {
            buttons.push(Button {
                rect,
                action,
                label: String::new(),
                hint: crate::i18n::tr(hint).into(),
            })
        };
        let clamp = |x: i32, y: i32, ww: u32, hh: u32| Rect {
            x: x.clamp(4, (w as i32 - ww as i32 - 4).max(4)),
            y: y.clamp(4, (h as i32 - hh as i32 - 4).max(4)),
            w: ww,
            h: hh,
        };
        let (surfaces, tools, actions) = if split {
            let tw = 36;
            let th = 428u32.min(h.saturating_sub(8));
            let mut tx = selection.x + selection.w as i32 + 4;
            if tx + tw as i32 + 4 > w as i32 {
                tx = if selection.x >= tw as i32 + 8 {
                    selection.x - tw as i32 - 4
                } else {
                    selection.x + selection.w as i32 - tw as i32 - 4
                };
            }
            let mut rail = clamp(tx, selection.y + selection.h as i32 - th as i32, tw, th);
            let bw = 360u32.min(w.saturating_sub(8));
            let bh = 36;
            let mut by = selection.y + selection.h as i32 + 4;
            if by + bh as i32 + 4 > h as i32 {
                by = if selection.h >= bh + 8 {
                    selection.y + selection.h as i32 - bh as i32 - 4
                } else {
                    // Leave the size badge visible above the selection.
                    selection.y - bh as i32 - 30
                };
            }
            let mut bar = clamp(selection.x + selection.w as i32 - bw as i32, by, bw, bh);
            // Keep the pixel-size badge readable even for tiny, top-edge selections.
            let image = size_badge(selection);
            let (bx, by) = badge_position(selection, &image, w, h);
            let badge = Rect {
                x: bx,
                y: by,
                w: image.width,
                h: image.height,
            };
            if overlap(bar, badge) {
                for y in [badge.y + badge.h as i32 + 4, badge.y - bar.h as i32 - 4] {
                    if y >= 4 && y + bar.h as i32 <= h as i32 - 4 {
                        bar.y = y;
                        break;
                    }
                }
            }
            if overlap(rail, bar) || overlap(rail, badge) {
                let candidates = [
                    rail.y,
                    bar.y - rail.h as i32 - 4,
                    bar.y + bar.h as i32 + 4,
                    badge.y + badge.h as i32 + 4,
                    badge.y - rail.h as i32 - 4,
                    4,
                    h as i32 - rail.h as i32 - 4,
                ];
                let mut found = false;
                for y in candidates {
                    let candidate = Rect { y, ..rail };
                    if y >= 4
                        && y + rail.h as i32 <= h as i32 - 4
                        && !overlap(candidate, bar)
                        && !overlap(candidate, badge)
                    {
                        rail = candidate;
                        found = true;
                        break;
                    }
                }
                if !found && overlap(rail, bar) {
                    if rail.x >= bar.w as i32 + 8 {
                        bar.x = rail.x - bar.w as i32 - 4;
                    } else {
                        bar.x = (rail.x + rail.w as i32 + 4).min(w as i32 - bar.w as i32 - 4);
                    }
                }
            }
            (vec![rail, bar], rail, bar)
        } else {
            let ww = 448u32.min(w.saturating_sub(8));
            let hh = 76;
            let mut y = selection.y + selection.h as i32 + 4;
            if y + hh as i32 + 4 > h as i32 {
                y = if selection.h >= hh + 8 {
                    selection.y + selection.h as i32 - hh as i32 - 4
                } else {
                    selection.y - hh as i32 - 30
                };
            }
            if selection.y < 26 {
                y = y.max(selection.y + 30);
            }
            let bar = clamp(selection.x + selection.w as i32 - ww as i32, y, ww, hh);
            (vec![bar], bar, bar)
        };
        let mut draw_actions: Vec<(Action, String)> = TOOLS
            .iter()
            .map(|t| (Action::Tool(*t), format!("{} [{}]", t.label(), t.key())))
            .collect();
        draw_actions.extend([
            (
                Action::PickColor,
                "Цвет текущего инструмента — нажмите для выбора".into(),
            ),
            (Action::Width, "Толщина · колесо / + / −".into()),
            (Action::Undo, "Отменить · Ctrl+Z".into()),
            (Action::Redo, "Повторить · Ctrl+Shift+Z".into()),
        ]);
        for (i, (action, hint)) in draw_actions.into_iter().enumerate() {
            let rect = if split {
                let step = (tools.h - 8) / 15;
                Rect {
                    x: tools.x + 4,
                    y: tools.y + 4 + i as i32 * step as i32,
                    w: 28,
                    h: step.min(28),
                }
            } else {
                let step = (tools.w - 8) / 15;
                Rect {
                    x: tools.x + 4 + i as i32 * step as i32,
                    y: tools.y + 4,
                    w: step,
                    h: 32,
                }
            };
            add(rect, action, &hint);
        }
        for (i, (action, hint)) in [
            (
                Action::Lens,
                "Поиск по картинке · Ctrl+G · передача в Google",
            ),
            (
                Action::Translate,
                "Перевести картинку · Ctrl+Shift+T · вставить в Google",
            ),
            (Action::Copy, "Копировать · Enter / Ctrl+C"),
            (
                Action::Save,
                "Сохранить · Ctrl+S · Сохранить как: Ctrl+Shift+S",
            ),
            (
                Action::Upload,
                "Загрузить на выбранный фотохостинг · ссылка публичная",
            ),
            (Action::Ocr, "Распознать текст · Ctrl+T"),
            (Action::Qr, "QR · прочитать / скопировать · Ctrl+Q"),
            (Action::Pin, "Закрепить снимок · поверх всех окон"),
            (Action::Record, "Записать область в MP4/GIF · Ctrl+Shift+R"),
            (Action::More, "Другие действия · F1"),
            (Action::Cancel, "Закрыть · Esc"),
        ]
        .into_iter()
        .enumerate()
        {
            let step = (actions.w - 8) / 11;
            add(
                Rect {
                    x: actions.x + 4 + i as i32 * step as i32,
                    y: actions.y + if split { 4 } else { 40 },
                    w: step,
                    h: 28,
                },
                action,
                hint,
            );
        }
        let x = surfaces.iter().map(|r| r.x).min().unwrap();
        let y = surfaces.iter().map(|r| r.y).min().unwrap();
        let rr = surfaces.iter().map(|r| r.x + r.w as i32).max().unwrap();
        let bb = surfaces.iter().map(|r| r.y + r.h as i32).max().unwrap();
        Self {
            panel: Rect {
                x,
                y,
                w: (rr - x) as u32,
                h: (bb - y) as u32,
            },
            buttons,
            surfaces,
        }
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.surfaces.iter().any(|r| r.contains(x, y))
    }
    pub fn hit(&self, x: i32, y: i32) -> Option<Action> {
        self.buttons
            .iter()
            .find(|b| b.rect.contains(x, y))
            .map(|b| b.action)
    }
}
#[derive(Clone, Copy)]
pub struct View {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}
impl View {
    pub fn fit(image: &Image, w: u32, h: u32) -> Self {
        Self {
            width: w,
            height: h,
            scale: (w as f32 / image.width as f32)
                .min(h as f32 / image.height as f32)
                .min(1.),
        }
    }
    pub fn point(self, x: i32, y: i32) -> Point {
        Point::new(x as f32 / self.scale, y as f32 / self.scale)
    }
    pub fn rect(self, r: Rect) -> Rect {
        Rect {
            x: (r.x as f32 * self.scale) as i32,
            y: (r.y as f32 * self.scale) as i32,
            w: (r.w as f32 * self.scale) as u32,
            h: (r.h as f32 * self.scale) as u32,
        }
    }
}
pub fn overlay(
    renderer: &Renderer,
    editor: &Editor,
    view: View,
    dark: bool,
    cursor: (i32, i32),
    _status: &str,
    magnify: bool,
) -> (Image, Layout) {
    let mut scene = renderer.flatten(&editor.image, &editor.annotations);
    if let Some(a) = &editor.draft {
        renderer.annotation(&mut scene, a)
    }
    let mut img = if view.scale == 1. && scene.width == view.width && scene.height == view.height {
        scene
    } else {
        let mut out = Image::new(view.width, view.height).unwrap();
        out.rgba
            .chunks_exact_mut(4)
            .for_each(|p| p.copy_from_slice(&[21, 23, 27, 255]));
        let scaled = render::resized(
            &scene,
            (scene.width as f32 * view.scale).max(1.) as u32,
            (scene.height as f32 * view.scale).max(1.) as u32,
        )
        .unwrap();
        for y in 0..scaled.height.min(out.height) as usize {
            let n = scaled.width.min(out.width) as usize * 4;
            out.rgba[y * out.width as usize * 4..y * out.width as usize * 4 + n].copy_from_slice(
                &scaled.rgba[y * scaled.width as usize * 4..y * scaled.width as usize * 4 + n],
            );
        }
        out
    };
    let selection = editor.selection().map(|r| view.rect(r));
    for y in 0..img.height {
        for x in 0..img.width {
            if !selection
                .map(|r| r.contains(x as i32, y as i32))
                .unwrap_or(false)
            {
                let i = (y as usize * img.width as usize + x as usize) * 4;
                for k in 0..3 {
                    img.rgba[i + k] = (img.rgba[i + k] as u16 * 40 / 100) as u8
                }
            }
        }
    }
    if let Some(s) = selection {
        render::stroke_rect(&mut img, s, [80, 167, 255, 255], 1.5);
        for (x, y) in [
            (s.x, s.y),
            (s.x + s.w as i32, s.y),
            (s.x, s.y + s.h as i32),
            (s.x + s.w as i32, s.y + s.h as i32),
        ] {
            render::fill_rect(
                &mut img,
                Rect {
                    x: x - 3,
                    y: y - 3,
                    w: 6,
                    h: 6,
                },
                [255, 255, 255, 255],
            )
        }
        if let Some(i) = editor.selected {
            if let Some(a) = editor.annotations.get(i) {
                let r = view.rect(a.rect());
                render::stroke_rect(
                    &mut img,
                    Rect {
                        x: r.x - 5,
                        y: r.y - 5,
                        w: r.w + 10,
                        h: r.h + 10,
                    },
                    [255, 255, 255, 220],
                    1.,
                );
                for (x, y) in [
                    (r.x - 5, r.y - 5),
                    (r.x + r.w as i32 + 5, r.y - 5),
                    (r.x - 5, r.y + r.h as i32 + 5),
                    (r.x + r.w as i32 + 5, r.y + r.h as i32 + 5),
                ] {
                    render::fill_rect(
                        &mut img,
                        Rect {
                            x: x - 3,
                            y: y - 3,
                            w: 6,
                            h: 6,
                        },
                        [80, 167, 255, 255],
                    )
                }
            }
        }
    }
    let layout = Layout::new(view.width, view.height, selection);

    if let Some(sel) = selection {
        for (image, rect) in panel_images(renderer, editor, &layout, dark, cursor, 0) {
            paste(&mut img, &image, rect.x, rect.y);
        }
        let badge = size_badge(editor.selection().unwrap());
        let pos = badge_position(sel, &badge, view.width, view.height);
        paste(&mut img, &badge, pos.0, pos.1);
    }
    let surface = if dark {
        [29, 32, 39, 255]
    } else {
        [250, 250, 249, 255]
    };
    let text = if dark {
        [239, 242, 248, 255]
    } else {
        [38, 43, 50, 255]
    };
    if magnify {
        let x = (cursor.0 + 26).clamp(8, (img.width as i32 - 144).max(8));
        let y = (cursor.1 + 26).clamp(8, (img.height as i32 - 164).max(8));
        render::round_rect(
            &mut img,
            Rect {
                x,
                y,
                w: 132,
                h: 154,
            },
            7.,
            surface,
        );
        let point = view.point(cursor.0, cursor.1);
        let source = &editor.image;
        for yy in 0..15 {
            for xx in 0..15 {
                let sx = (point.x as i32 + xx - 7).clamp(0, source.width as i32 - 1) as u32;
                let sy = (point.y as i32 + yy - 7).clamp(0, source.height as i32 - 1) as u32;
                render::fill_rect(
                    &mut img,
                    Rect {
                        x: x + 6 + xx * 8,
                        y: y + 6 + yy * 8,
                        w: 8,
                        h: 8,
                    },
                    source.pixel(sx, sy),
                );
            }
        }
        render::stroke_rect(
            &mut img,
            Rect {
                x: x + 62,
                y: y + 62,
                w: 8,
                h: 8,
            },
            [255, 255, 255, 255],
            1.,
        );
        let c = source.pixel(point.x.max(0.) as u32, point.y.max(0.) as u32);
        renderer.text(
            &mut img,
            x as f32 + 10.,
            y as f32 + 133.,
            &format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]),
            13.,
            text,
        );
    }
    (img, layout)
}
pub fn paste(dst: &mut Image, src: &Image, x: i32, y: i32) {
    for yy in 0..src.height as i32 {
        let dy = y + yy;
        if dy < 0 || dy >= dst.height as i32 {
            continue;
        }
        for xx in 0..src.width as i32 {
            let dx = x + xx;
            if dx < 0 || dx >= dst.width as i32 {
                continue;
            }
            let si = (yy as usize * src.width as usize + xx as usize) * 4;
            let di = (dy as usize * dst.width as usize + dx as usize) * 4;
            let alpha = src.rgba[si + 3] as u32;
            for k in 0..3 {
                dst.rgba[di + k] = ((src.rgba[si + k] as u32 * alpha
                    + dst.rgba[di + k] as u32 * (255 - alpha))
                    / 255) as u8;
            }
        }
    }
}
pub fn overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w as i32
        && a.x + a.w as i32 > b.x
        && a.y < b.y + b.h as i32
        && a.y + a.h as i32 > b.y
}
pub fn panel_images(
    renderer: &Renderer,
    editor: &Editor,
    layout: &Layout,
    dark: bool,
    cursor: (i32, i32),
    qr_count: usize,
) -> Vec<(Image, Rect)> {
    let mut out = vec![];
    for rect in &layout.surfaces {
        let mut image = Image::new(rect.w, rect.h).unwrap();
        let surface = if dark {
            [29, 32, 39, 255]
        } else {
            [248, 249, 251, 255]
        };
        let ink = if dark {
            [239, 242, 248, 255]
        } else {
            [38, 43, 50, 255]
        };
        image
            .rgba
            .chunks_exact_mut(4)
            .for_each(|p| p.copy_from_slice(&surface));
        render::stroke_rect(
            &mut image,
            Rect {
                x: 0,
                y: 0,
                w: rect.w,
                h: rect.h,
            },
            if dark {
                [68, 74, 86, 255]
            } else {
                [188, 194, 202, 255]
            },
            1.,
        );
        for button in &layout.buttons {
            if !rect.contains(button.rect.x, button.rect.y) {
                continue;
            }
            let b = Rect {
                x: button.rect.x - rect.x,
                y: button.rect.y - rect.y,
                ..button.rect
            };
            let active = matches!(button.action,Action::Tool(t)if t==editor.tool);
            let hover = button.rect.contains(cursor.0, cursor.1);
            let primary = button.action == Action::Copy;
            if active || hover || primary {
                render::round_rect(
                    &mut image,
                    b,
                    3.,
                    if primary {
                        [39, 131, 222, 255]
                    } else if dark {
                        [54, 62, 75, 255]
                    } else {
                        [222, 232, 244, 255]
                    },
                );
            }
            if active {
                render::stroke_rect(&mut image, b, [39, 131, 222, 255], 1.);
            }
            let color = if active || primary {
                [255, 255, 255, 255]
            } else {
                ink
            };
            match button.action {
                Action::Tool(t) => {
                    let tc = if t == Tool::Redact {
                        [24, 24, 27, 255]
                    } else if [Tool::Move, Tool::Picker, Tool::Select].contains(&t) {
                        color
                    } else {
                        editor.tool_colors.get(t.key()).copied().unwrap_or(
                            if t == Tool::Highlight {
                                [240, 177, 47, 255]
                            } else {
                                [39, 131, 222, 255]
                            },
                        )
                    };
                    // A one-pixel contrasting shadow keeps black/white tool colors legible.
                    let shadow = if tc[0] as u32 + tc[1] as u32 + tc[2] as u32 > 390 {
                        [10, 12, 16, 255]
                    } else {
                        [245, 247, 250, 255]
                    };
                    let luminance = tc[0] as u32 + tc[1] as u32 + tc[2] as u32;
                    if luminance < 90 || luminance > 650 {
                        tool_icon(
                            &mut image,
                            renderer,
                            t,
                            Rect {
                                x: b.x + 1,
                                y: b.y + 1,
                                ..b
                            },
                            shadow,
                        );
                    }
                    tool_icon(&mut image, renderer, t, b, tc);
                }
                Action::PickColor | Action::Color(_) => {
                    let c = editor.color;
                    let sw = Rect {
                        x: b.x + (b.w as i32 - 18) / 2,
                        y: b.y + (b.h as i32 - 18) / 2,
                        w: 18,
                        h: 18,
                    };
                    render::fill_rect(&mut image, sw, [c[0], c[1], c[2], 255]);
                    render::stroke_rect(&mut image, sw, ink, 1.);
                }
                Action::Width => {
                    let label = format!("{}", editor.width as i32);
                    let tw = renderer.text_width(&label, 12.);
                    renderer.text(
                        &mut image,
                        b.x as f32 + (b.w as f32 - tw) / 2.,
                        b.y as f32 + (b.h as f32 - 12.) / 2.,
                        &label,
                        12.,
                        color,
                    );
                }
                Action::Undo | Action::Redo => {
                    let cx = b.x as f32 + b.w as f32 / 2.;
                    let cy = b.y as f32 + b.h as f32 / 2.;
                    let sign = if button.action == Action::Undo {
                        1.
                    } else {
                        -1.
                    };
                    let mut line =
                        Annotation::new(Tool::Pen, Point::new(cx + sign * 9., cy + 7.), color, 1.7);
                    line.points = vec![
                        Point::new(cx + sign * 9., cy + 7.),
                        Point::new(cx + sign * 9., cy),
                        Point::new(cx + sign * 5., cy - 4.),
                        Point::new(cx - sign * 8., cy - 4.),
                    ];
                    renderer.annotation(&mut image, &line);
                    line.points = vec![
                        Point::new(cx - sign * 3., cy - 9.),
                        Point::new(cx - sign * 8., cy - 4.),
                        Point::new(cx - sign * 3., cy + 1.),
                    ];
                    renderer.annotation(&mut image, &line);
                }
                a => action_icon(&mut image, renderer, a, b, color),
            }
            if button.action == Action::Qr && qr_count > 0 {
                render::round_rect(
                    &mut image,
                    Rect {
                        x: b.x + b.w as i32 - 8,
                        y: b.y + 1,
                        w: 6,
                        h: 6,
                    },
                    2.,
                    [40, 180, 102, 255],
                );
            }
        }
        out.push((image, *rect));
    }
    out
}
pub fn size_badge(selection: Rect) -> Image {
    let label = format!("{}x{}", selection.w, selection.h);
    let mut image = Image::new(label.len() as u32 * 12 + 8, 22).unwrap();
    image
        .rgba
        .chunks_exact_mut(4)
        .for_each(|p| p.copy_from_slice(&[18, 19, 22, 245]));
    for (i, ch) in label.chars().enumerate() {
        let rows: [u8; 7] = match ch {
            '0' => [14, 17, 19, 21, 25, 17, 14],
            '1' => [4, 12, 4, 4, 4, 4, 14],
            '2' => [14, 17, 1, 2, 4, 8, 31],
            '3' => [30, 1, 1, 14, 1, 1, 30],
            '4' => [2, 6, 10, 18, 31, 2, 2],
            '5' => [31, 16, 16, 30, 1, 1, 30],
            '6' => [14, 16, 16, 30, 17, 17, 14],
            '7' => [31, 1, 2, 4, 8, 8, 8],
            '8' => [14, 17, 17, 14, 17, 17, 14],
            '9' => [14, 17, 17, 15, 1, 1, 14],
            _ => [0, 0, 17, 10, 4, 10, 17],
        };
        for (y, row) in rows.iter().enumerate() {
            for x in 0..5 {
                if row & (1 << (4 - x)) != 0 {
                    render::fill_rect(
                        &mut image,
                        Rect {
                            x: 4 + i as i32 * 12 + x * 2,
                            y: 4 + y as i32 * 2,
                            w: 2,
                            h: 2,
                        },
                        [255, 255, 255, 255],
                    );
                }
            }
        }
    }
    image
}
pub fn badge_position(selection: Rect, badge: &Image, w: u32, h: u32) -> (i32, i32) {
    (
        selection.x.clamp(0, (w as i32 - badge.width as i32).max(0)),
        if selection.y >= badge.height as i32 + 4 {
            selection.y - badge.height as i32 - 4
        } else {
            (selection.y + 4).min(h as i32 - badge.height as i32).max(0)
        },
    )
}
fn action_icon(img: &mut Image, r: &Renderer, action: Action, b: Rect, c: [u8; 4]) {
    let x = b.x as f32 + b.w as f32 / 2.;
    let y = b.y as f32 + b.h as f32 / 2.;
    let mut line = |points: Vec<Point>| {
        let mut a = Annotation::new(Tool::Pen, points[0], c, 1.7);
        a.points = points;
        r.annotation(img, &a);
    };
    match action {
        Action::Cancel => {
            line(vec![Point::new(x - 6., y - 6.), Point::new(x + 6., y + 6.)]);
            line(vec![Point::new(x + 6., y - 6.), Point::new(x - 6., y + 6.)]);
        }
        Action::Copy => {
            line(vec![
                Point::new(x - 9., y + 8.),
                Point::new(x - 9., y - 7.),
                Point::new(x + 3., y - 7.),
            ]);
            line(vec![
                Point::new(x - 4., y - 3.),
                Point::new(x + 9., y - 3.),
                Point::new(x + 9., y + 10.),
                Point::new(x - 4., y + 10.),
                Point::new(x - 4., y - 3.),
            ]);
        }
        Action::Save => {
            line(vec![
                Point::new(x - 10., y - 10.),
                Point::new(x + 7., y - 10.),
                Point::new(x + 10., y - 7.),
                Point::new(x + 10., y + 10.),
                Point::new(x - 10., y + 10.),
                Point::new(x - 10., y - 10.),
            ]);
            line(vec![
                Point::new(x - 5., y - 9.),
                Point::new(x - 5., y - 2.),
                Point::new(x + 5., y - 2.),
                Point::new(x + 5., y - 9.),
            ]);
            line(vec![
                Point::new(x - 5., y + 9.),
                Point::new(x - 5., y + 3.),
                Point::new(x + 5., y + 3.),
                Point::new(x + 5., y + 9.),
            ]);
        }
        Action::Lens => {
            drop(line);
            render::ellipse(
                img,
                Rect {
                    x: x as i32 - 10,
                    y: y as i32 - 10,
                    w: 16,
                    h: 16,
                },
                c,
                1.8,
                false,
            );
            let mut a = Annotation::new(Tool::Line, Point::new(x + 3., y + 3.), c, 2.);
            a.points[1] = Point::new(x + 11., y + 11.);
            r.annotation(img, &a);
        }
        Action::Record | Action::NewCapture => {
            line(vec![
                Point::new(x - 11., y - 8.),
                Point::new(x + 4., y - 8.),
                Point::new(x + 4., y + 8.),
                Point::new(x - 11., y + 8.),
                Point::new(x - 11., y - 8.),
            ]);
            line(vec![
                Point::new(x + 4., y - 4.),
                Point::new(x + 12., y - 9.),
                Point::new(x + 12., y + 9.),
                Point::new(x + 4., y + 4.),
            ]);
        }
        Action::Pin => {
            line(vec![
                Point::new(x - 6., y - 10.),
                Point::new(x + 6., y - 10.),
                Point::new(x + 4., y),
                Point::new(x + 9., y + 4.),
                Point::new(x - 9., y + 4.),
                Point::new(x - 4., y),
                Point::new(x - 6., y - 10.),
            ]);
            line(vec![Point::new(x, y + 4.), Point::new(x, y + 12.)]);
        }
        Action::Upload => {
            line(vec![
                Point::new(x - 11., y + 4.),
                Point::new(x - 11., y + 10.),
                Point::new(x + 11., y + 10.),
                Point::new(x + 11., y + 4.),
            ]);
            line(vec![
                Point::new(x, y + 4.),
                Point::new(x, y - 10.),
                Point::new(x - 6., y - 4.),
            ]);
            line(vec![Point::new(x, y - 10.), Point::new(x + 6., y - 4.)]);
        }
        Action::Qr => {
            for (dx, dy) in [(-10., -10.), (3., -10.), (-10., 3.)] {
                line(vec![
                    Point::new(x + dx, y + dy),
                    Point::new(x + dx + 7., y + dy),
                    Point::new(x + dx + 7., y + dy + 7.),
                    Point::new(x + dx, y + dy + 7.),
                    Point::new(x + dx, y + dy),
                ]);
                line(vec![
                    Point::new(x + dx + 3., y + dy + 2.5),
                    Point::new(x + dx + 3., y + dy + 4.5),
                ]);
            }
            line(vec![
                Point::new(x + 3., y + 3.),
                Point::new(x + 6., y + 3.),
                Point::new(x + 6., y + 7.),
                Point::new(x + 10., y + 7.),
                Point::new(x + 10., y + 10.),
            ]);
            line(vec![
                Point::new(x + 3., y + 8.),
                Point::new(x + 3., y + 10.),
            ]);
        }
        Action::Translate => {
            // A + translation mark: distinct from both QR and the text tool.
            line(vec![
                Point::new(x - 10., y + 3.),
                Point::new(x - 6., y - 9.),
                Point::new(x - 2., y + 3.),
            ]);
            line(vec![
                Point::new(x - 8.5, y - 1.),
                Point::new(x - 3.5, y - 1.),
            ]);
            line(vec![
                Point::new(x + 1., y - 3.),
                Point::new(x + 10., y - 3.),
            ]);
            line(vec![Point::new(x + 5., y - 6.), Point::new(x + 5., y - 3.)]);
            line(vec![
                Point::new(x + 8., y - 3.),
                Point::new(x + 6., y + 3.),
                Point::new(x + 1., y + 8.),
            ]);
            line(vec![
                Point::new(x + 2., y + 1.),
                Point::new(x + 5., y + 5.),
                Point::new(x + 11., y + 9.),
            ]);
        }
        Action::Ocr => {
            for (dx, dy, sx, sy) in [
                (-10., -10., 1., 1.),
                (10., -10., -1., 1.),
                (-10., 10., 1., -1.),
                (10., 10., -1., -1.),
            ] {
                line(vec![
                    Point::new(x + dx, y + dy + 5. * sy),
                    Point::new(x + dx, y + dy),
                    Point::new(x + dx + 5. * sx, y + dy),
                ]);
            }
            // Scan lines, not a T: OCR must not look like the text annotation tool.
            line(vec![Point::new(x - 5., y - 5.), Point::new(x + 5., y - 5.)]);
            line(vec![Point::new(x - 5., y), Point::new(x + 5., y)]);
            line(vec![Point::new(x - 5., y + 5.), Point::new(x + 2., y + 5.)]);
        }
        Action::More => {
            drop(line);
            for d in [-7, 0, 7] {
                render::ellipse(
                    img,
                    Rect {
                        x: x as i32 + d - 2,
                        y: y as i32 - 2,
                        w: 4,
                        h: 4,
                    },
                    c,
                    1.,
                    true,
                )
            }
        }
        _ => {}
    }
}
fn tool_icon(img: &mut Image, r: &Renderer, t: Tool, b: Rect, c: [u8; 4]) {
    let x = b.x as f32 + b.w as f32 / 2.;
    let y = b.y as f32 + b.h as f32 / 2.;
    let mut a = Annotation::new(t, Point::new(x - 9., y - 8.), c, 1.8);
    a.points.push(Point::new(x + 8., y + 8.));
    match t {
        Tool::Select => render::stroke_rect(
            img,
            Rect {
                x: x as i32 - 9,
                y: y as i32 - 8,
                w: 18,
                h: 16,
            },
            c,
            1.5,
        ),
        Tool::Move => {
            render::fill_rect(
                img,
                Rect {
                    x: x as i32 - 1,
                    y: y as i32 - 7,
                    w: 2,
                    h: 14,
                },
                c,
            );
            render::fill_rect(
                img,
                Rect {
                    x: x as i32 - 7,
                    y: y as i32 - 1,
                    w: 14,
                    h: 2,
                },
                c,
            );
            for triangle in [
                [(0., -10.), (-3.5, -6.), (3.5, -6.)],
                [(0., 10.), (-3.5, 6.), (3.5, 6.)],
                [(-10., 0.), (-6., -3.5), (-6., 3.5)],
                [(10., 0.), (6., -3.5), (6., 3.5)],
            ] {
                let mut path = tiny_skia::PathBuilder::new();
                path.move_to(x + triangle[0].0, y + triangle[0].1);
                path.line_to(x + triangle[1].0, y + triangle[1].1);
                path.line_to(x + triangle[2].0, y + triangle[2].1);
                path.close();
                if let (Some(path), Some(mut pm)) = (
                    path.finish(),
                    tiny_skia::PixmapMut::from_bytes(&mut img.rgba, img.width, img.height),
                ) {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(c[0], c[1], c[2], c[3]);
                    pm.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }
        }
        Tool::Pen => {
            a.tool = Tool::Pen;
            a.points = vec![
                Point::new(x - 8., y + 9.),
                Point::new(x - 6., y + 2.),
                Point::new(x + 5., y - 9.),
                Point::new(x + 9., y - 5.),
                Point::new(x - 2., y + 6.),
                Point::new(x - 8., y + 9.),
            ];
            r.annotation(img, &a);
            a.points = vec![Point::new(x - 5., y + 2.), Point::new(x - 1., y + 6.)];
            r.annotation(img, &a);
            a.points = vec![Point::new(x + 3., y - 7.), Point::new(x + 7., y - 3.)];
            r.annotation(img, &a);
        }
        Tool::Highlight => render::round_rect(
            img,
            Rect {
                x: x as i32 - 9,
                y: y as i32 - 6,
                w: 18,
                h: 12,
            },
            2.,
            [c[0], c[1], c[2], 220],
        ),
        Tool::Text => r.text(img, x - 6., y - 10., "T", 21., c),
        Tool::Blur => {
            for i in 0..3 {
                render::fill_rect(
                    img,
                    Rect {
                        x: x as i32 - 9 + i * 3,
                        y: y as i32 - 9 + i * 3,
                        w: (18 - i * 6) as u32,
                        h: (18 - i * 6) as u32,
                    },
                    [c[0], c[1], c[2], 70 + (i as u8 * 40)],
                )
            }
        }
        Tool::Pixelate => {
            for j in 0..3 {
                for i in 0..3 {
                    render::fill_rect(
                        img,
                        Rect {
                            x: x as i32 - 9 + i * 6,
                            y: y as i32 - 9 + j * 6,
                            w: 5,
                            h: 5,
                        },
                        [c[0], c[1], c[2], if (i + j) % 2 == 0 { 220 } else { 90 }],
                    )
                }
            }
        }
        Tool::Redact => render::round_rect(
            img,
            Rect {
                x: x as i32 - 9,
                y: y as i32 - 6,
                w: 18,
                h: 12,
            },
            2.,
            c,
        ),
        Tool::Step => {
            a.points = vec![Point::new(x, y)];
            a.number = 1;
            a.color = c;
            render::ellipse(
                img,
                Rect {
                    x: x as i32 - 10,
                    y: y as i32 - 10,
                    w: 20,
                    h: 20,
                },
                c,
                1.5,
                false,
            );
            r.text(img, x - 4., y - 7., "1", 14., c)
        }
        Tool::Picker => {
            a.tool = Tool::Pen;
            for points in [
                vec![
                    Point::new(x - 8., y + 9.),
                    Point::new(x - 7., y + 4.),
                    Point::new(x + 3., y - 6.),
                    Point::new(x + 7., y - 2.),
                    Point::new(x - 3., y + 8.),
                    Point::new(x - 8., y + 9.),
                ],
                vec![Point::new(x + 1., y - 8.), Point::new(x + 9., y)],
                vec![
                    Point::new(x + 4., y - 8.),
                    Point::new(x + 7., y - 11.),
                    Point::new(x + 12., y - 6.),
                    Point::new(x + 9., y - 3.),
                ],
            ] {
                a.points = points;
                r.annotation(img, &a);
            }
        }
        _ => r.annotation(img, &a),
    }
}
pub fn demo() -> Editor {
    let mut img = Image::new(1280, 800).unwrap();
    for y in 0..img.height {
        for x in 0..img.width {
            let i = (y as usize * img.width as usize + x as usize) * 4;
            img.rgba[i..i + 4].copy_from_slice(&[
                19 + (x / 130) as u8,
                28 + (y / 70) as u8,
                42 + (x / 100) as u8,
                255,
            ]);
        }
    }
    let r = Renderer::new();
    render::round_rect(
        &mut img,
        Rect {
            x: 150,
            y: 100,
            w: 980,
            h: 600,
        },
        12.,
        [249, 250, 252, 255],
    );
    render::fill_rect(
        &mut img,
        Rect {
            x: 150,
            y: 140,
            w: 980,
            h: 1,
        },
        [225, 229, 236, 255],
    );
    for (i, c) in [[238, 99, 91, 255], [242, 187, 65, 255], [72, 187, 119, 255]]
        .into_iter()
        .enumerate()
    {
        render::ellipse(
            &mut img,
            Rect {
                x: 171 + i as i32 * 19,
                y: 116,
                w: 11,
                h: 11,
            },
            c,
            1.,
            true,
        )
    }
    r.text(
        &mut img,
        244.,
        113.,
        "Northstar  /  Product workspace",
        13.,
        [100, 112, 132, 255],
    );
    r.text(
        &mut img,
        206.,
        193.,
        "Make the next step clear.",
        34.,
        [30, 39, 55, 255],
    );
    r.text(
        &mut img,
        208.,
        247.,
        "Capture an idea. Add context. Keep moving.",
        17.,
        [104, 117, 136, 255],
    );
    let cards = [
        ("01", "Capture", "Select just what matters."),
        ("02", "Explain", "Give your team the context."),
        ("03", "Share", "Your image. Your choice."),
    ];
    for (i, (num, title, body)) in cards.into_iter().enumerate() {
        let x = 208 + i as i32 * 280;
        render::round_rect(
            &mut img,
            Rect {
                x,
                y: 310,
                w: 256,
                h: 174,
            },
            8.,
            [238, 243, 250, 255],
        );
        r.text(
            &mut img,
            x as f32 + 20.,
            331.,
            num,
            13.,
            [39, 131, 222, 255],
        );
        r.text(
            &mut img,
            x as f32 + 20.,
            370.,
            title,
            23.,
            [38, 48, 66, 255],
        );
        r.text(
            &mut img,
            x as f32 + 20.,
            414.,
            body,
            13.,
            [91, 107, 131, 255],
        );
    }
    r.text(
        &mut img,
        208.,
        542.,
        "PRIVATE NOTE",
        12.,
        [124, 137, 157, 255],
    );
    r.text(
        &mut img,
        208.,
        574.,
        "token = sk-demo-not-a-real-secret",
        17.,
        [60, 74, 98, 255],
    );
    let mut e = Editor::new(img);
    e.selection = Some(Rect {
        x: 188,
        y: 177,
        w: 888,
        h: 436,
    });
    e.tool = Tool::Pen;
    let mut arrow = Annotation::new(Tool::Arrow, Point::new(586., 286.), [39, 131, 222, 255], 4.);
    arrow.points.push(Point::new(626., 365.));
    e.add(arrow);
    let mut redact = Annotation::new(Tool::Redact, Point::new(264., 574.), [0, 0, 0, 255], 3.);
    redact.points.push(Point::new(570., 596.));
    e.add(redact);
    let mut step = Annotation::new(Tool::Step, Point::new(1028., 205.), [39, 131, 222, 255], 3.);
    step.number = 1;
    e.add(step);
    e.selected = None;
    e
}

/// UI-only QR row inside the selected image, anchored near the QR action.
/// Never included in exports. Omit for regions too small to fit readable controls.
pub fn qr_result_rect(layout: &Layout, selection: Rect, w: u32, h: u32) -> Option<Rect> {
    let left = selection.x.max(0) + 4;
    let top = selection.y.max(0) + 4;
    let right = (selection.x + selection.w as i32).min(w as i32) - 4;
    let bottom = (selection.y + selection.h as i32).min(h as i32) - 4;
    if right - left < 96 || bottom - top < 24 {
        return None;
    }
    let ww = ((right - left) as u32).min(340);
    let qr = layout.buttons.iter().find(|b| b.action == Action::Qr)?.rect;
    let badge = size_badge(selection);
    let (bx, by) = badge_position(selection, &badge, w, h);
    let badge = Rect {
        x: bx,
        y: by,
        w: badge.width,
        h: badge.height,
    };
    let preferred_x = (qr.x + qr.w as i32 / 2 - ww as i32 / 2).clamp(left, right - ww as i32);
    let ys = if qr.y >= bottom {
        [bottom - 24, qr.y - 28, top, qr.y + qr.h as i32 + 4]
    } else if qr.y + qr.h as i32 <= top {
        [top, qr.y + qr.h as i32 + 4, bottom - 24, qr.y - 28]
    } else {
        [qr.y - 28, qr.y + qr.h as i32 + 4, bottom - 24, top]
    };
    for y in ys {
        for x in [
            preferred_x,
            right - ww as i32,
            (layout.surfaces[0].x - ww as i32 - 4).clamp(left, right - ww as i32),
            left,
        ] {
            let r = Rect { x, y, w: ww, h: 24 };
            if y >= top
                && y + 24 <= bottom
                && !overlap(r, badge)
                && !layout.surfaces.iter().any(|p| overlap(*p, r))
            {
                return Some(r);
            }
        }
    }
    None
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrResultAction {
    Open,
    Copy,
}
pub fn qr_result_hit(rect: Rect, x: i32, y: i32, is_url: bool) -> Option<QrResultAction> {
    if !rect.contains(x, y) {
        return None;
    }
    if x >= rect.x + rect.w as i32 - 28 || !is_url {
        Some(QrResultAction::Copy)
    } else {
        Some(QrResultAction::Open)
    }
}
pub fn qr_banner(renderer: &Renderer, text: &str, rect: Rect, copied: bool) -> Image {
    let mut image = Image::new(rect.w, rect.h).unwrap();
    render::fill_rect(
        &mut image,
        Rect {
            x: 0,
            y: 0,
            w: rect.w,
            h: rect.h,
        },
        [24, 31, 40, 255],
    );
    let url = crate::links::http_url(text).is_some();
    let limit = rect.w.saturating_sub(42) as f32;
    let label = renderer.elide(&format!("QR: {text}"), 12., limit);
    let color = if url {
        [107, 192, 255, 255]
    } else {
        [233, 239, 246, 255]
    };
    renderer.text(&mut image, 7., 5., &label, 12., color);
    if url {
        render::fill_rect(
            &mut image,
            Rect {
                x: 7,
                y: 19,
                w: renderer.text_width(&label, 12.).min(limit + 5.) as u32,
                h: 1,
            },
            color,
        );
    }
    let x = rect.w as i32 - 28;
    render::fill_rect(
        &mut image,
        Rect {
            x,
            y: 0,
            w: 28,
            h: rect.h,
        },
        if copied {
            [28, 91, 71, 255]
        } else {
            [43, 53, 68, 255]
        },
    );
    render::fill_rect(
        &mut image,
        Rect {
            x,
            y: 3,
            w: 1,
            h: 18,
        },
        [80, 92, 110, 255],
    );
    // Two overlapping sheets: a separate, explicit Copy target.
    for (dx, dy) in [(8, 5), (12, 9)] {
        render::stroke_rect(
            &mut image,
            Rect {
                x: x + dx,
                y: dy,
                w: 10,
                h: 11,
            },
            if copied {
                [133, 241, 188, 255]
            } else {
                [235, 242, 252, 255]
            },
            1.4,
        );
    }
    image
}
