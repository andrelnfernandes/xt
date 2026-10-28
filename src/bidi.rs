//! Trojan Source guard: detects, escapes and strips the bidi controls
//! (and C0/DEL/C1 control characters) so none of them reaches the terminal.

use ratatui::{buffer::Buffer, style::Style};

/// True for the 12 bidi controls: U+202A-U+202E, U+2066-U+2069, U+200E,
/// U+200F and U+061C.
pub fn is_bidi(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// True if `s` contains any bidi control.
pub fn has_bidi(s: &str) -> bool {
    s.chars().any(is_bidi)
}

/// Replaces each bidi control and each control character with `\u{XXXX}`
/// (uppercase hex, no leading zeros).
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if is_bidi(c) || c.is_control() {
            out.push_str(&format!("\\u{{{:X}}}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// Strips bidi and control characters from every cell of the frame, shows
/// `·` in a cell left empty, and applies `mark` to each cell it changed.
pub fn sanitize(buf: &mut Buffer, mark: Style) {
    for cell in &mut buf.content {
        let sym = cell.symbol();
        if matches!(sym.as_bytes(), [b] if (0x20..=0x7E).contains(b)) {
            continue;
        }
        if !sym.chars().any(|c| is_bidi(c) || c.is_control()) {
            continue;
        }
        let clean: String = sym
            .chars()
            .filter(|&c| !is_bidi(c) && !c.is_control())
            .collect();
        cell.set_symbol(if clean.is_empty() { "·" } else { &clean });
        cell.set_style(mark);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Style},
    };

    const TWELVE: [char; 12] = [
        '\u{061C}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}',
        '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
    ];

    #[test]
    fn knows_the_twelve() {
        for c in TWELVE {
            assert!(is_bidi(c), "{c:?}");
        }
        for c in ['a', 'ç', '─', '\u{200B}', '\u{200D}', '\u{FEFF}'] {
            assert!(!is_bidi(c), "{c:?}");
        }
        assert!(has_bidi("fatura\u{202E}fdp.sh"));
        assert!(!has_bidi("ação"));
    }

    #[test]
    fn strips_bidi_and_marks_the_cell() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf[(0, 0)].set_symbol("a\u{202E}");
        buf[(1, 0)].set_symbol("\u{202E}");
        buf[(2, 0)].set_symbol("e\u{301}\u{2066}");
        buf[(3, 0)].set_symbol("b");
        sanitize(&mut buf, Style::new().fg(Color::Red));
        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(0, 0)].fg, Color::Red);
        assert_eq!(buf[(1, 0)].symbol(), "·");
        assert_eq!(buf[(2, 0)].symbol(), "e\u{301}");
        assert_eq!(buf[(3, 0)].symbol(), "b");
        assert_eq!(buf[(3, 0)].fg, Color::Reset);
    }

    #[test]
    fn strips_control_chars_that_slipped_through() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf[(0, 0)].set_symbol("\u{1B}");
        buf[(1, 0)].set_symbol("x\u{9B}");
        sanitize(&mut buf, Style::new());
        assert_eq!(buf[(0, 0)].symbol(), "·");
        assert_eq!(buf[(1, 0)].symbol(), "x");
    }

    #[test]
    fn escapes_hidden_chars() {
        assert_eq!(escape("fatura\u{202E}fdp.sh"), "fatura\\u{202E}fdp.sh");
        assert_eq!(escape("a\u{1B}b"), "a\\u{1B}b");
        assert_eq!(escape("ação"), "ação");
    }
}
