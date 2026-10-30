//! Batch (Ctrl+B) in the viewer: asks for the file and the mask, shows what
//! would be written, and writes it in the background.

use super::*;
use crate::viewer::batch::{self, Job, Output, Preview, Source};

/// A Batch waiting for the user's look at what it would write.
pub struct BatchPreview {
    pub source: Source,
    pub file: PathBuf,
    pub preview: Preview,
}

/// Longest the screen waits for the thread in one tick.
const POLL: Duration = Duration::from_millis(20);

impl App {
    fn viewer_note(&mut self, note: String) {
        match &mut self.viewer {
            Some(v) => v.note = Some(note),
            None => self.message = Some(note),
        }
    }

    pub(crate) fn viewer_batch(&mut self) {
        // The output goes next to what is browsed, and there is no writing on
        // a remote machine yet.
        if self.remote.is_some() {
            self.viewer_note("Batch of records works only on this machine".into());
            return;
        }
        if let Some(Err(e)) = self.viewer.as_ref().map(Viewer::batch_source) {
            self.viewer_note(e);
            return;
        }
        let base = self.outer.as_deref().unwrap_or(self);
        let default = base.current_path().join("xy-batch.txt");
        let default = default.to_string_lossy().into_owned();
        let label = "BATCH: write to file: ".to_string();
        self.open_prompt(PromptKind::ViewerBatchFile, label, &default);
    }

    pub(crate) fn viewer_batch_file(&mut self, text: &str) {
        let base = self.outer.as_deref().unwrap_or(self).current_path();
        let file = ops::resolve_path(text, &base);
        if batch::refuses_records(&file) {
            self.viewer_note(
                "Batch of records cannot write .bat or .cmd: %VAR% would expand".into(),
            );
            return;
        }
        if fs::symlink_metadata(&file).is_ok() {
            self.viewer_note(format!("{} already exists", file.display()));
            return;
        }
        let Some(v) = &self.viewer else { return };
        let columns = v.table.as_ref().map_or(&[][..], |t| t.headers.as_slice());
        let hint = match v.batch_source() {
            Ok(source) => source.field_hint(columns),
            Err(_) => String::new(),
        };
        let whole = if batch::holds_records(&file) {
            "; empty = whole records, @a,b = those fields"
        } else {
            ""
        };
        let label = format!(
            "BATCH line per record (%{{{hint}}}{whole}) into {}: ",
            fit_name(&file)
        );
        self.open_prompt(PromptKind::ViewerBatchMask { file }, label, "");
    }

    pub(crate) fn viewer_batch_mask(&mut self, file: PathBuf, text: &str) {
        let Some(v) = &self.viewer else { return };
        let (open_path, open_size) = (v.path.clone(), v.size);
        let output = Output::parse(text, &file, &open_path, open_size);
        let found = output.and_then(|output| {
            let source = v.batch_source()?;
            let preview = crate::panic_guard::catch(|| batch::preview(&source, output))
                .unwrap_or_else(|| Err("the file could not be read".into()))?;
            Ok((source, preview))
        });
        match found {
            Ok((_, preview)) if preview.scanned == 0 => {
                self.viewer_note("Batch: no records".into());
            }
            Ok((source, preview)) => {
                self.batch_preview = Some(BatchPreview {
                    source,
                    file,
                    preview,
                });
            }
            Err(e) => self.viewer_note(e),
        }
    }

    pub(crate) fn batch_preview_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Some(p) = self.batch_preview.take() {
                    self.batch_job = Some(Job::start(p.source, p.preview.output, p.file));
                }
            }
            KeyCode::Esc => {
                self.batch_preview = None;
                self.viewer_note("Batch cancelled".into());
            }
            _ => {}
        }
    }

    pub(crate) fn batch_tick(&mut self, budget: Duration) {
        let Some(job) = &self.batch_job else { return };
        if !job.is_done() {
            let verb = if job.is_cancelling() {
                "Cancelling"
            } else {
                "Writing"
            };
            let note = format!(
                "{verb} {}: {} records (Esc cancels)",
                fit_name(&job.file),
                job.written()
            );
            self.viewer_note(note);
            // The work is in its own thread: this only waits for it.
            std::thread::sleep(budget.min(POLL));
            return;
        }
        let Some(job) = self.batch_job.take() else {
            return;
        };
        let file = job.file.clone();
        let note = match job.finish() {
            Ok(done) if done.cancelled => {
                format!(
                    "Batch cancelled after {} records: {} was not kept",
                    done.records,
                    file.display()
                )
            }
            Ok(done) => {
                if let Some(dir) = file.parent() {
                    self.rescan_everywhere(&[dir]);
                }
                let mut note = format!("Wrote {} records to {}", done.records, file.display());
                if done.cut {
                    note.push_str(". Only the part of the file the viewer read was written");
                }
                if let Some(warning) = done.stats.warning() {
                    note.push_str(&format!(". {warning}"));
                }
                note
            }
            Err(e) => format!("Batch failed: {e}"),
        };
        self.viewer_note(note);
    }
}
