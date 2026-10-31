//! Batch (Ctrl+B) in the viewer: asks for the file and the mask, shows what
//! would be written, and writes it in the background.

use super::*;
use crate::viewer::batch::{self, Job, Output, Preview, Source, TemplateInfo};

/// A Batch waiting for the user's look at what it would write.
pub struct BatchPreview {
    pub source: Source,
    pub file: PathBuf,
    pub preview: Preview,
}

/// What the screen explains while the mask is being typed.
pub struct BatchGuide {
    /// The name of the file to write.
    pub file: String,
    /// A mask that works on what is open.
    pub example: String,
    pub fields: String,
    /// An empty mask writes whole records.
    pub whole: bool,
    pub templates: Vec<TemplateInfo>,
    /// The template chosen, which is what the mask says.
    pub selected: Option<usize>,
    /// The template being read (F3), instead of the guide.
    pub reading: Option<TemplateReading>,
    /// Where templates go.
    pub template_dir: Option<PathBuf>,
    /// The open file is a template, which is not what a Batch reads.
    pub open_template: Option<String>,
    /// What was wrong with the mask just typed.
    pub problem: Option<String>,
}

/// A template on the screen, while its mask waits.
pub struct TemplateReading {
    pub name: String,
    pub lines: Vec<String>,
    /// The first line shown.
    pub top: usize,
}

/// How many lines PgUp and PgDn move a template.
const READING_PAGE: usize = 20;

/// The label of the mask prompt; the guide above it says the rest.
const MASK_LABEL: &str = "BATCH mask or :template: ";

/// The warning for a Batch of a template opened as if it were the data.
fn open_template_note(path: &Path, templates: &[TemplateInfo]) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    if !ext.eq_ignore_ascii_case(batch::TEMPLATE_EXTENSION) {
        return None;
    }
    let name = path.file_stem()?.to_str()?;
    Some(if templates.iter().any(|t| t.name == name) {
        format!(
            "This file is the template itself. To use it, open the data, press Ctrl+B there and type :{name}"
        )
    } else {
        "This file is a template, not data. Copy it to the folder below, open the data, press Ctrl+B there and type : and its name".to_string()
    })
}

/// The template a mask names.
fn template_typed(templates: &[TemplateInfo], typed: &str) -> Option<usize> {
    let name = typed.trim().strip_prefix(':')?;
    templates.iter().position(|t| t.name == name)
}

impl Prompt {
    /// The keys of the mask prompt that are not typing: whether this one was
    /// taken. ↑↓ choose a template while the mask is empty or names one, and
    /// otherwise go through the history, as PgUp and PgDn always do.
    pub(crate) fn batch_mask_key(&mut self, key: KeyEvent) -> bool {
        let PromptKind::ViewerBatchMask { guide, .. } = &mut self.kind else {
            return false;
        };
        if let Some(reading) = &mut guide.reading {
            let last = reading.lines.len().saturating_sub(1);
            match key.code {
                KeyCode::Up => reading.top = reading.top.saturating_sub(1),
                KeyCode::Down => reading.top = (reading.top + 1).min(last),
                KeyCode::PageUp => reading.top = reading.top.saturating_sub(READING_PAGE),
                KeyCode::PageDown => reading.top = (reading.top + READING_PAGE).min(last),
                KeyCode::Home => reading.top = 0,
                KeyCode::End => reading.top = last,
                KeyCode::F(3) | KeyCode::Esc => guide.reading = None,
                // Reading it was the last step before using it.
                KeyCode::Enter => {
                    guide.reading = None;
                    return false;
                }
                _ => {}
            }
            return true;
        }
        let count = guide.templates.len();
        let choosing = count > 0 && (self.input.text.is_empty() || guide.selected.is_some());
        match key.code {
            KeyCode::F(3) => {
                let Some(first) = guide.selected.or((count > 0).then_some(0)) else {
                    return true;
                };
                let name = guide.templates[first].name.clone();
                match batch::template_lines(&name) {
                    Ok(lines) => {
                        guide.selected = Some(first);
                        self.input.set_text(format!(":{name}"));
                        guide.reading = Some(TemplateReading {
                            name,
                            lines,
                            top: 0,
                        });
                    }
                    Err(e) => guide.problem = Some(e),
                }
                true
            }
            KeyCode::Up | KeyCode::Down if choosing => {
                guide.selected = match (key.code, guide.selected) {
                    (KeyCode::Down, None) => Some(0),
                    (KeyCode::Down, Some(i)) => Some((i + 1).min(count - 1)),
                    (_, None) => Some(count - 1),
                    (_, Some(0)) => None,
                    (_, Some(i)) => Some(i - 1),
                };
                let text = guide
                    .selected
                    .map_or(String::new(), |i| format!(":{}", guide.templates[i].name));
                self.input.set_text(text);
                true
            }
            KeyCode::PageUp | KeyCode::PageDown => {
                let code = if key.code == KeyCode::PageUp {
                    KeyCode::Up
                } else {
                    KeyCode::Down
                };
                self.input.handle_key(KeyEvent::new(code, key.modifiers));
                guide.selected = template_typed(&guide.templates, &self.input.text);
                true
            }
            KeyCode::Enter | KeyCode::Esc => false,
            _ => {
                self.input.handle_key(key);
                guide.selected = template_typed(&guide.templates, &self.input.text);
                true
            }
        }
    }
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
        self.open_batch_mask(file, "", None);
    }

    /// Asks for the mask, again with what was typed when it had a problem.
    fn open_batch_mask(&mut self, file: PathBuf, typed: &str, problem: Option<String>) {
        let Some(v) = &self.viewer else { return };
        let columns = v.table.as_ref().map_or(&[][..], |t| t.headers.as_slice());
        let (fields, example) = match v.batch_source() {
            Ok(source) => (source.field_hint(columns), source.example(columns)),
            Err(_) => (String::new(), String::new()),
        };
        let templates = batch::template_list();
        let guide = BatchGuide {
            file: fit_name(&file),
            example,
            fields,
            whole: batch::holds_records(&file),
            open_template: open_template_note(&v.path, &templates),
            selected: template_typed(&templates, typed),
            reading: None,
            templates,
            template_dir: batch::template_dir(),
            problem,
        };
        let kind = PromptKind::ViewerBatchMask { file, guide };
        self.open_prompt(kind, MASK_LABEL.to_string(), typed);
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
            // Nothing was written: the mask is there to be corrected.
            Err(e) => self.open_batch_mask(file, text, Some(e)),
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
