//! Syntax highlighting for the viewer, using syntect (the engine behind
//! Sublime Text and `bat`).
//!
//! Lines are highlighted lazily, only up to the last one shown, and the
//! result is cached; files longer than `MAX_LINES` are colored only up to
//! there, and lines longer than `MAX_LINE_CHARS` are not colored, so huge
//! files never slow the viewer down. Colors are adjusted to keep enough
//! contrast with the background of the current Xylem theme (the classic
//! blue one included).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

use ratatui::style::Color;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Lines past this one are shown without colors.
pub const MAX_LINES: usize = 20_000;

/// Longer lines are shown without colors: syntect takes ~9 ms per 10,000
/// characters of one JSON line (60 ms of Rust), and 11 s for a 16 MiB one.
pub const MAX_LINE_CHARS: usize = 10_000;

/// Name that `for_name` colors like `git status` does in a terminal.
pub const GIT_STATUS: &str = "git status";

/// Text that brings its own ANSI colors (git's graph): see [`from_ansi`].
pub const ANSI: &str = "ansi";

/// Minimum contrast ratio between text and background (WCAG uses 4.5 for
/// body text; 3.5 keeps the theme's colors closer to the original).
const MIN_CONTRAST: f64 = 3.5;

static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
static THEMES: OnceLock<ThemeSet> = OnceLock::new();
static ENABLED: AtomicBool = AtomicBool::new(true);
/// syntect theme and the RGB background it must contrast with.
static STYLE: RwLock<(&str, (u8, u8, u8))> = RwLock::new(("base16-eighties.dark", (52, 101, 164)));

fn syntaxes() -> &'static SyntaxSet {
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn themes() -> &'static ThemeSet {
    THEMES.get_or_init(ThemeSet::load_defaults)
}

/// Turns highlighting on or off for viewers opened from now on.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Picks the syntect theme for a Xylem theme and the background to keep
/// contrast with.
pub fn set_xt_theme(name: &str, background: Color) {
    let theme = match name {
        "light" => "InspiredGitHub",
        "solarized" => "Solarized (dark)",
        _ => "base16-eighties.dark",
    };
    *STYLE.write().unwrap_or_else(|e| e.into_inner()) = (theme, rgb_of(background));
}

/// Approximate RGB of a terminal color (named colors use the Tango
/// palette, the default of GNOME Terminal and Terminator).
fn rgb_of(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Blue => (52, 101, 164),
        Color::White => (238, 238, 236),
        Color::Gray => (211, 215, 207),
        Color::DarkGray => (85, 87, 83),
        // Reset (the terminal's own background) and the rest: assume dark.
        _ => (30, 30, 30),
    }
}

/// Highlighting state of one open file.
pub struct Highlighter {
    /// `None` when all lines were colored up front (`git status`).
    lines: Option<HighlightLines<'static>>,
    /// Colored segments of the lines already highlighted.
    done: Vec<Vec<(Color, String)>>,
    background: (u8, u8, u8),
    /// Language name, e.g. "Rust".
    pub language: String,
    failed: bool,
}

/// The language of `path` (by extension, file name or first line, like
/// `#!/bin/sh`), if known: "Rust", "Python", "Markdown"...
pub fn language(path: &Path, first_line: &str) -> Option<String> {
    find_syntax(path, first_line).map(|s| s.name.clone())
}

fn find_syntax(path: &Path, first_line: &str) -> Option<&'static SyntaxReference> {
    let ss = syntaxes();
    let name = path.file_name()?.to_string_lossy();
    let ext = path.extension().map(|e| e.to_string_lossy());
    ext.as_deref()
        .and_then(|e| ss.find_syntax_by_extension(e))
        // Whole names like "Makefile" or "Rakefile"; short ones ("t",
        // "go") would be mistaken for extensions.
        .or_else(|| {
            (ext.is_none() && name.chars().count() >= 4)
                .then(|| ss.find_syntax_by_extension(&name))
                .flatten()
        })
        .or_else(|| ss.find_syntax_by_first_line(first_line))
        .filter(|s| s.name != "Plain Text")
}

impl Highlighter {
    /// Colors for `name`: a file name, or `GIT_STATUS`.
    pub fn for_name(name: &Path, lines: &[String]) -> Option<Self> {
        if name == Path::new(GIT_STATUS) {
            return Self::git_status(lines);
        }
        Self::for_file(name, lines.first().map_or("", String::as_str))
    }

    /// `git status`, as git colors it in a terminal: the files staged in
    /// green, the others (not staged, untracked, in conflict) in red. Git
    /// indents the files with a tab, which the viewer turns into 8 spaces.
    fn git_status(lines: &[String]) -> Option<Self> {
        if !enabled() {
            return None;
        }
        let background = STYLE.read().unwrap_or_else(|e| e.into_inner()).1;
        let color = |rgb| {
            let (r, g, b) = readable(rgb, background);
            Color::Rgb(r, g, b)
        };
        let (green, red) = (color((78, 154, 6)), color((204, 0, 0)));
        let mut section = None;
        let done = lines
            .iter()
            .take(MAX_LINES)
            .map(|line| {
                if line.is_empty() {
                    section = None;
                } else if line.starts_with("Changes to be committed") {
                    section = Some(green);
                } else if ["Changes not staged", "Untracked files", "Unmerged paths"]
                    .iter()
                    .any(|h| line.starts_with(h))
                {
                    section = Some(red);
                }
                match section {
                    Some(c) if line.starts_with("        ") => vec![(c, line.clone())],
                    _ => Vec::new(),
                }
            })
            .collect();
        Some(Highlighter {
            lines: None,
            done,
            background,
            language: "Git status".into(),
            failed: false,
        })
    }

    /// A highlighter for `path`, if its language is known (by extension,
    /// file name or first line, like `#!/bin/sh`).
    pub fn for_file(path: &Path, first_line: &str) -> Option<Self> {
        if !enabled() {
            return None;
        }
        let syntax = find_syntax(path, first_line)?;
        let (theme_name, background) = *STYLE.read().unwrap_or_else(|e| e.into_inner());
        let theme: &'static Theme = themes().themes.get(theme_name)?;
        Some(Highlighter {
            lines: Some(HighlightLines::new(syntax, theme)),
            done: Vec::new(),
            background,
            language: syntax.name.clone(),
            failed: false,
        })
    }

    /// Colored segments of line `i` of `lines`, highlighting the lines
    /// before it first if needed. `None` past `MAX_LINES` or after an error.
    pub fn line(&mut self, i: usize, lines: &[String]) -> Option<&[(Color, String)]> {
        if i >= MAX_LINES || i >= lines.len() {
            return None;
        }
        while !self.failed && self.done.len() <= i {
            let Some(highlight) = &mut self.lines else {
                break;
            };
            let line = &lines[self.done.len()];
            // Skipped, not fed to syntect: the lines after it start from
            // the state before it.
            if line.len() > MAX_LINE_CHARS && line.chars().count() > MAX_LINE_CHARS {
                self.done.push(Vec::new());
                continue;
            }
            let text = format!("{line}\n");
            match highlight.highlight_line(&text, syntaxes()) {
                Ok(ranges) => {
                    let segments = ranges
                        .into_iter()
                        .map(|(style, s)| {
                            let c = style.foreground;
                            let fg = readable((c.r, c.g, c.b), self.background);
                            (
                                Color::Rgb(fg.0, fg.1, fg.2),
                                s.trim_end_matches('\n').to_string(),
                            )
                        })
                        .filter(|(_, s)| !s.is_empty())
                        .collect();
                    self.done.push(segments);
                }
                Err(_) => self.failed = true,
            }
        }
        self.done.get(i).map(Vec::as_slice)
    }
}

/// Text with ANSI colors, as `git log --graph --color` writes it: the text
/// without the escape sequences, and its colors. Only the text color of the
/// SGR sequences (`ESC [ ... m`) counts; any other sequence is dropped, and
/// other control characters are left for the viewer to show as `·`. Tabs
/// are expanded here, so the colors stay on their characters.
pub fn from_ansi(text: &[u8]) -> (Vec<u8>, Option<Highlighter>) {
    type Rgb = (u8, u8, u8);
    let text = String::from_utf8_lossy(text);
    let mut plain = String::with_capacity(text.len());
    let mut done = Vec::new();
    let mut fg: Option<Rgb> = None;
    for line in text.split_inclusive('\n') {
        let line = line.strip_suffix('\n').unwrap_or(line);
        let mut segments: Vec<(Option<Rgb>, String)> = Vec::new();
        let mut col = 0;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                if chars.next_if_eq(&'[').is_some() {
                    let mut params = String::new();
                    while let Some(p) = chars.next_if(|p| !('\u{40}'..='\u{7e}').contains(p)) {
                        params.push(p);
                    }
                    if chars.next() == Some('m') {
                        fg = sgr(&params, fg);
                    }
                }
                continue;
            }
            let piece = if c == '\t' {
                " ".repeat(8 - col % 8)
            } else {
                c.to_string()
            };
            col += piece.chars().count();
            plain.push_str(&piece);
            match segments.last_mut() {
                Some((color, s)) if *color == fg => s.push_str(&piece),
                _ => segments.push((fg, piece)),
            }
        }
        plain.push('\n');
        done.push(segments);
    }
    if !enabled() {
        return (plain.into_bytes(), None);
    }
    let background = STYLE.read().unwrap_or_else(|e| e.into_inner()).1;
    let text_color = crate::config::theme().text;
    let done = done
        .into_iter()
        .take(MAX_LINES)
        .map(|segments| {
            segments
                .into_iter()
                .map(|(c, s)| {
                    let color = c.map_or(text_color, |rgb| {
                        let (r, g, b) = readable(rgb, background);
                        Color::Rgb(r, g, b)
                    });
                    (color, s)
                })
                .collect()
        })
        .collect();
    let colors = Highlighter {
        lines: None,
        done,
        background,
        language: "Git graph".into(),
        failed: false,
    };
    (plain.into_bytes(), Some(colors))
}

/// The text color after the SGR sequence with these parameters (`1;32`).
fn sgr(params: &str, mut fg: Option<(u8, u8, u8)>) -> Option<(u8, u8, u8)> {
    let mut nums = params.split([';', ':']).map(|p| {
        if p.is_empty() {
            Some(0)
        } else {
            p.parse::<u16>().ok()
        }
    });
    while let Some(n) = nums.next() {
        let Some(n) = n else { break };
        match n {
            0 | 39 => fg = None,
            30..=37 => fg = Some(ansi16(n as usize - 30)),
            90..=97 => fg = Some(ansi16(n as usize - 90 + 8)),
            38 => {
                let mut next = || nums.next().flatten();
                fg = match next() {
                    Some(5) => next().map(|i| ansi256(i as usize)).or(fg),
                    Some(2) => match (next(), next(), next()) {
                        (Some(r), Some(g), Some(b)) => {
                            let byte = |v: u16| v.min(255) as u8;
                            Some((byte(r), byte(g), byte(b)))
                        }
                        _ => fg,
                    },
                    _ => fg,
                };
            }
            // Background colors' arguments, so they are not read as colors.
            48 => {
                let skip = match nums.next().flatten() {
                    Some(5) => 1,
                    Some(2) => 3,
                    _ => 0,
                };
                for _ in 0..skip {
                    nums.next();
                }
            }
            _ => {}
        }
    }
    fg
}

/// The 16 basic colors, in the Tango palette (see `rgb_of`).
fn ansi16(i: usize) -> (u8, u8, u8) {
    const PALETTE: [(u8, u8, u8); 16] = [
        (46, 52, 54),
        (204, 0, 0),
        (78, 154, 6),
        (196, 160, 0),
        (52, 101, 164),
        (117, 80, 123),
        (6, 152, 154),
        (211, 215, 207),
        (85, 87, 83),
        (239, 41, 41),
        (138, 226, 52),
        (252, 233, 79),
        (114, 159, 207),
        (173, 127, 168),
        (52, 226, 226),
        (238, 238, 236),
    ];
    PALETTE[i % 16]
}

/// The xterm 256-color palette.
fn ansi256(i: usize) -> (u8, u8, u8) {
    match i {
        0..=15 => ansi16(i),
        16..=231 => {
            let level = |v: usize| if v == 0 { 0 } else { (55 + v * 40) as u8 };
            let i = i - 16;
            (level(i / 36), level(i / 6 % 6), level(i % 6))
        }
        _ => {
            let v = (8 + (i.min(255) - 232) * 10) as u8;
            (v, v, v)
        }
    }
}

/// `fg` adjusted (lightened on dark backgrounds, darkened on light ones)
/// until it has enough contrast with `bg`.
fn readable(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let toward: f64 = if luminance(bg) < 0.4 { 255.0 } else { 0.0 };
    let mut c = fg;
    for step in 1..=10 {
        if contrast(c, bg) >= MIN_CONTRAST {
            break;
        }
        let t = step as f64 / 10.0;
        let mix = |a: u8| (a as f64 + (toward - a as f64) * t).round() as u8;
        c = (mix(fg.0), mix(fg.1), mix(fg.2));
    }
    c
}

/// Relative luminance (WCAG).
fn luminance((r, g, b): (u8, u8, u8)) -> f64 {
    let lin = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(String::from).collect()
    }

    #[test]
    fn ansi_colors_without_the_sequences() {
        let text = b"* \x1b[33m1a2b3c4\x1b[m \x1b[33m(\x1b[1;36mHEAD -> \x1b[m\x1b[1;32mmain\x1b[m\x1b[33m)\x1b[m first\n\
            \x1b[31m|\x1b[m\x1b[2J\x1b]0;t\x07\ta\n";
        let (plain, colors) = from_ansi(text);
        let plain = String::from_utf8(plain).unwrap();
        assert_eq!(
            plain, "* 1a2b3c4 (HEAD -> main) first\n|]0;t\x07  a\n",
            "only the escape characters go; tabs expanded"
        );
        let mut h = colors.unwrap();
        let first = h.line(0, &lines(&plain)).unwrap().to_vec();
        let color_of = |segs: &[(Color, String)], word: &str| {
            segs.iter().find(|(_, s)| s.contains(word)).map(|(c, _)| *c)
        };
        assert_ne!(color_of(&first, "1a2b3c4"), color_of(&first, "main"));
        assert_ne!(color_of(&first, "HEAD"), color_of(&first, "main"));
        assert_eq!(
            color_of(&first, "first"),
            Some(crate::config::theme().text),
            "uncolored text"
        );
        let joined: String = first.iter().map(|(_, s)| s.as_str()).collect();
        assert_eq!(joined, "* 1a2b3c4 (HEAD -> main) first");
    }

    #[test]
    fn sgr_text_colors() {
        assert_eq!(sgr("1;32", None), Some(ansi16(2)));
        assert_eq!(sgr("", Some((1, 2, 3))), None);
        assert_eq!(sgr("39", Some((1, 2, 3))), None);
        assert_eq!(sgr("38;5;196", None), Some((255, 0, 0)));
        assert_eq!(sgr("38;5;244", None), Some((128, 128, 128)));
        assert_eq!(sgr("38;2;10;20;300", None), Some((10, 20, 255)));
        assert_eq!(sgr("48;5;10", None), None, "background only");
        assert_eq!(sgr("48;2;1;2;3;91", None), Some(ansi16(9)));
        assert_eq!(sgr("38;5", Some((1, 2, 3))), Some((1, 2, 3)), "incomplete");
    }

    #[test]
    fn detects_languages() {
        let rs = Highlighter::for_file(Path::new("main.rs"), "fn main() {}").unwrap();
        assert_eq!(rs.language, "Rust");
        let sh = Highlighter::for_file(Path::new("deploy"), "#!/bin/bash").unwrap();
        assert_eq!(sh.language, "Bourne Again Shell (bash)");
        let py = Highlighter::for_file(Path::new("tool"), "#!/usr/bin/env python3").unwrap();
        assert_eq!(py.language, "Python");
        assert!(Highlighter::for_file(Path::new("Makefile"), "all:").is_some());
        assert!(Highlighter::for_file(Path::new("notes.txt"), "hello").is_none());
        assert!(Highlighter::for_file(Path::new("data.xyz"), "hello").is_none());
        assert!(
            Highlighter::for_file(Path::new("t"), "hello").is_none(),
            "not Perl"
        );
    }

    #[test]
    fn colors_keywords_differently_from_names() {
        let text = lines("fn main() {\n    let x = 1;\n}");
        let mut h = Highlighter::for_file(Path::new("a.rs"), &text[0]).unwrap();
        let segs = h.line(1, &text).unwrap().to_vec();
        let joined: String = segs.iter().map(|(_, s)| s.as_str()).collect();
        assert_eq!(joined, "    let x = 1;");
        let color_of = |word: &str| segs.iter().find(|(_, s)| s.trim() == word).map(|(c, _)| *c);
        assert_ne!(color_of("let"), color_of("x"));
        assert!(h.line(MAX_LINES, &text).is_none());
        assert!(h.line(99, &text).is_none());
    }

    /// syntect takes ~11 s on a 16 MiB JSON line (~9 ms per 10,000
    /// characters): such lines are shown plain, and the next ones still
    /// get their colors.
    #[test]
    fn very_long_lines_are_not_colored() {
        let text = vec!["let x = 1; ".repeat(1_000), "    let y = 2;".to_string()];
        let mut h = Highlighter::for_file(Path::new("a.rs"), "").unwrap();
        assert_eq!(h.line(0, &text), Some(&[][..]));
        assert!(h.line(1, &text).is_some_and(|segs| segs.len() > 1));
    }

    #[test]
    fn colors_git_status_like_git() {
        let text = lines(
            "On branch main\n\
             Changes to be committed:\n  \
             (use \"git restore --staged <file>...\" to unstage)\n        \
             new file:   a.rs\n\
             \n\
             Changes not staged for commit:\n        \
             modified:   b.rs\n\
             \n\
             Untracked files:\n        \
             c.rs\n",
        );
        let mut h = Highlighter::for_name(Path::new(GIT_STATUS), &text).unwrap();
        assert_eq!(h.language, "Git status");
        let color = |h: &mut Highlighter, i| h.line(i, &text).unwrap().first().map(|(c, _)| *c);
        let staged = color(&mut h, 3).expect("staged file colored");
        let changed = color(&mut h, 6).expect("changed file colored");
        assert_ne!(staged, changed);
        assert_eq!(color(&mut h, 9), Some(changed), "untracked like changed");
        // Headers and hints keep the normal color.
        for i in [0, 1, 2, 5] {
            assert_eq!(color(&mut h, i), None, "line {i}");
        }
    }

    #[test]
    fn keeps_contrast_with_the_background() {
        let blue = (52, 101, 164);
        let dark_blue_text = (40, 80, 160);
        assert!(contrast(dark_blue_text, blue) < MIN_CONTRAST);
        assert!(contrast(readable(dark_blue_text, blue), blue) >= MIN_CONTRAST);
        // On a light background, light text gets darker.
        let white = (255, 255, 255);
        let pale = (200, 200, 120);
        let fixed = readable(pale, white);
        assert!(contrast(fixed, white) >= MIN_CONTRAST);
        assert!(luminance(fixed) < luminance(pale));
        // Colors that are already fine stay the same.
        assert_eq!(readable((255, 255, 255), (0, 0, 0)), (255, 255, 255));
    }
}
