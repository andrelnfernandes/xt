//! Desenho da tela no layout clássico do XTree:
//!
//! ```text
//! ┌ caminho atual ──────────────────────────┐
//! │ árvore                      │ estatíst. │
//! ├─────────────────────────────┤           │
//! │ arquivos do diretório       │           │
//! └─────────────────────────────┴───────────┘
//!  menu de comandos (2 linhas)
//! ```

use std::ops::Range;
use std::time::SystemTime;

use chrono::{DateTime, Local};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph};

use crate::app::{App, FileView, Focus};
use crate::model::tree::{FileEntry, FileKind};
use crate::ops::Operation;
use crate::viewer::{self, ViewMode};

const BG: Color = Color::Blue;
const FG: Color = Color::White;
const BORDER: Color = Color::Cyan;
const KEY: Color = Color::Yellow;
const DIM: Color = Color::LightBlue;

const STATS_WIDTH: u16 = 28;
const SMALL_FILE_WINDOW: u16 = 8;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let base = Style::new().fg(FG).bg(BG);
    frame.render_widget(Block::new().style(base), frame.area());

    if app.viewer.is_some() {
        draw_viewer(frame, app);
        return;
    }

    let [header, main, menu] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    let [left, stats] =
        Layout::horizontal([Constraint::Min(20), Constraint::Length(STATS_WIDTH)]).areas(main);

    draw_header(frame, app, header);
    draw_stats(frame, app, stats);
    draw_menu(frame, app, menu);

    match app.focus {
        Focus::Tree => {
            let [tree, files] =
                Layout::vertical([Constraint::Min(3), Constraint::Length(SMALL_FILE_WINDOW)])
                    .areas(left);
            draw_tree(frame, app, tree);
            draw_files(frame, app, files);
        }
        Focus::Files => draw_files(frame, app, left),
    }
}

fn panel(title: impl Into<Line<'static>>, active: bool) -> Block<'static> {
    let border = if active { KEY } else { BORDER };
    Block::bordered()
        .border_type(BorderType::Plain)
        .border_style(Style::new().fg(border))
        .title(title)
}

fn highlight(active: bool) -> Style {
    if active {
        Style::new().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::new().fg(Color::Black).bg(Color::Gray)
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let path = app.tree.path(app.header_dir());
    let clock = Local::now().format("%d/%m/%Y %H:%M").to_string();
    let [left, right] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(clock.len() as u16)])
            .areas(area);
    frame.render_widget(
        Line::from(vec![
            Span::styled("Path: ", Style::new().fg(KEY)),
            Span::raw(path.to_string_lossy().into_owned()),
        ]),
        left,
    );
    frame.render_widget(Line::from(clock), right);
}

fn draw_tree(frame: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Tree;
    app.tree_page = area.height.saturating_sub(2) as usize;

    let (range, mut state) = visible_window(&mut app.tree_state, app.rows.len(), app.tree_page);
    let items: Vec<ListItem> = app.rows[range]
        .iter()
        .map(|row| {
            let node = app.tree.node(row.id);
            let name = node.name.to_string_lossy().into_owned();
            let name_style = if node.error.is_some() {
                Style::new().fg(Color::LightRed)
            } else {
                Style::new()
            };
            ListItem::new(Line::from(vec![
                Span::styled(row.prefix.clone(), Style::new().fg(DIM)),
                Span::styled(name, name_style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(panel(" Tree ", active))
        .highlight_style(highlight(active));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_files(frame: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Files;
    app.file_page = area.height.saturating_sub(2) as usize;

    let width = area.width.saturating_sub(2) as usize;
    let error = match app.view {
        FileView::Dir => app.tree.node(app.current()).error.as_ref(),
        _ => None,
    };
    let (range, mut state) = visible_window(&mut app.file_state, app.files.len(), app.file_page);
    let items: Vec<ListItem> = if let Some(err) = error {
        vec![ListItem::new(Span::styled(
            format!("Cannot read directory: {err}"),
            Style::new().fg(Color::LightRed),
        ))]
    } else if app.files.is_empty() {
        vec![ListItem::new(Span::styled(
            "No matching files",
            Style::new().fg(DIM),
        ))]
    } else {
        app.files[range]
            .iter()
            .map(|&f| file_line(app.file(f), width))
            .collect()
    };

    let kind = match app.view {
        FileView::Dir => "Files",
        FileView::Branch(_) => "Branch",
        FileView::Showall => "Showall",
    };
    let title = format!(
        " {kind}: {} ({}) ",
        app.filespec.text(),
        fmt_num(app.files.len() as u64)
    );
    let list = List::new(items)
        .block(panel(title, active))
        .highlight_style(highlight(active));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Ajusta a rolagem de `state` para o item selecionado ficar visível.
/// Devolve a faixa de itens que cabe em `height` linhas e o estado a usar
/// para desenhar só essa faixa (listas grandes não são montadas inteiras).
fn visible_window(state: &mut ListState, len: usize, height: usize) -> (Range<usize>, ListState) {
    let height = height.max(1);
    let sel = state.selected().unwrap_or(0).min(len.saturating_sub(1));
    let mut offset = state.offset().min(sel);
    if sel >= offset + height {
        offset = sel + 1 - height;
    }
    // Sem espaço vazio no fim quando a lista encolhe.
    offset = offset.min(len.saturating_sub(height));
    *state.offset_mut() = offset;
    let range = offset..(offset + height).min(len);
    (
        range,
        ListState::default().with_selected(Some(sel - offset)),
    )
}

/// Uma linha da janela de arquivos: nome, tamanho, data e permissões.
fn file_line(f: &FileEntry, width: usize) -> ListItem<'static> {
    let size = fmt_num(f.size);
    let date = f.modified.map(fmt_time).unwrap_or_default();
    let mode = fmt_mode(f.mode, f.kind);
    let details = format!(" {size:>15} {date:16} {mode}");

    // 2 colunas para a marca de tag.
    let name_width = width.saturating_sub(details.chars().count() + 2).max(8);
    let mut name: String = f.name.to_string_lossy().into_owned();
    if f.kind == FileKind::Symlink {
        name.push('@');
    }
    let name = fit(&name, name_width);

    let style = match f.kind {
        _ if f.tagged => Style::new().fg(KEY).add_modifier(Modifier::BOLD),
        FileKind::Symlink => Style::new().fg(Color::LightCyan),
        FileKind::Other => Style::new().fg(Color::LightMagenta),
        FileKind::File if f.mode & 0o111 != 0 => Style::new().fg(Color::LightGreen),
        FileKind::File => Style::new(),
    };
    let mark = if f.tagged { "■ " } else { "  " };
    ListItem::new(Line::from(vec![
        Span::styled(mark, Style::new().fg(KEY)),
        Span::styled(name, style),
        Span::styled(details, Style::new().fg(DIM)),
    ]))
}

fn draw_stats(frame: &mut Frame, app: &App, area: Rect) {
    let id = app.header_dir();
    let logged = app.tree.logged_totals();
    let matching_bytes: u64 = app.files.iter().map(|&f| app.file(f).size).sum();
    let (tagged, tagged_bytes) = app
        .files
        .iter()
        .map(|&f| app.file(f))
        .filter(|f| f.tagged)
        .fold((0u64, 0u64), |(n, b), f| (n + 1, b + f.size));
    let section = match app.view {
        FileView::Dir => "DIRECTORY",
        FileView::Branch(_) => "BRANCH",
        FileView::Showall => "SHOWALL",
    };

    let label = |s: &'static str| Line::from(Span::styled(s, Style::new().fg(KEY)));
    let value = |k: &str, v: String| {
        Line::from(vec![
            Span::raw(format!(" {k:<7}")),
            Span::styled(v, Style::new().add_modifier(Modifier::BOLD)),
        ])
    };

    let mut lines = vec![
        label("FILE"),
        value("Spec", fit(app.filespec.text(), STATS_WIDTH as usize - 10)),
        Line::default(),
    ];

    if let Some((total, free)) = disk_space(&app.tree.path(id)) {
        lines.extend([
            label("DISK"),
            value("Total", fmt_num(total)),
            value("Free", fmt_num(free)),
            Line::default(),
        ]);
    }

    lines.extend([
        label("LOGGED"),
        value("Dirs", fmt_num(logged.dirs)),
        value("Files", fmt_num(logged.files)),
        value("Bytes", fmt_num(logged.bytes)),
        Line::default(),
        label(section),
        value("Match", fmt_num(app.files.len() as u64)),
        value("Bytes", fmt_num(matching_bytes)),
        Line::default(),
        label("TAGGED"),
        value("Files", fmt_num(tagged)),
        value("Bytes", fmt_num(tagged_bytes)),
    ]);

    frame.render_widget(Paragraph::new(lines).block(panel(" xt ", false)), area);
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let lines = if let Some(prompt) = &app.prompt {
        let col = area.x + (prompt.label.chars().count() + prompt.input.cursor) as u16;
        frame.set_cursor_position((col.min(area.right().saturating_sub(1)), area.y));
        let mut help = vec![("Enter", "accept"), ("Esc", "cancel"), ("Ctrl+U", "clear")];
        if prompt.toggles_paths() {
            help.push(("F6", "with/without paths"));
        }
        if prompt.label.starts_with("FILESPEC") {
            help.push(("e.g.", "*.rs *.toml -*.bak"));
        }
        vec![
            Line::from(vec![
                Span::styled(prompt.label.clone(), bold),
                Span::raw(prompt.input.text.clone()),
            ]),
            menu_line("", &help),
        ]
    } else if let Some(confirm) = &app.confirm {
        vec![
            Line::from(vec![
                Span::styled(format!("{} ", confirm.question), bold),
                key("Y"),
                Span::raw("es / "),
                key("N"),
                Span::raw("o"),
            ]),
            Line::default(),
        ]
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
        let (first, second) = match app.focus {
            Focus::Tree => (
                menu_words(
                    "DIR",
                    &[
                        "Branch", "Delete", "Filespec", "Invert", "Makedir", "Rename", "Showall",
                        "Tag", "Untag", "Quit",
                    ],
                ),
                menu_line(
                    "",
                    &[
                        ("←→", "close/open"),
                        ("*", "log branch"),
                        ("F2", "relog"),
                        ("Ctrl+T/U", "all logged"),
                        ("Enter", "files"),
                    ],
                ),
            ),
            Focus::Files => (
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
                        "Rename",
                        "Showall",
                        "Tag",
                        "Untag",
                        "View",
                        "Quit",
                    ],
                ),
                menu_line(
                    "",
                    &[
                        ("Ctrl+C M D A", "tagged"),
                        ("Alt+C M", "tagged with paths"),
                        ("Ctrl+T/U", "all"),
                        ("F2", "relog"),
                        ("Esc", "tree"),
                    ],
                ),
            ),
        };
        let first = match &app.message {
            Some(msg) => Line::from(Span::styled(
                msg.clone(),
                Style::new()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )),
            None => first,
        };
        vec![first, second]
    };
    frame.render_widget(Paragraph::new(lines), area);
}

/// Rodapé durante Copy/Move/Delete: arquivo atual e barra de progresso,
/// ou a pergunta de conflito.
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
                    op.kind.verb(),
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
        Span::styled("█".repeat(filled), Style::new().fg(KEY)),
        Span::styled("░".repeat(BAR - filled), Style::new().fg(DIM)),
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

/// Linha de menu no estilo do XTree: a primeira letra de cada palavra é a tecla.
fn menu_words(title: &str, words: &[&str]) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{title:<9}"),
        Style::new().add_modifier(Modifier::BOLD),
    )];
    for word in words {
        let (first, rest) = word.split_at(1);
        spans.push(key(first));
        spans.push(Span::raw(format!("{rest} ")));
    }
    Line::from(spans)
}

fn draw_viewer(frame: &mut Frame, app: &mut App) {
    let Some(v) = &mut app.viewer else { return };
    let [header, body, menu] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(frame.area());

    let mut title = vec![
        Span::styled("View: ", Style::new().fg(KEY)),
        Span::raw(v.path.to_string_lossy().into_owned()),
        Span::styled(format!("  {} bytes", fmt_num(v.size)), Style::new().fg(DIM)),
    ];
    if v.truncated() {
        title.push(Span::styled(
            format!("  (showing first {})", fmt_num(viewer::MAX_BYTES)),
            Style::new().fg(Color::LightRed),
        ));
    }
    frame.render_widget(Line::from(title), header);

    let inner = panel("", false).inner(body);
    v.page = inner.height as usize;
    let lines: Vec<Line> = v
        .visible_lines(inner.height as usize)
        .into_iter()
        .map(Line::from)
        .collect();
    let mode = match v.mode {
        ViewMode::Text => " TEXT ",
        ViewMode::Hex => " HEX ",
    };
    frame.render_widget(Paragraph::new(lines).block(panel(mode, true)), body);

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
    frame.render_widget(
        Paragraph::new(vec![
            menu_line(
                "VIEW",
                &[
                    ("↑↓ PgUp/PgDn", "scroll"),
                    ("←→", "pan"),
                    ("H", "hex/text"),
                    ("Esc", "close"),
                ],
            ),
            Line::from(format!("{:9}{position}  ({percent}%)", "")),
        ]),
        menu,
    );
}

fn key(k: &str) -> Span<'static> {
    Span::styled(
        k.to_string(),
        Style::new().fg(KEY).add_modifier(Modifier::BOLD),
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

fn disk_space(path: &std::path::Path) -> Option<(u64, u64)> {
    let s = nix::sys::statvfs::statvfs(path).ok()?;
    let frag = s.fragment_size() as u64;
    Some((s.blocks() as u64 * frag, s.blocks_available() as u64 * frag))
}

/// Corta ou completa com espaços para caber em `width` colunas.
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

/// 1234567 -> "1,234,567"
fn fmt_num(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn fmt_time(t: SystemTime) -> String {
    DateTime::<Local>::from(t)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// Permissões no formato do `ls -l`, ex.: "-rwxr-xr-x".
fn fmt_mode(mode: u32, kind: FileKind) -> String {
    let mut s = String::with_capacity(10);
    s.push(match kind {
        FileKind::File => '-',
        FileKind::Symlink => 'l',
        FileKind::Other => '?',
    });
    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 0o7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(if bits & 1 != 0 { 'x' } else { '-' });
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(999), "999");
        assert_eq!(fmt_num(1000), "1,000");
        assert_eq!(fmt_num(1234567), "1,234,567");
    }

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
    fn window_follows_selection() {
        let mut state = ListState::default().with_selected(Some(0));
        let (range, local) = visible_window(&mut state, 100, 10);
        assert_eq!((range, local.selected()), (0..10, Some(0)));

        state.select(Some(15));
        let (range, local) = visible_window(&mut state, 100, 10);
        assert_eq!((range, local.selected()), (6..16, Some(9)));

        // Subindo dentro da janela, ela não se move.
        state.select(Some(8));
        assert_eq!(visible_window(&mut state, 100, 10).0, 6..16);

        // A lista encolheu: a janela recua.
        state.select(Some(2));
        assert_eq!(visible_window(&mut state, 5, 10).0, 0..5);
        assert_eq!(visible_window(&mut state, 0, 10).0, 0..0);
    }

    #[test]
    fn renders_without_panicking() {
        use ratatui::{Terminal, backend::TestBackend};
        let tmp = tempfile::tempdir().unwrap();
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
