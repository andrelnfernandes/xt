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
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph};

use crate::app::{self, App, BookmarksView, FileView, Focus, KbyteShow, MenuMode, Pane};
use crate::charset::Charset;
use crate::config::{self, IconStyle, TerminalTheme, formats, theme};
use crate::filetype;
use crate::git::StatusCache;
use crate::help::{self, Help};
use crate::icons::{DirKind, SLOT_WIDTH};
use crate::input::Command;
use crate::model::tree::{FileEntry, FileKind, NodeId};
use crate::ops::{Conflict, Facts, Operation};
use crate::pane::{Count, FileDisplay, FileRef};
use crate::platform;
use crate::settings::{self, Settings};
use crate::term::{self, Terminal};
use crate::viewer::{self, ViewMode, Viewer};

const STATS_WIDTH: u16 = 28;
const SMALL_FILE_WINDOW: u16 = 8;
/// Below this height, the tree is shown without the small file window.
const MIN_TREE_WITH_FILES: u16 = 2 * SMALL_FILE_WINDOW;

pub fn draw(frame: &mut Frame, app: &mut App) {
    draw_screen(frame, app);
    // Last step, after every early return: no bidi or control character
    // may reach the terminal, whatever drew it.
    crate::bidi::sanitize(frame.buffer_mut(), Style::new().fg(theme().error));
}

fn draw_screen(frame: &mut Frame, app: &mut App) {
    let base = Style::new().fg(theme().text).bg(theme().background);
    frame.render_widget(Block::new().style(base), frame.area());

    if let Some(help) = &mut app.help {
        draw_help(frame, help);
        return;
    }
    if app.editor_pty.is_some() {
        draw_editor(frame, app);
        return;
    }
    if app.viewer.is_some() {
        draw_viewer(frame, app);
        return;
    }
    if let Some(settings) = &app.settings {
        draw_settings(frame, settings);
        return;
    }
    if let Some(view) = &app.bookmarks_view {
        draw_bookmarks(frame, app, view);
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
        draw_files(
            frame,
            &mut app.panes[active],
            true,
            list,
            git,
            app.options.icons,
        );
        if let Some(v) = &mut app.autoview {
            let name = v
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let title = match &v.note {
                Some(note) => format!(" Autoview: {name} — {note} "),
                None => format!(" Autoview: {name} "),
            };
            draw_viewer_body(frame, v, title, view);
        }
    } else if app.is_split() {
        // Two windows side by side; the statistics panel goes away to make room.
        let halves: [Rect; 2] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(main);
        let active = app.active;
        for (i, (pane, area)) in app.panes.iter_mut().zip(halves).enumerate() {
            draw_pane(frame, pane, i == active, true, area, git, app.options.icons);
        }
    } else {
        let [left, stats] =
            Layout::horizontal([Constraint::Min(20), Constraint::Length(STATS_WIDTH)]).areas(main);
        // The pane first: the Usage view leaves its selected row for the panel.
        draw_pane(
            frame,
            &mut app.panes[0],
            true,
            false,
            left,
            git,
            app.options.icons,
        );
        draw_stats(frame, app, stats);
    }
}

/// Width of the bars of the Usage view (less on a narrow screen).
const USAGE_BAR: usize = 16;

/// Alt+W: the folders and files of a directory by the space they take, and
/// a line with the totals at the bottom.
fn draw_usage(frame: &mut Frame, pane: &mut Pane, active: bool, area: Rect) {
    use crate::usage::{self, Item};
    let Some(view) = pane.usage.as_mut() else {
        return;
    };
    let tree = &pane.tree;
    let path = tree.path(view.dir);
    let width = area.width.saturating_sub(2) as usize;
    let rows = view.rows(tree);
    let counter = position(view.list.selected(), rows.len());
    // Room for " Usage: ", the counter and a bit of border between them.
    let room = width.saturating_sub(12 + counter.width());
    let title = format!(" Usage: {} ", fit_tail(&path.to_string_lossy(), room));
    let block = panel(title, active).title_top(counter);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [list_area, footer] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);
    view.page = list_area.height as usize;

    let total = view.total(tree);
    let whole = if view.apparent {
        total.apparent
    } else {
        total.disk
    };
    let largest = rows
        .iter()
        .filter(|r| r.mark != '>')
        .map(|r| r.size)
        .max()
        .unwrap_or(0);
    let bar_width = USAGE_BAR.min(width.saturating_sub(40) / 2).max(4);
    let dim = Style::new().fg(theme().dim);
    let items: Vec<ListItem> = rows
        .iter()
        .map(|r| {
            let weight = usage::weight(r.size, whole);
            let (size, bar, pct) = if r.mark == '>' {
                // Another file system: what it uses, not part of this one.
                let other = match r.item {
                    Item::Dir(id) => crate::platform::disk_space(&tree.path(id))
                        .map(|d| usage::size(d.total.saturating_sub(d.free)))
                        .unwrap_or_default(),
                    Item::File(_) => String::new(),
                };
                (other, String::new(), "-".to_string())
            } else {
                (
                    usage::size(r.size),
                    usage::bar(r.size, largest, bar_width),
                    usage::percent(r.size, whole),
                )
            };
            let name = match r.item {
                Item::Dir(_) => format!("/{}", r.name),
                Item::File(_) => format!(" {}", r.name),
            };
            let mut spans = vec![
                Span::styled(format!("{} ", r.mark), Style::new().fg(theme().key)),
                Span::raw(format!("{size:>10} ")),
            ];
            if r.mark == '>' {
                spans.push(Span::raw(" ".repeat(bar_width)));
            } else {
                spans.extend(bar_spans(bar, bar_width, weight));
            }
            spans.push(Span::styled(format!(" {pct:>6}"), dim));
            if view.counts {
                spans.push(Span::styled(format!(" {:>10}", fmt_num(r.files)), dim));
            }
            let style = if r.mark == '>' { dim } else { Style::new() };
            spans.push(Span::styled(format!("  {name}"), style));
            ListItem::new(Line::from(spans))
        })
        .collect();
    if view.list.selected().is_none_or(|i| i >= rows.len()) {
        view.list.select(Some(rows.len().saturating_sub(1)));
    }
    view.shown = view.list.selected().and_then(|i| rows.get(i).cloned());
    let list = List::new(items).highlight_style(highlight(active));
    frame.render_stateful_widget(list, list_area, &mut view.list);

    // The totals, and how much of the disk they are.
    let mut parts = vec![
        format!("{} on disk", usage::size(total.disk)),
        format!("{} apparent", usage::size(total.apparent)),
        format!("{} files", fmt_num(total.files)),
    ];
    if let Some(d) = crate::platform::disk_space(&path) {
        let used = d.total.saturating_sub(d.free);
        let at_top = crate::info::mount_of(&path).is_some_and(|m| m.point == path);
        let of_used = usage::percent(total.disk, used);
        parts.push(if at_top {
            format!("{of_used} of the disk's used space found")
        } else {
            format!("{of_used} of used")
        });
        parts.push(format!("{} of disk", usage::percent(total.disk, d.total)));
    }
    let unreadable = tree
        .subtree(view.dir)
        .into_iter()
        .filter(|&id| tree.node(id).error.is_some())
        .count();
    if unreadable > 0 {
        parts.push(format!(
            "{} folders could not be read",
            fmt_num(unreadable as u64)
        ));
    }
    if view.order != usage::Order::Size {
        parts.push(format!("sorted by {}", view.order.label()));
    }
    frame.render_widget(
        Line::from(Span::styled(fit(&parts.join(" · "), width), dim)),
        footer,
    );
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
    icons: IconStyle,
) {
    if pane.usage.is_some() {
        draw_usage(frame, pane, active, area);
        return;
    }
    if pane.focus == Focus::Files {
        draw_files(frame, pane, active, area, git, icons);
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
    } else if let Some(remote) = &pane.remote {
        format!(" Remote: {} (read-only) ", remote.address.destination())
    } else {
        " Tree ".to_string()
    };
    // Too low for both (e.g. above the terminal's panel): the tree alone,
    // rather than a tree of one or two lines.
    if area.height < MIN_TREE_WITH_FILES {
        draw_tree(frame, pane, active, title, area, icons);
        return;
    }
    let [tree, files] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(SMALL_FILE_WINDOW)]).areas(area);
    draw_tree(frame, pane, active, title, tree, icons);
    draw_files(frame, pane, false, files, git, icons);
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

/// What the directory of a tree row is, for its icon.
fn dir_kind(pane: &Pane, id: NodeId) -> DirKind {
    let root = id == crate::model::tree::ROOT;
    if root && pane.archive.is_some() {
        DirKind::Archive
    } else if root && pane.remote.is_some() {
        DirKind::Remote
    } else if pane.tree.has_git(id) {
        DirKind::Repo
    } else if pane.tree.node(id).expanded {
        DirKind::Open
    } else {
        DirKind::Closed
    }
}

fn draw_tree(
    frame: &mut Frame,
    pane: &mut Pane,
    active: bool,
    title: String,
    area: Rect,
    icons: IconStyle,
) {
    pane.tree_page = area.height.saturating_sub(2) as usize;

    let (range, mut state) = visible_window(&mut pane.tree_state, pane.rows.len(), pane.tree_page);
    // Alt+K: size, file count or share to the right of each directory.
    let kbyte = pane.kbyte;
    let percent = kbyte.show == KbyteShow::Percent;
    let totals = (kbyte.show != KbyteShow::None).then(|| {
        pane.tree
            .dir_totals(kbyte.branch || percent, kbyte.tagged_only)
    });
    // Percent: the largest subdirectory of each parent, for the bars.
    let largest = |parent: NodeId, totals: &[(u64, u64)]| {
        let tree = &pane.tree;
        tree.node(parent)
            .children
            .iter()
            .filter(|&&c| tree.show_hidden || !tree.node(c).hidden)
            .map(|&c| totals[c].1)
            .max()
            .unwrap_or(0)
    };
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
            // Only with icons on: the repository check looks through the
            // subdirectories and files of the row.
            let icon = match icons {
                IconStyle::None => None,
                _ => crate::icons::dir_icon(dir_kind(pane, row.id), icons),
            };
            let mut spans = vec![Span::styled(
                row.prefix.clone(),
                Style::new().fg(theme().dim),
            )];
            if let Some(ic) = icon {
                let ic_style = if node.error.is_some() {
                    Style::new().fg(theme().error)
                } else {
                    Style::new().fg(theme().key)
                };
                spans.push(Span::styled(ic, ic_style));
            }
            spans.push(Span::styled(name.clone(), name_style));
            if let Some(totals) = &totals {
                let (count, bytes) = totals[row.id];
                let icon_len = if icon.is_some() { SLOT_WIDTH } else { 0 };
                let used = row.prefix.chars().count() + icon_len + name.chars().count();
                let mut bar = None;
                let value = match (node.scanned, kbyte.show) {
                    (false, _) => "?".to_string(),
                    (_, KbyteShow::Size) => format!("{}K", fmt_num(bytes.div_ceil(1024))),
                    (_, KbyteShow::Percent) => {
                        let (whole, top) = match node.parent {
                            Some(p) => (totals[p].1, largest(p, totals)),
                            None => (bytes, bytes),
                        };
                        let (b, pct) = share(bytes, whole, top, width.saturating_sub(used + 1));
                        bar = b;
                        pct
                    }
                    _ => fmt_num(count),
                };
                let bar_len = bar.as_ref().map_or(0, |_| SHARE_BAR + 1);
                let pad = width.saturating_sub(used + bar_len + value.chars().count() + 1);
                spans.push(Span::raw(" ".repeat(pad + 1)));
                if let Some(bar) = bar {
                    let whole = node.parent.map_or(bytes, |p| totals[p].1);
                    let weight = crate::usage::weight(bytes, whole);
                    spans.extend(bar_spans(bar, SHARE_BAR, weight));
                    spans.push(Span::raw(" "));
                }
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

/// Width of the Alt+K Percent bars.
const SHARE_BAR: usize = 8;

/// Alt+K Percent: `bytes` as a share of `whole` ("41%", "5.8%", "<0.1%"), and a bar
/// of `bytes` against `largest` (the biggest sibling fills it), in eighths
/// of a character. Without `room` for both, only the share.
fn share(bytes: u64, whole: u64, largest: u64, room: usize) -> (Option<String>, String) {
    let pct = if whole == 0 {
        0.0
    } else {
        bytes as f64 * 100.0 / whole as f64
    };
    let text = if pct > 0.0 && pct < 0.05 {
        "<0.1%".to_string()
    } else if pct == 0.0 || pct >= 9.95 {
        format!("{pct:.0}%")
    } else {
        format!("{pct:.1}%")
    };
    let text = format!("{text:>5}");
    if room < SHARE_BAR + 1 + text.len() {
        return (None, text);
    }
    (Some(crate::usage::bar(bytes, largest, SHARE_BAR)), text)
}

/// A bar for the Alt+K Percent and the Usage view: the filled part in the
/// color of its weight (the theme's bar_low, bar_mid, bar_high), on a track
/// that shows the whole width.
fn bar_spans(filled: String, width: usize, weight: crate::usage::Weight) -> Vec<Span<'static>> {
    use crate::usage::Weight;
    let color = match weight {
        Weight::Light => theme().bar_low,
        Weight::Medium => theme().bar_mid,
        Weight::Heavy => theme().bar_high,
    };
    let track = width.saturating_sub(filled.chars().count());
    vec![
        Span::styled(filled, Style::new().fg(color)),
        Span::styled("░".repeat(track), Style::new().fg(theme().border)),
    ]
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
    draw_tree(
        frame,
        picker.pane_mut(),
        true,
        title,
        main,
        app.options.icons,
    );

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
    icons: IconStyle,
) {
    let git = git.filter(|g| {
        pane.archive.is_none() && pane.remote.is_none() && g.in_repo(&pane.current_path())
    });
    // Status letters of a file in the repository ("  " if unchanged).
    let marker =
        |pane: &Pane, f: FileRef| git.map(|g| g.marker(&pane.file_path(f)).unwrap_or(*b"  "));
    let kind = match pane.view {
        FileView::Dir => "Files",
        FileView::Branch(_) => "Branch",
        FileView::Showall => "Showall",
    };
    let title = format!(
        " {}{kind}: {} ({}) {} ",
        crate::icons::view_icon(pane.view, icons).unwrap_or_default(),
        pane.filespec.text(),
        fmt_num(pane.files.len() as u64),
        pane.sort.label()
    );
    let error = match pane.view {
        FileView::Dir => pane.tree.node(pane.current()).error.is_some(),
        _ => false,
    };
    if pane.display.multi_column() && !pane.files.is_empty() && !error {
        draw_file_columns(frame, pane, active, area, title, &marker, icons);
        return;
    }
    pane.file_cols = 1;
    pane.file_page = area.height.saturating_sub(2) as usize;

    let width = area.width.saturating_sub(2) as usize;
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
        let visible = pane.files[range].to_vec();
        let details: Vec<Detail> = visible.iter().map(|&f| file_detail(pane, f)).collect();
        visible
            .iter()
            .zip(details)
            .map(|(&f, detail)| file_line(pane.file(f), width, detail, marker(pane, f), icons))
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
    icons: IconStyle,
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
    let icon_w = if icons == IconStyle::None {
        0
    } else {
        SLOT_WIDTH
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
    // Tag mark + git + icon + name + (size and permissions) + space between columns.
    let cell_w = 2 + git_w + icon_w + name_w + if with_size { 23 } else { 0 } + 1;
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
                let icon = crate::icons::file_icon(
                    &f.name,
                    f.kind,
                    crate::platform::is_executable(&f.name, f.mode),
                    icons,
                )
                .unwrap_or_default();
                let mut cell = format!(
                    "{}{}{}{}",
                    if f.tagged { "■ " } else { "  " },
                    git.map(|xy| git_text(&xy)).unwrap_or_default(),
                    icon,
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

/// What the one-column file window shows after each name (Alt+F).
enum Detail {
    /// Size, date and permissions.
    Standard,
    NamesOnly,
    /// "user:group" ("-" where unknown: inside archives, on Windows).
    Owner(String),
    /// What the file is ("PNG image, 800 x 600").
    Type(String),
}

/// The detail of file `f` in the current display mode. Owner and Type are
/// worked out here, for the visible files only (types are remembered).
fn file_detail(pane: &mut Pane, f: FileRef) -> Detail {
    match pane.display {
        FileDisplay::LongNames => Detail::NamesOnly,
        FileDisplay::Owner => {
            let owner = match (&pane.remote, &pane.archive) {
                (Some(remote), _) => remote.owner(&pane.file_path(f)),
                (None, Some(_)) => None,
                (None, None) => platform::owner(&pane.file_path(f)),
            };
            Detail::Owner(owner.unwrap_or_else(|| "-".into()))
        }
        FileDisplay::Type => {
            let path = pane.file_path(f);
            let e = pane.file(f);
            let (size, modified, kind) = (e.size, e.modified, e.kind);
            if let Some((s, m, what)) = pane.types.get(&path)
                && (*s, *m) == (size, modified)
            {
                return Detail::Type(what.clone());
            }
            let what = match &pane.remote {
                Some(remote) => filetype::describe_with(
                    &path,
                    kind,
                    size,
                    || remote.read(&path, filetype::HEAD_BYTES).ok(),
                    || remote.read_link(&path).ok(),
                ),
                None => filetype::describe(&path, kind, size, pane.archive.is_none()),
            };
            if pane.types.len() > 10_000 {
                pane.types.clear();
            }
            pane.types.insert(path, (size, modified, what.clone()));
            Detail::Type(what)
        }
        _ => Detail::Standard,
    }
}

fn file_line(
    f: &FileEntry,
    width: usize,
    detail: Detail,
    git: Option<[u8; 2]>,
    icons: IconStyle,
) -> ListItem<'static> {
    let size = fmt_num(f.size);
    let mode = fmt_mode(f.mode, f.kind);
    // In narrow windows (split screen), the least useful columns are dropped.
    let details = match detail {
        Detail::NamesOnly => String::new(),
        Detail::Standard => {
            let date = f.modified.map(fmt_time).unwrap_or_default();
            if width >= 72 {
                format!(" {size:>15} {date:16} {mode}")
            } else if width >= 50 {
                format!(" {size:>13} {date:16}")
            } else {
                format!(" {size:>13}")
            }
        }
        Detail::Owner(owner) => {
            let owner = fit(&owner, 17);
            if width >= 72 {
                format!(" {size:>15} {owner} {mode}")
            } else if width >= 50 {
                format!(" {size:>13} {owner}")
            } else {
                format!(" {owner}")
            }
        }
        // Up to almost half of the line, the rest for the name.
        Detail::Type(what) => format!(" {}", fit(&what, (width * 45 / 100).clamp(12, 50))),
    };

    // 2 columns for the tag mark, 3 for the Git status, then the icon.
    let git = git.map(|xy| git_text(&xy));
    let icon = crate::icons::file_icon(
        &f.name,
        f.kind,
        crate::platform::is_executable(&f.name, f.mode),
        icons,
    );
    let icon_len = if icon.is_some() { SLOT_WIDTH } else { 0 };
    let marks = 2 + git.as_ref().map_or(0, String::len) + icon_len;
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
    if let Some(ic) = icon {
        spans.push(Span::styled(ic, style));
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
        let mount = crate::info::mount_of(&path);
        match mount.as_ref().and_then(crate::rclone::of) {
            // A cloud service mounted by rclone: its name, then the remote.
            Some(remote) => {
                let service = remote.service.as_deref().unwrap_or("rclone");
                disk.push(Line::from(vec![
                    label("DISK ".into()),
                    bold(fit(service, width.saturating_sub(5))),
                ]));
                disk.push(Line::from(fit(&format!("rclone {}", remote.spec), width)));
            }
            None => {
                let point = mount
                    .map(|m| m.point.to_string_lossy().into_owned())
                    .unwrap_or_default();
                disk.push(Line::from(vec![
                    label("DISK ".into()),
                    Span::raw(fit_tail(&point, width.saturating_sub(5))),
                ]));
            }
        }
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

    // Alt+W: what the row under the cursor takes, in place of the statistics.
    if let Some(row) = app.usage.as_ref().and_then(|u| u.shown.clone()) {
        use crate::usage;
        let text = |k: &str, v: String| {
            let pad = width.saturating_sub(k.chars().count() + v.chars().count());
            Line::from(vec![Span::raw(format!("{k}{}", " ".repeat(pad))), bold(v)])
        };
        let mut lines = vec![
            Line::from(label("USAGE".into())),
            Line::from(bold(fit(&row.name, width))),
            text("On disk", usage::size(row.disk)),
            text("Apparent", usage::size(row.apparent)),
            text("Files", fmt_num(row.files)),
        ];
        if let Some(m) = row.modified {
            lines.push(text("Modified", fmt_time(m)));
        }
        if let Some(d) = crate::platform::disk_space(&path) {
            let used = d.total.saturating_sub(d.free);
            lines.push(text("Of used", usage::percent(row.disk, used)));
            lines.push(text("Of disk", usage::percent(row.disk, d.total)));
        }
        let note = match row.mark {
            '>' => "Another disk: Enter analyzes it",
            '!' => "Could not be read",
            '.' => "Something inside could not be read: it may take more",
            'H' => "Another hard link of a file already counted",
            '@' => "A link or special file, not followed",
            'e' => "Empty",
            _ => "",
        };
        if !note.is_empty() {
            lines.push(Line::from(""));
            for chunk in wrap(note, width) {
                lines.push(Line::from(Span::styled(
                    chunk,
                    Style::new().fg(theme().dim),
                )));
            }
        }
        statistics = lines;
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
        let remote = app.terminal.as_ref().is_some_and(term::Terminal::is_remote);
        let (back, full, tagged) = (
            key_for(Command::Terminal),
            key_for(Command::TerminalFull),
            key_for(Command::TerminalTagged),
        );
        let mut keys = vec![
            (back.as_str(), "back to Xylem"),
            (
                full.as_str(),
                if app.terminal_full {
                    "panel"
                } else {
                    "full screen"
                },
            ),
        ];
        if !remote {
            keys.push((tagged.as_str(), "type the tagged files"));
        }
        keys.push(("Shift+PgUp/PgDn", "scroll back"));
        let about = if remote {
            "a shell on the remote machine; the tree follows its cd if the shell reports it (OSC 7); exit ends the session"
        } else {
            "the tree follows the shell's cd; $XY_TAGGED has the tagged files; exit closes the shell"
        };
        vec![
            menu_line("TERMINAL", &keys),
            match &app.message {
                // E.g. the shell went outside the tree.
                Some(msg) => Line::from(Span::styled(
                    format!("{:9}{msg}", ""),
                    Style::new()
                        .fg(theme().message)
                        .add_modifier(Modifier::BOLD),
                )),
                None => menu_line("", &[("", about)]),
            },
        ]
    } else if app.usage.is_some() && app.job.is_none() {
        let about = "> another disk  ! unreadable  . incomplete  H hard link  @ link  e empty";
        vec![
            menu_line(
                "USAGE",
                &[
                    ("Enter", "open"),
                    ("←", "back"),
                    ("\\", "to the tree"),
                    ("A", "disk/apparent"),
                    ("C", "files"),
                    ("S", "sort"),
                    ("F3", "reread"),
                    ("Esc", "close"),
                ],
            ),
            match &app.message {
                Some(msg) => Line::from(Span::styled(
                    format!("{:9}{msg}", ""),
                    Style::new()
                        .fg(theme().message)
                        .add_modifier(Modifier::BOLD),
                )),
                None => menu_line("", &[("", about)]),
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
        if app.options.git
            && app.menu_mode() == MenuMode::Normal
            && app.archive.is_none()
            && app.remote.is_none()
        {
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
            None if app.selected_name_has_bidi() => Line::from(Span::styled(
                "Name has hidden bidirectional characters: it may not be what it looks like",
                Style::new().fg(theme().error),
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

    let help_key = help_key(app);
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
        MenuMode::Normal if app.remote.is_some() => (
            menu_words(
                "REMOTE",
                &[
                    "Branch", "Copy", "Filespec", "Invert", "Log", "Open", "Showall", "Tag",
                    "Untag", "View", "Yank", "Quit",
                ],
            ),
            menu_line(
                "",
                &[
                    (help_key.as_str(), "help"),
                    ("Enter", if tree { "files" } else { "tree" }),
                    ("F3", "reread"),
                    ("Alt+X", "terminal there"),
                    ("F8", "split with this machine"),
                    ("F9", "menus"),
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
                    (help_key.as_str(), "help"),
                    ("Esc", "close archive"),
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
                    "Branch", "Compare", "Delete", "Filespec", "Invert", "Log", "Makedir", "Oops",
                    "Rename", "Showall", "Tag", "Untag", "Yank", "eXecute", "Quit",
                ],
            ),
            menu_line(
                "",
                &[
                    (help_key.as_str(), "help"),
                    ("*", "log branch"),
                    ("F3", "relog"),
                    ("Enter", "files"),
                    ("Alt+X", "terminal"),
                    ("F8", "split"),
                    ("Alt+B", "bookmarks"),
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
                    (help_key.as_str(), "help"),
                    ("F3", "relog"),
                    ("Esc", "tree"),
                    ("Alt+X", "terminal"),
                    ("F8", "split"),
                    ("Alt+B", "bookmarks"),
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
                        "Rename",
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
                        "Rename",
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
                        "Bookmarks",
                        "Compare branch",
                        "Delete",
                        "Help",
                        "Info",
                        "Kbyte",
                        "Prune",
                        "Rename",
                        "Sort",
                        "usage (W)",
                        "eXecute",
                    ]
                } else {
                    &[
                        "Attributes",
                        "Bookmarks",
                        "Copy",
                        "Delete",
                        "File display",
                        "Help",
                        "Info",
                        "Move",
                        "Open terminal",
                        "Rename",
                        "Sort",
                        "usage (W)",
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
    let help = if prompt.is_bookmark_jump() {
        vec![
            ("0-9, a-z", "jump"),
            ("' / Enter", "list"),
            ("+", "bookmark here"),
            ("Esc", "cancel"),
        ]
    } else if prompt.is_bookmark_set() {
        vec![("0-9, a-z", "assign key"), ("Esc", "cancel")]
    } else if prompt.is_rename_tagged() {
        vec![
            ("Enter", "rename"),
            ("F2", "case"),
            ("↑↓", "history"),
            ("Esc", "cancel"),
            ("e.g.", "*.jpg  new_*.*  %03d  s/a/b/"),
        ]
    } else {
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
        if prompt.is_goto() {
            help.push(("↑↓", "history"));
        }
        if prompt.label.starts_with("FILESPEC") {
            help.push(("e.g.", "*.rs -*.bak >TODAY-7 >s1M"));
        }
        help
    };
    vec![
        Line::from(vec![
            Span::styled(prompt.label.clone(), bold),
            Span::raw(if prompt.is_secret() {
                "•".repeat(prompt.input.text.chars().count())
            } else {
                prompt.input.text.clone()
            }),
        ]),
        menu_line("", &help),
    ]
}

/// Footer during Copy/Move/Delete: current file and progress bar,
/// or the conflict question with the size and date of both files.
fn op_lines(op: &Operation) -> Vec<Line<'static>> {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let name = |p: &std::path::Path| {
        p.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    };

    let first = match &op.conflict {
        Some(c) => Line::from(vec![
            Span::styled(
                format!("{} already exists. Overwrite? ", name(&c.dst)),
                bold,
            ),
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
                    if op.to_trash {
                        "Trashing"
                    } else {
                        op.kind.verb()
                    },
                    (op.done_files + op.skipped + op.errors.len() + 1).min(op.total_files()),
                    op.total_files()
                ),
                bold,
            ),
            Span::raw(op.current().map(name).unwrap_or_default()),
        ]),
    };

    if let Some(c) = &op.conflict {
        return vec![first, conflict_facts(c)];
    }

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

/// "new: 3 bytes, 2026-10-12 13:02   existing: 16 bytes, 2026-09-30 08:15
/// (older, larger)": what is being copied against what is already there.
fn conflict_facts(c: &Conflict) -> Line<'static> {
    use std::cmp::Ordering::*;
    let facts = |f: &Facts| {
        let date = f.modified.map_or_else(|| "date unknown".into(), fmt_time);
        format!("{} bytes, {date}", fmt_num(f.size))
    };
    let when = match (c.existing.modified, c.new.modified) {
        (Some(existing), Some(new)) => match existing.cmp(&new) {
            Greater => "newer",
            Less => "older",
            Equal => "same date",
        },
        _ => "date unknown",
    };
    let size = match c.existing.size.cmp(&c.new.size) {
        Greater => "larger",
        Less => "smaller",
        Equal => "same size",
    };
    Line::from(vec![
        Span::raw(format!("{:9}", "")),
        Span::styled("new: ", Style::new().fg(theme().dim)),
        Span::raw(format!("{}   ", facts(&c.new))),
        Span::styled("existing: ", Style::new().fg(theme().dim)),
        Span::raw(format!("{} ", facts(&c.existing))),
        Span::styled(
            format!("({when}, {size})"),
            Style::new().add_modifier(Modifier::BOLD),
        ),
    ])
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
            Span::styled("Xylem configuration", bold),
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

fn draw_bookmarks(frame: &mut Frame, app: &App, view: &BookmarksView) {
    let [header, body, menu] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .areas(frame.area());
    let bold = Style::new().add_modifier(Modifier::BOLD);

    frame.render_widget(
        Line::from(vec![
            Span::styled("Xylem bookmarks", bold),
            Span::raw(format!(
                "   {} bookmark{}",
                app.bookmarks.len(),
                if app.bookmarks.len() == 1 { "" } else { "s" }
            )),
        ]),
        header,
    );

    let block = panel(" Directory Bookmarks (Alt+B / F11) ", true);
    let inner = block.inner(body);
    frame.render_widget(block, body);

    let home = platform::home_dir();
    let format_path = |p: &std::path::Path| -> String {
        if let Some(h) = &home
            && let Ok(rel) = p.strip_prefix(h)
        {
            format!("~/{}", rel.display())
        } else {
            p.display().to_string()
        }
    };

    let mut lines: Vec<Line> = Vec::new();
    if app.bookmarks.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "  No bookmarks defined yet.",
            bold,
        )));
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::raw("  Press "),
            key("Ins"),
            Span::raw(", "),
            key("+"),
            Span::raw(" or "),
            key("m"),
            Span::raw(" to bookmark the current directory:"),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format_path(&app.current_path()),
                Style::new().fg(theme().key).add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        let width = inner.width as usize;
        for (i, (k, bm)) in app.bookmarks.iter().enumerate() {
            let exists = bm.path.is_dir();
            let path_str = format_path(&bm.path);
            let label_part = match &bm.label {
                Some(lbl) => format!(" ({lbl})"),
                None => String::new(),
            };
            let cursor = if i == view.selected { "▶ " } else { "  " };
            let status = if !exists { " [not found]" } else { "" };

            let full_text = format!("{path_str}{label_part}{status}");
            let text_fitted = fit(&full_text, width.saturating_sub(10));

            let mut spans = vec![
                Span::raw(cursor),
                key(&format!("[{k}]")),
                Span::raw(" "),
                Span::raw(text_fitted),
            ];
            if !exists {
                spans.push(Span::styled(" [invalid]", Style::new().fg(theme().message)));
            }
            let line = Line::from(spans);
            if i == view.selected {
                lines.push(line.style(highlight(true)));
            } else {
                lines.push(line);
            }
        }
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("  Current directory: ", bold),
            Span::raw(format_path(&app.current_path())),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);

    frame.render_widget(
        menu_line(
            "",
            &[
                ("Enter / key", "jump"),
                ("Ins / +", "add current dir"),
                ("Del / d", "remove"),
                ("↑↓ / j k", "navigate"),
                ("Esc", "exit"),
            ],
        ),
        menu,
    );
}

/// The help key to show in the footer: F1, unless the terminal is one of
/// those that keep F1 for their own help (VTE: GNOME Terminal, Ptyxis,
/// Terminator, Tilix) and Alt+H also opens it.
fn help_key(app: &App) -> String {
    let alt_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::ALT);
    let vte = std::env::var_os("VTE_VERSION").is_some();
    if vte && app.keymap.lookup(alt_h, app.focus) == Some(Command::Help) {
        "Alt+H".into()
    } else {
        app.keymap
            .key_for(Command::Help)
            .unwrap_or_else(|| "F1".into())
    }
}

/// The help (F1 / Alt+H): a topic of the manual, with its links.
fn draw_help(frame: &mut Frame, help: &mut Help) {
    let [header, body, menu] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(frame.area());
    frame.render_widget(
        Line::from(vec![
            Span::styled("Help: ", Style::new().fg(theme().key)),
            Span::styled(help.title(), Style::new().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("  [{} · {}]", help.lang, help::language_label(&help.lang)),
                Style::new().fg(theme().dim),
            ),
        ]),
        header,
    );

    let block = panel("", true);
    let inner = block.inner(body);
    frame.render_widget(block, body);
    help.height = inner.height as usize;
    let selected = help.selected;
    let scroll = help.scroll;
    let page = help.page(inner.width as usize);
    let total = page.lines.len();
    let style = |kind: help::Kind| match kind {
        help::Kind::Text => Style::new(),
        help::Kind::Heading => Style::new().fg(theme().key).add_modifier(Modifier::BOLD),
        help::Kind::Strong => Style::new().add_modifier(Modifier::BOLD),
        help::Kind::Code => Style::new().fg(theme().key),
        help::Kind::Link(id) if Some(id) == selected => highlight(true),
        help::Kind::Link(_) => Style::new().add_modifier(Modifier::UNDERLINED),
    };
    let lines: Vec<Line> = page
        .lines
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|pieces| {
            Line::from(
                pieces
                    .iter()
                    .map(|p| Span::styled(p.text.clone(), style(p.kind)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);

    let percent = ((scroll + inner.height as usize).min(total.max(1)) * 100) / total.max(1);
    let keys = |pairs: &[(&str, &str)]| {
        let mut spans = vec![Span::raw(format!("{:9}", ""))];
        for (k, what) in pairs {
            spans.push(key(k));
            spans.push(Span::raw(format!(" {what}   ")));
        }
        Line::from(spans)
    };
    frame.render_widget(
        Paragraph::new(vec![
            {
                let mut first = keys(&[
                    ("Tab", "link"),
                    ("Enter", "follow"),
                    ("Backspace", "back"),
                    ("I", "index"),
                    ("K", "keys"),
                    ("L", "lang"),
                    ("Esc", "close"),
                ]);
                first.spans[0] = Span::styled(
                    format!("{:<9}", "HELP"),
                    Style::new().add_modifier(Modifier::BOLD),
                );
                first
            },
            Line::from(Span::styled(
                format!("{:9}{percent}%", ""),
                Style::new().fg(theme().dim),
            )),
        ]),
        menu,
    );
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
    if let Some((kind, _)) = v.compression {
        title.push(Span::styled(
            format!("  {} → {} bytes", kind.name(), fmt_num(v.data_len())),
            Style::new().fg(theme().dim),
        ));
    }
    if v.charset() != Charset::Utf8 {
        title.push(Span::styled(
            format!("  {}", v.charset().name()),
            Style::new().fg(theme().dim),
        ));
    }
    if v.truncated() {
        // The hex view shows every byte read: the line budget cuts only
        // the text.
        let shown = if v.lines_cut() && v.mode == ViewMode::Text {
            Some(format!("{} lines", fmt_num(v.line_count() as u64)))
        } else if !v.bytes_cut() {
            None
        } else if v.compression.is_some() {
            Some(format!("{} bytes decompressed", fmt_num(v.data_len())))
        } else {
            Some(format!("{} MiB", v.limit() >> 20))
        };
        if let Some(shown) = shown {
            title.push(Span::styled(
                format!("  (showing first {shown})"),
                Style::new().fg(theme().error),
            ));
        }
    }
    if let Some(problem) = v.problem() {
        title.push(Span::styled(
            format!("  ({problem})"),
            Style::new().fg(theme().error),
        ));
    }
    if v.bidi_count() > 0 {
        title.push(Span::styled(
            format!(
                "  {} bidirectional control character{} (shown as ·)",
                v.bidi_count(),
                if v.bidi_count() == 1 { "" } else { "s" }
            ),
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
        let mut words = vec![
            "Find", "Backward", "Hex", "Colors", "Wrap", "#Num", "Goto", "Edit", "cOdepage",
        ];
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
                let wrap_badge = if v.wrap { "[Wrap] " } else { "" };
                let num_badge = if v.line_numbers { "[#] " } else { "" };
                Line::from(format!(
                    "{:9}{wrap_badge}{num_badge}{language}{position}  ({percent}%)",
                    ""
                ))
            }
        };
        vec![first, second]
    };
    frame.render_widget(Paragraph::new(lines), menu);
}

fn draw_editor(frame: &mut Frame, app: &mut App) {
    let colors = if app.options.editor_theme {
        TerminalTheme::Xylem
    } else {
        TerminalTheme::Native
    };
    let Some(ed) = &mut app.editor_pty else {
        return;
    };
    let [header, body, menu] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    let title = vec![
        Span::styled("Edit: ", Style::new().fg(theme().key)),
        Span::raw(ed.path.to_string_lossy().into_owned()),
    ];
    frame.render_widget(Line::from(title), header);

    draw_terminal_panel(frame, &mut ed.term, colors, " EDIT ", body);

    let first = Line::from(vec![
        Span::styled(
            format!("{:<9}", "EDIT"),
            Style::new().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Quit the editor to return to Xylem",
            Style::new().fg(theme().dim),
        ),
    ]);
    frame.render_widget(Paragraph::new(vec![first]), menu);
}

fn draw_terminal(frame: &mut Frame, term: &mut Terminal, colors: TerminalTheme, area: Rect) {
    draw_terminal_panel(frame, term, colors, " Terminal ", area);
}

fn draw_terminal_panel(
    frame: &mut Frame,
    term: &mut Terminal,
    colors: TerminalTheme,
    title: &'static str,
    area: Rect,
) {
    let block = panel(title, true);
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
    let digits = v.line_count().to_string().len().max(1);
    let show_numbers = v.line_numbers && v.mode == ViewMode::Text;
    let t = theme();
    let lines: Vec<Line> = v
        .styled(inner.height as usize)
        .into_iter()
        .map(|row| {
            let mut spans = Vec::new();
            if show_numbers {
                match row.line_num {
                    Some(num) => {
                        spans.push(Span::styled(
                            format!("{num:>digits$} "),
                            Style::new().fg(t.dim),
                        ));
                    }
                    None => {
                        spans.push(Span::styled(
                            format!("{:>digits$} ", ""),
                            Style::new().fg(t.dim),
                        ));
                    }
                }
                spans.push(Span::styled("│ ", Style::new().fg(t.border)));
            }
            for (text, color, marked) in row.segments {
                spans.push(match (marked, color) {
                    (true, _) => Span::styled(text, mark),
                    (false, Some(c)) => Span::styled(text, Style::new().fg(c)),
                    (false, None) => Span::raw(text),
                });
            }
            Line::from(spans)
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

/// Splits `text` into lines of at most `width` columns, at spaces.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
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
        // Alt+W has no letter of its own in a word: it says so.
        assert!(footer(&mut app).contains("usage (W)"));
    }

    fn draw_row0(app: &mut App) -> String {
        use ratatui::{Terminal, backend::TestBackend};
        let mut term = Terminal::new(TestBackend::new(120, 20)).unwrap();
        term.draw(|f| draw(f, app)).unwrap();
        let buf = term.backend().buffer();
        (0..120).map(|x| buf[(x, 0)].symbol()).collect()
    }

    fn draw_text(app: &mut App) -> String {
        use ratatui::{Terminal, backend::TestBackend};
        let mut term = Terminal::new(TestBackend::new(140, 20)).unwrap();
        term.draw(|f| draw(f, app)).unwrap();
        let buf = term.backend().buffer();
        (0..20)
            .flat_map(|y| (0..140).map(move |x| (x, y)))
            .map(|(x, y)| buf[(x, y)].symbol())
            .collect()
    }

    #[test]
    fn viewer_header_tells_what_was_cut() {
        use crate::viewer::{LoadOptions, Viewer};
        let tmp = crate::testing::tempdir();
        let mut app = App::new(tmp.path().to_path_buf());
        let load = LoadOptions::view(1 << 20);
        // Lines long enough for the line budget: these are cut by bytes.
        let text = format!("{}\n", "0123456789abcdef".repeat(4)).repeat(1 << 15);
        let raw = text.as_bytes()[..1 << 20].to_vec();
        app.viewer = Some(Viewer::from_file(
            "x.txt".into(),
            text.len() as u64,
            raw,
            load,
        ));
        let row0 = draw_row0(&mut app);
        assert!(row0.contains("(showing first 1 MiB)"), "{row0}");
        let gz = crate::decompress::test_util::gz(text.as_bytes());
        let n = gz.len() as u64;
        app.viewer = Some(Viewer::from_file("x.txt.gz".into(), n, gz, load));
        let row0 = draw_row0(&mut app);
        assert!(
            row0.contains(&format!(
                "(showing first {} bytes decompressed)",
                fmt_num(1 << 20)
            )),
            "{row0}"
        );
        assert!(row0.contains("gzip → 1"), "{row0}");
    }

    /// The note goes away at the first key; the header keeps saying that
    /// the compressed file was damaged, so it never looks complete.
    #[test]
    fn viewer_header_keeps_a_decompression_failure() {
        use crate::viewer::{LoadOptions, Viewer};
        let tmp = crate::testing::tempdir();
        let mut app = App::new(tmp.path().to_path_buf());
        let load = LoadOptions::view(1 << 20);
        let down = ratatui::crossterm::event::KeyEvent::new(
            ratatui::crossterm::event::KeyCode::Down,
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        let mut gz = crate::decompress::test_util::gz(&b"line of text\n".repeat(10_000));
        gz.truncate(gz.len() / 2);
        for (raw, shown) in [
            (gz, "(truncated or corrupt)"),
            (
                b"\x1F\x8B".to_vec(),
                "(not decompressed: truncated or corrupt)",
            ),
        ] {
            let n = raw.len() as u64;
            let mut v = Viewer::from_file("x.txt.gz".into(), n, raw, load);
            assert!(v.note.is_some());
            v.handle_key(down);
            assert_eq!(v.note, None);
            app.viewer = Some(v);
            let row0 = draw_row0(&mut app);
            assert!(row0.contains(shown), "{row0}");
        }
    }

    /// Cut by the line budget, not by the bytes read: the header counts lines.
    #[test]
    fn viewer_header_counts_the_lines_the_budget_kept() {
        use crate::viewer::{LoadOptions, Viewer};
        let tmp = crate::testing::tempdir();
        let mut app = App::new(tmp.path().to_path_buf());
        let raw = vec![b'\n'; 1 << 20];
        let v = Viewer::from_file("nl.txt".into(), 1 << 20, raw, LoadOptions::view(1 << 20));
        let lines = format!("(showing first {} lines)", fmt_num(v.line_count() as u64));
        app.viewer = Some(v);
        let row0 = draw_row0(&mut app);
        assert!(row0.contains(&lines), "{row0}");
        assert!(!row0.contains("MiB)"), "{row0}");
        // The hex view shows every byte: nothing was cut there.
        app.viewer
            .as_mut()
            .unwrap()
            .handle_key(ratatui::crossterm::event::KeyEvent::new(
                ratatui::crossterm::event::KeyCode::Char('h'),
                ratatui::crossterm::event::KeyModifiers::NONE,
            ));
        let row0 = draw_row0(&mut app);
        assert!(!row0.contains("(showing first"), "{row0}");
    }

    #[test]
    fn viewer_header_names_the_charset_before_the_cut() {
        use crate::viewer::{LoadOptions, Viewer};
        let tmp = crate::testing::tempdir();
        let mut app = App::new(tmp.path().to_path_buf());
        let text = "ação\n".repeat(1 << 18);
        let mut raw = crate::charset::test_util::encode(&text, &crate::charset::CP850);
        raw.truncate(1 << 20);
        app.viewer = Some(Viewer::from_file(
            "x.txt".into(),
            text.len() as u64,
            raw,
            LoadOptions::view(1 << 20),
        ));
        let row0 = draw_row0(&mut app);
        let at = row0.find("CP850").unwrap_or_else(|| panic!("{row0}"));
        assert!(at < row0.find("(showing first").unwrap(), "{row0}");
        let all = draw_text(&mut app);
        assert!(all.contains("cOdepage") || all.contains("Odepage"), "{all}");
    }

    #[test]
    fn autoview_title_shows_the_note() {
        let tmp = crate::testing::tempdir();
        std::fs::write(
            tmp.path().join("big.xz"),
            crate::decompress::test_util::xz_header_with_dict_props(40),
        )
        .unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.handle_key(ratatui::crossterm::event::KeyEvent::new(
            ratatui::crossterm::event::KeyCode::F(7),
            ratatui::crossterm::event::KeyModifiers::NONE,
        ));
        let text = draw_text(&mut app);
        assert!(text.contains("(V to view)"), "{text}");
    }

    #[test]
    fn viewer_header_counts_bidi_controls() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        let mut app = App::new(tmp.path().to_path_buf());
        app.viewer = Some(crate::viewer::Viewer::from_bytes(
            "x.txt".into(),
            6,
            "a\u{202E}b\u{2066}c".as_bytes().to_vec(),
        ));
        let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        let row0: String = (0..100).map(|x| buf[(x, 0)].symbol()).collect();
        assert!(
            row0.contains("2 bidirectional control characters (shown as ·)"),
            "{row0}"
        );
        app.viewer = Some(crate::viewer::Viewer::from_bytes(
            "x.txt".into(),
            3,
            "a\u{202E}b".as_bytes().to_vec(),
        ));
        term.draw(|f| draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        let row0: String = (0..100).map(|x| buf[(x, 0)].symbol()).collect();
        assert!(
            row0.contains("1 bidirectional control character (shown as ·)"),
            "{row0}"
        );
    }

    #[test]
    fn bidi_in_names_never_reaches_the_terminal() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        std::fs::write(tmp.path().join("fatura\u{202E}fdp.sh"), "").unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.apply(crate::input::Command::SwitchWindow);
        let mut term = Terminal::new(TestBackend::new(80, 20)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        assert!(
            buf.content
                .iter()
                .all(|c| !crate::bidi::has_bidi(c.symbol()))
        );
        let screen: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(screen.contains("faturafdp.sh"), "{screen}");
    }

    #[test]
    fn footer_warns_about_a_name_with_hidden_bidi() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        std::fs::write(tmp.path().join("fatura\u{202E}fdp.sh"), "").unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.apply(crate::input::Command::SwitchWindow);
        let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        let row: String = (0..100).map(|x| buf[(x, 18)].symbol()).collect();
        assert!(
            row.contains("Name has hidden bidirectional characters"),
            "{row}"
        );
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
    fn share_and_bar() {
        // The largest sibling fills the bar; half of it, half the bar.
        assert_eq!(
            share(30, 100, 30, 40),
            (Some("████████".into()), "  30%".into())
        );
        assert_eq!(
            share(15, 100, 30, 40),
            (Some("████".into()), "  15%".into())
        );
        // Eighths of a character: 1/16 of the largest is one cell.
        assert_eq!(share(2, 100, 32, 40).0.as_deref(), Some("▌"));
        assert_eq!(share(1, 1000, 30, 40).1, " 0.1%");
        assert_eq!(share(58, 1000, 30, 40).1, " 5.8%");
        // Something, but too little to show: not "0.0%".
        assert_eq!(share(1, 100_000, 30, 40).1, "<0.1%");
        assert_eq!(share(0, 0, 0, 40), (Some(String::new()), "   0%".into()));
        // No room for the bar: only the share.
        assert_eq!(share(30, 100, 30, 10), (None, "  30%".into()));
    }

    #[test]
    fn alt_k_percent_in_the_tree() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        for (dir, size) in [("big", 3000), ("small", 1000)] {
            std::fs::create_dir(tmp.path().join(dir)).unwrap();
            std::fs::write(tmp.path().join(dir).join("f"), vec![0u8; size]).unwrap();
        }
        let mut app = App::new(tmp.path().to_path_buf());
        app.apply(crate::input::Command::ExpandAll);
        while app.busy() {
            app.tick(std::time::Duration::from_millis(50));
        }
        app.kbyte.show = KbyteShow::Percent;
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        let line = |name: &str| {
            (0..30)
                .map(|y| (0..100).map(|x| buf[(x, y)].symbol()).collect::<String>())
                .find(|l| l.contains(name))
                .unwrap_or_default()
        };
        assert!(line("big").contains("████████   75%"), "{}", line("big"));
        assert!(
            line("small").contains("██▋░░░░░   25%"),
            "{}",
            line("small")
        );
        assert!(line("% of parent").contains("Tree"));
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

    #[test]
    fn renders_bookmarks_modal() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = crate::testing::tempdir();
        let mut app = App::new(tmp.path().to_path_buf());

        // Empty bookmarks view
        app.bookmarks_view = Some(BookmarksView { selected: 0 });
        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let screen = format!("{:?}", term.backend().buffer());
        assert!(screen.contains("Directory Bookmarks"));
        assert!(screen.contains("No bookmarks defined yet"));

        // With bookmarks
        app.set_bookmark("1".into(), tmp.path().to_path_buf());
        app.set_bookmark("x".into(), tmp.path().join("sub"));
        term.draw(|f| draw(f, &mut app)).unwrap();
        let screen = format!("{:?}", term.backend().buffer());
        assert!(screen.contains("[1]"));
        assert!(screen.contains("[x]"));
        assert!(screen.contains("2 bookmarks"));
    }

    #[test]
    fn icons_switch_in_file_list() {
        let tmp = crate::testing::tempdir();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.options.icons = IconStyle::None;
        let screen_none = draw_text(&mut app);
        assert!(!screen_none.contains("\u{e7a8}"));

        app.options.icons = IconStyle::NerdFonts;
        let screen_icons = draw_text(&mut app);
        assert!(screen_icons.contains("\u{e7a8}"));
    }

    fn entry(name: &str) -> FileEntry {
        FileEntry {
            name: name.into(),
            size: 10,
            disk: 10,
            link: None,
            modified: None,
            mode: 0o644,
            kind: FileKind::File,
            tagged: false,
        }
    }

    /// The row as the terminal shows it, `width` columns wide.
    fn item_text(item: ListItem<'static>, width: u16) -> String {
        use ratatui::{Terminal, backend::TestBackend};
        let mut term = Terminal::new(TestBackend::new(width, 1)).unwrap();
        term.draw(|f| f.render_widget(List::new([item]), f.area()))
            .unwrap();
        let buf = term.backend().buffer();
        (0..width).map(|x| buf[(x, 0)].symbol()).collect()
    }

    #[test]
    fn icons_keep_the_details_column_with_git_status() {
        // Unchanged, modified, untracked and a mix: the size, date and
        // permissions start in the same column on every row.
        for xy in [*b"  ", *b" M", *b"??", *b"AM"] {
            let item = file_line(
                &entry("main.rs"),
                80,
                Detail::Standard,
                Some(xy),
                IconStyle::NerdFonts,
            );
            assert_eq!(item.width(), 80, "{}", String::from_utf8_lossy(&xy));
        }
    }

    #[test]
    fn git_status_letters_stay_with_icons() {
        // Index and work tree apart: " M" is not "M ".
        for xy in [*b" M", *b"M ", *b"??"] {
            let item = file_line(
                &entry("main.rs"),
                80,
                Detail::Standard,
                Some(xy),
                IconStyle::NerdFonts,
            );
            let text = item_text(item, 80);
            let letters = String::from_utf8_lossy(&xy);
            assert!(
                text.starts_with(&format!("  {letters} \u{e7a8} main.rs")),
                "{text}"
            );
        }
    }

    #[test]
    fn branch_title_is_not_a_git_branch() {
        // B is XTree's branch view (the files of a subtree), not Git's.
        let tmp = crate::testing::tempdir();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.options.icons = IconStyle::NerdFonts;
        app.panes[0].set_view(FileView::Branch(crate::model::tree::ROOT));
        let screen = draw_text(&mut app);
        assert!(screen.contains("\u{f0645} Branch: "), "{screen}");
        assert!(!screen.contains('\u{e725}'));
    }

    #[test]
    fn tree_marks_git_repositories() {
        let tmp = crate::testing::tempdir();
        std::fs::create_dir_all(tmp.path().join("proj").join(".git")).unwrap();
        // A worktree or submodule: `.git` is a file.
        std::fs::create_dir(tmp.path().join("wt")).unwrap();
        std::fs::write(tmp.path().join("wt").join(".git"), "gitdir: ../x").unwrap();
        std::fs::create_dir(tmp.path().join("plain")).unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.apply(crate::input::Command::ExpandAll);
        while app.busy() {
            app.tick(std::time::Duration::from_millis(50));
        }
        app.options.icons = IconStyle::NerdFonts;
        let screen = draw_text(&mut app);
        assert!(screen.contains("\u{e5fb} proj"), "{screen}");
        assert!(screen.contains("\u{e5fb} wt"), "{screen}");
        assert!(!screen.contains("\u{e5fb} plain"), "{screen}");
    }
}
