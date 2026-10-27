//! In-memory decompression of gzip, xz, zstd and bzip2 for the viewer.
//! Files are hostile by default, so output, time and panics are all capped.

// Nothing outside the tests calls this module until the viewer uses it
// (Task 8); the allow goes away then.
#![cfg_attr(not(test), allow(dead_code))]

use std::io::Read;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    Gzip,
    Xz,
    Zstd,
    Bzip2,
}

impl Compression {
    pub fn name(self) -> &'static str {
        match self {
            Compression::Gzip => "gzip",
            Compression::Xz => "xz",
            Compression::Zstd => "zstd",
            Compression::Bzip2 => "bzip2",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Compression::Gzip => "gz",
            Compression::Xz => "xz",
            Compression::Zstd => "zst",
            Compression::Bzip2 => "bz2",
        }
    }
}

/// Identifies the format by its magic bytes.
pub fn sniff(head: &[u8]) -> Option<Compression> {
    if head.starts_with(&[0x1F, 0x8B]) {
        Some(Compression::Gzip)
    } else if head.starts_with(b"\xFD7zXZ\0") {
        Some(Compression::Xz)
    } else if head.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        Some(Compression::Zstd)
    } else if head.starts_with(b"BZh") {
        Some(Compression::Bzip2)
    } else {
        None
    }
}

pub const CHUNK: usize = 64 << 10;
pub const VIEW_DICT: u32 = 64 << 20;
pub const AUTOVIEW_DICT: u32 = 16 << 20;
pub const VIEW_TIME: Duration = Duration::from_secs(3);
pub const AUTOVIEW_TIME: Duration = Duration::from_millis(100);
pub const TRUNCATED: &str = "compressed data truncated or corrupt";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub output: usize,
    pub dict: u32,
    pub time: Duration,
}

pub struct Decompressed {
    pub data: Vec<u8>,
    pub cut: bool,
    pub note: Option<String>,
}

fn duration_text(t: Duration) -> String {
    if t < Duration::from_secs(1) {
        format!("{} ms", t.as_millis())
    } else {
        format!("{} s", t.as_secs())
    }
}

fn truncated() -> Decompressed {
    Decompressed {
        data: Vec::new(),
        cut: false,
        note: Some(TRUNCATED.to_string()),
    }
}

/// Decompresses one level of `input`, stopping at the output and time limits.
pub fn decompress(input: &[u8], kind: Compression, limits: Limits) -> Decompressed {
    guarded(|| match kind {
        Compression::Gzip => read_all(flate2::read::MultiGzDecoder::new(input), limits),
        Compression::Bzip2 => read_all(bzip2::read::MultiBzDecoder::new(input), limits),
        // Task 6 adds xz and zstd.
        Compression::Xz | Compression::Zstd => truncated(),
    })
}

/// A panic inside `f` becomes empty data plus the `TRUNCATED` note.
fn guarded(f: impl FnOnce() -> Decompressed) -> Decompressed {
    crate::panic_guard::catch(f).unwrap_or_else(truncated)
}

fn read_all(mut decoder: impl Read, limits: Limits) -> Decompressed {
    let start = Instant::now();
    let mut data: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; CHUNK];
    let mut cut = false;
    let mut note = None;
    loop {
        let room = limits.output - data.len();
        if room == 0 {
            // Cut only if there is more to read; an error here (bad CRC at
            // the end of the stream) still means corrupt data.
            match decoder.read(&mut buf[..1]) {
                Ok(0) => {}
                Ok(_) => cut = true,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => note = Some(TRUNCATED.to_string()),
            }
            break;
        }
        let want = room.min(CHUNK);
        match decoder.read(&mut buf[..want]) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                note = Some(TRUNCATED.to_string());
                break;
            }
        }
        if start.elapsed() >= limits.time {
            cut = true;
            note = Some(format!("decompression stopped after {}", duration_text(limits.time)));
            break;
        }
    }
    data.shrink_to_fit();
    Decompressed { data, cut, note }
}

#[cfg(test)]
pub(crate) mod test_util {
    use std::io::Write;

    pub fn gz(data: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    pub fn bz2(data: &[u8]) -> Vec<u8> {
        let mut enc = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::{bz2, gz};
    use super::*;
    use proptest::prelude::*;

    const TEXT: &[u8] = b"linha de log\n";

    fn limits(output: usize) -> Limits {
        Limits { output, dict: VIEW_DICT, time: VIEW_TIME }
    }
    const MIB: usize = 1 << 20;

    #[test]
    fn sniffs_by_signature() {
        assert_eq!(sniff(&gz(b"x")), Some(Compression::Gzip));
        assert_eq!(sniff(b"\xFD7zXZ\0..."), Some(Compression::Xz));
        assert_eq!(sniff(b"\x28\xB5\x2F\xFD..."), Some(Compression::Zstd));
        assert_eq!(sniff(b"BZh91AY"), Some(Compression::Bzip2));
        assert_eq!(sniff(b"plain"), None);
        assert_eq!(sniff(b"\x1F"), None);
    }

    #[test]
    fn autoview_limits_are_tighter_than_the_viewer_ones() {
        assert!(AUTOVIEW_TIME < VIEW_TIME && AUTOVIEW_DICT < VIEW_DICT);
    }

    #[test]
    fn names_and_extensions() {
        assert_eq!(Compression::Zstd.name(), "zstd");
        assert_eq!(Compression::Zstd.extension(), "zst");
        assert_eq!(Compression::Bzip2.extension(), "bz2");
    }

    #[test]
    fn gzip_and_bzip2_round_trip() {
        for (kind, packed) in [(Compression::Gzip, gz(TEXT)), (Compression::Bzip2, bz2(TEXT))] {
            let d = decompress(&packed, kind, limits(MIB));
            assert_eq!((d.data.as_slice(), d.cut, d.note), (TEXT, false, None));
        }
    }

    #[test]
    fn gzip_members_are_joined() {
        let mut two = gz(b"one\n");
        two.extend(gz(b"two\n"));
        assert_eq!(decompress(&two, Compression::Gzip, limits(MIB)).data, b"one\ntwo\n");
    }

    #[test]
    fn bomb_stops_at_the_output_limit() {
        let d = decompress(&gz(&vec![0; 16 * MIB]), Compression::Gzip, limits(MIB));
        assert_eq!(d.data.len(), MIB);
        assert!(d.cut);
    }

    #[test]
    fn exact_fit_is_not_cut() {
        let d = decompress(&gz(&vec![7; MIB]), Compression::Gzip, limits(MIB));
        assert_eq!(d.data.len(), MIB);
        assert!(!d.cut);
    }

    #[test]
    fn stops_at_the_time_limit() {
        let big = gz(&TEXT.repeat(50_000));
        let d = decompress(
            &big,
            Compression::Gzip,
            Limits { time: Duration::ZERO, ..limits(64 * MIB) },
        );
        assert!(d.data.len() <= CHUNK && d.cut);
        assert_eq!(d.note.as_deref(), Some("decompression stopped after 0 ms"));
    }

    #[test]
    fn truncated_keeps_what_was_decoded() {
        let all = TEXT.repeat(10_000);
        let packed = gz(&all);
        let d = decompress(&packed[..packed.len() / 2], Compression::Gzip, limits(64 * MIB));
        assert!(!d.data.is_empty() && all.starts_with(&d.data));
        assert_eq!(d.note.as_deref(), Some(TRUNCATED));
    }

    #[test]
    fn only_the_signature_is_truncated_not_a_panic() {
        let d = decompress(b"\x1F\x8B", Compression::Gzip, limits(MIB));
        assert!(d.data.is_empty());
        assert_eq!(d.note.as_deref(), Some(TRUNCATED));
    }

    #[test]
    fn one_level_only() {
        let inner = gz(b"hello");
        assert_eq!(decompress(&gz(&inner), Compression::Gzip, limits(MIB)).data, inner);
    }

    #[test]
    fn corrupt_crc_at_the_exact_limit_is_flagged() {
        let mut packed = gz(&TEXT.repeat(100));
        let n = packed.len();
        packed[n - 8] ^= 0xFF; // CRC32 trailer
        let size = TEXT.len() * 100;
        let d = decompress(&packed, Compression::Gzip, limits(size));
        assert_eq!(d.note.as_deref(), Some(TRUNCATED));
    }

    #[test]
    fn a_panicking_reader_becomes_truncated() {
        struct Boom;
        impl Read for Boom {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                panic!("boom")
            }
        }
        let d = guarded(|| read_all(Boom, limits(MIB)));
        assert!(d.data.is_empty() && !d.cut);
        assert_eq!(d.note.as_deref(), Some(TRUNCATED));
        assert!(!crate::panic_guard::active());
    }

    #[test]
    fn guard_catches_a_panic() {
        assert_eq!(crate::panic_guard::catch(|| -> u8 { panic!("boom") }), None);
        assert!(!crate::panic_guard::active());
    }

    #[test]
    fn duration_text_units() {
        assert_eq!(duration_text(Duration::from_millis(250)), "250 ms");
        assert_eq!(duration_text(Duration::from_millis(3200)), "3 s");
    }

    proptest! {
        #[test]
        fn never_panics(bytes in prop::collection::vec(any::<u8>(), 0..4096), k in 0usize..4) {
            let kind = [Compression::Gzip, Compression::Xz, Compression::Zstd, Compression::Bzip2][k];
            let _ = sniff(&bytes);
            let _ = decompress(&bytes, kind, limits(MIB));
        }
    }
}
