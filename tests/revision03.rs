use luma_shot::{config::Config, hosting, image::Image, unicode::decode_input, video::Gif};
#[test]
fn utf16_cyrillic_and_surrogates() {
    let mut pending = 0;
    let source = "Привет, мир! 🦀";
    let got: String = source
        .encode_utf16()
        .filter_map(|u| decode_input(&mut pending, u))
        .collect();
    assert_eq!(got, source);
    assert_eq!(decode_input(&mut pending, 0xdc00), None);
    assert_eq!(decode_input(&mut pending, 0xd800), None);
    assert_eq!(decode_input(&mut pending, b'A' as u16), Some('A'));
    assert_eq!(decode_input(&mut pending, 0xdc00), None);
}
#[test]
fn silent_defaults_and_hosts() {
    let c = Config::default();
    assert!(!c.sound_enabled);
    assert!(!c.confirm_google);
    assert!(c.auto_qr);
    assert!(!c.history_enabled);
    for host in ["litterbox", "catbox", "imgur"] {
        let mut c = c.clone();
        c.upload_host = host.into();
        assert!(c.validate().is_ok());
    }
}
#[test]
fn uploads_are_only_png_multipart_and_allowlisted() {
    let image = Image::new(7, 9).unwrap();
    for host in ["litterbox", "catbox", "imgur"] {
        let r = hosting::request(&image, host).unwrap();
        let png = image.png().unwrap();
        assert!(r.body.windows(png.len()).any(|w| w == png));
        assert!(r
            .content_type
            .starts_with("multipart/form-data; boundary=PrtSc"));
        assert!(r
            .body
            .windows(b"image/png\r\n\r\n\x89PN".len())
            .any(|w| w == b"image/png\r\n\r\n\x89PN"));
    }
    assert!(hosting::request(&image, "https://evil.example").is_err());
    let r = hosting::request(&image, "litterbox").unwrap();
    assert!(String::from_utf8_lossy(&r.body).contains("\r\n\r\n1h\r\n"));
}
#[test]
fn upload_responses_reject_untrusted_links() {
    assert_eq!(
        hosting::link(b"https://litter.catbox.moe/abc.png\n", "litterbox").unwrap(),
        "https://litter.catbox.moe/abc.png"
    );
    for bad in [
        "http://files.catbox.moe/a.png",
        "https://files.catbox.moe.evil/a.png",
        "https://files.catbox.moe/a b",
        "<html>error</html>",
        "https://files.catbox.moe/",
    ] {
        assert!(hosting::link(bad.as_bytes(), "catbox").is_err(), "{bad}");
    }
    assert!(hosting::link(
        br#"{"success":false,"data":{"link":"https://i.imgur.com/a.png"}}"#,
        "imgur"
    )
    .is_err());
    assert_eq!(
        hosting::link(
            br#"{"success":true,"data":{"link":"https://i.imgur.com/a.png"}}"#,
            "imgur"
        )
        .unwrap(),
        "https://i.imgur.com/a.png"
    );
}
#[test]
fn gif_stream_has_real_frames_and_correct_timing() {
    let path = std::env::temp_dir().join(format!("prtsc-{}.gif", luma_shot::util::unique_id()));
    let mut g = Gif::new(&path, 16, 12, 15).unwrap();
    for n in 0..15 {
        let mut i = Image::new(16, 12).unwrap();
        for p in i.rgba.chunks_exact_mut(4) {
            p.copy_from_slice(&[n * 16, 255 - n * 16, 0, 255]);
        }
        g.frame(&i).unwrap();
    }
    assert!(g.frame(&Image::new(2, 2).unwrap()).is_err());
    g.finish().unwrap();
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = decoder
        .read_info(std::fs::File::open(&path).unwrap())
        .unwrap();
    assert_eq!((reader.width(), reader.height()), (16, 12));
    let mut count = 0;
    let mut time = 0;
    let mut pixels = vec![];
    while let Some(f) = reader.read_next_frame().unwrap() {
        count += 1;
        time += f.delay;
        pixels.push(f.buffer[0]);
    }
    assert_eq!(count, 15);
    assert_eq!(time, 100);
    assert!(pixels.first() != pixels.last());
    std::fs::remove_file(path).unwrap();
}
