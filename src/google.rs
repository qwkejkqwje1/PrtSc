//! Explicit, single-use browser handoff. No screenshot or HTML is written to disk.
use crate::image::Image;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};
pub struct Handoff {
    listener: TcpListener,
    token: String,
    payload: std::sync::mpsc::Receiver<Result<Payload, String>>,
    fast: bool,
    started: Instant,
}
pub struct Payload {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub encode_ms: f64,
}
pub fn encode(image: Image, fast: bool) -> Result<Payload, String> {
    let now = Instant::now();
    let image = if fast && image.width.max(image.height) > 2048 {
        let scale = 2048.0 / image.width.max(image.height) as f64;
        crate::render::resized(
            &image,
            (image.width as f64 * scale).round().max(1.) as u32,
            (image.height as f64 * scale).round().max(1.) as u32,
        )?
    } else {
        image
    };
    let bytes = if fast {
        let mut out = Vec::new();
        jpeg_encoder::Encoder::new(&mut out, 90)
            .encode(
                &image.rgba,
                image.width as u16,
                image.height as u16,
                jpeg_encoder::ColorType::Rgba,
            )
            .map_err(|e| e.to_string())?;
        out
    } else {
        image.png()?
    };
    Ok(Payload {
        bytes,
        width: image.width,
        height: image.height,
        encode_ms: now.elapsed().as_secs_f64() * 1000.,
    })
}
impl Handoff {
    pub fn new(image: &Image) -> Result<Self, String> {
        Self::from_owned(image.clone(), false)
    }
    /// Bind immediately; encoding overlaps browser activation instead of delaying it.
    pub fn from_owned(image: Image, fast: bool) -> Result<Self, String> {
        let started = Instant::now();
        let mut random = [0u8; 24];
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Security::Cryptography::*;
            if BCryptGenRandom(
                std::ptr::null_mut(),
                random.as_mut_ptr(),
                random.len() as u32,
                BCRYPT_USE_SYSTEM_PREFERRED_RNG,
            ) < 0
            {
                return Err("Не удалось получить безопасное случайное число".into());
            }
        }
        #[cfg(not(windows))]
        {
            std::fs::File::open("/dev/urandom")
                .and_then(|mut f| f.read_exact(&mut random))
                .map_err(|e| e.to_string())?;
        }
        let token = random.iter().map(|b| format!("{b:02x}")).collect();
        let listener =
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let (tx, payload) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("google-image-encoder".into())
            .spawn(move || {
                let _ = tx.send(encode(image, fast));
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            listener,
            token,
            payload,
            fast,
            started,
        })
    }
    pub fn url(&self) -> String {
        format!(
            "http://{}/{}/",
            self.listener.local_addr().unwrap(),
            self.token
        )
    }
    pub fn html_for(&self, payload: &Payload) -> String {
        let nonce = &self.token;
        let (extension, mime) = if self.fast {
            ("jpg", "image/jpeg")
        } else {
            ("png", "image/png")
        };
        let encoded = STANDARD.encode(&payload.bytes);
        format!(
            r#"<!doctype html><html lang="ru"><meta charset="utf-8"><meta name="referrer" content="no-referrer"><title>PrtSc → Google</title><p>Открываем Google…</p><form method="post" enctype="multipart/form-data" action="https://lens.google.com/v3/upload?ep=ccm&amp;hl=ru"><input name="encoded_image" type="file" accept="{mime}"><input name="hl" value="ru" type="hidden"><button>Google</button></form><script nonce="{nonce}">const e='{encoded}';let b;if(Uint8Array.fromBase64){{b=Uint8Array.fromBase64(e)}}else{{const s=atob(e);b=new Uint8Array(s.length);for(let i=0;i<s.length;i++)b[i]=s.charCodeAt(i)}};const d=new DataTransfer();d.items.add(new File([b],'PrtSc.{extension}',{{type:'{mime}'}}));document.querySelector('input[type=file]').files=d.files;document.querySelector('form').submit();</script><noscript>Для передачи нужен JavaScript.</noscript></html>"#
        )
    }
    pub fn serve(self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(45);
        let host = self
            .listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .to_string();
        let target = format!("GET /{}/ HTTP/1.", self.token);
        let mut payload = None;
        // Browsers may preconnect without sending headers. Never let an idle socket
        // delay a real request behind a blocking read timeout.
        let mut pending: Vec<(std::net::TcpStream, Vec<u8>, Instant)> = Vec::new();
        while Instant::now() < deadline {
            for _ in 0..16 {
                match self.listener.accept() {
                    Ok((stream, addr)) if addr.ip().is_loopback() && pending.len() < 32 => {
                        stream.set_nonblocking(true).map_err(|e| e.to_string())?;
                        let _ = stream.set_nodelay(true);
                        pending.push((stream, Vec::new(), Instant::now()));
                    }
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => return Err(e.to_string()),
                }
            }
            let mut i = 0;
            while i < pending.len() {
                let (stream, bytes, since) = &mut pending[i];
                let mut buf = [0u8; 1024];
                let mut closed = since.elapsed() > Duration::from_secs(2);
                for _ in 0..8 {
                    match stream.read(&mut buf) {
                        Ok(0) => {
                            closed = true;
                            break;
                        }
                        Ok(n) => {
                            bytes.extend_from_slice(&buf[..n]);
                            if bytes.windows(4).any(|w| w == b"\r\n\r\n") || bytes.len() >= 8192 {
                                break;
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(_) => {
                            closed = true;
                            break;
                        }
                    }
                }
                let complete = bytes.windows(4).any(|w| w == b"\r\n\r\n");
                if !complete {
                    if closed || bytes.len() >= 8192 {
                        pending.swap_remove(i);
                    } else {
                        i += 1;
                    }
                    continue;
                }
                let (mut stream, bytes, _) = pending.swap_remove(i);
                stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));
                let request = String::from_utf8_lossy(&bytes);
                if !request.starts_with(&target)
                    || !request
                        .lines()
                        .any(|l| l.trim().eq_ignore_ascii_case(&format!("Host: {host}")))
                    || request
                        .lines()
                        .any(|l| l.to_ascii_lowercase().starts_with("origin:"))
                {
                    let _ = stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
                if payload.is_none() {
                    payload = Some(
                        self.payload
                            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                            .map_err(|e| e.to_string())??,
                    )
                }
                let p = payload.as_ref().unwrap();
                let html = self.html_for(p);
                let headers=format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; script-src 'nonce-{}'; form-action https://lens.google.com https://www.google.com https://accounts.google.com https://consent.google.com; base-uri 'none'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n",html.len(),self.token);
                if stream
                    .write_all(headers.as_bytes())
                    .and_then(|_| stream.write_all(html.as_bytes()))
                    .is_err()
                {
                    continue;
                }
                let metrics = serde_json::json!({"transport":"inline-single-response","mode":if self.fast{"fast"}else{"original"},"encode_ms":p.encode_ms,"payload_bytes":p.bytes.len(),"width":p.width,"height":p.height,"handoff_ms":self.started.elapsed().as_secs_f64()*1000.,"note":"Local handoff only, not Google result latency"});
                let dir = crate::util::data_dir();
                if crate::util::private_dir(&dir).is_ok() {
                    let _ = crate::config::atomic_write(
                        &dir.join("last-search.json"),
                        &serde_json::to_vec_pretty(&metrics).unwrap(),
                    );
                }
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Err("Браузер не открыл поиск за 45 секунд. Повторите поиск.".into())
    }
}
