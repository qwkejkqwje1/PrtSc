use luma_shot::{
    google::Handoff,
    hotkey,
    image::{Image, Rect},
    ui::{self, QrResultAction},
};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};
#[test]
fn qr_open_and_copy_are_separate_targets() {
    let r = Rect {
        x: 100,
        y: 200,
        w: 240,
        h: 24,
    };
    assert_eq!(
        ui::qr_result_hit(r, 101, 201, true),
        Some(QrResultAction::Open)
    );
    assert_eq!(
        ui::qr_result_hit(r, 339, 201, true),
        Some(QrResultAction::Copy)
    );
    assert_eq!(
        ui::qr_result_hit(r, 101, 201, false),
        Some(QrResultAction::Copy)
    );
    assert_eq!(ui::qr_result_hit(r, 99, 201, true), None);
    assert_eq!(ui::qr_result_hit(r, 340, 201, true), None);
}
#[test]
fn physical_shortcut_events_roundtrip() {
    for modifiers in 0..16 {
        for key in [
            44, 65, 83, 48, 57, 112, 120, 135, 32, 19, 45, 36, 35, 33, 34,
        ] {
            let text = hotkey::format_event(modifiers, key).unwrap();
            assert_eq!(
                hotkey::parse(&text).unwrap(),
                hotkey::Hotkey { modifiers, key }
            );
        }
    }
    for (m, k) in [(0, 16), (0, 17), (0, 18), (0, 0x7b), (6, 82)] {
        assert!(hotkey::format_event(m, k).is_err());
    }
}
#[test]
fn badge_is_legible_and_icon_has_optical_sizes() {
    let b = ui::size_badge(Rect {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    });
    assert_eq!(b.height, 22);
    assert!(b.width >= 100);
    let ico = include_bytes!("../assets/luma.ico");
    assert_eq!(&ico[..4], &[0, 0, 1, 0]);
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    assert_eq!(count, 8);
    for i in 0..count {
        let p = 6 + 16 * i;
        let len = u32::from_le_bytes(ico[p + 8..p + 12].try_into().unwrap()) as usize;
        let off = u32::from_le_bytes(ico[p + 12..p + 16].try_into().unwrap()) as usize;
        assert_eq!(&ico[off..off + 8], b"\x89PNG\r\n\x1a\n");
        assert!(off + len <= ico.len());
    }
}
#[test]
fn browser_idle_preconnections_never_block_real_request() {
    let h = Handoff::from_owned(Image::new(10, 10).unwrap(), true).unwrap();
    let url = h.url();
    let u = url.strip_prefix("http://").unwrap();
    let (host, path) = u.split_once('/').unwrap();
    let idle: Vec<_> = (0..8).map(|_| TcpStream::connect(host).unwrap()).collect();
    let worker = std::thread::spawn(move || h.serve());
    std::thread::sleep(Duration::from_millis(30));
    let start = Instant::now();
    let mut real = TcpStream::connect(host).unwrap();
    real.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    write!(real, "GET /{path} HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
    let mut response = String::new();
    real.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(
        start.elapsed() < Duration::from_millis(250),
        "idle preconnect delay {:?}",
        start.elapsed()
    );
    worker.join().unwrap().unwrap();
    drop(idle);
}
#[test]
fn handoff_rejects_origin_and_wrong_host_before_valid_request() {
    let h = Handoff::new(&Image::new(10, 10).unwrap()).unwrap();
    let url = h.url();
    let u = url.strip_prefix("http://").unwrap();
    let (host, path) = u.split_once('/').unwrap();
    let worker = std::thread::spawn(move || h.serve());
    for headers in [
        "Host: attacker.invalid\r\n".to_string(),
        format!("Host: {host}\r\nOrigin: https://attacker.invalid\r\n"),
    ] {
        let mut s = TcpStream::connect(host).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        write!(s, "GET /{path} HTTP/1.1\r\n{headers}\r\n").unwrap();
        let mut response = String::new();
        s.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 404"));
    }
    let mut s = TcpStream::connect(host).unwrap();
    write!(s, "GET /{path} HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
    let mut r = String::new();
    s.read_to_string(&mut r).unwrap();
    assert!(r.contains("200 OK"));
    worker.join().unwrap().unwrap();
}
