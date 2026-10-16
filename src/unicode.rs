//! WM_CHAR supplies UTF-16 units, not Unicode scalar values.
pub fn decode_input(pending: &mut u16, unit: u16) -> Option<char> {
    if (0xD800..=0xDBFF).contains(&unit) {
        *pending = unit;
        return None;
    }
    if (0xDC00..=0xDFFF).contains(&unit) {
        let high = std::mem::take(pending);
        if high == 0 {
            return None;
        }
        return char::from_u32(0x10000 + (((high as u32 - 0xD800) << 10) | (unit as u32 - 0xDC00)));
    }
    *pending = 0;
    char::from_u32(unit as u32)
}
