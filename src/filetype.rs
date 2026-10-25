//! What a file is, for the Type display of the file window (Alt+F): a short
//! description in the spirit of the `file` command, from the first bytes
//! (signatures of common formats, `#!` scripts, text and its language).

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::highlight;
use crate::model::tree::FileKind;

/// How much of the file is read: enough for the TAR signature, at 257.
const HEAD: usize = 512;

/// Describes `path`. With `on_disk` false (inside an archive), only the
/// name is used.
pub fn describe(path: &Path, kind: FileKind, size: u64, on_disk: bool) -> String {
    if kind == FileKind::Other {
        return special(path, on_disk);
    }
    if !on_disk {
        return describe_with(path, kind, size, || None, || None);
    }
    let link = || fs::read_link(path).ok().map(|t| t.display().to_string());
    describe_with(path, kind, size, || read_head(path), link)
}

/// The same, with the first bytes and a link's target from elsewhere (a
/// remote machine); `head` and `link` are only called when needed.
pub fn describe_with(
    path: &Path,
    kind: FileKind,
    size: u64,
    head: impl FnOnce() -> Option<Vec<u8>>,
    link: impl FnOnce() -> Option<String>,
) -> String {
    match kind {
        FileKind::Symlink => {
            return match link() {
                Some(target) => format!("symbolic link to {target}"),
                None => "symbolic link".into(),
            };
        }
        FileKind::Other => return "special file".into(),
        FileKind::File => {}
    }
    if size == 0 {
        return "empty".into();
    }
    match head() {
        Some(head) => from_content(path, &head),
        None => {
            highlight::language(path, "").map_or_else(|| "file".into(), |l| format!("{l} file"))
        }
    }
}

/// How much `describe_with`'s `head` should read.
pub const HEAD_BYTES: u64 = HEAD as u64;

fn read_head(path: &Path) -> Option<Vec<u8>> {
    let mut head = Vec::with_capacity(HEAD);
    File::open(path)
        .ok()?
        .take(HEAD as u64)
        .read_to_end(&mut head)
        .ok()?;
    Some(head)
}

/// Pipes, sockets and devices.
fn special(path: &Path, on_disk: bool) -> String {
    #[cfg(unix)]
    if on_disk && let Ok(meta) = fs::symlink_metadata(path) {
        use std::os::unix::fs::FileTypeExt;
        let t = meta.file_type();
        let what = if t.is_fifo() {
            "named pipe (FIFO)"
        } else if t.is_socket() {
            "socket"
        } else if t.is_block_device() {
            "block device"
        } else if t.is_char_device() {
            "character device"
        } else {
            "special file"
        };
        return what.into();
    }
    let _ = (path, on_disk);
    "special file".into()
}

fn from_content(path: &Path, head: &[u8]) -> String {
    if let Some(format) = signature(path, head) {
        return format;
    }
    if let Some(script) = script(head) {
        return script;
    }
    text(path, head).unwrap_or_else(|| "data".into())
}

/// The extension in lowercase ("" if none).
fn ext(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

/// Known formats, by their signature (magic number).
fn signature(path: &Path, h: &[u8]) -> Option<String> {
    let starts = |magic: &[u8]| h.starts_with(magic);
    let at = |offset: usize, magic: &[u8]| h.get(offset..offset + magic.len()) == Some(magic);
    let fixed = |s: &str| Some(s.to_string());
    if starts(b"\x7fELF") {
        return Some(elf(h));
    }
    if starts(b"\x89PNG\r\n\x1a\n") {
        return Some(match (be32(h, 16), be32(h, 20)) {
            (Some(w), Some(hgt)) => format!("PNG image, {w} x {hgt}"),
            _ => "PNG image".into(),
        });
    }
    if starts(b"GIF87a") || starts(b"GIF89a") {
        return Some(match (le16(h, 6), le16(h, 8)) {
            (Some(w), Some(hgt)) => format!("GIF image, {w} x {hgt}"),
            _ => "GIF image".into(),
        });
    }
    if starts(b"%PDF-") {
        let version: String = h[5..]
            .iter()
            .take_while(|b| b.is_ascii_digit() || **b == b'.')
            .map(|&b| b as char)
            .collect();
        return Some(format!("PDF document, version {version}"));
    }
    if starts(b"PK\x03\x04") || starts(b"PK\x05\x06") {
        return Some(match ext(path).as_str() {
            "docx" | "xlsx" | "pptx" => "Microsoft Office document (ZIP)".into(),
            "odt" | "ods" | "odp" | "odg" => "OpenDocument (ZIP)".into(),
            "jar" | "war" => "Java archive (ZIP)".into(),
            "apk" => "Android package (ZIP)".into(),
            "epub" => "EPUB e-book (ZIP)".into(),
            _ => "ZIP archive".into(),
        });
    }
    if starts(b"RIFF") {
        return match h.get(8..12) {
            Some(b"WEBP") => fixed("WebP image"),
            Some(b"WAVE") => fixed("WAVE audio"),
            Some(b"AVI ") => fixed("AVI video"),
            _ => fixed("RIFF data"),
        };
    }
    if at(4, b"ftyp") {
        return match h.get(8..12) {
            Some(b"M4A ") => fixed("MPEG-4 audio"),
            Some(b"heic" | b"heix" | b"mif1") => fixed("HEIF image"),
            Some(b"avif") => fixed("AVIF image"),
            Some(b"qt  ") => fixed("QuickTime video"),
            _ => fixed("MPEG-4 video"),
        };
    }
    if at(257, b"ustar") {
        return fixed("TAR archive");
    }
    if starts(b"!<arch>\n") {
        return fixed(if ext(path) == "deb" {
            "Debian package"
        } else {
            "ar archive"
        });
    }
    let table: &[(&[u8], &str)] = &[
        (b"\xff\xd8\xff", "JPEG image"),
        (b"BM", "BMP image"),
        (b"II*\0", "TIFF image"),
        (b"MM\0*", "TIFF image"),
        (b"8BPS", "Photoshop image"),
        (b"\x1f\x8b", "gzip compressed data"),
        (b"\xfd7zXZ\0", "xz compressed data"),
        (b"BZh", "bzip2 compressed data"),
        (b"\x28\xb5\x2f\xfd", "Zstandard compressed data"),
        (b"7z\xbc\xaf\x27\x1c", "7-Zip archive"),
        (b"Rar!\x1a\x07", "RAR archive"),
        (b"\xed\xab\xee\xdb", "RPM package"),
        (b"SQLite format 3\0", "SQLite 3 database"),
        (b"OggS", "Ogg audio/video"),
        (b"fLaC", "FLAC audio"),
        (b"ID3", "MP3 audio"),
        (b"\x1a\x45\xdf\xa3", "Matroska/WebM video"),
        (b"\0asm", "WebAssembly binary"),
        (b"\xca\xfe\xba\xbe", "Java class file"),
        (b"\xcf\xfa\xed\xfe", "Mach-O executable"),
        (b"\xce\xfa\xed\xfe", "Mach-O executable"),
        (b"MZ", "Windows executable"),
        (
            b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1",
            "Microsoft Office document (OLE)",
        ),
        (b"{\\rtf", "RTF document"),
        (b"OTTO", "OpenType font"),
        (b"wOFF", "WOFF font"),
        (b"wOF2", "WOFF2 font"),
    ];
    if let Some((_, name)) = table.iter().find(|(magic, _)| starts(magic)) {
        return fixed(name);
    }
    // MP3 without a tag: a frame header.
    if h.len() > 1 && h[0] == 0xff && matches!(h[1], 0xfb | 0xf3 | 0xf2) {
        return fixed("MP3 audio");
    }
    None
}

/// "ELF 64-bit executable" and the like.
fn elf(h: &[u8]) -> String {
    let bits = match h.get(4) {
        Some(1) => "32-bit ",
        Some(2) => "64-bit ",
        _ => "",
    };
    let kind = h.get(16..18).map(|t| match h.get(5) {
        Some(2) => u16::from_be_bytes([t[0], t[1]]),
        _ => u16::from_le_bytes([t[0], t[1]]),
    });
    let what = match kind {
        Some(1) => "relocatable object",
        Some(2) => "executable",
        // Shared libraries and position-independent executables alike.
        Some(3) => "executable/library",
        Some(4) => "core dump",
        _ => "file",
    };
    format!("ELF {bits}{what}")
}

/// `#!/usr/bin/env python3` → "python3 script".
fn script(h: &[u8]) -> Option<String> {
    let line = h.strip_prefix(b"#!")?;
    let line = &line[..line.iter().position(|&b| b == b'\n').unwrap_or(line.len())];
    let line = String::from_utf8_lossy(line);
    let mut words = line.split_whitespace();
    let program = words.next()?;
    let mut name = program.rsplit('/').next().unwrap_or(program);
    if name == "env" {
        // `env -S python3 -u`: the first word that is not an option.
        name = words.find(|w| !w.starts_with('-'))?;
    }
    Some(format!("{name} script"))
}

/// "Rust, UTF-8 text", "ASCII text, CRLF"..., or `None` for binary data.
fn text(path: &Path, h: &[u8]) -> Option<String> {
    if h.starts_with(b"\xff\xfe") || h.starts_with(b"\xfe\xff") {
        return Some("UTF-16 text".into());
    }
    let body = h.strip_prefix(b"\xef\xbb\xbf").unwrap_or(h);
    if body.contains(&0) {
        return None;
    }
    // A character cut at the end of the 512 bytes is still text.
    let valid = match std::str::from_utf8(body) {
        Ok(s) => s,
        Err(e) if e.error_len().is_none() => std::str::from_utf8(&body[..e.valid_up_to()]).ok()?,
        Err(_) => return None,
    };
    let encoding = if valid.is_ascii() { "ASCII" } else { "UTF-8" };
    let first_line = valid.lines().next().unwrap_or("");
    let mut out = match highlight::language(path, first_line) {
        Some(language) => format!("{language}, {encoding} text"),
        None => format!("{encoding} text"),
    };
    if valid.contains("\r\n") {
        out.push_str(", CRLF");
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(name: &str, contents: &[u8]) -> String {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join(name);
        fs::write(&path, contents).unwrap();
        describe(&path, FileKind::File, contents.len() as u64, true)
    }

    #[test]
    fn recognizes_formats_by_their_first_bytes() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend(800u32.to_be_bytes());
        png.extend(600u32.to_be_bytes());
        assert_eq!(of("pic", &png), "PNG image, 800 x 600");
        assert_eq!(of("doc.bin", b"%PDF-1.7\n..."), "PDF document, version 1.7");
        assert_eq!(
            of("a.docx", b"PK\x03\x04rest"),
            "Microsoft Office document (ZIP)"
        );
        assert_eq!(of("a.zip", b"PK\x03\x04rest"), "ZIP archive");
        assert_eq!(of("x", b"\xfd7zXZ\0..."), "xz compressed data");
        let mut elf = b"\x7fELF\x02\x01\x01".to_vec();
        elf.resize(16, 0);
        elf.extend([2, 0]);
        assert_eq!(of("prog", &elf), "ELF 64-bit executable");
        let mut tar = vec![0u8; 512];
        tar[257..262].copy_from_slice(b"ustar");
        assert_eq!(of("t", &tar), "TAR archive");
    }

    #[test]
    fn scripts_text_and_data() {
        assert_eq!(of("run", b"#!/bin/sh\necho hi\n"), "sh script");
        assert_eq!(
            of("tool", b"#!/usr/bin/env -S python3 -u\n"),
            "python3 script"
        );
        assert_eq!(of("main.rs", b"fn main() {}\n"), "Rust, ASCII text");
        assert_eq!(of("notes", "ação\r\nok\r\n".as_bytes()), "UTF-8 text, CRLF");
        // A multi-byte character cut by the 512-byte limit.
        let mut long = vec![b'a'; HEAD - 1];
        long.extend("ç".as_bytes());
        assert_eq!(of("long.txt", &long), "ASCII text");
        assert_eq!(of("blob", b"\x01\x02\0\x03"), "data");
        assert_eq!(of("empty", b""), "empty");
    }

    #[test]
    fn links_special_files_and_archive_entries() {
        let tmp = crate::testing::tempdir();
        let inside = Path::new("/x/pkg.zip/src/lib.rs");
        assert_eq!(describe(inside, FileKind::File, 10, false), "Rust file");
        assert_eq!(
            describe(Path::new("/x/a.zip/blob"), FileKind::File, 10, false),
            "file"
        );
        #[cfg(unix)]
        {
            let link = tmp.path().join("link");
            std::os::unix::fs::symlink("target.txt", &link).unwrap();
            assert_eq!(
                describe(&link, FileKind::Symlink, 0, true),
                "symbolic link to target.txt"
            );
            let fifo = tmp.path().join("fifo");
            nix::unistd::mkfifo(&fifo, nix::sys::stat::Mode::S_IRWXU).unwrap();
            assert_eq!(
                describe(&fifo, FileKind::Other, 0, true),
                "named pipe (FIFO)"
            );
        }
        let _ = tmp;
    }
}
