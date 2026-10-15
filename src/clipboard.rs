//! Clipboard integration via OSC 52 escape sequences and system clipboard tools.
//!
//! OSC 52 (`\x1b]52;c;<base64>\x07`) works across local and remote/SSH sessions
//! on modern terminals (Kitty, WezTerm, Alacritty, Foot, Ghostty, iTerm2,
//! Windows Terminal, tmux) without requiring external packages.
//!
//! Additionally, native clipboard utilities (`wl-copy` on Wayland, `xclip`/`xsel`
//! on X11, `pbcopy` on macOS, `clip` on Windows) are attempted as fallback.

use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Copies text to the system clipboard using OSC 52 and native clipboard tools.
pub fn set_clipboard(text: &str) -> io::Result<()> {
    // 1. Send OSC 52 escape sequence directly to stdout (works across SSH, tmux, etc.)
    let osc52 = format_osc52(text);
    let mut out = io::stdout();
    let _ = out.write_all(osc52.as_bytes());
    let _ = out.flush();

    // 2. Also run native clipboard tool as fallback in background
    let _ = set_native_clipboard(text);

    Ok(())
}

/// Formats the OSC 52 escape sequence for clipboard copy.
/// Wraps with DCS passthrough if running inside tmux.
pub fn format_osc52(text: &str) -> String {
    let b64 = to_base64(text.as_bytes());
    if std::env::var_os("TMUX").is_some() {
        format!("\x1bPtmux;\x1b\x1b]52;c;{b64}\x07\x1b\\")
    } else {
        format!("\x1b]52;c;{b64}\x07")
    }
}

/// Standard base64 encoding without external dependencies.
pub fn to_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Tries native platform clipboard commands.
fn set_native_clipboard(text: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() && run_clipboard_cmd("wl-copy", &[], text)
        {
            return Ok(());
        }
        if std::env::var_os("DISPLAY").is_some() {
            if run_clipboard_cmd("xclip", &["-selection", "clipboard"], text) {
                return Ok(());
            }
            if run_clipboard_cmd("xsel", &["--clipboard", "--input"], text) {
                return Ok(());
            }
        }
        #[cfg(target_os = "macos")]
        {
            if run_clipboard_cmd("pbcopy", &[], text) {
                return Ok(());
            }
        }
    }
    #[cfg(windows)]
    {
        if run_clipboard_cmd("clip", &[], text) {
            return Ok(());
        }
    }
    Ok(())
}

fn run_clipboard_cmd(program: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }
    child.wait().is_ok_and(|s| s.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encoding() {
        assert_eq!(to_base64(b""), "");
        assert_eq!(to_base64(b"f"), "Zg==");
        assert_eq!(to_base64(b"fo"), "Zm8=");
        assert_eq!(to_base64(b"foo"), "Zm9v");
        assert_eq!(to_base64(b"foob"), "Zm9vYg==");
        assert_eq!(to_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(to_base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(
            to_base64("/tmp/arquivo com acento: ação.txt".as_bytes()),
            "L3RtcC9hcnF1aXZvIGNvbSBhY2VudG86IGHDp8Ojby50eHQ="
        );
    }

    #[test]
    fn osc52_format() {
        let osc = format_osc52("hello");
        assert!(osc.contains("aGVsbG8="));
        if std::env::var_os("TMUX").is_none() {
            assert_eq!(osc, "\x1b]52;c;aGVsbG8=\x07");
        }
    }
}
