//! Text or byte search: in tagged files (Ctrl+S) and inside the
//! viewer (F, B, /, \, +, -, Space), as in ZTreeWin.
//!
//! Text is searched line by line: a `*` in the middle of the term matches anything
//! on the same line (`erro*disco`), and a term never crosses a
//! line break. When ignoring case, accents count too
//! (`AÇÃO` finds `ação`).

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Lines longer than this (e.g. binary files) are searched in
/// chunks; a term split between two chunks may not be found.
const MAX_LINE: usize = 1024 * 1024;

/// What to search for and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSpec {
    pub text: String,
    pub case_sensitive: bool,
    /// `text` is hexadecimal pairs: "4d 5a 90".
    pub hex: bool,
}

impl SearchSpec {
    pub fn new(text: &str) -> Self {
        SearchSpec {
            text: text.to_string(),
            case_sensitive: false,
            hex: false,
        }
    }

    /// E.g. "text, ignore case".
    pub fn mode_label(&self) -> &'static str {
        match (self.hex, self.case_sensitive) {
            (true, _) => "hex",
            (false, true) => "text, match case",
            (false, false) => "text, ignore case",
        }
    }
}

/// A term ready to search for.
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Parts separated by the inner `*`, already lowercased if applicable.
    Text {
        parts: Vec<Vec<char>>,
        case_sensitive: bool,
    },
    Bytes(Vec<u8>),
}

impl Pattern {
    pub fn new(spec: &SearchSpec) -> Result<Self, String> {
        if spec.text.is_empty() {
            return Err("Empty search".into());
        }
        if spec.hex {
            let bytes = spec
                .text
                .split_whitespace()
                .map(|h| u8::from_str_radix(h, 16).map_err(|_| format!("invalid hex byte '{h}'")))
                .collect::<Result<Vec<u8>, String>>()?;
            if bytes.is_empty() {
                return Err("Empty search".into());
            }
            return Ok(Pattern::Bytes(bytes));
        }
        let case = spec.case_sensitive;
        let norm = |s: &str| -> Vec<char> { s.chars().map(|c| fold(c, case)).collect() };
        // `*` is a wildcard only between two characters; at the ends it is literal,
        // and then so are all the others (ZTreeWin rule).
        let t = &spec.text;
        let wildcards = !t.starts_with('*') && !t.ends_with('*');
        let parts = if wildcards {
            t.split('*').filter(|p| !p.is_empty()).map(norm).collect()
        } else {
            vec![norm(t)]
        };
        Ok(Pattern::Text {
            parts,
            case_sensitive: case,
        })
    }

    /// First occurrence in a line, starting at character `from`:
    /// returns (start, end) in characters.
    pub fn find_in_line(&self, line: &str, from: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = line.chars().collect();
        self.find_in_chars(&chars, from)
    }

    /// Last occurrence that starts before character `before`.
    pub fn rfind_in_line(&self, line: &str, before: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = line.chars().collect();
        let mut best = None;
        let mut from = 0;
        while let Some((s, e)) = self.find_in_chars(&chars, from) {
            if s >= before {
                break;
            }
            best = Some((s, e));
            from = s + 1;
        }
        best
    }

    fn find_in_chars(&self, chars: &[char], from: usize) -> Option<(usize, usize)> {
        match self {
            Pattern::Text {
                parts,
                case_sensitive,
            } => {
                let line: Vec<char> = chars.iter().map(|&c| fold(c, *case_sensitive)).collect();
                // Each part as far left as possible after the previous one:
                // if it doesn't match that way, starting further right won't either.
                let first = find_chars(&line, &parts[0], from)?;
                let mut end = first + parts[0].len();
                for part in &parts[1..] {
                    end = find_chars(&line, part, end)? + part.len();
                }
                Some((first, end))
            }
            Pattern::Bytes(bytes) => {
                // Bytes in a text line: compares against the UTF-8 bytes.
                let text: String = chars.iter().collect();
                let hay = text.as_bytes();
                let pos = find_bytes(hay, bytes, char_to_byte(&text, from))?;
                let s = text[..pos].chars().count();
                let e = text[..pos + bytes.len()].chars().count();
                Some((s, e))
            }
        }
    }

    /// Byte position of the next occurrence in `data` starting at `from`
    /// (for the viewer's hex mode).
    pub fn find_in_bytes(&self, data: &[u8], from: usize) -> Option<(usize, usize)> {
        let needle = self.as_bytes();
        let pos = match self {
            Pattern::Text {
                case_sensitive: false,
                ..
            } => find_bytes_ignore_ascii_case(data, &needle, from)?,
            _ => find_bytes(data, &needle, from)?,
        };
        Some((pos, pos + needle.len()))
    }

    /// Last occurrence in `data` that starts before `before`.
    pub fn rfind_in_bytes(&self, data: &[u8], before: usize) -> Option<(usize, usize)> {
        let mut best = None;
        let mut from = 0;
        while let Some((s, e)) = self.find_in_bytes(data, from) {
            if s >= before {
                break;
            }
            best = Some((s, e));
            from = s + 1;
        }
        best
    }

    /// The term as bytes (the text with literal `*`).
    fn as_bytes(&self) -> Vec<u8> {
        match self {
            Pattern::Bytes(b) => b.clone(),
            Pattern::Text { parts, .. } => {
                let joined: Vec<String> = parts.iter().map(|p| p.iter().collect()).collect();
                joined.join("*").into_bytes()
            }
        }
    }

    /// Does the content have the term? Text is read line by line.
    pub fn matches_reader(&self, mut reader: impl BufRead) -> io::Result<bool> {
        if let Pattern::Bytes(needle) = self {
            return stream_contains(reader, needle);
        }
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let n = Read::take(&mut reader, MAX_LINE as u64).read_until(b'\n', &mut buf)?;
            if n == 0 {
                return Ok(false);
            }
            if self
                .find_in_line(&String::from_utf8_lossy(&buf), 0)
                .is_some()
            {
                return Ok(true);
            }
        }
    }
}

/// Lowercase, if case-insensitive (`Ç` → `ç`).
fn fold(c: char, case_sensitive: bool) -> char {
    if case_sensitive {
        c
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

fn find_chars(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.is_empty() || from > hay.len() || needle.len() > hay.len() - from {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

fn find_bytes(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from > hay.len() || needle.len() > hay.len() - from {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn find_bytes_ignore_ascii_case(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from > hay.len() || needle.len() > hay.len() - from {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
        .map(|p| p + from)
}

fn char_to_byte(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map_or(s.len(), |(i, _)| i)
}

/// Searches for bytes in a stream, in overlapping blocks (to find the term
/// even when split between two blocks).
fn stream_contains(mut reader: impl BufRead, needle: &[u8]) -> io::Result<bool> {
    let mut carry: Vec<u8> = Vec::new();
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            return Ok(false);
        }
        let n = chunk.len();
        carry.extend_from_slice(chunk);
        reader.consume(n);
        if find_bytes(&carry, needle, 0).is_some() {
            return Ok(true);
        }
        let keep = needle.len().saturating_sub(1).min(carry.len());
        carry.drain(..carry.len() - keep);
    }
}

/// Opens a file's content for the search.
pub type Opener = Box<dyn Fn(&Path) -> io::Result<Box<dyn BufRead>>>;

/// A search over the tagged files in progress (Ctrl+S). Each item
/// carries a caller-supplied identifier (`T`), returned in the result.
pub struct SearchJob<T> {
    pub spec: SearchSpec,
    pattern: Pattern,
    items: Vec<(T, PathBuf)>,
    /// Reads an item's content (from disk, or from inside a ZIP/TAR).
    open: Opener,
    next: usize,
    pub found: usize,
    pub errors: Vec<String>,
}

impl<T: Copy> SearchJob<T> {
    pub fn new(spec: SearchSpec, items: Vec<(T, PathBuf)>, open: Opener) -> Result<Self, String> {
        Ok(SearchJob {
            pattern: Pattern::new(&spec)?,
            spec,
            items,
            open,
            next: 0,
            found: 0,
            errors: Vec::new(),
        })
    }

    pub fn total(&self) -> usize {
        self.items.len()
    }

    pub fn done(&self) -> usize {
        self.next
    }

    pub fn is_done(&self) -> bool {
        self.next >= self.items.len()
    }

    pub fn current(&self) -> Option<&Path> {
        self.items.get(self.next).map(|(_, p)| p.as_path())
    }

    /// Searches the next files until `budget` runs out. Returns, for
    /// each one, whether it contains the term. Files that could not be read
    /// stay tagged (and the error is kept), like ZTreeWin's "Keep".
    pub fn step(&mut self, budget: Duration) -> Vec<(T, bool)> {
        let start = Instant::now();
        let mut results = Vec::new();
        while let Some((id, path)) = self.items.get(self.next) {
            let found = (self.open)(path).and_then(|r| self.pattern.matches_reader(r));
            match found {
                Ok(f) => {
                    self.found += usize::from(f);
                    results.push((*id, f));
                }
                Err(e) => self.errors.push(format!("{}: {e}", path.display())),
            }
            self.next += 1;
            if start.elapsed() >= budget {
                break;
            }
        }
        results
    }

    pub fn summary(&self) -> String {
        let mut s = format!(
            "{} of {} tagged files contain \"{}\"",
            self.found, self.next, self.spec.text
        );
        if let Some(e) = self.errors.first() {
            s.push_str(&format!(
                " ({} unreadable, kept tagged; first: {e})",
                self.errors.len()
            ));
        }
        s
    }
}

/// Opens a file from disk for the search.
pub fn open_file(path: &Path) -> io::Result<Box<dyn BufRead>> {
    Ok(Box::new(BufReader::new(File::open(path)?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn pat(text: &str) -> Pattern {
        Pattern::new(&SearchSpec::new(text)).unwrap()
    }

    #[test]
    fn finds_text_ignoring_case_including_accents() {
        assert_eq!(pat("ação").find_in_line("Uma AÇÃO aqui", 0), Some((4, 8)));
        assert_eq!(pat("x").find_in_line("axbx", 2), Some((3, 4)));
        assert_eq!(pat("x").rfind_in_line("axbx", 3), Some((1, 2)));
        let case = Pattern::new(&SearchSpec {
            case_sensitive: true,
            ..SearchSpec::new("Ação")
        })
        .unwrap();
        assert_eq!(case.find_in_line("AÇÃO", 0), None);
    }

    #[test]
    fn wildcard_only_inside_the_term() {
        assert_eq!(
            pat("erro*disco").find_in_line("um erro no disco!", 0),
            Some((3, 16))
        );
        assert_eq!(pat("erro*disco").find_in_line("disco e erro", 0), None);
        // At the ends, `*` is literal (and so are all the others).
        assert_eq!(pat("*a*b").find_in_line("x *a*b y", 0), Some((2, 6)));
        assert_eq!(pat("*a*b").find_in_line("x a  b y", 0), None);
    }

    #[test]
    fn hex_patterns() {
        let spec = SearchSpec {
            hex: true,
            ..SearchSpec::new("4d 5A")
        };
        let p = Pattern::new(&spec).unwrap();
        assert_eq!(p.find_in_bytes(b"xxMZyy", 0), Some((2, 4)));
        assert!(p.matches_reader(Cursor::new(b"..MZ..".to_vec())).unwrap());
        assert!(
            Pattern::new(&SearchSpec {
                hex: true,
                ..SearchSpec::new("zz")
            })
            .is_err()
        );
        assert!(Pattern::new(&SearchSpec::new("")).is_err());
    }

    #[test]
    fn text_in_bytes_ignores_ascii_case() {
        assert_eq!(pat("abc").find_in_bytes(b"..ABC..abc", 0), Some((2, 5)));
        assert_eq!(pat("abc").rfind_in_bytes(b"..ABC..abc", 9), Some((7, 10)));
    }

    #[test]
    fn readers_do_not_cross_lines_but_find_split_bytes() {
        let p = pat("foo bar");
        assert!(
            p.matches_reader(Cursor::new(b"a\nfoo bar\nz".to_vec()))
                .unwrap()
        );
        assert!(!p.matches_reader(Cursor::new(b"foo\nbar".to_vec())).unwrap());
        // Bytes split between two reader blocks are still found.
        let needle = Pattern::Bytes(b"XYZ".to_vec());
        let reader = BufReader::with_capacity(4, Cursor::new(b"aaXYZbb".to_vec()));
        assert!(needle.matches_reader(reader).unwrap());
    }

    #[test]
    fn job_reports_matches_and_errors() {
        let tmp = crate::testing::tempdir();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        std::fs::write(&a, "tem a palavra chave\n").unwrap();
        std::fs::write(&b, "não tem\n").unwrap();
        let missing = tmp.path().join("missing");
        let items = vec![(1, a), (2, b), (3, missing)];
        let mut job = SearchJob::new(SearchSpec::new("CHAVE"), items, Box::new(open_file)).unwrap();
        let mut results = Vec::new();
        while !job.is_done() {
            results.extend(job.step(Duration::ZERO));
        }
        assert_eq!(results, [(1, true), (2, false)]);
        assert_eq!(job.errors.len(), 1);
        assert!(
            job.summary()
                .starts_with("1 of 3 tagged files contain \"CHAVE\" (1 unreadable")
        );
    }
}
