//! Pattern and mask engine for batch renaming tagged files.
//!
//! Supports:
//! - Wildcard masks: `*.jpg`, `new_*.*`, `*_backup.*`, `???_*.*`
//! - Search & replace: `s/from/to/`, `s/from/to/i`, `/from/to/`
//! - Incremental counters: `%03d`, `%02d`, `%d`, `<001>`, `<01>`, `<#>`
//! - Case conversion: `<lower>`, `<upper>`, `<title>`, `*.<lower>`, or CaseMode (`F2`)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseMode {
    #[default]
    None,
    Lower,
    Upper,
    Title,
}

impl CaseMode {
    pub fn next(self) -> Self {
        match self {
            CaseMode::None => CaseMode::Lower,
            CaseMode::Lower => CaseMode::Upper,
            CaseMode::Upper => CaseMode::Title,
            CaseMode::Title => CaseMode::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CaseMode::None => "",
            CaseMode::Lower => "lowercase",
            CaseMode::Upper => "UPPERCASE",
            CaseMode::Title => "Title Case",
        }
    }

    pub fn apply(self, s: &str) -> String {
        match self {
            CaseMode::None => s.to_string(),
            CaseMode::Lower => s.to_lowercase(),
            CaseMode::Upper => s.to_uppercase(),
            CaseMode::Title => title_case(s),
        }
    }
}

pub fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut new_word = true;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if new_word {
                out.extend(c.to_uppercase());
                new_word = false;
            } else {
                out.extend(c.to_lowercase());
            }
        } else {
            new_word = true;
            out.push(c);
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct RenameMask {
    pattern: String,
    case_mode: CaseMode,
}

impl RenameMask {
    pub fn new(pattern: &str, case_mode: CaseMode) -> Self {
        Self {
            pattern: pattern.trim().to_string(),
            case_mode,
        }
    }

    pub fn apply(&self, original_name: &str, index: usize) -> String {
        let pat = self.pattern.as_str();

        // 1. Check for search & replace: s/from/to/[i] or /from/to/[i]
        if let Some(replaced) = try_search_replace(pat, original_name) {
            return self.case_mode.apply(&replaced);
        }

        // 2. Global case conversion tokens
        let lower_pat = pat.to_lowercase();
        if lower_pat == "<lower>" || lower_pat == "<l>" {
            return original_name.to_lowercase();
        }
        if lower_pat == "<upper>" || lower_pat == "<u>" {
            return original_name.to_uppercase();
        }
        if lower_pat == "<title>" || lower_pat == "<cap>" || lower_pat == "<c>" {
            return title_case(original_name);
        }

        // 3. Counter substitution in the pattern (%03d, %02d, %d, <001>, <01>, <#>)
        let pat_with_counter = substitute_counter(pat, index);

        // 4. Split filename into stem and extension
        let (stem, ext) = split_stem_ext(original_name);

        // 5. Check if pattern specifies an extension (contains dot)
        let result = if let Some(dot_idx) = pat_with_counter.rfind('.') {
            let stem_pat = &pat_with_counter[..dot_idx];
            let ext_pat = &pat_with_counter[dot_idx + 1..];
            let new_stem = apply_sub_pattern(stem, stem_pat);
            let new_ext = apply_sub_pattern(ext, ext_pat);
            if new_ext.is_empty() {
                new_stem
            } else {
                format!("{new_stem}.{new_ext}")
            }
        } else {
            apply_sub_pattern(original_name, &pat_with_counter)
        };

        // 6. Apply case conversion if specified
        self.case_mode.apply(&result)
    }
}

fn split_stem_ext(name: &str) -> (&str, &str) {
    if name.starts_with('.') && name[1..].find('.').is_none() {
        return (name, "");
    }
    match name.rfind('.') {
        Some(idx) => (&name[..idx], &name[idx + 1..]),
        None => (name, ""),
    }
}

fn try_search_replace(pattern: &str, source: &str) -> Option<String> {
    let p = pattern.strip_prefix("s/").unwrap_or_else(|| pattern.strip_prefix('/').unwrap_or(""));
    if p.is_empty() || (!pattern.starts_with("s/") && !pattern.starts_with('/')) {
        return None;
    }
    let parts: Vec<&str> = p.split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    let from = parts[0];
    let to = parts[1];
    let flags = parts.get(2).copied().unwrap_or("");
    let case_insensitive = flags.contains('i');

    if from.is_empty() {
        return Some(source.to_string());
    }

    if case_insensitive {
        Some(replace_case_insensitive(source, from, to))
    } else {
        Some(source.replace(from, to))
    }
}

fn replace_case_insensitive(source: &str, from: &str, to: &str) -> String {
    let mut result = String::new();
    let mut remaining = source;
    let from_len = from.len();
    while let Some(idx) = remaining
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| remaining[i..].to_lowercase().starts_with(&from.to_lowercase()))
    {
        result.push_str(&remaining[..idx]);
        result.push_str(to);
        remaining = &remaining[idx + from_len..];
    }
    result.push_str(remaining);
    result
}

fn substitute_counter(pattern: &str, index: usize) -> String {
    let mut out = pattern.to_string();

    // %04d, %03d, %02d, %d
    if out.contains("%04d") {
        out = out.replace("%04d", &format!("{index:04}"));
    }
    if out.contains("%03d") {
        out = out.replace("%03d", &format!("{index:03}"));
    }
    if out.contains("%02d") {
        out = out.replace("%02d", &format!("{index:02}"));
    }
    if out.contains("%d") {
        out = out.replace("%d", &format!("{index}"));
    }

    // Angle bracket counters: <0001>, <001>, <01>, <#>, <1>
    if out.contains("<0001>") {
        out = out.replace("<0001>", &format!("{index:04}"));
    }
    if out.contains("<001>") {
        out = out.replace("<001>", &format!("{index:03}"));
    }
    if out.contains("<01>") {
        out = out.replace("<01>", &format!("{index:02}"));
    }
    if out.contains("<#>") {
        out = out.replace("<#>", &format!("{index}"));
    }
    if out.contains("<1>") {
        out = out.replace("<1>", &format!("{index}"));
    }

    out
}

fn apply_sub_pattern(source: &str, pat: &str) -> String {
    let lower_pat = pat.to_lowercase();
    if lower_pat == "<lower>" || lower_pat == "<l>" {
        return source.to_lowercase();
    }
    if lower_pat == "<upper>" || lower_pat == "<u>" {
        return source.to_uppercase();
    }
    if lower_pat == "<title>" || lower_pat == "<cap>" || lower_pat == "<c>" {
        return title_case(source);
    }

    if pat == "*" {
        return source.to_string();
    }

    if pat.contains('*') {
        let parts: Vec<&str> = pat.split('*').collect();
        let mut out = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                out.push_str(source);
            }
            if !part.is_empty() {
                out.push_str(&map_question_marks(part, source, 0));
            }
        }
        out
    } else if pat.contains('?') {
        map_question_marks(pat, source, 0)
    } else {
        pat.to_string()
    }
}

fn map_question_marks(pattern: &str, source: &str, start_idx: usize) -> String {
    let src_chars: Vec<char> = source.chars().collect();
    let mut out = String::new();
    let mut idx = start_idx;
    for c in pattern.chars() {
        if c == '?' {
            if idx < src_chars.len() {
                out.push(src_chars[idx]);
                idx += 1;
            }
        } else {
            out.push(c);
            idx += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_extension_change() {
        let mask = RenameMask::new("*.jpg", CaseMode::None);
        assert_eq!(mask.apply("photo.jpeg", 1), "photo.jpg");
        assert_eq!(mask.apply("document.txt", 1), "document.jpg");
        assert_eq!(mask.apply("archive.tar.gz", 1), "archive.tar.jpg");
    }

    #[test]
    fn test_wildcard_prefix_and_suffix() {
        let m1 = RenameMask::new("new_*.*", CaseMode::None);
        assert_eq!(m1.apply("file.txt", 1), "new_file.txt");

        let m2 = RenameMask::new("*_backup.*", CaseMode::None);
        assert_eq!(m2.apply("file.txt", 1), "file_backup.txt");

        let m3 = RenameMask::new("prefix_*_suffix.*", CaseMode::None);
        assert_eq!(m3.apply("file.txt", 1), "prefix_file_suffix.txt");
    }

    #[test]
    fn test_strip_extension() {
        let mask = RenameMask::new("*.", CaseMode::None);
        assert_eq!(mask.apply("photo.jpeg", 1), "photo");
    }

    #[test]
    fn test_incremental_counters() {
        let m1 = RenameMask::new("photo_%03d.*", CaseMode::None);
        assert_eq!(m1.apply("a.jpg", 1), "photo_001.jpg");
        assert_eq!(m1.apply("b.jpg", 42), "photo_042.jpg");

        let m2 = RenameMask::new("file_%02d.txt", CaseMode::None);
        assert_eq!(m2.apply("old.dat", 5), "file_05.txt");

        let m3 = RenameMask::new("item_<01>.*", CaseMode::None);
        assert_eq!(m3.apply("x.png", 3), "item_03.png");

        let m4 = RenameMask::new("item_<#>.*", CaseMode::None);
        assert_eq!(m4.apply("x.png", 3), "item_3.png");
    }

    #[test]
    fn test_search_and_replace() {
        let m1 = RenameMask::new("s/jpeg/jpg/", CaseMode::None);
        assert_eq!(m1.apply("my_photo.jpeg", 1), "my_photo.jpg");

        let m2 = RenameMask::new("s/ /_/g", CaseMode::None);
        assert_eq!(m2.apply("my holiday trip.png", 1), "my_holiday_trip.png");

        let m3 = RenameMask::new("s/JPEG/jpg/i", CaseMode::None);
        assert_eq!(m3.apply("MY_PHOTO.JPEG", 1), "MY_PHOTO.jpg");

        let m4 = RenameMask::new("/IMG_/pic_/", CaseMode::None);
        assert_eq!(m4.apply("IMG_001.jpg", 1), "pic_001.jpg");
    }

    #[test]
    fn test_case_conversions() {
        // Whole file lowercase via mask token
        let m1 = RenameMask::new("<lower>", CaseMode::None);
        assert_eq!(m1.apply("DSC_001.JPG", 1), "dsc_001.jpg");

        // Whole file uppercase via mask token
        let m2 = RenameMask::new("<upper>", CaseMode::None);
        assert_eq!(m2.apply("file_name.txt", 1), "FILE_NAME.TXT");

        // Lowercase extension only via mask
        let m3 = RenameMask::new("*.<lower>", CaseMode::None);
        assert_eq!(m3.apply("DSC_001.JPG", 1), "DSC_001.jpg");

        // Title case via mask token
        let m4 = RenameMask::new("<title>", CaseMode::None);
        assert_eq!(m4.apply("hello_world.txt", 1), "Hello_World.Txt");

        // CaseMode toggle (e.g. F2 in prompt)
        let m5 = RenameMask::new("new_*.*", CaseMode::Lower);
        assert_eq!(m5.apply("FILE_NAME.TXT", 1), "new_file_name.txt");
    }

    #[test]
    fn test_question_mark_stencil() {
        let mask = RenameMask::new("???_v2.*", CaseMode::None);
        assert_eq!(mask.apply("document.txt", 1), "doc_v2.txt");
    }
}
