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

use std::time::SystemTime;

use chrono::{DateTime, Local};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph};

use crate::app::{App, Focus};
use crate::model::tree::{FileEntry, FileKind};

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
    let path = app.tree.path(app.current());
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

    let items: Vec<ListItem> = app
        .rows
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
    frame.render_stateful_widget(list, area, &mut app.tree_state);
}

fn draw_files(frame: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Files;
    app.file_page = area.height.saturating_sub(2) as usize;

    let node = app.tree.node(app.current());
    let width = area.width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = if let Some(err) = &node.error {
        vec![ListItem::new(Span::styled(
            format!("Cannot read directory: {err}"),
            Style::new().fg(Color::LightRed),
        ))]
    } else if node.files.is_empty() {
        vec![ListItem::new(Span::styled(
            "No files",
            Style::new().fg(DIM),
        ))]
    } else {
        node.files.iter().map(|f| file_line(f, width)).collect()
    };

    let title = format!(" Files ({}) ", node.files.len());
    let list = List::new(items)
        .block(panel(title, active))
        .highlight_style(highlight(active));
    frame.render_stateful_widget(list, area, &mut app.file_state);
}

/// Uma linha da janela de arquivos: nome, tamanho, data e permissões.
fn file_line(f: &FileEntry, width: usize) -> ListItem<'static> {
    let size = fmt_num(f.size);
    let date = f.modified.map(fmt_time).unwrap_or_default();
    let mode = fmt_mode(f.mode, f.kind);
    let details = format!(" {size:>15} {date:16} {mode}");

    let name_width = width.saturating_sub(details.chars().count()).max(8);
    let mut name: String = f.name.to_string_lossy().into_owned();
    if f.kind == FileKind::Symlink {
        name.push('@');
    }
    let name = fit(&name, name_width);

    let style = match f.kind {
        FileKind::Symlink => Style::new().fg(Color::LightCyan),
        FileKind::Other => Style::new().fg(Color::LightMagenta),
        FileKind::File if f.mode & 0o111 != 0 => Style::new().fg(Color::LightGreen),
        FileKind::File => Style::new(),
    };
    ListItem::new(Line::from(vec![
        Span::styled(name, style),
        Span::styled(details, Style::new().fg(DIM)),
    ]))
}

fn draw_stats(frame: &mut Frame, app: &App, area: Rect) {
    let id = app.current();
    let node = app.tree.node(id);
    let logged = app.tree.logged_totals();
    let dir_bytes: u64 = node.files.iter().map(|f| f.size).sum();

    let label = |s: &'static str| Line::from(Span::styled(s, Style::new().fg(KEY)));
    let value = |k: &str, v: String| {
        Line::from(vec![
            Span::raw(format!(" {k:<7}")),
            Span::styled(v, Style::new().add_modifier(Modifier::BOLD)),
        ])
    };

    let mut lines = vec![
        label("FILE"),
        value("Spec", "*".into()),
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
        label("DIRECTORY"),
        value("Files", fmt_num(node.files.len() as u64)),
        value("Bytes", fmt_num(dir_bytes)),
    ]);

    frame.render_widget(Paragraph::new(lines).block(panel(" xt ", false)), area);
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if app.confirm_quit {
        vec![
            Line::from(vec![
                Span::styled("Quit xt? ", Style::new().add_modifier(Modifier::BOLD)),
                key("Y"),
                Span::raw("es / "),
                key("N"),
                Span::raw("o"),
            ]),
            Line::default(),
        ]
    } else {
        let first = match app.focus {
            Focus::Tree => menu_line(
                "DIR",
                &[("←→", "collapse/expand"), ("*", "expand branch"), ("Q", "quit")],
            ),
            Focus::Files => menu_line("FILE", &[("Esc", "back to tree"), ("Q", "quit")]),
        };
        let second = menu_line(
            "",
            &[
                ("↑↓", "move"),
                ("PgUp/PgDn", "page"),
                ("Home/End", "top/bottom"),
                ("Enter/Tab", "switch window"),
            ],
        );
        vec![first, second]
    };
    frame.render_widget(Paragraph::new(lines), area);
}

fn key(k: &str) -> Span<'static> {
    Span::styled(k.to_string(), Style::new().fg(KEY).add_modifier(Modifier::BOLD))
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
    DateTime::<Local>::from(t).format("%Y-%m-%d %H:%M").to_string()
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
