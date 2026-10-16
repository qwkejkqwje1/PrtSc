//! Only explicit uploads of the flattened selection. No cookies or background upload.
use crate::image::Image;
pub struct Upload {
    pub host: &'static str,
    pub path: &'static str,
    pub content_type: String,
    pub body: Vec<u8>,
}
pub fn request(image: &Image, service: &str) -> Result<Upload, String> {
    let png = image.png()?;
    if png.len() > 50_000_000 {
        return Err("Image exceeds 50 MB".into());
    }
    let boundary = format!("PrtSc{}", crate::util::unique_id());
    let mut body = Vec::with_capacity(png.len() + 512);
    let temporary = match service {
        "litterbox" => true,
        "catbox" => false,
        "imgur" => false,
        _ => return Err("Unknown image host".into()),
    };
    if service != "imgur" {
        for (name, value) in [
            ("reqtype", "fileupload"),
            ("time", if temporary { "1h" } else { "" }),
        ] {
            if value.is_empty() {
                continue;
            }
            body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
        }
    }
    let field = if service == "imgur" {
        "image"
    } else {
        "fileToUpload"
    };
    body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"PrtSc.png\"\r\nContent-Type: image/png\r\n\r\n").as_bytes());
    body.extend_from_slice(&png);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Ok(Upload {
        host: if temporary {
            "litterbox.catbox.moe"
        } else if service == "imgur" {
            "api.imgur.com"
        } else {
            "catbox.moe"
        },
        path: if temporary {
            "/resources/internals/api.php"
        } else if service == "imgur" {
            "/3/image"
        } else {
            "/user/api.php"
        },
        content_type: format!("multipart/form-data; boundary={boundary}"),
        body,
    })
}
pub fn link(response: &[u8], service: &str) -> Result<String, String> {
    let text = std::str::from_utf8(response)
        .map_err(|_| "Host response is not UTF-8")?
        .trim();
    let owned;
    let text = if service == "imgur" {
        let j: serde_json::Value = serde_json::from_str(text).map_err(|_| "Invalid Imgur JSON")?;
        if j["success"] != true {
            return Err("Imgur rejected the upload".into());
        }
        owned = j["data"]["link"]
            .as_str()
            .ok_or("Imgur returned no link")?
            .to_string();
        owned.as_str()
    } else {
        text
    };
    let prefix = match service {
        "litterbox" => "https://litter.catbox.moe/",
        "catbox" => "https://files.catbox.moe/",
        "imgur" => "https://i.imgur.com/",
        _ => return Err("Unknown image host".into()),
    };
    if !text.starts_with(prefix)
        || text.len() > 2048
        || text[prefix.len()..].is_empty()
        || text
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '<' || c == '"')
    {
        return Err("Host did not return a valid HTTPS image link".into());
    }
    Ok(text.to_string())
}
