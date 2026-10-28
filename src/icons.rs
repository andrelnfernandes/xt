//! Nerd Fonts icons for files, directories and window titles.
//!
//! When enabled (`icons = "nerd-fonts"`), each icon takes `SLOT_WIDTH`
//! display columns: the glyph and a space, so columns stay aligned. When
//! disabled (`icons = "none"`), lookups return `None`. Git status keeps its
//! two letters: one icon cannot tell the index from the work tree.
//!
//! Each glyph is named as in Nerd Fonts' `glyphnames.json` (v3), so it can be
//! checked there: the glyph itself does not show without a Nerd Font.

use std::ffi::OsStr;
use std::path::Path;

use crate::config::IconStyle;
use crate::model::tree::FileKind;
use crate::pane::FileView;

/// Display columns of every icon: the glyph and the space after it, which
/// the wider glyphs of a non-Mono Nerd Font spill into.
pub const SLOT_WIDTH: usize = 2;

/// What a directory of the tree is, for its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirKind {
    Closed,
    Open,
    /// Holds a `.git`.
    Repo,
    /// Root of an archive opened as a folder.
    Archive,
    /// Root of a remote machine (SFTP).
    Remote,
}

/// Returns the icon for a directory in the tree.
pub fn dir_icon(kind: DirKind, style: IconStyle) -> Option<&'static str> {
    if style == IconStyle::None {
        return None;
    }
    Some(match kind {
        DirKind::Closed => "\u{f07b} ",  // fa-folder
        DirKind::Open => "\u{f07c} ",    // fa-folder_open
        DirKind::Repo => "\u{e5fb} ",    // custom-folder_git
        DirKind::Archive => "\u{f410} ", // oct-file_zip
        DirKind::Remote => "\u{f048b} ", // md-server
    })
}

/// Returns the icon before the title of the files window.
pub fn view_icon(view: FileView, style: IconStyle) -> Option<&'static str> {
    if style == IconStyle::None {
        return None;
    }
    Some(match view {
        FileView::Dir => "\u{f07c} ",        // fa-folder_open
        FileView::Branch(_) => "\u{f0645} ", // md-file_tree: a subtree
        FileView::Showall => "\u{f0349} ",   // md-magnify
    })
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
        return Some("\u{f0c1} "); // fa-link
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
        Some("\u{f013} ") // fa-gear: executable
    } else {
        Some("\u{f0214} ") // md-file
    }
}

/// Matches well-known exact filenames (case-insensitive).
fn match_exact_name(name: &str) -> Option<&'static str> {
    match name {
        "dockerfile" | "dockerfile.dev" | "docker-compose.yml" | "docker-compose.yaml" => {
            Some("\u{f0868} ") // md-docker
        }
        "makefile" | "cmakelists.txt" | "justfile" => {
            Some("\u{e779} ") // dev-gnu
        }
        "cargo.toml" | "cargo.lock" => {
            Some("\u{e7a8} ") // dev-rust
        }
        "package.json" | "package-lock.json" | "tsconfig.json" => {
            Some("\u{e60c} ") // seti-javascript
        }
        ".gitignore" | ".gitmodules" | ".gitattributes" | ".gitconfig" => {
            Some("\u{e702} ") // dev-git
        }
        "license" | "licence" | "license.md" | "license.txt" => {
            Some("\u{f495} ") // oct-law
        }
        _ => None,
    }
}

/// Matches file extensions (case-insensitive).
fn match_extension(ext: &str) -> Option<&'static str> {
    match ext {
        // Rust
        "rs" => Some("\u{e7a8} "), // dev-rust
        // Python
        "py" | "pyw" | "ipynb" => Some("\u{e73c} "), // dev-python
        // C / C++
        "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "hxx" => Some("\u{e61d} "), // custom-cpp
        // Go
        "go" => Some("\u{e627} "), // seti-go
        // Java & Kotlin
        "java" | "jar" => Some("\u{e738} "), // dev-java
        "kt" | "kts" => Some("\u{e634} "),   // seti-kotlin
        // JavaScript & TypeScript
        "js" | "mjs" | "cjs" | "jsx" => Some("\u{e60c} "), // seti-javascript
        "ts" | "mts" | "tsx" => Some("\u{e628} "),         // seti-typescript
        // Web
        "html" | "htm" => Some("\u{f13b} "), // fa-html5
        "css" | "scss" | "sass" | "less" => Some("\u{e749} "), // dev-css3
        // Shell
        "sh" | "bash" | "zsh" | "fish" | "ksh" => Some("\u{e795} "), // dev-terminal
        // Other languages
        "lua" => Some("\u{e620} "),       // seti-lua
        "rb" => Some("\u{e739} "),        // dev-ruby
        "php" => Some("\u{e608} "),       // seti-php
        "pl" | "pm" => Some("\u{e769} "), // dev-perl
        "zig" => Some("\u{e6a9} "),       // seti-zig
        "swift" => Some("\u{e755} "),     // dev-swift
        // Config and Data
        "toml" | "yaml" | "yml" | "json" | "json5" | "ini" | "conf" | "xml" | "sql" | "csv"
        | "tsv" => Some("\u{e615} "), // seti-config
        // Documents and Markdown
        "md" | "markdown" => Some("\u{f48a} "), // oct-markdown
        "pdf" => Some("\u{f1c1} "),             // fa-file_pdf
        "txt" | "log" => Some("\u{f15c} "),     // fa-file_lines
        // Archives and Compressed
        "zip" | "tar" | "gz" | "xz" | "zst" | "bz2" | "7z" | "rar" | "tgz" | "txz" | "tbz2" => {
            Some("\u{f410} ") // oct-file_zip
        }
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tiff" => {
            Some("\u{f03e} ") // fa-image
        }
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" => {
            Some("\u{f0fce} ") // md-movie_open
        }
        // Audio
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" | "opus" => {
            Some("\u{f0386} ") // md-music_circle
        }
        // Fonts
        "ttf" | "otf" | "woff" | "woff2" => Some("\u{f031} "), // fa-font
        // Packages / Binaries
        "deb" | "rpm" | "apk" | "iso" | "bin" | "exe" | "dll" | "so" | "dylib" => {
            Some("\u{f471} ") // oct-file_binary
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
        assert_eq!(dir_icon(DirKind::Closed, IconStyle::None), None);
        assert_eq!(view_icon(FileView::Dir, IconStyle::None), None);
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
            Some("\u{f013} ")
        );
        assert_eq!(
            file_icon(OsStr::new("LICENSE"), FileKind::File, false, style),
            Some("\u{f495} ")
        );
    }

    #[test]
    fn dir_icons_follow_state() {
        let style = IconStyle::NerdFonts;
        // fa-folder and fa-folder_open.
        assert_eq!(dir_icon(DirKind::Closed, style), Some("\u{f07b} "));
        assert_eq!(dir_icon(DirKind::Open, style), Some("\u{f07c} "));
        assert_eq!(dir_icon(DirKind::Repo, style), Some("\u{e5fb} "));
        assert_eq!(dir_icon(DirKind::Archive, style), Some("\u{f410} "));
        assert_eq!(dir_icon(DirKind::Remote, style), Some("\u{f048b} "));
    }

    #[test]
    fn every_icon_fills_the_slot() {
        // As the screen measures it.
        use ratatui::text::Span;
        let style = IconStyle::NerdFonts;
        let kinds = [
            DirKind::Closed,
            DirKind::Open,
            DirKind::Repo,
            DirKind::Archive,
            DirKind::Remote,
        ];
        let views = [FileView::Dir, FileView::Branch(0), FileView::Showall];
        let names = ["a.rs", "Makefile", "x.mkv", "LICENSE", "noext"];
        let icons = kinds
            .map(|k| dir_icon(k, style))
            .into_iter()
            .chain(views.map(|v| view_icon(v, style)))
            .chain(names.map(|n| file_icon(OsStr::new(n), FileKind::File, false, style)));
        for icon in icons {
            assert_eq!(Span::raw(icon.unwrap()).width(), SLOT_WIDTH, "{icon:?}");
        }
    }
}
