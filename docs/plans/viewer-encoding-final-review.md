# Final review findings — fix wave (one dispatch)

Source: whole-branch review a7c163c..f6f8e83 (opus). Reviewer's measurement harness: ~/.cache/xt-dev/review-bench
(build: `cargo build --release --offline --manifest-path ~/.cache/xt-dev/review-bench/Cargo.toml`;
`target/release/review-bench 1024` = Autoview limits, `review-bench 2048 bzip2` = V limits; `target/release/lines 16 nl`;
`target/release/frame 64`). It compiles the project's real decompress.rs/panic_guard.rs via `#[path]` — after the fixes,
re-run it to show the before/after numbers (adjust its paths if a file moved).

Branch stack (chained, local, unpushed): bidi-guard → decompress → viewer-decompress → viewer-charset → viewer-max-size → ci-audit.
Each fix goes on the branch where the problem is introduced; after committing on a lower branch, rebase the stack on top of it
with `git switch ci-audit && git rebase --update-refs <fixed-branch>` (git 2.47 here), which also moves the intermediate
branch refs. Work bottom-up (PR 1 first). Every branch head must pass `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
and `cargo test` at the end (check each head).

## Critical

### C1 — time limit bypassed by many empty members (branch `decompress`, PR 2)
`read_all` (src/decompress.rs ~199-239) checks the clock only after `read()` returns, but every decoder loops over empty
members/streams/blocks/frames inside ONE `read()` producing no output. Measured: 1 MiB of empty bzip2 streams (14 bytes each)
under Autoview limits = 10.8 s (should stop at ~100 ms); 2 MiB under V = 21 s; 32 MiB empty gzip members = 6 s; 16 MiB empty xz
blocks = 2.1 s; 128 MiB empty zstd frames = 4.3 s. libbz2-rs-sys allocates+zeroes ~3.6 MB per `BZh9` header.
Ruling F1: put the deadline on the INPUT side.
- A private `Deadline` reader wrapping the input `&[u8]`: hands out at most a small piece per `read` (≤ 4 KiB; pick what keeps
  checks frequent enough for bzip2 — the test bound below decides), checks the shared `Instant` on every read and returns
  `io::ErrorKind::TimedOut` once `elapsed >= limits.time`.
- Use the `bufread::` decoders (flate2 `bufread::MultiGzDecoder`, bzip2 `bufread::MultiBzDecoder`) over a small-capacity
  `BufReader` around `Deadline`, so the decoder cannot pull a large chunk between checks; xz and zstd read from `Deadline`
  too; `ZstdFrames` also checks the clock in its frame loop.
- `TimedOut` maps to the time note (`decompression stopped after {limit}`) + `cut = true`, never to TRUNCATED; data decoded
  so far is kept.
- Tests (generous, non-flaky bounds): N empty bzip2 streams (≥ 1 MiB of them) and N empty gzip members under Autoview limits
  return within 2 s with the time note and `cut`; same idea for xz empty blocks and zstd empty frames if cheap. Keep all
  existing decompress tests green.
- Amend the spec (docs/plans/viewer-encoding.md, Seção 2, "Tempo"): the premise "uma leitura sozinha é limitada pela entrada"
  was false; describe the input-side deadline.

### C2 — `text_lines` memory blow-up with many tiny lines (branch `viewer-max-size`, PR 5)
`text_lines` (src/viewer.rs ~1037) builds one `String` per line (24 B struct + heap block ≥ allocator minimum), stores 16 B per
bidi mark and expands tabs up to 8×; it runs even for hex views. Measured: 16 MiB of `\n` → 402 MiB peak (×25); 64 MiB → 1.6 GiB;
with `auto` = RAM/32 the peak reaches ~0.9× RAM, and 256 MiB of newlines is a 208-byte `.bz2`. Before this branch the worst
case was ~470 MiB.
Ruling F2: a memory budget in `text_lines` (not a storage rewrite).
- Estimate memory as you build: per line `size_of::<String>()` + (if non-empty) its byte length rounded up to 16 with a minimum
  of 32 (allocator chunk), + 16 per bidi mark. Stop adding lines once the estimate exceeds 2 × the read limit
  (`load.limits.output`; for `from_bytes`/generated text use `MAX_BYTES`).
- When it stops: the viewer is `cut` (`truncated()` true) and the header says `  (showing first {fmt_num(n)} lines)` in the error
  color, where n is the number of lines kept (this replaces the bytes/MiB cut text when the line budget is what cut it).
- Thread the limit through `from_file` → `build` → `text_lines` (and `set_charset` re-decode) without changing behavior for
  normal files: a 16 MiB log with ~100-byte lines must still load completely.
- Tests: 16 MiB of `\n` under `LoadOptions::view(1 << 20)` (or similar small limit) → cut by the line budget, line count bounded
  (assert the estimate formula's bound), header text; a normal file under the same limit is not affected.
- Re-run the reviewer's `lines` bench and report the new peak for 16 MiB and 64 MiB of `\n` (target: ≤ ~3× the limit).
- Amend the spec (Seção 2, "Memória total"): per-line overhead and the budget.

## Important

### I1 — redraw cost proportional to the visible line's full length (branch `viewer-max-size`, PR 5)
`src/viewer.rs` ~609: `line_str.chars().skip(left).collect()` + `per_char` + segment loop over the whole rest of the line;
~652 (wrap): `Vec<char>` of the whole line + `wrap_line_ranges` over all of it; plus per-char `bidi.binary_search`.
Measured 68 ms/frame for a 16 MiB single line, 254 ms for 64 MiB (~1 s at 256 MiB).
Ruling F3:
- Non-wrap: take only `left..left + width` chars and build colors only for that slice.
- Wrap: compute only the sub-rows needed (`sub_top .. sub_top + remaining height`) — stop `wrap_line_ranges` early and collect
  chars only up to the end of those rows.
- Syntax colors: measure syntect on a 16 MiB single line once (it is cached per line, first 20 000 lines). If it costs more than
  ~100 ms, do not color lines longer than 10 000 chars (state it in the report and the viewer doc).
- Re-run the reviewer's `frame` bench and report before/after.

## Minor items included in this wave (cheap, accuracy/safety)

- T4 (branch `bidi-guard`): `ConfirmKind::OpenHidden` must carry the sources it asked about (not re-read the selection on
  'y'), closing the window where the selection changes between the question and 'y' and an unchecked file opens. Test: ask, move
  the cursor, answer 'y' → the opened set is the one asked about (test via whatever is observable without spawning a program,
  e.g. a test seam or `open_after`; if not observable cheaply, assert the ConfirmKind contents).
- M5 (branch `decompress`): Alt+F5 on a `.tar.xz` with an oversized dictionary shows the crate's raw "needed memory too big
  for mem_limit_kb"; map it to `memory_note(VIEW_DICT)` (spec asks "erro claro"). Keep the archive test, assert the new text.
- M1 (branch `viewer-decompress`): failure notes vanish at the first key (`note` is cleared in `handle_key`), so a truncated/
  corrupt `.gz` then looks complete. Add a persistent header span in the error color for a decompression failure/truncation
  (e.g. `  (truncated or corrupt)` / `  (not decompressed: <reason>)` — keep it short and consistent with the notes).
- M6 (branch `viewer-decompress`): "(V to view)" is appended even when V's 64 MiB limit would refuse too; append it only when
  the requirement fits V's limit, if the error tells you the requested size (zstd `WindowSizeTooBig { requested }`; xz: if not
  available, keep the suffix — say so).
- M8 (branch `viewer-decompress`): `viewer_decompress` field lacks its `///` doc comment in src/config.rs.
- M2 (branch `viewer-decompress`): docs/manual/viewer.md (+5 translations) claim `(showing first N bytes decompressed)` appears
  for all three notes; it only appears for output/time cuts. Fix the wording in all 6.
- M4 (wherever introduced; `git log -S` to find it): docs/manual/fr/viewer.md uses "Le visualiseur"; the French manual elsewhere
  says "la visionneuse". Align.
- M3 + M9 (branch `viewer-max-size`): docs/manual/ztreewin.md (+5) still says the viewer reads "up to 16 MiB" — update to the
  configurable limit; document in viewer.md/configuration.md (+5 each) that 1024 MiB costs roughly 2.5 GiB of RAM and ~5 s to
  open ordinary text (spec measurement), and that very short lines cost more (now bounded by the line budget).
- M7 (branch `ci-audit`): pin `cargo install cargo-audit --locked --version <current exact version>` and add a weekly
  `schedule:` trigger (cron) for the audit job only — keep the `test` job's triggers unchanged (use `if: github.event_name !=
  'schedule'` on `test`, or a separate workflow file for the scheduled audit; pick the simpler correct one and say which).

## Not in this wave (stay deferred in the ledger)
All other "minor (deferred)" ledger lines.

## Bookkeeping
- At the end, update docs/plans/viewers.md ("Como retomar") and the top of docs/plans/viewer-encoding.md: final review done,
  fix wave applied (list C1, C2, I1 + minors), on which branch each landed; open decision for the owner: RUSTSEC-2025-0141
  (bincode unmaintained, via syntect; only deserializes syntect's embedded dumps, not user files).
- Commit messages end with: Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>. Never push.
