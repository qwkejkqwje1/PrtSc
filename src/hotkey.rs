//! Platform-independent parser for RegisterHotKey modifier/key values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: u32,
    pub key: u32,
}
pub fn parse(input: &str) -> Result<Hotkey, String> {
    let mut modifiers = 0;
    let mut key = None;
    for item in input.split('+') {
        let token = item.trim().to_ascii_uppercase();
        let modifier = match token.as_str() {
            "CTRL" | "CONTROL" => 2,
            "ALT" => 1,
            "SHIFT" => 4,
            "WIN" | "WINDOWS" => 8,
            _ => 0,
        };
        if modifier != 0 {
            if modifiers & modifier != 0 {
                return Err("Duplicate shortcut modifier".into());
            }
            modifiers |= modifier;
            continue;
        }
        let k = match token.as_str() {
            "PRINTSCREEN" | "PRTSC" | "PRINT SCREEN" => 0x2c,
            "SPACE" => 0x20,
            "PAUSE" => 0x13,
            "INSERT" => 0x2d,
            "HOME" => 0x24,
            "END" => 0x23,
            "PAGEUP" => 0x21,
            "PAGEDOWN" => 0x22,
            _ if token.len() == 1 && token.as_bytes()[0].is_ascii_alphanumeric() => {
                token.as_bytes()[0] as u32
            }
            _ if token.starts_with('F') => {
                let n = token[1..]
                    .parse::<u32>()
                    .map_err(|_| "Use PrintScreen, A–Z, 0–9 or F1–F24")?;
                if !(1..=24).contains(&n) {
                    return Err("Function key: F1–F24".into());
                }
                0x70 + n - 1
            }
            _ => {
                return Err("Клавиша: PrintScreen, A–Z, 0–9, F1–F24; сочетание: Ctrl+Alt+S".into())
            }
        };
        if key.replace(k).is_some() {
            return Err("Only one key plus Ctrl/Alt/Shift/Win modifiers".into());
        }
    }
    let result = Hotkey {
        modifiers,
        key: key.ok_or("Specify a capture key")?,
    };
    if result
        == (Hotkey {
            modifiers: 6,
            key: 82,
        })
    {
        return Err("Ctrl+Shift+R зарезервировано для видеозаписи".into());
    }
    if result.key == 0x7b {
        return Err("F12 is reserved by Windows debuggers".into());
    }
    Ok(result)
}

/// Convert a physical Windows key event to a layout-independent configuration string.
pub fn format_event(modifiers: u32, key: u32) -> Result<String, String> {
    let name =
        match key {
            0x30..=0x39 | 0x41..=0x5a => (key as u8 as char).to_string(),
            0x70..=0x87 => format!("F{}", key - 0x70 + 1),
            0x2c => "PrintScreen".into(),
            0x20 => "Space".into(),
            0x13 => "Pause".into(),
            0x2d => "Insert".into(),
            0x24 => "Home".into(),
            0x23 => "End".into(),
            0x21 => "PageUp".into(),
            0x22 => "PageDown".into(),
            _ => return Err(
                "Use A–Z, 0–9, F1–F24, PrintScreen, Space, Pause, Home/End/PageUp/PageDown/Insert"
                    .into(),
            ),
        };
    let mut parts = Vec::new();
    for (mask, label) in [(2, "Ctrl"), (1, "Alt"), (4, "Shift"), (8, "Win")] {
        if modifiers & mask != 0 {
            parts.push(label.to_string());
        }
    }
    parts.push(name);
    let text = parts.join("+");
    parse(&text)?;
    Ok(text)
}
