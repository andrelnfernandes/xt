//! In-memory decompression of gzip, xz, zstd and bzip2 for the viewer.
//! Files are hostile by default, so output, time and panics are all capped.

use std::cell::Cell;
use std::io::{self, BufReader, Read};
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
pub(crate) const VIEW_DICT: u32 = 64 << 20;
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
    /// The memory a refused stream asked for, when the format says (the
    /// zstd window; xz does not say).
    pub needed: Option<u64>,
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
        needed: None,
    }
}

/// xz reader that refuses blocks whose dictionary needs more than `dict` bytes.
pub(crate) fn xz_reader<R: Read>(r: R, dict: u32) -> lzma_rust2::XzReader<R> {
    lzma_rust2::XzReader::new_mem_limit(r, true, lzma_rust2::lzma2_get_memory_usage(dict))
}

/// zstd reader for ONE frame; refuses windows larger than `window` bytes.
pub(crate) fn zstd_reader<R: Read>(
    r: R,
    window: u32,
) -> io::Result<ruzstd::decoding::StreamingDecoder<R, ruzstd::decoding::FrameDecoder>> {
    ruzstd::decoding::StreamingDecoder::new_with_max_window_size(r, window as u64)
        .map_err(frame_error)
}

fn frame_error(e: ruzstd::decoding::errors::FrameDecoderError) -> io::Error {
    use ruzstd::decoding::errors::FrameDecoderError as E;
    match e {
        E::WindowSizeTooBig { .. } => io::Error::new(io::ErrorKind::OutOfMemory, e),
        e => io::Error::new(io::ErrorKind::InvalidData, e),
    }
}

/// The window a refused zstd frame asked for.
fn window_asked(e: &io::Error) -> Option<u64> {
    use ruzstd::decoding::errors::FrameDecoderError as E;
    match e.get_ref()?.downcast_ref::<E>()? {
        E::WindowSizeTooBig { requested, .. } => Some(*requested),
        _ => None,
    }
}

/// How much compressed input a decoder gets per read. The clock is checked
/// before each piece, and some inputs cost a lot per byte: an empty bzip2
/// stream is 14 bytes, yet it allocates and clears about 3.6 MB.
const PIECE: usize = 1024;

/// The time limit, shared by `Deadline`, `ZstdFrames` and `read_all`.
struct Clock {
    start: Instant,
    limit: Duration,
    /// A read was refused because the time was up.
    fired: Cell<bool>,
}

impl Clock {
    fn new(limit: Duration) -> Self {
        Clock {
            start: Instant::now(),
            limit,
            fired: Cell::new(false),
        }
    }

    fn expired(&self) -> bool {
        self.start.elapsed() >= self.limit
    }

    /// `TimedOut` once the time is up.
    fn check(&self) -> io::Result<()> {
        if self.expired() {
            self.fired.set(true);
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "decompression time limit",
            ));
        }
        Ok(())
    }
}

/// The compressed input, handed out a piece at a time, with the clock
/// checked before each piece. Decoders go through empty members, streams,
/// blocks or frames inside a single `read` without any output, so the time
/// limit has to be on the input side: a check between output reads alone
/// never runs while they do.
struct Deadline<'a> {
    input: &'a [u8],
    clock: &'a Clock,
}

impl Read for Deadline<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.clock.check()?;
        let n = buf.len().min(PIECE).min(self.input.len());
        let (piece, rest) = self.input.split_at(n);
        buf[..n].copy_from_slice(piece);
        self.input = rest;
        Ok(n)
    }
}

/// Reads every zstd frame of a slice as one stream, skipping skippable frames.
struct ZstdFrames<'a> {
    rest: &'a [u8],
    window: u32,
    clock: &'a Clock,
    current:
        Option<ruzstd::decoding::StreamingDecoder<Deadline<'a>, ruzstd::decoding::FrameDecoder>>,
}

impl<'a> ZstdFrames<'a> {
    fn new(input: &'a [u8], window: u32, clock: &'a Clock) -> Self {
        ZstdFrames {
            rest: input,
            window,
            clock,
            current: None,
        }
    }
}

impl Read for ZstdFrames<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use ruzstd::decoding::errors::{FrameDecoderError as E, ReadFrameHeaderError as H};
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            // Empty frames give no output: the time is checked per frame too.
            self.clock.check()?;
            if let Some(frame) = &mut self.current {
                match frame.read(buf)? {
                    0 => self.rest = self.current.take().map_or(&[], |f| f.into_inner().input),
                    n => return Ok(n),
                }
            } else if self.rest.is_empty() {
                return Ok(0);
            } else {
                let input = Deadline {
                    input: self.rest,
                    clock: self.clock,
                };
                match zstd_reader(input, self.window) {
                    Ok(frame) => self.current = Some(frame),
                    Err(e) => {
                        // A skippable frame is 4 bytes of magic, 4 of length, then the payload.
                        let skip = e.get_ref().and_then(|s| match s.downcast_ref::<E>() {
                            Some(E::ReadFrameHeaderError(H::SkipFrame { length, .. })) => {
                                Some(*length as usize)
                            }
                            _ => None,
                        });
                        match skip {
                            Some(len) => {
                                self.rest =
                                    self.rest.get(8usize.saturating_add(len)..).unwrap_or(&[])
                            }
                            None => return Err(e),
                        }
                    }
                }
            }
        }
    }
}

/// True for errors that mean the stream asks for more memory than allowed.
fn is_memory_error(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::OutOfMemory || e.to_string().contains("mem_limit")
}

fn error_note(e: &io::Error, limits: Limits) -> String {
    if is_memory_error(e) {
        memory_note(limits.dict)
    } else {
        TRUNCATED.to_string()
    }
}

pub fn memory_note(limit: u32) -> String {
    format!(
        "needs more than {} MiB of memory to decompress",
        limit >> 20
    )
}

/// A decoder's memory error (the xz one only says "needed memory too big
/// for mem_limit_kb") as `memory_note(limit)`; other errors pass as they are.
pub(crate) fn clear_memory_error(e: io::Error, limit: u32) -> io::Error {
    if is_memory_error(&e) {
        io::Error::new(io::ErrorKind::OutOfMemory, memory_note(limit))
    } else {
        e
    }
}

fn time_note(limits: Limits) -> String {
    format!("decompression stopped after {}", duration_text(limits.time))
}

/// Decompresses one level of `input`, stopping at the output and time limits.
pub fn decompress(input: &[u8], kind: Compression, limits: Limits) -> Decompressed {
    guarded(|| {
        let clock = Clock::new(limits.time);
        let pieces = Deadline {
            input,
            clock: &clock,
        };
        // The `bufread` decoders see only what the small buffer holds, so
        // they come back to `Deadline`, and to the clock, every `PIECE` bytes.
        match kind {
            Compression::Gzip => read_all(
                flate2::bufread::MultiGzDecoder::new(BufReader::with_capacity(PIECE, pieces)),
                limits,
                &clock,
            ),
            Compression::Bzip2 => read_all(
                bzip2::bufread::MultiBzDecoder::new(BufReader::with_capacity(PIECE, pieces)),
                limits,
                &clock,
            ),
            Compression::Xz => read_all(xz_reader(pieces, limits.dict), limits, &clock),
            Compression::Zstd => {
                read_all(ZstdFrames::new(input, limits.dict, &clock), limits, &clock)
            }
        }
    })
}

/// A panic inside `f` becomes empty data plus the `TRUNCATED` note.
fn guarded(f: impl FnOnce() -> Decompressed) -> Decompressed {
    crate::panic_guard::catch(f).unwrap_or_else(truncated)
}

fn read_all(mut decoder: impl Read, limits: Limits, clock: &Clock) -> Decompressed {
    let mut data: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; CHUNK];
    let mut cut = false;
    let mut note = None;
    let mut error = None;
    loop {
        let room = limits.output - data.len();
        if room == 0 {
            // Cut only if there is more to read; an error here (bad CRC at
            // the end of the stream) still means corrupt data.
            match decoder.read(&mut buf[..1]) {
                Ok(0) => {}
                Ok(_) => cut = true,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => error = Some(e),
            }
            break;
        }
        let want = room.min(CHUNK);
        match decoder.read(&mut buf[..want]) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                error = Some(e);
                break;
            }
        }
        if clock.expired() {
            cut = true;
            note = Some(time_note(limits));
            break;
        }
    }
    let mut needed = None;
    if let Some(e) = error {
        // The time running out comes back as an error from inside the
        // decoder, whatever it wraps it in: a cut, keeping what was decoded.
        if clock.fired.get() || e.kind() == io::ErrorKind::TimedOut {
            cut = true;
            note = Some(time_note(limits));
        } else {
            needed = window_asked(&e);
            note = Some(error_note(&e, limits));
        }
    }
    data.shrink_to_fit();
    Decompressed {
        data,
        cut,
        note,
        needed,
    }
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

    fn crc32(bytes: &[u8]) -> [u8; 4] {
        let mut c = flate2::Crc::new();
        c.update(bytes);
        c.sum().to_le_bytes()
    }

    /// Stream header (CRC32 check) + LZMA2 block header whose dictionary is `props` (40 = 4 GiB).
    pub(crate) fn xz_header_with_dict_props(props: u8) -> Vec<u8> {
        let mut v = b"\xFD7zXZ\0".to_vec();
        v.extend([0x00, 0x01]);
        v.extend(crc32(&[0x00, 0x01]));
        // size=(2+1)*4, flags, LZMA2, 1 prop byte, padding
        let block = [0x02, 0x00, 0x21, 0x01, props, 0x00, 0x00, 0x00];
        v.extend(block);
        v.extend(crc32(&block));
        v
    }

    /// About `bytes` of empty xz blocks after one stream header; each block
    /// declares a 16 MiB dictionary and ends at once (20 bytes per block).
    pub(crate) fn xz_empty_blocks(bytes: usize) -> Vec<u8> {
        let mut block = vec![0x02, 0x00, 0x21, 0x01, 24, 0x00, 0x00, 0x00];
        block.extend(crc32(&block.clone()));
        block.extend([0x00, 0, 0, 0]); // LZMA2 end marker + padding
        block.extend([0, 0, 0, 0]); // CRC32 of no data
        let mut v = b"\xFD7zXZ\0".to_vec();
        v.extend([0x00, 0x01]);
        v.extend(crc32(&[0x00, 0x01]));
        v.extend(block.repeat(bytes / block.len()));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::{bz2, gz, xz_header_with_dict_props};
    use super::*;
    use proptest::prelude::*;
    use std::io::Write;

    const TEXT: &[u8] = b"linha de log\n";

    fn limits(output: usize) -> Limits {
        Limits {
            output,
            dict: VIEW_DICT,
            time: VIEW_TIME,
        }
    }
    const MIB: usize = 1 << 20;

    #[test]
    fn xz_round_trip_and_dictionary_limit() {
        let mut enc =
            lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::with_preset(6)).unwrap();
        enc.write_all(TEXT).unwrap();
        let packed = enc.finish().unwrap();
        assert_eq!(decompress(&packed, Compression::Xz, limits(MIB)).data, TEXT);
        let small = Limits {
            dict: 1 << 20,
            ..limits(MIB)
        }; // preset 6 = 8 MiB dictionary
        assert_eq!(
            decompress(&packed, Compression::Xz, small).note,
            Some(memory_note(1 << 20))
        );
        let huge = xz_header_with_dict_props(40);
        assert_eq!(
            decompress(&huge, Compression::Xz, limits(MIB)).note,
            Some(memory_note(VIEW_DICT))
        );
    }

    fn zstd(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        ruzstd::encoding::compress(data, &mut out, ruzstd::encoding::CompressionLevel::Fastest);
        out
    }

    #[test]
    fn zstd_frames_and_window_limit() {
        let mut two = zstd(b"one\n");
        two.extend(zstd(b"two\n"));
        assert_eq!(
            decompress(&two, Compression::Zstd, limits(MIB)).data,
            b"one\ntwo\n"
        );
        // Magic, descriptor 0x00, window descriptor 0x88 = 2^(10+17) = 128 MiB.
        let big_window = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x88];
        let d = decompress(&big_window, Compression::Zstd, limits(MIB));
        assert_eq!(d.note, Some(memory_note(VIEW_DICT)));
        assert_eq!(d.needed, Some(128 << 20));
        // xz does not say how much it needs.
        let huge = xz_header_with_dict_props(40);
        assert_eq!(decompress(&huge, Compression::Xz, limits(MIB)).needed, None);
    }

    #[test]
    fn zstd_skippable_frames_are_skipped() {
        let mut v = vec![0x50, 0x2A, 0x4D, 0x18, 3, 0, 0, 0, 9, 9, 9];
        v.extend(zstd(b"after\n"));
        let d = decompress(&v, Compression::Zstd, limits(MIB));
        assert_eq!((d.data.as_slice(), d.note), (&b"after\n"[..], None));
    }

    #[test]
    fn zstd_output_limit_spans_frames() {
        let mut two = zstd(&[b'a'; 100]);
        two.extend(zstd(&[b'b'; 100]));
        let d = decompress(&two, Compression::Zstd, limits(150));
        assert_eq!(d.data.len(), 150);
        assert!(d.cut);
    }

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
    fn names_and_extensions() {
        assert_eq!(Compression::Zstd.name(), "zstd");
        assert_eq!(Compression::Zstd.extension(), "zst");
        assert_eq!(Compression::Bzip2.extension(), "bz2");
    }

    #[test]
    fn gzip_and_bzip2_round_trip() {
        for (kind, packed) in [
            (Compression::Gzip, gz(TEXT)),
            (Compression::Bzip2, bz2(TEXT)),
        ] {
            let d = decompress(&packed, kind, limits(MIB));
            assert_eq!((d.data.as_slice(), d.cut, d.note), (TEXT, false, None));
        }
    }

    #[test]
    fn gzip_members_are_joined() {
        let mut two = gz(b"one\n");
        two.extend(gz(b"two\n"));
        assert_eq!(
            decompress(&two, Compression::Gzip, limits(MIB)).data,
            b"one\ntwo\n"
        );
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
            Limits {
                time: Duration::ZERO,
                ..limits(64 * MIB)
            },
        );
        assert!(d.data.len() <= CHUNK && d.cut);
        assert_eq!(d.note.as_deref(), Some("decompression stopped after 0 ms"));
    }

    #[test]
    fn truncated_keeps_what_was_decoded() {
        let all = TEXT.repeat(10_000);
        let packed = gz(&all);
        let d = decompress(
            &packed[..packed.len() / 2],
            Compression::Gzip,
            limits(64 * MIB),
        );
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
        assert_eq!(
            decompress(&gz(&inner), Compression::Gzip, limits(MIB)).data,
            inner
        );
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
        let d = guarded(|| read_all(Boom, limits(MIB), &Clock::new(VIEW_TIME)));
        assert!(d.data.is_empty() && !d.cut);
        assert_eq!(d.note.as_deref(), Some(TRUNCATED));
        assert!(!crate::panic_guard::active());
    }

    #[test]
    fn guard_catches_a_panic() {
        assert_eq!(crate::panic_guard::catch(|| -> u8 { panic!("boom") }), None);
        assert!(!crate::panic_guard::active());
    }

    /// An empty bzip2 stream: "BZh9", the end-of-stream magic and CRC 0.
    const EMPTY_BZIP2: [u8; 14] = [
        b'B', b'Z', b'h', b'9', 0x17, 0x72, 0x45, 0x38, 0x50, 0x90, 0, 0, 0, 0,
    ];
    /// An empty gzip member: header, a final empty block, CRC 0 and size 0.
    const EMPTY_GZIP: [u8; 20] = [
        0x1F, 0x8B, 8, 0, 0, 0, 0, 0, 0, 0xFF, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    /// An empty zstd frame: single segment, content size 0, an empty last block.
    const EMPTY_ZSTD: [u8; 9] = [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x00, 0x01, 0x00, 0x00];

    /// Decoders go through empty members, blocks or frames inside a single
    /// `read`, with no output at all: the clock must stop them anyway. The
    /// bound is loose on purpose (Autoview's limit is 100 ms).
    fn assert_stops_on_time(input: &[u8], kind: Compression) {
        let autoview = Limits {
            output: MIB,
            dict: AUTOVIEW_DICT,
            time: AUTOVIEW_TIME,
        };
        let start = Instant::now();
        let d = decompress(input, kind, autoview);
        let took = start.elapsed();
        assert!(took < Duration::from_secs(2), "{kind:?} took {took:?}");
        assert!(d.cut, "{kind:?} took {took:?}, note {:?}", d.note);
        assert_eq!(
            d.note.as_deref(),
            Some("decompression stopped after 100 ms"),
            "{kind:?}"
        );
    }

    #[test]
    fn empty_bzip2_streams_stop_on_time() {
        assert_stops_on_time(&EMPTY_BZIP2.repeat(MIB / 14), Compression::Bzip2);
    }

    #[test]
    fn empty_gzip_members_stop_on_time() {
        assert_stops_on_time(&EMPTY_GZIP.repeat(8 * MIB / 20), Compression::Gzip);
    }

    #[test]
    fn empty_xz_blocks_stop_on_time() {
        assert_stops_on_time(
            &super::test_util::xz_empty_blocks(16 * MIB),
            Compression::Xz,
        );
    }

    #[test]
    fn empty_zstd_frames_stop_on_time() {
        assert_stops_on_time(&EMPTY_ZSTD.repeat(16 * MIB / 9), Compression::Zstd);
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
