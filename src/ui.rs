//! Screen drawing in the classic XTree layout:
//!
//! ```text
//! ┌ current path ───────────────────────────┐
//! │ tree                        │ stats     │
//! ├─────────────────────────────┤           │
//! │ directory files             │           │
//! └─────────────────────────────┴───────────┘
//!  command menu (2 lines)
//! ```

use std::ops::Range;
use std::time::SystemTime;

use chrono::{DateTime, Local};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph};

use crate::app::{self, App, FileView, Focus, KbyteShow, MenuMode, Pane};
use crate::config::TerminalTheme;
use crate::config::{self, formats, theme};
use crate::git::StatusCache;
use crate::input::Command;
use crate::model::tree::{FileEntry, FileKind};
use crate::ops::Operation;
use crate::pane::{Count, FileDisplay, FileRef};
use crate::settings::{self, Settings};
use crate::term::{self, Terminal};
use crate::viewer::{self, ViewMode, Viewer};

const STATS_WIDTH: u16 = 28;
const SMALL_FILE_WINDOW: u16 = 8;
/// Below this height, the tree is shown without the small file window.
const MIN_TREE_WITH_FILES: u16 = 2 * SMALL_FILE_WINDOW;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let base = Style::new().fg(theme().text).bg(theme().background);
    frame.render_widget(Block::new().style(base), frame.area());

    if app.viewer.is_some() {
        draw_viewer(frame, app);
        return;
    }
    if let Some(settings) = &app.settings {
        draw_settings(frame, settings);
        return;
    }

    let [header, main, menu] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    draw_header(frame, app, header);
    if app.picker.is_some() {
        draw_picker(frame, app, main, menu);
        return;
    }
    draw_menu(frame, app, menu);
    let git = app.options.git.then_some(&app.git_status);

    // Alt+X: the terminal takes the bottom of the screen; the tree stays
    // above it, following the shell's `cd`s. Alt+Z: the whole screen.
    let main = if app.terminal_focus
        && let Some(term) = &mut app.terminal
    {
        if app.terminal_full {
            draw_terminal(frame, term, app.options.terminal_theme, main);
            return;
        }
        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(main);
        draw_terminal(frame, term, app.options.terminal_theme, bottom);
        top
    } else {
        main
    };

    if app.autoview.is_some() {
        // F7: list on the left, contents of the selected file on the right.
        let [list, view] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(main);
        let active = app.active;
        draw_files(frame, &mut app.panes[active], true, list, git);
        if let Some(v) = &mut app.autoview {
            let name = v
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            draw_viewer_body(frame, v, format!(" Autoview: {name} "), view);
        }
    } else if app.is_split() {
        // Two windows side by side; the statistics panel goes away to make room.
        let halves: [Rect; 2] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(main);
        let active = app.active;
        for (i, (pane, area)) in app.panes.iter_mut().zip(halves).enumerate() {
            draw_pane(frame, pane, i == active, true, area, git);
        }
    } else {
        let [left, stats] =
            Layout::horizontal([Constraint::Min(20), Constraint::Length(STATS_WIDTH)]).areas(main);
        draw_stats(frame, app, stats);
        draw_pane(frame, &mut app.panes[0], true, false, left, git);
    }
}

/// One window: tree on top and files below, or just the files.
/// With a split screen, the tree title shows the current directory.
fn draw_pane(
    frame: &mut Frame,
    pane: &mut Pane,
    active: bool,
    split: bool,
    area: Rect,
    git: Option<&StatusCache>,
) {
    if pane.focus == Focus::Files {
        draw_files(frame, pane, active, area, git);
        return;
    }
    let width = area.width.saturating_sub(4) as usize;
    let title = if split {
        format!(
            " {} ",
            fit_tail(&pane.current_path().to_string_lossy(), width)
        )
    } else if let Some(archive) = &pane.archive {
        let name = archive.file_name().unwrap_or_default().to_string_lossy();
        format!(" Archive: {name} (read-only) ")
    } else {
        " Tree ".to_string()
    };
    // Too low for both (e.g. above the terminal's panel): the tree alone,
    // rather than a tree of one or two lines.
    if area.height < MIN_TREE_WITH_FILES {
        draw_tree(frame, pane, active, title, area);
        return;
    }
    let [tree, files] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(SMALL_FILE_WINDOW)]).areas(area);
    draw_tree(frame, pane, active, title, tree);
    draw_files(frame, pane, false, files, git);
}

fn panel(title: impl Into<Line<'static>>, active: bool) -> Block<'static> {
    let border = if active {
        theme().active_border
    } else {
        theme().border
    };
    Block::bordered()
        .border_type(BorderType::Plain)
        .border_style(Style::new().fg(border))
        .title(title)
}

fn highlight(active: bool) -> Style {
    if active {
        Style::new()
            .fg(theme().highlight_text)
            .bg(theme().highlight)
    } else {
        Style::new()
            .fg(theme().highlight_text)
            .bg(theme().inactive_highlight)
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let path = app.tree.path(app.header_dir());
    let clock = Local::now().format(formats().date.pattern()).to_string();
    let [left, right] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(clock.len() as u16)])
            .areas(area);
    frame.render_widget(
        Line::from(vec![
            Span::styled("Path: ", Style::new().fg(theme().key)),
            Span::raw(path.to_string_lossy().into_owned()),
        ]),
        left,
    );
    frame.render_widget(Line::from(clock), right);
}

fn draw_tree(frame: &mut Frame, pane: &mut Pane, active: bool, title: String, area: Rect) {
    pane.tree_page = area.height.saturating_sub(2) as usize;

    let (range, mut state) = visible_window(&mut pane.tree_state, pane.rows.len(), pane.tree_page);
    // Alt+K: size or file count to the right of each directory.
    let kbyte = pane.kbyte;
    let totals = (kbyte.show != KbyteShow::None)
        .then(|| pane.tree.dir_totals(kbyte.branch, kbyte.tagged_only));
    let width = area.width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = pane.rows[range]
        .iter()
        .map(|row| {
            let node = pane.tree.node(row.id);
            let name = node.name.to_string_lossy().into_owned();
            let name_style = if node.error.is_some() {
                Style::new().fg(theme().error)
            } else {
                Style::new()
            };
            let mut spans = vec![
                Span::styled(row.prefix.clone(), Style::new().fg(theme().dim)),
                Span::styled(name.clone(), name_style),
            ];
            if let Some(totals) = &totals {
                let (count, bytes) = totals[row.id];
                let value = match (node.scanned, kbyte.show) {
                    (false, _) => "?".to_string(),
                    (_, KbyteShow::Size) => format!("{}K", fmt_num(bytes.div_ceil(1024))),
                    _ => fmt_num(count),
                };
                let used = row.prefix.chars().count() + name.chars().count();
                let pad = width.saturating_sub(used + value.chars().count() + 1);
                spans.push(Span::raw(" ".repeat(pad + 1)));
                spans.push(Span::styled(value, Style::new().fg(theme().dim)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = match kbyte.show {
        KbyteShow::None => title,
        _ => format!("{} · {} ", title.trim_end(), kbyte.describe()),
    };
    // As in ZTreeWin: line of the cursor / lines in the tree.
    let position = position(pane.tree_state.selected(), pane.rows.len());
    let list = List::new(items)
        .block(panel(title, active).title_top(position))
        .highlight_style(highlight(active));
    frame.render_stateful_widget(list, area, &mut state);
}

/// `7/2,027` on a window's border: the cursor's line and how many there are.
fn position(selected: Option<usize>, len: usize) -> Line<'static> {
    let at = if len == 0 {
        0
    } else {
        selected.unwrap_or(0).min(len - 1) + 1
    };
    Line::from(format!("{}/{}", fmt_num(at as u64), fmt_num(len as u64))).right_aligned()
}

/// F2 in a prompt: the tree to pick a folder from, with the prompt's question
/// in the footer.
fn draw_picker(frame: &mut Frame, app: &mut App, main: Rect, menu: Rect) {
    let question = app
        .prompt
        .as_ref()
        .map(|p| p.label.clone())
        .unwrap_or_default();
    let Some(picker) = &mut app.picker else {
        return;
    };
    let windows = picker.panes.len();
    let width = main.width.saturating_sub(4) as usize;
    let path = picker.pane().current_path();
    let title = format!(
        " Folder: {} ",
        fit_tail(&path.to_string_lossy(), width.saturating_sub(10))
    );
    draw_tree(frame, picker.pane_mut(), true, title, main);

    let mut keys = vec![
        ("↑↓", "move"),
        ("←→", "close/open"),
        ("Enter", "choose"),
        ("Esc", "back"),
    ];
    if windows > 1 {
        keys.push(("Tab", "other window"));
    }
    let lines = vec![
        menu_line("FOLDER", &keys),
        Line::from(Span::styled(
            format!("{:9}{}", "", question.trim_end()),
            Style::new().fg(theme().dim),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), menu);
}

/// With `git`, files in a repository show their status before the name.
fn draw_files(
    frame: &mut Frame,
    pane: &mut Pane,
    active: bool,
    area: Rect,
    git: Option<&StatusCache>,
) {
    let git = git.filter(|g| pane.archive.is_none() && g.in_repo(&pane.current_path()));
    // Status letters of a file in the repository ("  " if unchanged).
    let marker =
        |pane: &Pane, f: FileRef| git.map(|g| g.marker(&pane.file_path(f)).unwrap_or(*b"  "));
    let kind = match pane.view {
        FileView::Dir => "Files",
        FileView::Branch(_) => "Branch",
        FileView::Showall => "Showall",
    };
    let title = format!(
        " {kind}: {} ({}) {} ",
        pane.filespec.text(),
        fmt_num(pane.files.len() as u64),
        pane.sort.label()
    );
    let error = match pane.view {
        FileView::Dir => pane.tree.node(pane.current()).error.is_some(),
        _ => false,
    };
    if pane.display.multi_column() && !pane.files.is_empty() && !error {
        draw_file_columns(frame, pane, active, area, title, &marker);
        return;
    }
    pane.file_cols = 1;
    pane.file_page = area.height.saturating_sub(2) as usize;

    let width = area.width.saturating_sub(2) as usize;
    let names_only = pane.display == FileDisplay::LongNames;
    let (range, mut state) = visible_window(&mut pane.file_state, pane.files.len(), pane.file_page);
    let error = match pane.view {
        FileView::Dir => pane.tree.node(pane.current()).error.as_ref(),
        _ => None,
    };
    let items: Vec<ListItem> = if let Some(err) = error {
        vec![ListItem::new(Span::styled(
            format!("Cannot read directory: {err}"),
            Style::new().fg(theme().error),
        ))]
    } else if pane.files.is_empty() {
        vec![ListItem::new(Span::styled(
            "No matching files",
            Style::new().fg(theme().dim),
        ))]
    } else {
        pane.files[range]
            .iter()
            .map(|&f| file_line(pane.file(f), width, names_only, marker(pane, f)))
            .collect()
    };

    let position = position(pane.file_state.selected(), pane.files.len());
    let list = List::new(items)
        .block(panel(title, active).title_bottom(position))
        .highlight_style(highlight(active));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Adjusts the scroll of `state` so the selected item stays visible.
/// Returns the range of items that fits in `height` lines and the state to use
/// to draw only that range (large lists are not built in full).
fn visible_window(state: &mut ListState, len: usize, height: usize) -> (Range<usize>, ListState) {
    let height = height.max(1);
    let sel = state.selected().unwrap_or(0).min(len.saturating_sub(1));
    let mut offset = state.offset().min(sel);
    if sel >= offset + height {
        offset = sel + 1 - height;
    }
    // No empty space at the end when the list shrinks.
    offset = offset.min(len.saturating_sub(height));
    *state.offset_mut() = offset;
    let range = offset..(offset + height).min(len);
    (
        range,
        ListState::default().with_selected(Some(sel - offset)),
    )
}

/// Longer names are cut with `…` in the column modes, so that several columns
/// fit (as in XTree, with fixed-width columns).
const MAX_COLUMN_NAME: usize = 20;

/// Multi-column modes (Alt+F): files run left to right and then onto the next
/// row down, as in XTree.
fn draw_file_columns(
    frame: &mut Frame,
    pane: &mut Pane,
    active: bool,
    area: Rect,
    title: String,
    marker: &dyn Fn(&Pane, FileRef) -> Option<[u8; 2]>,
) {
    let git_w = if pane
        .files
        .first()
        .is_some_and(|&f| marker(pane, f).is_some())
    {
        3
    } else {
        0
    };
    let block = panel(title, active);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let (width, height) = (inner.width as usize, (inner.height as usize).max(1));
    let with_size = pane.display == FileDisplay::NamesSize;

    let n = pane.files.len();
    let name_len = |f: &FileEntry| {
        f.name.to_string_lossy().chars().count() + usize::from(f.kind == FileKind::Symlink)
    };
    let name_w = pane
        .files
        .iter()
        .map(|&f| name_len(pane.file(f)))
        .max()
        .unwrap_or(1)
        .min(MAX_COLUMN_NAME);
    // Tag mark + name + (size and permissions) + space between columns.
    let cell_w = 2 + git_w + name_w + if with_size { 23 } else { 0 } + 1;
    let cols = (width / cell_w).max(1);
    pane.file_cols = cols;
    pane.file_page = height * cols;

    // Scroll by rows, with the selected file always visible.
    let sel = pane.file_state.selected().unwrap_or(0).min(n - 1);
    let rows = n.div_ceil(cols);
    let sel_row = sel / cols;
    let mut top = pane.file_state.offset().min(sel_row);
    if sel_row >= top + height {
        top = sel_row + 1 - height;
    }
    top = top.min(rows.saturating_sub(height));
    *pane.file_state.offset_mut() = top;

    let lines: Vec<Line> = (top..(top + height).min(rows))
        .map(|r| {
            let mut spans = Vec::new();
            for i in r * cols..(r * cols + cols).min(n) {
                let git = marker(pane, pane.files[i]);
                let f = pane.file(pane.files[i]);
                let mut name = f.name.to_string_lossy().into_owned();
                if f.kind == FileKind::Symlink {
                    name.push('@');
                }
                let mut cell = format!(
                    "{}{}{}",
                    if f.tagged { "■ " } else { "  " },
                    git.map(|xy| git_text(&xy)).unwrap_or_default(),
                    fit(&name, name_w)
                );
                if with_size {
                    cell.push_str(&format!(
                        " {:>11} {}",
                        fmt_num(f.size),
                        fmt_mode(f.mode, f.kind)
                    ));
                }
                cell.push(' ');
                let style = if i == sel {
                    highlight(active)
                } else {
                    file_style(f)
                };
                spans.push(Span::styled(cell, style));
            }
            Line::from(spans)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Name color by file type (and whether it is tagged).
fn file_style(f: &FileEntry) -> Style {
    match f.kind {
        _ if f.tagged => Style::new().fg(theme().tagged).add_modifier(Modifier::BOLD),
        FileKind::Symlink => Style::new().fg(theme().symlink),
        FileKind::Other => Style::new().fg(theme().special),
        FileKind::File if crate::platform::is_executable(&f.name, f.mode) => {
            Style::new().fg(theme().executable)
        }
        FileKind::File => Style::new(),
    }
}

/// One row of the files window: name, size, date and permissions (or just
/// the name, in long-names mode).
/// `M ` → `"M  "`: the two status letters and a space.
fn git_text(xy: &[u8; 2]) -> String {
    format!("{}{} ", char::from(xy[0]), char::from(xy[1]))
}

fn file_line(
    f: &FileEntry,
    width: usize,
    names_only: bool,
    git: Option<[u8; 2]>,
) -> ListItem<'static> {
    let size = fmt_num(f.size);
    let date = f.modified.map(fmt_time).unwrap_or_default();
    let mode = fmt_mode(f.mode, f.kind);
    // In narrow windows (split screen), the least useful columns are dropped.
    let details = if names_only {
        String::new()
    } else if width >= 72 {
        format!(" {size:>15} {date:16} {mode}")
    } else if width >= 50 {
        format!(" {size:>13} {date:16}")
    } else {
        format!(" {size:>13}")
    };

    // 2 columns for the tag mark, 3 for the Git status.
    let git = git.map(|xy| git_text(&xy));
    let marks = 2 + git.as_ref().map_or(0, String::len);
    let name_width = width.saturating_sub(details.chars().count() + marks).max(8);
    let mut name: String = f.name.to_string_lossy().into_owned();
    if f.kind == FileKind::Symlink {
        name.push('@');
    }
    let name = fit(&name, name_width);

    let style = file_style(f);
    let mark = if f.tagged { "■ " } else { "  " };
    let mut spans = vec![Span::styled(mark, Style::new().fg(theme().tagged))];
    if let Some(git) = git {
        spans.push(Span::styled(
            git,
            Style::new().fg(theme().key).add_modifier(Modifier::BOLD),
        ));
    }
    spans.extend([
        Span::styled(name, style),
        Span::styled(details, Style::new().fg(theme().dim)),
    ]);
    ListItem::new(Line::from(spans))
}

/// The panel on the right, as in ZTreeWin: three boxes (FILE, DISK, DISK
/// Statistics) separated by lines.
fn draw_stats(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel("", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let width = inner.width as usize;

    let label = |s: String| Span::styled(s, Style::new().fg(theme().key));
    let bold = |s: String| Span::styled(s, Style::new().add_modifier(Modifier::BOLD));
    // "Logged", "Current Directory"...: centered.
    let heading = |s: &str| Line::from(label(s.to_string())).centered();
    // Name on the left, number on the right; the number goes to the next
    // line when both don't fit (e.g. "Available" and a disk in terabytes).
    let value = |k: &str, v: u64| -> Vec<Line<'static>> {
        let v = fmt_num(v);
        let (k_len, v_len) = (k.chars().count(), v.chars().count());
        if k_len + 1 + v_len <= width {
            let pad = " ".repeat(width - k_len - v_len);
            vec![Line::from(vec![Span::raw(format!("{k}{pad}")), bold(v)])]
        } else {
            vec![heading(k), Line::from(bold(v)).right_aligned()]
        }
    };
    let count = |c: Count| [value("Files", c.files), value("Bytes", c.bytes)].concat();

    let id = app.header_dir();
    let path = app.tree.path(id);
    let file = vec![Line::from(vec![
        label("FILE ".into()),
        bold(fit(app.filespec.text(), width.saturating_sub(5))),
    ])];

    let mut disk = Vec::new();
    let mut total_lines = 0;
    if let Some(space) = crate::platform::disk_space(&path) {
        let point = crate::info::mount_of(&path)
            .map(|m| m.point.to_string_lossy().into_owned())
            .unwrap_or_default();
        disk.push(Line::from(vec![
            label("DISK ".into()),
            Span::raw(fit_tail(&point, width.saturating_sub(5))),
        ]));
        disk.extend(value("Available", space.available));
        // Lines of "Total": one, or two when the number goes below.
        let total = value("Total", space.total);
        total_lines = total.len();
        disk.extend(total);
    }

    let stats = app.disk_stats();
    let node = app.tree.node(id);
    let here = node
        .files
        .iter()
        .filter(|f| app.tree.show_hidden || !f.is_hidden())
        .fold(Count::default(), |c, f| Count {
            files: c.files + 1,
            bytes: c.bytes + f.size,
        });
    // Just the name, also for the tree's root ("/" for the root of the disk).
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    let mut statistics = vec![
        Line::from(label("DISK Statistics".into())),
        heading("Logged"),
    ];
    statistics.extend(count(stats.logged));
    statistics.push(heading("Matching"));
    statistics.extend(count(stats.matching));
    statistics.push(heading("Tagged"));
    statistics.extend(count(stats.tagged));
    statistics.push(heading("Current Directory"));
    statistics.push(Line::from(bold(fit(&name, width))));
    statistics.extend(count(here));
    if let Ok(meta) = std::fs::metadata(&path) {
        let mode = format!(
            "d{}",
            crate::platform::format_mode(crate::platform::mode(&meta))
        );
        let k = if cfg!(windows) { "Attributes" } else { "Perms" };
        let pad = " ".repeat(width.saturating_sub(k.len() + mode.chars().count()));
        statistics.push(Line::from(vec![Span::raw(format!("{k}{pad}")), bold(mode)]));
    }

    // Short screens: without the "DISK Statistics" title, then without the
    // total size of the disk, so that the current directory still fits.
    let needed =
        |file: &[Line], disk: &[Line], stats: &[Line]| file.len() + disk.len() + stats.len() + 2;
    let room = inner.height as usize;
    if needed(&file, &disk, &statistics) > room {
        statistics.remove(0);
    }
    if needed(&file, &disk, &statistics) > room {
        disk.truncate(disk.len() - total_lines);
    }

    let separator =
        Line::from(format!("├{}┤", "─".repeat(width))).style(Style::new().fg(theme().border));
    let mut y = inner.y;
    for (i, section) in [file, disk, statistics].into_iter().enumerate() {
        if section.is_empty() || y >= inner.bottom() {
            continue;
        }
        if i > 0 {
            let row = Rect::new(area.x, y, area.width, 1);
            frame.render_widget(separator.clone(), row);
            y += 1;
        }
        let height = (section.len() as u16).min(inner.bottom().saturating_sub(y));
        frame.render_widget(
            Paragraph::new(section),
            Rect::new(inner.x, y, inner.width, height),
        );
        y += height;
    }
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let lines = if let Some(prompt) = &app.prompt {
        prompt_lines(frame, prompt, area)
    } else if app.terminal_focus {
        let key_for = |cmd| app.keymap.key_for(cmd).unwrap_or_default();
        vec![
            menu_line(
                "TERMINAL",
                &[
                    (&key_for(Command::Terminal), "back to xt"),
                    (
                        &key_for(Command::TerminalFull),
                        if app.terminal_full {
                            "panel"
                        } else {
                            "full screen"
                        },
                    ),
                    (&key_for(Command::TerminalTagged), "type the tagged files"),
                    ("Shift+PgUp/PgDn", "scroll back"),
                ],
            ),
            match &app.message {
                // E.g. the shell went outside the tree.
                Some(msg) => Line::from(Span::styled(
                    format!("{:9}{msg}", ""),
                    Style::new()
                        .fg(theme().message)
                        .add_modifier(Modifier::BOLD),
                )),
                None => menu_line(
                    "",
                    &[(
                        "",
                        "the tree follows the shell's cd; $XT_TAGGED has the tagged files; exit closes the shell",
                    )],
                ),
            },
        ]
    } else if let Some(job) = &app.archive_job {
        vec![
            Line::from(vec![
                Span::styled(
                    format!(
                        "Archiving {} of {}: ",
                        (job.done() + 1).min(job.total()),
                        job.total()
                    ),
                    bold,
                ),
                Span::raw(job.current().unwrap_or("").to_string()),
            ]),
            menu_line(
                "",
                &[
                    (
                        "",
                        &format!(
                            "{} of {} bytes",
                            fmt_num(job.done_bytes),
                            fmt_num(job.total_bytes)
                        ),
                    ),
                    ("Esc", "cancel"),
                ],
            ),
        ]
    } else if let Some(job) = &app.search_job {
        let name = job
            .current()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        vec![
            Line::from(vec![
                Span::styled(
                    format!("Searching {} of {}: ", job.done() + 1, job.total()),
                    bold,
                ),
                Span::raw(name),
                Span::styled(
                    format!("   {} found", job.found),
                    Style::new().fg(theme().dim),
                ),
            ]),
            menu_line("", &[("Esc", "cancel")]),
        ]
    } else if let Some(confirm) = &app.confirm {
        // Question on one line and options on the other, so long paths don't
        // push the options off the screen.
        let mut options = menu_words("", confirm.options);
        options.spans.extend(
            menu_line("", &[("Esc", "cancel")])
                .spans
                .into_iter()
                .skip(1),
        );
        vec![Line::styled(confirm.question.clone(), bold), options]
    } else if let Some(op) = &app.op {
        op_lines(op)
    } else if let Some(job) = &app.job {
        vec![
            Line::from(vec![
                Span::styled("Logging... ", bold),
                Span::raw(format!("{} dirs", fmt_num(job.log.dirs))),
            ]),
            menu_line("", &[("Esc", "cancel")]),
        ]
    } else {
        let (first, mut second) = idle_menu(app);
        if app.options.git && app.menu_mode() == MenuMode::Normal && app.archive.is_none() {
            second.spans.push(key("Ctrl+G"));
            second.spans.push(Span::raw(" git   "));
        }
        let first = match &app.message {
            Some(msg) => Line::from(Span::styled(
                msg.clone(),
                Style::new()
                    .fg(theme().message)
                    .add_modifier(Modifier::BOLD),
            )),
            None => first,
        };
        vec![first, second]
    };
    frame.render_widget(Paragraph::new(lines), area);
}

/// Footer menu when nothing is in progress: the normal one or the Ctrl/Alt one,
/// depending on the key held down (or chosen with F9).
fn idle_menu(app: &App) -> (Line<'static>, Line<'static>) {
    let tree = app.focus == Focus::Tree;
    // With F9, the next letter gets the modifier: tell how to back out.
    let sticky = app.held.is_none() && app.sticky != MenuMode::Normal;
    let hint: (&str, &str) = if sticky {
        ("Esc", "cancel")
    } else {
        ("F9", "next menu")
    };

    match app.menu_mode() {
        MenuMode::Git => (
            menu_words("GIT", crate::git::MENU),
            menu_line(
                "",
                &[
                    (
                        "",
                        if tree {
                            "on this directory (or the tagged files)"
                        } else {
                            "on the tagged files (or this one)"
                        },
                    ),
                    ("Esc", "cancel"),
                ],
            ),
        ),
        MenuMode::Normal if tree && app.archive.is_some() => (
            menu_words(
                "ARCHIVE",
                &[
                    "Branch", "Compare", "Filespec", "Invert", "Showall", "Tag", "Untag", "Yank",
                    "Quit",
                ],
            ),
            menu_line(
                "",
                &[
                    ("Esc", "close archive"),
                    ("←→", "close/open"),
                    ("Enter", "files (Copy extracts)"),
                    ("Alt+X", "terminal"),
                    ("F8", "split"),
                    ("F9", "Ctrl/Alt menus"),
                ],
            ),
        ),
        MenuMode::Normal if tree => (
            menu_words(
                "DIR",
                &[
                    "Branch", "Compare", "Delete", "Filespec", "Invert", "Log", "Makedir",
                    "Oops", "Rename", "Showall", "Tag", "Untag", "Yank", "eXecute", "Quit",
                ],
            ),
            menu_line(
                "",
                &[
                    ("←→", "close/open"),
                    ("*", "log branch"),
                    ("F3", "relog"),
                    ("Enter", "files"),
                    ("Alt+X", "terminal"),
                    ("F8", "split"),
                    ("F9", "menus"),
                ],
            ),
        ),
        MenuMode::Normal => (
            menu_words(
                "FILE",
                &[
                    "Attributes",
                    "Branch",
                    "Copy",
                    "Delete",
                    "Filespec",
                    "Invert",
                    "Move",
                    "Open",
                    "Rename",
                    "Showall",
                    "Tag",
                    "Untag",
                    "View",
                    "Edit",
                    "Yank",
                    "eXecute",
                    "Quit",
                ],
            ),
            menu_line(
                "",
                &[
                    ("F3", "relog"),
                    ("Esc", "tree"),
                    ("Alt+X", "terminal"),
                    ("F8", "split"),
                    ("F9", "Ctrl/Alt menus"),
                ],
            ),
        ),
        MenuMode::Ctrl => (
            menu_words(
                "CTRL",
                if tree {
                    &[
                        "Attributes",
                        "Copy",
                        "Delete",
                        "Move",
                        "Tag",
                        "Untag",
                        "Yank",
                    ]
                } else {
                    &[
                        "Attributes",
                        "Copy",
                        "Delete",
                        "Move",
                        "Open",
                        "Tag",
                        "Untag",
                        "Yank",
                    ]
                },
            ),
            menu_line(
                "",
                &[
                    (
                        "",
                        if tree {
                            "on tagged files here · Tag/Untag: all logged · Yank: paths to clipboard"
                        } else {
                            "on tagged files in list · Copy/Move: no paths · Yank: paths to clipboard"
                        },
                    ),
                    hint,
                ],
            ),
        ),
        MenuMode::Alt => (
            menu_words(
                "ALT",
                if tree {
                    &[
                        "Attributes",
                        "Compare branch",
                        "Delete",
                        "Info",
                        "Kbyte",
                        "Prune",
                        "Sort",
                        "eXecute",
                    ]
                } else {
                    &[
                        "Attributes",
                        "Copy",
                        "Delete",
                        "File display",
                        "Info",
                        "Move",
                        "Open terminal",
                        "Sort",
                        "eXecute",
                    ]
                },
            ),
            menu_line(
                "",
                &[
                    if tree {
                        (
                            "Alt+X",
                            "terminal (Alt+Z full) · Attributes/Delete: tagged files",
                        )
                    } else {
                        (
                            "Alt+X",
                            "terminal · F5: archive · Alt+O: tagged · Copy/Move: with paths",
                        )
                    },
                    hint,
                ],
            ),
        ),
    }
}

/// Footer prompt: the label, the typed text (with the cursor) and the keys.
fn prompt_lines(frame: &mut Frame, prompt: &app::Prompt, area: Rect) -> Vec<Line<'static>> {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let col = area.x + (prompt.label.chars().count() + prompt.input.cursor) as u16;
    frame.set_cursor_position((col.min(area.right().saturating_sub(1)), area.y));
    let mut help = vec![("Enter", "accept"), ("Esc", "cancel"), ("Ctrl+U", "clear")];
    if prompt.picks_folder() {
        help.push(("F2", "tree"));
    }
    if prompt.toggles_paths() {
        help.push(("F6", "with/without paths"));
        help.push(("F7", "paths from"));
    }
    if prompt.is_search() {
        help.push(("F2", "match case"));
        help.push(("F4", "text/hex"));
        help.push(("↑↓", "history"));
    }
    if prompt.label.starts_with("FILESPEC") {
        help.push(("e.g.", "*.rs -*.bak >TODAY-7 >s1M"));
    }
    vec![
        Line::from(vec![
            Span::styled(prompt.label.clone(), bold),
            Span::raw(prompt.input.text.clone()),
        ]),
        menu_line("", &help),
    ]
}

/// Footer during Copy/Move/Delete: current file and progress bar,
/// or the conflict question.
fn op_lines(op: &Operation) -> Vec<Line<'static>> {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let name = |p: &std::path::Path| {
        p.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    };

    let first = match &op.conflict {
        Some(dst) => Line::from(vec![
            Span::styled(format!("{} already exists. Overwrite? ", name(dst)), bold),
            key("Y"),
            Span::raw("es  "),
            key("N"),
            Span::raw("o  "),
            key("A"),
            Span::raw("ll  "),
            key("S"),
            Span::raw("kip all  "),
            key("Esc"),
            Span::raw(" cancel"),
        ]),
        None => Line::from(vec![
            Span::styled(
                format!(
                    "{} {} of {}: ",
                    if op.to_trash { "Trashing" } else { op.kind.verb() },
                    (op.done_files + op.skipped + op.errors.len() + 1).min(op.total_files()),
                    op.total_files()
                ),
                bold,
            ),
            Span::raw(op.current().map(name).unwrap_or_default()),
        ]),
    };

    const BAR: usize = 30;
    let fraction = if op.total_bytes == 0 {
        0.0
    } else {
        op.done_bytes as f64 / op.total_bytes as f64
    };
    let filled = ((fraction * BAR as f64) as usize).min(BAR);
    let second = Line::from(vec![
        Span::raw(format!("{:9}", "")),
        Span::styled("█".repeat(filled), Style::new().fg(theme().key)),
        Span::styled("░".repeat(BAR - filled), Style::new().fg(theme().dim)),
        Span::raw(format!(
            " {:>3}%  {} of {} bytes   ",
            (fraction * 100.0) as u32,
            fmt_num(op.done_bytes),
            fmt_num(op.total_bytes)
        )),
        key("Esc"),
        Span::raw(" cancel"),
    ]);
    vec![first, second]
}

/// XTree-style menu line: the key of each word is the first uppercase
/// letter ("Copy" → C, "eXecute" → X).
fn menu_words(title: &str, words: &[&str]) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{title:<9}"),
        Style::new().add_modifier(Modifier::BOLD),
    )];
    for word in words {
        let (at, c) = word
            .char_indices()
            .find(|(_, c)| c.is_uppercase())
            .unwrap_or((0, word.chars().next().unwrap_or(' ')));
        let after = at + c.len_utf8();
        spans.push(Span::raw(word[..at].to_string()));
        spans.push(key(&word[at..after]));
        spans.push(Span::raw(format!("{} ", &word[after..])));
    }
    Line::from(spans)
}

/// Configuration screen: the page's options, with letter, description and
/// value; with F4, the theme list on top.
fn draw_settings(frame: &mut Frame, s: &Settings) {
    let [header, body, menu] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .areas(frame.area());
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let (title, items) = settings::PAGES[s.page];

    frame.render_widget(
        Line::from(vec![
            Span::styled("xt configuration", bold),
            Span::raw(format!(
                "   page {} of {}: {title}",
                s.page + 1,
                settings::PAGES.len()
            )),
            Span::styled(
                if s.dirty() { "   (modified)" } else { "" },
                Style::new().fg(theme().message),
            ),
        ]),
        header,
    );

    let block = panel(format!(" {title} "), true);
    let inner = block.inner(body);
    frame.render_widget(block, body);
    let width = inner.width as usize;
    let mut lines: Vec<Line> = items
        .iter()
        .enumerate()
        .map(|(i, (letter, label, opt))| {
            let value = opt.value(&s.config);
            let fill = width.saturating_sub(label.chars().count() + value.chars().count() + 8);
            let line = Line::from(vec![
                Span::raw("  "),
                key(&letter.to_string()),
                Span::raw(format!("  {label} {} ", ".".repeat(fill))),
                Span::styled(value, bold),
            ]);
            if i == s.selected {
                line.style(highlight(true))
            } else {
                line
            }
        })
        .collect();
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::raw("  Theme: "),
        Span::styled(s.config.theme.clone(), bold),
        Span::styled("  (F4 to choose)", Style::new().fg(theme().dim)),
    ]));
    frame.render_widget(Paragraph::new(lines), inner);

    if let Some(picker) = &s.picker {
        let height = (config::THEMES.len() + 2) as u16;
        let area = Rect {
            x: inner.x + 4,
            y: inner.y + 1,
            width: 30.min(inner.width.saturating_sub(4)),
            height: height.min(inner.height),
        }
        .intersection(inner);
        let items: Vec<ListItem> = config::THEMES
            .iter()
            .map(|(name, _)| ListItem::new(format!(" {name}")))
            .collect();
        let mut state = ListState::default().with_selected(Some(picker.index));
        frame.render_widget(Clear, area);
        frame.render_stateful_widget(
            List::new(items)
                .block(panel(" Theme ", true))
                .style(Style::new().fg(theme().text).bg(theme().background))
                .highlight_style(highlight(true)),
            area,
            &mut state,
        );
    }

    let lines = if let Some((opt, input)) = &s.editing {
        let label = match opt {
            settings::Opt::Opener => "Open with: ",
            _ => "Editor: ",
        };
        let col = menu.x + (label.len() + input.cursor) as u16;
        frame.set_cursor_position((col.min(menu.right().saturating_sub(1)), menu.y));
        vec![
            Line::from(vec![
                Span::styled(label, bold),
                Span::raw(input.text.clone()),
            ]),
            menu_line(
                "",
                &[
                    ("Enter", "accept"),
                    ("Esc", "cancel"),
                    ("", "empty = default"),
                ],
            ),
        ]
    } else if s.picker.is_some() {
        vec![
            menu_line(
                "THEME",
                &[("↑↓", "preview"), ("Enter", "choose"), ("Esc", "cancel")],
            ),
            Line::default(),
        ]
    } else {
        let first = match &s.message {
            Some(m) => Line::styled(
                m.clone(),
                Style::new()
                    .fg(theme().message)
                    .add_modifier(Modifier::BOLD),
            ),
            None => menu_line(
                "CONFIG",
                &[
                    ("A-Z", "change"),
                    ("↑↓ Enter", "select/change"),
                    ("PgUp/PgDn 1-4", "page"),
                    ("F4", "theme"),
                ],
            ),
        };
        vec![first, menu_line("", &[("F3", "save"), ("Esc", "cancel")])]
    };
    frame.render_widget(Paragraph::new(lines), menu);
}

fn draw_viewer(frame: &mut Frame, app: &mut App) {
    let tagged = app
        .tagged_view
        .as_ref()
        .map(|tv| (tv.index + 1, tv.items.len()));
    // D: diff of a changed file (Git on), or back from the diff.
    let diff = app
        .viewer
        .as_ref()
        .is_some_and(|v| v.behind.is_some() || (v.on_disk && app.can_diff(&v.path)));
    let Some(v) = &mut app.viewer else { return };
    let [header, body, menu] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    let mut title = vec![
        Span::styled("View: ", Style::new().fg(theme().key)),
        Span::raw(v.path.to_string_lossy().into_owned()),
        Span::styled(
            format!("  {} bytes", fmt_num(v.size)),
            Style::new().fg(theme().dim),
        ),
    ];
    if v.truncated() {
        title.push(Span::styled(
            format!("  (showing first {})", fmt_num(viewer::MAX_BYTES)),
            Style::new().fg(theme().error),
        ));
    }
    if let Some((i, n)) = tagged {
        title.push(Span::styled(
            format!("  [tagged {i} of {n}]"),
            Style::new().fg(theme().key),
        ));
    }
    frame.render_widget(Line::from(title), header);

    let mode = match v.mode {
        ViewMode::Text => " TEXT ",
        ViewMode::Hex => " HEX ",
    };
    draw_viewer_body(frame, v, mode.to_string(), body);

    let total = v.line_count().max(1);
    let position = match v.mode {
        ViewMode::Text => format!(
            "Line {} of {}",
            fmt_num(v.top as u64 + 1),
            fmt_num(total as u64)
        ),
        ViewMode::Hex => format!("Offset {:08X}", v.top * viewer::HEX_WIDTH),
    };
    let percent = ((v.top + v.page).min(total) * 100) / total;
    let note = v.note.clone();
    let lines = if let Some(prompt) = &app.prompt {
        prompt_lines(frame, prompt, menu)
    } else {
        let mut words = vec!["Find", "Backward", "Hex", "Colors", "Edit"];
        if diff {
            words.push("Diff");
        }
        if tagged.is_some() {
            words.extend(["Next", "Prev", "Untag"]);
        }
        let mut first = menu_words("VIEW", &words);
        first.spans.extend(
            menu_line(
                "",
                &[
                    ("/ \\", "find here"),
                    ("Space", "next hit"),
                    ("Esc", "close"),
                ],
            )
            .spans
            .into_iter()
            .skip(1),
        );
        let second = match note {
            Some(n) => Line::styled(
                format!("{:9}{n}", ""),
                Style::new()
                    .fg(theme().message)
                    .add_modifier(Modifier::BOLD),
            ),
            None => {
                let language = v
                    .language()
                    .map(|l| format!("{l}  ·  "))
                    .unwrap_or_default();
                Line::from(format!("{:9}{language}{position}  ({percent}%)", ""))
            }
        };
        vec![first, second]
    };
    frame.render_widget(Paragraph::new(lines), menu);
}

/// The viewer contents in a frame (also used by Autoview).
fn draw_terminal(frame: &mut Frame, term: &mut Terminal, colors: TerminalTheme, area: Rect) {
    let block = panel(" Terminal ", true);
    let inner = block.inner(area);
    term.resize(inner.height, inner.width);
    // Starts from the terminal's own colors, so that `apply_theme` sees the
    // cells the shell has not painted as default ones.
    frame.render_widget(Clear, inner);
    let parser = term.parser();
    let screen = parser.screen();
    let mut cursor = tui_term::widget::Cursor::default();
    cursor.hide();
    frame.render_widget(
        tui_term::widget::PseudoTerminal::new(screen)
            .block(block)
            .cursor(cursor),
        area,
    );
    let t = theme();
    term::apply_theme(frame.buffer_mut(), inner, colors, t.text, t.background);
    // The real cursor, where the shell left it (not while looking back).
    if !screen.hide_cursor() && screen.scrollback() == 0 {
        let (row, col) = screen.cursor_position();
        frame.set_cursor_position((inner.x + col, inner.y + row));
    }
}

fn draw_viewer_body(frame: &mut Frame, v: &mut Viewer, title: String, area: Rect) {
    let inner = panel("", false).inner(area);
    v.page = inner.height as usize;
    v.width = inner.width as usize;
    let mark = highlight(true).add_modifier(Modifier::BOLD);
    let lines: Vec<Line> = v
        .styled(inner.height as usize)
        .into_iter()
        .map(|segments| {
            Line::from(
                segments
                    .into_iter()
                    .map(|(text, color, marked)| match (marked, color) {
                        (true, _) => Span::styled(text, mark),
                        (false, Some(c)) => Span::styled(text, Style::new().fg(c)),
                        (false, None) => Span::raw(text),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).block(panel(title, true)), area);
}

fn key(k: &str) -> Span<'static> {
    Span::styled(
        k.to_string(),
        Style::new().fg(theme().key).add_modifier(Modifier::BOLD),
    )
}

fn menu_line(title: &str, entries: &[(&str, &str)]) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{title:<9}"),
        Style::new().add_modifier(Modifier::BOLD),
    )];
    for (k, desc) in entries {
        spans.push(key(k));
        spans.push(Span::raw(format!(" {desc}   ")));
    }
    Line::from(spans)
}

/// Cuts or pads with spaces to fit `width` columns.
fn fit(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len <= width {
        format!("{s:<width$}")
    } else {
        let mut out: String = s.chars().take(width.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Like `fit`, but keeps the end of the text: "…/projetos/xt".
fn fit_tail(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len <= width {
        return s.to_string();
    }
    let tail: String = s.chars().skip(len + 1 - width.max(1)).collect();
    format!("…{tail}")
}

/// Number with the configured thousands separator.
fn fmt_num(n: u64) -> String {
    group_digits(n, formats().number.separator())
}

/// 1234567 -> "1,234,567" (with `,`).
fn group_digits(n: u64, separator: Option<char>) -> String {
    let digits = n.to_string();
    let Some(sep) = separator else {
        return digits;
    };
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(c);
    }
    out
}

fn fmt_time(t: SystemTime) -> String {
    DateTime::<Local>::from(t)
        .format(formats().date.pattern())
        .to_string()
}

/// Permissions in `ls -l` format, e.g.: "-rwxr-xr-x" (on Windows, the
/// attributes: "-r--a").
fn fmt_mode(mode: u32, kind: FileKind) -> String {
    let mut s = String::with_capacity(10);
    s.push(match kind {
        FileKind::File => '-',
        FileKind::Symlink => 'l',
        FileKind::Other => '?',
    });
    s.push_str(&crate::platform::format_mode(mode));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers() {
        let comma = Some(',');
        assert_eq!(group_digits(0, comma), "0");
        assert_eq!(group_digits(999, comma), "999");
        assert_eq!(group_digits(1000, comma), "1,000");
        assert_eq!(group_digits(1234567, comma), "1,234,567");
        assert_eq!(group_digits(1234567, Some('.')), "1.234.567");
        assert_eq!(group_digits(1234567, None), "1234567");
    }

    #[cfg(unix)]
    #[test]
    fn formats_modes() {
        assert_eq!(fmt_mode(0o755, FileKind::File), "-rwxr-xr-x");
        assert_eq!(fmt_mode(0o640, FileKind::Symlink), "lrw-r-----");
    }

    #[test]
    fn fits_names() {
        assert_eq!(fit("abc", 5), "abc  ");
        assert_eq!(fit("abcdef", 4), "abc…");
    }

    #[test]
    fn fits_tails() {
        assert_eq!(fit_tail("/a/b", 10), "/a/b");
        assert_eq!(fit_tail("/home/andre/xt", 6), "…re/xt");
    }

    #[test]
    fn window_follows_selection() {
        let mut state = ListState::default().with_selected(Some(0));
        let (range, local) = visible_window(&mut state, 100, 10);
        assert_eq!((range, local.selected()), (0..10, Some(0)));

        state.select(Some(15));
        let (range, local) = visible_window(&mut state, 100, 10);
        assert_eq!((range, local.selected()), (6..16, Some(9)));

        // Moving up inside the window, it doesn't move.
        state.select(Some(8));
        assert_eq!(visible_window(&mut state, 100, 10).0, 6..16);

        // The list shrank: the window moves back.
        state.select(Some(2));
        assert_eq!(visible_window(&mut state, 5, 10).0, 0..5);
        assert_eq!(visible_window(&mut state, 0, 10).0, 0..0);
    }

    #[test]
    fn menu_highlights_first_uppercase_letter() {
        let line = menu_words("", &["Copy", "eXecute"]);
        let keys: Vec<&str> = line
            .spans
            .iter()
            // The key letters are the bold parts (the theme may
            // be getting switched by another test at the same time).
            .filter(|s| {
                s.style.add_modifier.contains(Modifier::BOLD) && !s.content.trim().is_empty()
            })
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(keys, ["C", "X"]);
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text.trim(), "Copy eXecute");
    }

    #[test]
    fn footer_shows_menu_for_mode() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        let mut app = App::new(tmp.path().to_path_buf());
        let footer = |app: &mut App| {
            let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
            term.draw(|f| draw(f, app)).unwrap();
            let buf = term.backend().buffer();
            (0..100).map(|x| buf[(x, 18)].symbol()).collect::<String>()
        };
        assert!(footer(&mut app).starts_with("DIR "));
        app.held = Some(MenuMode::Ctrl);
        assert!(footer(&mut app).starts_with("CTRL "));
        app.held = None;
        app.sticky = MenuMode::Alt;
        assert!(footer(&mut app).starts_with("ALT "));
    }

    #[test]
    fn renders_files_in_columns() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        for name in ["aa", "bb", "cc", "dd", "ee"] {
            std::fs::write(tmp.path().join(name), "").unwrap();
        }
        let mut app = App::new(tmp.path().to_path_buf());
        app.apply(crate::input::Command::SwitchWindow);
        app.display = FileDisplay::Names;
        let mut term = Terminal::new(TestBackend::new(60, 12)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        let row: String = (0..60).map(|x| buf[(x, 2)].symbol()).collect();
        assert!(
            row.contains("aa") && row.contains("bb") && row.contains("cc"),
            "{row}"
        );
        assert!(app.file_cols > 1);
    }

    #[test]
    fn renders_split_panes() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        std::fs::create_dir(tmp.path().join("left-dir")).unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.apply(crate::input::Command::Split);
        for (w, h) in [(120, 30), (30, 10)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            term.draw(|f| draw(f, &mut app)).unwrap();
        }
        let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let screen = format!("{:?}", term.backend().buffer());
        assert_eq!(screen.matches("left-dir").count(), 2, "one per pane");
        assert!(
            !screen.contains("DISK Statistics"),
            "no stats panel when split"
        );
    }

    #[test]
    fn renders_without_panicking() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("file.txt"), "hi").unwrap();

        let mut app = App::new(tmp.path().to_path_buf());
        for (w, h) in [(100, 30), (40, 12), (10, 5)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            term.draw(|f| draw(f, &mut app)).unwrap();
        }
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let screen = format!("{:?}", term.backend().buffer());
        assert!(screen.contains("sub"));
        assert!(screen.contains("file.txt"));
    }
}
