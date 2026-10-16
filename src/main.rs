#![cfg_attr(windows, windows_subsystem = "windows")]
use luma_shot::{image::Image, render::Renderer, ui};
mod platform;
fn main() {
    if let Err(e) = run() {
        platform::error(&e);
        std::process::exit(1)
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    luma_shot::i18n::configure(
        if args.iter().any(|a| a == "--english") {
            "en"
        } else {
            "ru"
        },
        if args.iter().any(|a| a == "--lightshot") {
            "lightshot"
        } else if args.iter().any(|a| a == "--compact") {
            "compact"
        } else {
            "standard"
        },
    );
    if let Some(n) = args.iter().position(|a| a == "--encoder-test") {
        return platform::encoder_test(std::path::Path::new(
            args.get(n + 1).ok_or("Output path required")?,
        ));
    }
    if args.iter().any(|a| a == "--licenses") {
        println!("{}", luma_shot::LICENSE_TEXT);
        return Ok(());
    }
    if args.iter().any(|a| a == "--version") {
        println!("PrtSc {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("PrtSc — native, local-first screenshot utility\n\nDefault: tray and global PrintScreen shortcut\n--demo                  Open a synthetic editor scene (no screen capture)\n--capture PATH          Capture the desktop to PNG/JPEG/WebP and exit\n--render-demo PATH      Render the real overlay to PNG without a desktop\n--light                 Use light theme for --render-demo\n--bench PATH            Write isolated core benchmark results as JSON\n--version               Version\n\nGoogle receives only the final selection when you choose Google. Optional confirmation is in Settings.");
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--render-demo") {
        let path = args.get(i + 1).ok_or("--render-demo requires a PNG path")?;
        let e = ui::demo();
        let r = Renderer::new();
        let (image, _) = ui::overlay(
            &r,
            &e,
            ui::View::fit(&e.image, 1280, 800),
            !args.iter().any(|a| a == "--light"),
            (0, 0),
            "",
            false,
        );
        return image.save(std::path::Path::new(path));
    }
    if args.iter().any(|a| a == "--google-test-server") {
        let image = ui::demo();
        let r = Renderer::new();
        let final_image = r
            .flatten(&image.image, &image.annotations)
            .crop(image.selection().unwrap())?;
        let handoff = luma_shot::google::Handoff::from_owned(
            final_image,
            args.iter().any(|a| a == "--fast-search"),
        )?;
        println!("{}", handoff.url());
        return handoff.serve();
    }
    if let Some(i) = args.iter().position(|a| a == "--bench") {
        let path = args.get(i + 1).ok_or("--bench requires a JSON path")?;
        let mut image = Image::new(3840, 2160)?;
        for (y, row) in image.rgba.chunks_exact_mut(3840 * 4).enumerate() {
            for (x, p) in row.chunks_exact_mut(4).enumerate() {
                p.copy_from_slice(&[(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
            }
        }
        let mut samples = vec![];
        let mut bytes = 0;
        for _ in 0..12 {
            let now = std::time::Instant::now();
            let out = image.png()?;
            samples.push(now.elapsed().as_secs_f64() * 1000.);
            bytes = out.len()
        }
        samples.sort_by(f64::total_cmp);
        let report = serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"benchmark":"CPU PNG encoding only; not capture-to-clipboard or a Lightshot comparison","width":3840,"height":2160,"iterations":samples.len(),"median_ms":samples[samples.len()/2],"max_ms":samples.last(),"png_bytes":bytes,"capture_latency_ms":null,"clipboard_latency_ms":null,"idle_memory_bytes":null,"idle_cpu_percent":null});
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap())
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--capture") {
        let p = args.get(i + 1).ok_or("--capture requires an output path")?;
        return platform::capture_file(std::path::Path::new(p));
    }
    platform::run(args.iter().any(|a| a == "--demo"))
}
