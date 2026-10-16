use luma_shot::{
    config::Config,
    editor::{Annotation, Editor, Point, Tool, TOOLS},
    google::Handoff,
    image::{Image, Rect},
    render::Renderer,
    ui,
};
#[test]
fn only_one_secure_redaction_tool() {
    assert!(TOOLS.contains(&Tool::Redact));
    assert!(!TOOLS.contains(&Tool::Blur));
    assert!(!TOOLS.contains(&Tool::Pixelate));
}
#[test]
fn palette_and_export_settings_roundtrip() {
    let mut c = Config::default();
    c.palette[0] = [12, 34, 56, 255];
    c.format = "jpg".into();
    c.jpeg_quality = 63;
    c.auto_qr = true;
    let d: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(d.palette, c.palette);
    assert_eq!(d.jpeg_quality, 63);
    assert!(d.validate().is_ok());
}
#[test]
fn old_settings_ignore_removed_integrations() {
    let c: Config = serde_json::from_str(
        r#"{"theme":"light","gemini_model":"old","s3_bucket":"unused","lens_experimental":true}"#,
    )
    .unwrap();
    assert_eq!(c.theme, "light");
    let s = serde_json::to_string(&c).unwrap();
    for x in ["gemini", "s3_", "lens_experimental"] {
        assert!(!s.contains(x));
    }
}
#[test]
fn jpeg_quality_changes_actual_bytes() {
    let mut i = Image::new(128, 96).unwrap();
    for (n, p) in i.rgba.chunks_exact_mut(4).enumerate() {
        p[..3].copy_from_slice(&[
            (n * 31 % 255) as u8,
            (n * 13 % 255) as u8,
            (n * 7 % 255) as u8,
        ]);
    }
    let d = std::env::temp_dir().join(luma_shot::util::unique_id());
    std::fs::create_dir_all(&d).unwrap();
    let a = d.join("low.jpg");
    let b = d.join("high.jpg");
    i.save_options(&a, 15, "fast").unwrap();
    i.save_options(&b, 98, "fast").unwrap();
    assert!(std::fs::metadata(b).unwrap().len() > std::fs::metadata(a).unwrap().len());
    std::fs::remove_dir_all(d).unwrap();
}
#[test]
fn png_all_compressions_preserve_pixels() {
    let i = Image::new(64, 64).unwrap();
    let a = i.png_with_compression("fast").unwrap();
    let b = i.png_with_compression("best").unwrap();
    assert_ne!(a, b);
    let d = std::env::temp_dir().join(format!("prtsc-{}.png", luma_shot::util::unique_id()));
    i.save_options(&d, 70, "best").unwrap();
    assert_eq!(Image::load_png(&d).unwrap(), i);
    std::fs::remove_file(d).unwrap();
}
#[test]
fn opaque_export_has_no_original_secret_pixels() {
    let mut i = Image::new(40, 30).unwrap();
    i.rgba.fill(255);
    let mut e = Editor::new(i);
    e.selection = Some(Rect {
        x: 0,
        y: 0,
        w: 40,
        h: 30,
    });
    let mut a = Annotation::new(Tool::Redact, Point::new(4., 5.), [255, 0, 0, 10], 1.);
    a.points[1] = Point::new(30., 20.);
    e.add(a);
    let out = Renderer::new().flatten(&e.image, &e.annotations);
    for y in 5..20 {
        for x in 4..30 {
            assert_eq!(out.pixel(x, y), [24, 24, 27, 255]);
        }
    }
    e.undo();
    assert_eq!(e.annotations.len(), 0);
}
#[test]
fn compact_toolbar_has_no_ai_and_readable_hints() {
    let l = ui::Layout::new(
        1280,
        800,
        Some(Rect {
            x: 200,
            y: 100,
            w: 800,
            h: 400,
        }),
    );
    assert_eq!(l.panel.w, 448);
    assert_eq!(l.panel.h, 76);
    assert!(l.buttons.iter().all(|b| !b.hint.is_empty()));
    assert!(l.buttons.iter().any(|b| b.action == ui::Action::Record));
}
#[test]
fn browser_handoff_auto_submits_only_png_to_google() {
    let h = Handoff::new(&Image::new(2, 3).unwrap()).unwrap();
    let html = h.html_for(&luma_shot::google::encode(Image::new(2, 3).unwrap(), false).unwrap());
    assert!(h.url().starts_with("http://127.0.0.1:"));
    assert!(html.contains("https://lens.google.com/v3/upload"));
    assert!(html.contains("name=\"encoded_image\""));
    assert!(html.contains(".submit()"));
    assert!(!html.contains("gemini"));
    assert!(!html.contains("http://",));
}
#[test]
fn handoff_uses_random_unrelated_tokens() {
    let i = Image::new(1, 1).unwrap();
    let a = Handoff::new(&i).unwrap();
    let b = Handoff::new(&i).unwrap();
    assert_ne!(a.url().split('/').nth(3), b.url().split('/').nth(3));
    assert_eq!(a.url().split('/').nth(3).unwrap().len(), 48);
}
#[test]
fn loopback_handoff_rejects_other_paths_and_is_single_use() {
    use std::{
        io::{Read, Write},
        net::TcpStream,
    };
    let h = Handoff::new(&Image::new(1, 1).unwrap()).unwrap();
    let u = h.url();
    let parts: Vec<_> = u.split('/').collect();
    let host = parts[2].to_string();
    let token = parts[3].to_string();
    let worker = std::thread::spawn(move || h.serve());
    let mut s = TcpStream::connect(&host).unwrap();
    write!(s, "GET /wrong/ HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
    let mut bad = String::new();
    s.read_to_string(&mut bad).unwrap();
    assert!(bad.contains("404"));
    let mut s = TcpStream::connect(&host).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    write!(s, "GET /{token}/ HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
    let mut ok = String::new();
    s.read_to_string(&mut ok).unwrap();
    assert!(ok.contains("200 OK"));
    assert!(ok.contains("Cache-Control: no-store"));
    assert!(ok.contains("frame-ancestors 'none'"));
    assert!(ok.contains("atob("));
    assert!(!ok.contains("fetch("));
    worker.join().unwrap().unwrap();
    assert!(TcpStream::connect(&host).is_err());
}
