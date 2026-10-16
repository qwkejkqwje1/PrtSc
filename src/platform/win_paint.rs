use super::*;
pub(super) struct FrameCache {
    pub(super) annotations: Vec<Annotation>,
    scale: f32,
    pub scene: Image,
    plain: native::Raster,
    dim: native::Raster,
    back: native::Raster,
    chrome_key: String,
    chrome: Vec<(native::Raster, Rect)>,
}
impl FrameCache {
    fn new(s: &Session, annotations: Vec<Annotation>) -> Result<Self, String> {
        let mut scene = s.renderer.flatten(&s.editor.image, &annotations);
        if s.view.scale != 1. || scene.width != s.view.width || scene.height != s.view.height {
            let scaled = luma_shot::render::resized(
                &scene,
                (scene.width as f32 * s.view.scale).max(1.) as u32,
                (scene.height as f32 * s.view.scale).max(1.) as u32,
            )?;
            scene = Image::new(s.view.width, s.view.height)?;
            scene
                .rgba
                .chunks_exact_mut(4)
                .for_each(|p| p.copy_from_slice(&[22, 25, 31, 255]));
            ui::paste(&mut scene, &scaled, 0, 0);
        }
        let plain = native::Raster::new(&scene, false)?;
        let dim = native::Raster::new(&scene, true)?;
        let back = native::Raster::new(&scene, false)?;
        Ok(Self {
            annotations,
            scale: s.view.scale,
            scene,
            plain,
            dim,
            back,
            chrome_key: String::new(),
            chrome: vec![],
        })
    }
}
fn intersect(a: RECT, b: RECT) -> RECT {
    RECT {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    }
}
fn wr(r: Rect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.x + r.w as i32,
        bottom: r.y + r.h as i32,
    }
}
pub(super) unsafe fn paint(
    s: &mut Session,
    dc: HDC,
    dirty: RECT,
    dark: bool,
    colors: &[[u8; 4]; 7],
) -> Result<(), String> {
    s.paint_count += 1;
    if !s.font_ready
        && s.editor.selection().is_some()
        && !matches!(s.drag, Drag::Selection(_) | Drag::RegionResize { .. })
    {
        s.renderer = Renderer::new();
        s.font_ready = true;
    }
    let exclude = match s.drag {
        Drag::Move { index, .. } | Drag::Resize { index, .. } => Some(index),
        _ => None,
    };
    let annotations: Vec<_> = s
        .editor
        .annotations
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != exclude)
        .map(|(_, a)| a.clone())
        .collect();
    if s.cache
        .as_ref()
        .map(|c| c.annotations != annotations || c.scale != s.view.scale)
        .unwrap_or(true)
    {
        s.cache = Some(FrameCache::new(s, annotations)?);
        s.scene_builds += 1;
    }
    let c = s.cache.as_mut().unwrap();
    let window_dc = dc;
    let dc = c.back.dc;
    // Compose the entire update offscreen; only the finished frame reaches HWND.
    c.dim.paint(dc, dirty);
    let selection = s.editor.selection().map(|r| s.view.rect(r));
    if let Some(r) = selection {
        let rr = intersect(dirty, wr(r));
        if rr.right > rr.left && rr.bottom > rr.top {
            c.plain.paint(dc, rr);
        }
    }
    // Draw only a cropped patch for a live annotation, never re-render the desktop.
    let draft = s
        .editor
        .draft
        .as_ref()
        .or_else(|| exclude.and_then(|i| s.editor.annotations.get(i)));
    if let Some(d) = draft {
        let mut d = d.clone();
        for p in &mut d.points {
            p.x *= s.view.scale;
            p.y *= s.view.scale;
        }
        d.width *= s.view.scale;
        let r = d.rect();
        let pad = (d.width * 12.).ceil() as i32 + 8;
        let r = Rect {
            x: r.x - pad,
            y: r.y - pad,
            w: r.w + 2 * pad as u32,
            h: r.h + 2 * pad as u32,
        }
        .clamp(c.scene.width, c.scene.height);
        if r.valid() {
            let mut patch = c.scene.crop(r)?;
            d.translate(-r.x as f32, -r.y as f32);
            s.renderer.annotation(&mut patch, &d);
            let raster = native::Raster::new(&patch, false)?;
            let saved = SaveDC(dc);
            if let Some(sel) = selection {
                IntersectClipRect(dc, sel.x, sel.y, sel.x + sel.w as i32, sel.y + sel.h as i32);
            }
            BitBlt(
                dc, r.x, r.y, r.w as i32, r.h as i32, raster.dc, 0, 0, SRCCOPY,
            );
            RestoreDC(dc, saved);
        }
    }
    if let Some(r) = selection {
        let pen = CreatePen(PS_SOLID, 1, 0x00ffa750);
        let old = SelectObject(dc, pen);
        let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
        Rectangle(dc, r.x, r.y, r.x + r.w as i32, r.y + r.h as i32);
        SelectObject(dc, brush);
        SelectObject(dc, old);
        DeleteObject(pen);
        for (x, y) in [
            (r.x, r.y),
            (r.x + r.w as i32, r.y),
            (r.x, r.y + r.h as i32),
            (r.x + r.w as i32, r.y + r.h as i32),
            (r.x + r.w as i32 / 2, r.y),
            (r.x + r.w as i32 / 2, r.y + r.h as i32),
            (r.x, r.y + r.h as i32 / 2),
            (r.x + r.w as i32, r.y + r.h as i32 / 2),
        ] {
            let rr = RECT {
                left: x - 3,
                top: y - 3,
                right: x + 3,
                bottom: y + 3,
            };
            FillRect(dc, &rr, GetStockObject(WHITE_BRUSH) as _);
        }
    }
    // Hide tools until the region is committed. Hover-only moves touch a small cached panel.
    if let Some(sel) = selection {
        let badge = ui::size_badge(s.editor.selection().unwrap());
        let (x, y) = ui::badge_position(sel, &badge, s.view.width, s.view.height);
        let raster = native::Raster::new(&badge, false)?;
        BitBlt(
            dc,
            x,
            y,
            badge.width as i32,
            badge.height as i32,
            raster.dc,
            0,
            0,
            SRCCOPY,
        );
    }
    if selection.is_some() && !matches!(s.drag, Drag::Selection(_) | Drag::RegionResize { .. }) {
        let mut layout = Layout::with_palette(s.view.width, s.view.height, selection, colors);
        luma_shot::dock::apply(&mut layout, s.view.width, s.view.height);
        let hovered = layout.hit(s.cursor.0, s.cursor.1);
        let key = format!(
            "{:?}:{:?}:{}:{:?}:{:?}:{}:{}:{}:{}",
            layout.surfaces,
            s.editor.tool,
            s.editor.width,
            s.editor.color,
            hovered,
            dark,
            luma_shot::i18n::english(),
            s.qr_values.len(),
            format!("{}:{:?}", s.editor.selected.is_some(), s.editor.tool_colors)
        );
        if c.chrome_key != key {
            c.chrome.clear();
            for (chrome, r) in ui::panel_images(
                &s.renderer,
                &s.editor,
                &layout,
                dark,
                s.cursor,
                s.qr_values.len(),
            ) {
                let mut patch = c.scene.crop(r)?;
                for y in 0..patch.height {
                    for x in 0..patch.width {
                        if !selection.unwrap().contains(r.x + x as i32, r.y + y as i32) {
                            let i = (y as usize * patch.width as usize + x as usize) * 4;
                            for k in 0..3 {
                                patch.rgba[i + k] = (patch.rgba[i + k] as u16 * 40 / 100) as u8;
                            }
                        }
                    }
                }
                ui::paste(&mut patch, &chrome, 0, 0);
                c.chrome.push((native::Raster::new(&patch, false)?, r));
            }
            c.chrome_key = key;
        }
        for (raster, r) in &c.chrome {
            BitBlt(
                dc, r.x, r.y, r.w as i32, r.h as i32, raster.dc, 0, 0, SRCCOPY,
            );
        }
        if !s.qr_values.is_empty() {
            if let Some(rect) =
                ui::qr_result_rect(&layout, selection.unwrap(), s.view.width, s.view.height)
            {
                let text = s.qr_values.join(" · ");
                let image = ui::qr_banner(&s.renderer, &text, rect, s.qr_copied);
                let raster = native::Raster::new(&image, false)?;
                BitBlt(
                    dc,
                    rect.x,
                    rect.y,
                    rect.w as i32,
                    rect.h as i32,
                    raster.dc,
                    0,
                    0,
                    SRCCOPY,
                );
            }
        }
        if let Some(button) = layout
            .buttons
            .iter()
            .find(|b| b.rect.contains(s.cursor.0, s.cursor.1))
        {
            let mut hint = button.hint.clone();
            while s.renderer.text_width(&hint, 12.) > 340. {
                hint.pop();
            }
            let width = s.renderer.text_width(&hint, 12.).ceil() as u32 + 16;
            let mut image = Image::new(width, 24)?;
            image
                .rgba
                .chunks_exact_mut(4)
                .for_each(|p| p.copy_from_slice(&[22, 25, 31, 255]));
            s.renderer
                .text(&mut image, 8., 6., &hint, 12., [240, 243, 248, 255]);
            let x = (s.cursor.0 - width as i32 / 2)
                .clamp(0, (s.view.width as i32 - width as i32).max(0));
            let y = if button.rect.y >= 28 {
                button.rect.y - 28
            } else {
                button.rect.y + button.rect.h as i32 + 4
            };
            let raster = native::Raster::new(&image, false)?;
            BitBlt(dc, x, y, width as i32, 24, raster.dc, 0, 0, SRCCOPY);
        }
        s.layout = Some(layout);
    } else {
        s.layout = None;
        let text = wide(luma_shot::i18n::tr(
            if matches!(s.drag, Drag::Selection(_)) {
                "Отпустите кнопку — появятся инструменты"
            } else {
                "Выделите область мышью · Alt + щелчок — окно · Esc — отмена"
            },
        ));
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, 0x00ffffff);
        let old = SelectObject(dc, GetStockObject(DEFAULT_GUI_FONT));
        TextOutW(dc, 20, 20, text.as_ptr(), text.len() as i32 - 1);
        SelectObject(dc, old);
    }
    if let Some(i) = s.editor.selected {
        if let Some(a) = s.editor.annotations.get(i) {
            let r = s.view.rect(a.rect());
            let pen = CreatePen(PS_DOT, 1, 0x00ffffff);
            let old = SelectObject(dc, pen);
            let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
            Rectangle(
                dc,
                r.x - 5,
                r.y - 5,
                r.x + r.w as i32 + 5,
                r.y + r.h as i32 + 5,
            );
            SelectObject(dc, brush);
            SelectObject(dc, old);
            DeleteObject(pen);
        }
    }
    if alt() {
        let (x, y) = s.cursor;
        let px = (x + 26).clamp(4, (s.view.width as i32 - 134).max(4));
        let py = (y + 26).clamp(4, (s.view.height as i32 - 158).max(4));
        SetStretchBltMode(dc, COLORONCOLOR);
        StretchBlt(
            dc,
            px,
            py,
            120,
            120,
            c.plain.dc,
            (x - 7).clamp(0, (c.plain.width - 15).max(0)),
            (y - 7).clamp(0, (c.plain.height - 15).max(0)),
            15,
            15,
            SRCCOPY,
        );
        let pen = CreatePen(PS_SOLID, 1, 0x0000ffff);
        let old = SelectObject(dc, pen);
        MoveToEx(dc, px + 56, py + 56, null_mut());
        LineTo(dc, px + 64, py + 64);
        MoveToEx(dc, px + 64, py + 56, null_mut());
        LineTo(dc, px + 56, py + 64);
        SelectObject(dc, old);
        DeleteObject(pen);
    }
    GdiFlush();
    c.back.paint(window_dc, dirty);
    s.present_count += 1;
    Ok(())
}
