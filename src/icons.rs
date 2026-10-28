//! Nerd Fonts icon lookups for files, directories and Git markers.
//!
//! When enabled (`icons = "nerd-fonts"`), each icon occupies exactly 2 display
//! columns (1 Nerd Font glyph + 1 trailing space) to ensure columns remain
//! aligned in the TUI. When disabled (`icons = "none"`), lookups return `None`.

use std::ffi::OsStr;
use std::path::Path;

use crate::config::IconStyle;
use crate::model::tree::FileKind;

/// Returns the icon for a directory in the tree or file list.
pub fn dir_icon(
    open: bool,
    is_repo: bool,
    is_archive: bool,
    is_remote: bool,
    style: IconStyle,
) -> Option<&'static str> {
    if style == IconStyle::None {
        return None;
    }
    if is_archive {
        Some("\u{f410} ") //  package
    } else if is_remote {
        Some("\u{f048b} ") // 󰒋 cloud/remote folder
    } else if is_repo {
        Some("\u{e5fb} ") //  folder-git
    } else if open {
        Some("\u{f07b} ") //  folder-open
    } else {
        Some("\u{f07c} ") //  folder
    }
}

/// Returns the icon for a file based on its name, kind and executable bit.
pub fn file_icon(
    name: &OsStr,
    kind: FileKind,
    is_exec: bool,
    style: IconStyle,
) -> Option<&'static str> {
    if style == IconStyle::None {
        return None;
    }
    if kind == FileKind::Symlink {
        return Some("\u{f0c1} "); //  link
    }

    let name_str = name.to_string_lossy();
    let lower_name = name_str.to_ascii_lowercase();

    // Exact filename match
    if let Some(icon) = match_exact_name(&lower_name) {
        return Some(icon);
    }

    // Extension match
    if let Some(ext) = Path::new(&*name_str).extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        if let Some(icon) = match_extension(&ext_lower) {
            return Some(icon);
        }
    }

    if is_exec {
        Some("\u{f0633} ") // 󰘳 binary / executable
    } else {
        Some("\u{f0214} ") // 󰈔 generic file
    }
}

/// Returns the icon for a Git status marker (e.g. `b"M "`, `b"? "`).
/// Each icon is 3 columns (1 glyph + 2 trailing spaces) matching `git_text`.
pub fn git_icon(status: [u8; 2], style: IconStyle) -> Option<&'static str> {
    if style == IconStyle::None {
        return None;
    }
    if status == *b"? " || status == *b"??" {
        Some("\u{f457}  ") //  untracked / new
    } else if status == *b"M " || status == *b" M" || status == *b"MM" {
        Some("\u{f459}  ") //  modified
    } else if status == *b"A " || status == *b" A" {
        Some("\u{f457}  ") //  added
    } else if status == *b"D " || status == *b" D" {
        Some("\u{f458}  ") //  deleted
    } else if status[0] == b'U' || status[1] == b'U' {
        Some("\u{f4db}  ") //  conflict
    } else if status == *b"R " || status == *b" R" {
        Some("\u{f45a}  ") //  renamed
    } else {
        None
    }
}

/// Matches well-known exact filenames (case-insensitive).
fn match_exact_name(name: &str) -> Option<&'static str> {
    match name {
        "dockerfile" | "dockerfile.dev" | "docker-compose.yml" | "docker-compose.yaml" => {
            Some("\u{f0868} ") // 󰡨 docker
        }
        "makefile" | "cmakelists.txt" | "justfile" => {
            Some("\u{e779} ") //  build
        }
        "cargo.toml" | "cargo.lock" => {
            Some("\u{e7a8} ") //  rust
        }
        "package.json" | "package-lock.json" | "tsconfig.json" => {
            Some("\u{e60c} ") //  js / node
        }
        ".gitignore" | ".gitmodules" | ".gitattributes" | ".gitconfig" => {
            Some("\u{e702} ") //  git
        }
        "license" | "licence" | "license.md" | "license.txt" => {
            Some("\u{f48e} ") //  license
        }
        _ => None,
    }
}

/// Matches file extensions (case-insensitive).
fn match_extension(ext: &str) -> Option<&'static str> {
    match ext {
        // Rust
        "rs" => Some("\u{e7a8} "), // 
        // Python
        "py" | "pyw" | "ipynb" => Some("\u{e73c} "), // 
        // C / C++
        "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "hxx" => Some("\u{e61d} "), // 
        // Go
        "go" => Some("\u{e627} "), // 
        // Java & Kotlin
        "java" | "jar" => Some("\u{e738} "), // 
        "kt" | "kts" => Some("\u{e634} "),   // 
        // JavaScript & TypeScript
        "js" | "mjs" | "cjs" | "jsx" => Some("\u{e60c} "), // 
        "ts" | "mts" | "tsx" => Some("\u{e628} "),         // 
        // Web
        "html" | "htm" => Some("\u{f13b} "), // 
        "css" | "scss" | "sass" | "less" => Some("\u{e749} "), // 
        // Shell
        "sh" | "bash" | "zsh" | "fish" | "ksh" => Some("\u{e795} "), // 
        // Other languages
        "lua" => Some("\u{e620} "),       // 
        "rb" => Some("\u{e739} "),        // 
        "php" => Some("\u{e608} "),       // 
        "pl" | "pm" => Some("\u{e769} "), // 
        "zig" => Some("\u{e6a9} "),       // 
        "swift" => Some("\u{e755} "),     // 
        // Config and Data
        "toml" | "yaml" | "yml" | "json" | "json5" | "ini" | "conf" | "xml" | "sql" | "csv"
        | "tsv" => Some("\u{e615} "), // 
        // Documents and Markdown
        "md" | "markdown" => Some("\u{f48a} "), // 
        "pdf" => Some("\u{f1c1} "),             // 
        "txt" | "log" => Some("\u{f15c} "),     // 
        // Archives and Compressed
        "zip" | "tar" | "gz" | "xz" | "zst" | "bz2" | "7z" | "rar" | "tgz" | "txz" | "tbz2" => {
            Some("\u{f410} ") // 
        }
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tiff" => {
            Some("\u{f03e} ") // 
        }
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" => {
            Some("\u{f0fce} ") // 󰿎
        }
        // Audio
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" | "opus" => {
            Some("\u{f0386} ") // 󰎆
        }
        // Fonts
        "ttf" | "otf" | "woff" | "woff2" => Some("\u{f031} "), // 
        // Packages / Binaries
        "deb" | "rpm" | "apk" | "iso" | "bin" | "exe" | "dll" | "so" | "dylib" => {
            Some("\u{f471} ") // 
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_style_returns_none() {
        assert_eq!(
            file_icon(
                OsStr::new("main.rs"),
                FileKind::File,
                false,
                IconStyle::None
            ),
            None
        );
        assert_eq!(dir_icon(false, false, false, false, IconStyle::None), None);
        assert_eq!(git_icon(*b"M ", IconStyle::None), None);
    }

    #[test]
    fn nerd_fonts_returns_appropriate_icons() {
        let style = IconStyle::NerdFonts;
        assert_eq!(
            file_icon(OsStr::new("main.rs"), FileKind::File, false, style),
            Some("\u{e7a8} ")
        );
        assert_eq!(
            file_icon(OsStr::new("app.py"), FileKind::File, false, style),
            Some("\u{e73c} ")
        );
        assert_eq!(
            file_icon(OsStr::new("README.md"), FileKind::File, false, style),
            Some("\u{f48a} ")
        );
        assert_eq!(
            file_icon(OsStr::new("archive.tar.gz"), FileKind::File, false, style),
            Some("\u{f410} ")
        );
        assert_eq!(
            file_icon(OsStr::new("some_link"), FileKind::Symlink, false, style),
            Some("\u{f0c1} ")
        );
        assert_eq!(
            file_icon(OsStr::new("run_tool"), FileKind::File, true, style),
            Some("\u{f0633} ")
        );
    }

    #[test]
    fn dir_icons_follow_state() {
        let style = IconStyle::NerdFonts;
        assert_eq!(
            dir_icon(false, false, false, false, style),
            Some("\u{f07c} ")
        );
        assert_eq!(
            dir_icon(true, false, false, false, style),
            Some("\u{f07b} ")
        );
        assert_eq!(
            dir_icon(false, true, false, false, style),
            Some("\u{e5fb} ")
        );
        assert_eq!(
            dir_icon(false, false, true, false, style),
            Some("\u{f410} ")
        );
        assert_eq!(
            dir_icon(false, false, false, true, style),
            Some("\u{f048b} ")
        );
    }

    #[test]
    fn git_markers_have_icons() {
        let style = IconStyle::NerdFonts;
        assert_eq!(git_icon(*b"M ", style), Some("\u{f459}  "));
        assert_eq!(git_icon(*b"? ", style), Some("\u{f457}  "));
        assert_eq!(git_icon(*b"D ", style), Some("\u{f458}  "));
    }
}
