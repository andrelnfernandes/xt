//! Text encodings for the viewer: UTF-16, CP437, CP850, Windows-1252, plus
//! detection and a CP850/1252 guess for data that is not valid UTF-8.

use crate::viewer::looks_binary;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charset {
    Utf8,
    Utf16Le,
    Utf16Be,
    Cp437,
    Cp850,
    Cp1252,
}

impl Charset {
    pub fn name(self) -> &'static str {
        match self {
            Charset::Utf8 => "UTF-8",
            Charset::Utf16Le => "UTF-16LE",
            Charset::Utf16Be => "UTF-16BE",
            Charset::Cp437 => "CP437",
            Charset::Cp850 => "CP850",
            Charset::Cp1252 => "Windows-1252",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Charset::Utf8 => "Unicode",
            Charset::Utf16Le => "Unicode, little-endian",
            Charset::Utf16Be => "Unicode, big-endian",
            Charset::Cp437 => "DOS, United States",
            Charset::Cp850 => "DOS, Western Europe",
            Charset::Cp1252 => "Windows, Western Europe",
        }
    }

    /// Cycle order: UTF-8 -> CP850 -> 1252 -> CP437 -> UTF-16LE -> UTF-16BE.
    pub fn next(self) -> Charset {
        match self {
            Charset::Utf8 => Charset::Cp850,
            Charset::Cp850 => Charset::Cp1252,
            Charset::Cp1252 => Charset::Cp437,
            Charset::Cp437 => Charset::Utf16Le,
            Charset::Utf16Le => Charset::Utf16Be,
            Charset::Utf16Be => Charset::Utf8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Detected {
    pub charset: Charset,
    pub binary: bool,
}

const GUESS_WINDOW: usize = 64 * 1024;
const UTF16_MIN_BYTES: usize = 16;

/// Detection steps 2 to 6 (decompression happens before this).
/// `fallback` is the fixed legacy charset; `None` uses the heuristic.
pub fn detect(data: &[u8], fallback: Option<Charset>) -> Detected {
    let text = |charset| Detected {
        charset,
        binary: false,
    };
    // 2. BOM.
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return text(Charset::Utf8);
    }
    if data.starts_with(&[0xFF, 0xFE]) {
        return text(Charset::Utf16Le);
    }
    if data.starts_with(&[0xFE, 0xFF]) {
        return text(Charset::Utf16Be);
    }
    // 3. UTF-16 without BOM (before the binary test: it is full of zeros).
    if let Some(cs) = sniff_utf16(data) {
        return text(cs);
    }
    // 4. Null byte: hex.
    if looks_binary(data) {
        return Detected {
            charset: Charset::Utf8,
            binary: true,
        };
    }
    // 5. Valid UTF-8, allowing a sequence cut at the end.
    match std::str::from_utf8(data) {
        Ok(_) => return text(Charset::Utf8),
        Err(e) if e.error_len().is_none() => return text(Charset::Utf8),
        Err(_) => {}
    }
    // 6. Legacy.
    text(fallback.unwrap_or_else(|| guess_legacy(data)))
}

fn sniff_utf16(data: &[u8]) -> Option<Charset> {
    let probe = &data[..data.len().min(crate::viewer::BINARY_PROBE)];
    if probe.len() < UTF16_MIN_BYTES {
        return None;
    }
    let (mut even_zero, mut odd_zero) = (0usize, 0usize);
    for (i, &b) in probe.iter().enumerate() {
        if b == 0 {
            if i % 2 == 0 {
                even_zero += 1;
            } else {
                odd_zero += 1;
            }
        }
    }
    let even_n = probe.len().div_ceil(2);
    let odd_n = probe.len() / 2;
    // Integer math: 70% of one parity are zero and at most 5% of the other.
    if odd_zero * 100 >= odd_n * 70 && even_zero * 100 <= even_n * 5 {
        Some(Charset::Utf16Le)
    } else if even_zero * 100 >= even_n * 70 && odd_zero * 100 <= odd_n * 5 {
        Some(Charset::Utf16Be)
    } else {
        None
    }
}

pub fn decode(data: &[u8], charset: Charset) -> String {
    match charset {
        Charset::Utf8 => {
            let data = data.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(data);
            String::from_utf8_lossy(data).into_owned()
        }
        Charset::Utf16Le => decode_utf16(data, &[0xFF, 0xFE], u16::from_le_bytes),
        Charset::Utf16Be => decode_utf16(data, &[0xFE, 0xFF], u16::from_be_bytes),
        Charset::Cp437 => decode_table(data, &CP437),
        Charset::Cp850 => decode_table(data, &CP850),
        Charset::Cp1252 => decode_table(data, &CP1252),
    }
}

fn decode_utf16(data: &[u8], bom: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let data = data.strip_prefix(bom).unwrap_or(data);
    // as_chunks drops an odd trailing byte (it lands in the remainder).
    let (pairs, _) = data.as_chunks::<2>();
    let units = pairs.iter().map(|&c| unit(c));
    char::decode_utf16(units)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}

fn decode_table(data: &[u8], table: &[char; 128]) -> String {
    data.iter()
        .map(|&b| {
            if b < 0x80 {
                b as char
            } else {
                table[(b - 0x80) as usize]
            }
        })
        .collect()
}

/// Chooses CP850 or Windows-1252 from the first 64 KiB. Ties go to 1252.
pub fn guess_legacy(data: &[u8]) -> Charset {
    let data = &data[..data.len().min(GUESS_WINDOW)];
    let (mut p850, mut p1252) = (0u32, 0u32);
    for (i, &b) in data.iter().enumerate() {
        if b < 0x80 {
            continue;
        }
        let idx = (b - 0x80) as usize;
        let prev = i.checked_sub(1).map(|j| data[j]);
        let next = data.get(i + 1).copied();
        let neighbor = [prev, next]
            .into_iter()
            .flatten()
            .find(|n| n.is_ascii_alphabetic());
        if let Some(n) = neighbor {
            let lower = n.is_ascii_lowercase();
            let same_case = |c: char| {
                c.is_alphabetic()
                    && if lower {
                        c.is_lowercase()
                    } else {
                        c.is_uppercase()
                    }
            };
            if same_case(CP850[idx]) {
                p850 += 1;
            }
            if same_case(CP1252[idx]) {
                p1252 += 1;
            }
        }
        if (0xB3..=0xDA).contains(&b) {
            let boxish = |n: Option<u8>| n.is_some_and(|n| (0xB0..=0xDF).contains(&n));
            if boxish(prev) || boxish(next) {
                p850 += 1;
            }
        }
        if matches!(b, 0x81 | 0x8D | 0x8F | 0x90 | 0x9D) {
            p850 += 1;
        }
    }
    if p850 > p1252 {
        Charset::Cp850
    } else {
        Charset::Cp1252
    }
}

#[rustfmt::skip]
pub(crate) const CP437: [char; 128] = [
    // 0x80
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    // 0x90
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
    // 0xA0
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
    // 0xB0
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
    // 0xC0
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    // 0xD0
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
    // 0xE0
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
    // 0xF0
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{A0}',
];

#[rustfmt::skip]
pub(crate) const CP850: [char; 128] = [
    // 0x80
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    // 0x90
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', 'ø', '£', 'Ø', '×', 'ƒ',
    // 0xA0
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '®', '¬', '½', '¼', '¡', '«', '»',
    // 0xB0
    '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'À', '©', '╣', '║', '╗', '╝', '¢', '¥', '┐',
    // 0xC0
    '└', '┴', '┬', '├', '─', '┼', 'ã', 'Ã', '╚', '╔', '╩', '╦', '╠', '═', '╬', '¤',
    // 0xD0
    'ð', 'Ð', 'Ê', 'Ë', 'È', 'ı', 'Í', 'Î', 'Ï', '┘', '┌', '█', '▄', '¦', 'Ì', '▀',
    // 0xE0
    'Ó', 'ß', 'Ô', 'Ò', 'õ', 'Õ', 'µ', 'þ', 'Þ', 'Ú', 'Û', 'Ù', 'ý', 'Ý', '¯', '´',
    // 0xF0
    '\u{AD}', '±', '‗', '¾', '¶', '§', '÷', '¸', '°', '¨', '·', '¹', '³', '²', '■', '\u{A0}',
];

#[rustfmt::skip]
pub(crate) const CP1252: [char; 128] = [
    // 0x80
    '€', '\u{FFFD}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{FFFD}', 'Ž', '\u{FFFD}',
    // 0x90
    '\u{FFFD}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{FFFD}', 'ž', 'Ÿ',
    // 0xA0
    '\u{A0}', '¡', '¢', '£', '¤', '¥', '¦', '§', '¨', '©', 'ª', '«', '¬', '\u{AD}', '®', '¯',
    // 0xB0
    '°', '±', '²', '³', '´', 'µ', '¶', '·', '¸', '¹', 'º', '»', '¼', '½', '¾', '¿',
    // 0xC0
    'À', 'Á', 'Â', 'Ã', 'Ä', 'Å', 'Æ', 'Ç', 'È', 'É', 'Ê', 'Ë', 'Ì', 'Í', 'Î', 'Ï',
    // 0xD0
    'Ð', 'Ñ', 'Ò', 'Ó', 'Ô', 'Õ', 'Ö', '×', 'Ø', 'Ù', 'Ú', 'Û', 'Ü', 'Ý', 'Þ', 'ß',
    // 0xE0
    'à', 'á', 'â', 'ã', 'ä', 'å', 'æ', 'ç', 'è', 'é', 'ê', 'ë', 'ì', 'í', 'î', 'ï',
    // 0xF0
    'ð', 'ñ', 'ò', 'ó', 'ô', 'õ', 'ö', '÷', 'ø', 'ù', 'ú', 'û', 'ü', 'ý', 'þ', 'ÿ',
];

#[cfg(test)]
pub(crate) mod test_util {
    /// Encodes `s` with a 0x80-0xFF table (reverse lookup); ASCII passes through.
    pub(crate) fn encode(s: &str, table: &[char; 128]) -> Vec<u8> {
        s.chars()
            .map(|c| {
                if (c as u32) < 0x80 {
                    c as u8
                } else {
                    0x80 + table
                        .iter()
                        .position(|&t| t == c)
                        .expect("char not in table") as u8
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::encode;
    use super::*;
    use proptest::prelude::*;

    const PT: &str = "Ação, coração e informação não são fáceis; café à vista.";

    #[test]
    fn tables_are_ascii_below_0x80_and_known_above() {
        assert_eq!(decode(b"abc\t\n", Charset::Cp850), "abc\t\n");
        assert_eq!(
            decode(&[0x87, 0xA0, 0xC6, 0xE4, 0xB3, 0xDA], Charset::Cp850),
            "çáãõ│┌"
        );
        assert_eq!(decode(&[0x87, 0xE0, 0xB3], Charset::Cp437), "çα│");
        assert_eq!(
            decode(&[0x80, 0x87, 0xE7, 0x81], Charset::Cp1252),
            "€‡ç\u{FFFD}"
        );
    }

    #[test]
    fn guesses_cp850_and_1252_from_portuguese() {
        assert_eq!(guess_legacy(&encode(PT, &CP850)), Charset::Cp850);
        assert_eq!(guess_legacy(&encode(PT, &CP1252)), Charset::Cp1252);
        assert_eq!(guess_legacy(b"plain ascii"), Charset::Cp1252); // tie
    }

    #[test]
    fn detects_in_order() {
        assert_eq!(detect("ação".as_bytes(), None).charset, Charset::Utf8);
        assert_eq!(detect(&"ação".as_bytes()[..4], None).charset, Charset::Utf8); // cut mid-sequence
        assert_eq!(detect(&encode(PT, &CP850), None).charset, Charset::Cp850);
        assert_eq!(
            detect(&encode(PT, &CP1252), Some(Charset::Cp437)).charset,
            Charset::Cp437
        );
        let le: Vec<u8> = "plain text here"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let be: Vec<u8> = "plain text here"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect();
        assert_eq!(
            (detect(&le, None).charset, detect(&le, None).binary),
            (Charset::Utf16Le, false)
        );
        assert_eq!(detect(&be, None).charset, Charset::Utf16Be);
        assert!(detect(b"\x7FELF\0\x01\x02", None).binary);
    }

    #[test]
    fn utf16_with_bom_crlf_and_a_cut() {
        let reg = "\u{FEFF}Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER]\r\n";
        let mut bytes: Vec<u8> = reg.encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(detect(&bytes, None).charset, Charset::Utf16Le);
        bytes.push(0x41); // odd byte at the end
        assert!(
            decode(&bytes, Charset::Utf16Le)
                .starts_with("Windows Registry Editor Version 5.00\r\n")
        );
        assert_eq!(decode(&[0x00, 0xD8], Charset::Utf16Le), "\u{FFFD}"); // lone surrogate
    }

    #[test]
    fn next_cycles_through_all_six() {
        let mut c = Charset::Utf8;
        let mut seen = vec![];
        for _ in 0..6 {
            seen.push(c);
            c = c.next();
        }
        assert_eq!(c, Charset::Utf8);
        assert_eq!(seen[1], Charset::Cp850);
        assert_eq!(seen[5], Charset::Utf16Be);
    }

    proptest! {
        #[test]
        fn never_panics(bytes in prop::collection::vec(any::<u8>(), 0..4096), k in 0usize..6) {
            let cs = [Charset::Utf8, Charset::Utf16Le, Charset::Utf16Be, Charset::Cp437, Charset::Cp850, Charset::Cp1252][k];
            let _ = detect(&bytes, None);
            let _ = guess_legacy(&bytes);
            let _ = decode(&bytes, cs);
        }
    }
}
