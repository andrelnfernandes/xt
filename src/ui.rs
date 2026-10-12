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
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph};

use crate::app::{self, App, FileView, Focus, KbyteShow, MenuMode, Pane};
use crate::config::{self, formats, theme};
use crate::model::tree::{FileEntry, FileKind};
use crate::ops::Operation;
use crate::pane::FileDisplay;
use crate::settings::{self, Settings};
use crate::viewer::{self, ViewMode};

const STATS_WIDTH: u16 = 28;
const SMALL_FILE_WINDOW: u16 = 8;

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
    draw_menu(frame, app, menu);

    if app.is_split() {
        // Duas janelas lado a lado; o painel de estatísticas sai para dar espaço.
        let halves: [Rect; 2] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .areas(main);
        let active = app.active;
        for (i, (pane, area)) in app.panes.iter_mut().zip(halves).enumerate() {
            draw_pane(frame, pane, i == active, true, area);
        }
    } else {
        let [left, stats] =
            Layout::horizontal([Constraint::Min(20), Constraint::Length(STATS_WIDTH)]).areas(main);
        draw_stats(frame, app, stats);
        draw_pane(frame, &mut app.panes[0], true, false, left);
    }
}

/// Uma janela: árvore em cima e arquivos embaixo, ou só os arquivos.
/// Com a tela dividida, o título da árvore mostra o diretório atual.
fn draw_pane(frame: &mut Frame, pane: &mut Pane, active: bool, split: bool, area: Rect) {
    match pane.focus {
        Focus::Tree => {
            let [tree, files] =
                Layout::vertical([Constraint::Min(3), Constraint::Length(SMALL_FILE_WINDOW)])
                    .areas(area);
            let title = if split {
                let width = tree.width.saturating_sub(4) as usize;
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
            draw_tree(frame, pane, active, title, tree);
            draw_files(frame, pane, false, files);
        }
        Focus::Files => draw_files(frame, pane, active, area),
    }
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
    // Alt+K: tamanho ou nº de arquivos à direita de cada diretório.
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
    let list = List::new(items)
        .block(panel(title, active))
        .highlight_style(highlight(active));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_files(frame: &mut Frame, pane: &mut Pane, active: bool, area: Rect) {
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
        draw_file_columns(frame, pane, active, area, title);
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
            .map(|&f| file_line(pane.file(f), width, names_only))
            .collect()
    };

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

/// Nomes maiores são cortados com `…` nos modos de colunas, para caberem
/// várias colunas (como no XTree, de colunas de largura fixa).
const MAX_COLUMN_NAME: usize = 20;

/// Modos de várias colunas (Alt+F): os arquivos vão da esquerda para a
/// direita e depois para a linha de baixo, como no XTree.
fn draw_file_columns(frame: &mut Frame, pane: &mut Pane, active: bool, area: Rect, title: String) {
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
    // Marca de tag + nome + (tamanho e permissões) + espaço entre colunas.
    let cell_w = 2 + name_w + if with_size { 23 } else { 0 } + 1;
    let cols = (width / cell_w).max(1);
    pane.file_cols = cols;
    pane.file_page = height * cols;

    // Rolagem por linhas, com o arquivo selecionado sempre visível.
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
                let f = pane.file(pane.files[i]);
                let mut name = f.name.to_string_lossy().into_owned();
                if f.kind == FileKind::Symlink {
                    name.push('@');
                }
                let mut cell = format!(
                    "{}{}",
                    if f.tagged { "■ " } else { "  " },
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

/// Cor do nome conforme o tipo (e se está marcado).
fn file_style(f: &FileEntry) -> Style {
    match f.kind {
        _ if f.tagged => Style::new().fg(theme().tagged).add_modifier(Modifier::BOLD),
        FileKind::Symlink => Style::new().fg(theme().symlink),
        FileKind::Other => Style::new().fg(theme().special),
        FileKind::File if f.mode & 0o111 != 0 => Style::new().fg(theme().executable),
        FileKind::File => Style::new(),
    }
}

/// Uma linha da janela de arquivos: nome, tamanho, data e permissões (ou
/// só o nome, no modo de nomes longos).
fn file_line(f: &FileEntry, width: usize, names_only: bool) -> ListItem<'static> {
    let size = fmt_num(f.size);
    let date = f.modified.map(fmt_time).unwrap_or_default();
    let mode = fmt_mode(f.mode, f.kind);
    // Em janelas estreitas (tela dividida), as colunas menos úteis saem.
    let details = if names_only {
        String::new()
    } else if width >= 72 {
        format!(" {size:>15} {date:16} {mode}")
    } else if width >= 50 {
        format!(" {size:>13} {date:16}")
    } else {
        format!(" {size:>13}")
    };

    // 2 colunas para a marca de tag.
    let name_width = width.saturating_sub(details.chars().count() + 2).max(8);
    let mut name: String = f.name.to_string_lossy().into_owned();
    if f.kind == FileKind::Symlink {
        name.push('@');
    }
    let name = fit(&name, name_width);

    let style = file_style(f);
    let mark = if f.tagged { "■ " } else { "  " };
    ListItem::new(Line::from(vec![
        Span::styled(mark, Style::new().fg(theme().tagged)),
        Span::styled(name, style),
        Span::styled(details, Style::new().fg(theme().dim)),
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

    let label = |s: &'static str| Line::from(Span::styled(s, Style::new().fg(theme().key)));
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
        prompt_lines(frame, prompt, area)
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
        // Pergunta numa linha e opções na outra, para caminhos longos não
        // empurrarem as opções para fora da tela.
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
        let (first, second) = idle_menu(app);
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

/// Menu do rodapé sem nada em andamento: o normal ou o de Ctrl/Alt,
/// conforme a tecla pressionada (ou escolhida com F9).
fn idle_menu(app: &App) -> (Line<'static>, Line<'static>) {
    let tree = app.focus == Focus::Tree;
    // Com F9, a próxima letra recebe o modificador: avisa como desistir.
    let sticky = app.held.is_none() && app.sticky != MenuMode::Normal;
    let hint: (&str, &str) = if sticky {
        ("Esc", "cancel")
    } else {
        ("F9", "next menu")
    };

    match app.menu_mode() {
        MenuMode::Normal if tree && app.archive.is_some() => (
            menu_words(
                "ARCHIVE",
                &[
                    "Branch", "Compare", "Filespec", "Invert", "Showall", "Tag", "Untag", "Quit",
                ],
            ),
            menu_line(
                "",
                &[
                    ("Esc", "close archive"),
                    ("←→", "close/open"),
                    ("Enter", "files (Copy extracts)"),
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
                    "Rename", "Showall", "Tag", "Untag", "eXecute", "Quit",
                ],
            ),
            menu_line(
                "",
                &[
                    ("←→", "close/open"),
                    ("*", "log branch"),
                    ("F3", "relog"),
                    ("Enter", "files"),
                    ("F8", "split"),
                    ("F9", "Ctrl/Alt menus"),
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
                    "eXecute",
                    "Quit",
                ],
            ),
            menu_line(
                "",
                &[
                    ("F3", "relog"),
                    ("Esc", "tree"),
                    ("F8", "split"),
                    ("F9", "Ctrl/Alt menus"),
                ],
            ),
        ),
        MenuMode::Ctrl => (
            menu_words(
                "CTRL",
                if tree {
                    &["Attributes", "Copy", "Delete", "Move", "Tag", "Untag"]
                } else {
                    &[
                        "Attributes",
                        "Copy",
                        "Delete",
                        "Move",
                        "Open",
                        "Tag",
                        "Untag",
                    ]
                },
            ),
            menu_line(
                "",
                &[
                    (
                        "",
                        if tree {
                            "on tagged files here · Tag/Untag: all logged files"
                        } else {
                            "on tagged files in the list · Copy/Move without paths"
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
                    &["Attributes", "Compare branch", "Delete", "Prune", "Sort"]
                } else {
                    &["Attributes", "Copy", "Delete", "Move", "Sort"]
                },
            ),
            menu_line(
                "",
                &[
                    if tree {
                        ("", "Attributes/Delete: tagged files here")
                    } else {
                        ("F5", "open archive · Copy/Move: tagged with paths")
                    },
                    hint,
                ],
            ),
        ),
    }
}

/// Prompt no rodapé: o rótulo, o texto digitado (com o cursor) e as teclas.
fn prompt_lines(frame: &mut Frame, prompt: &app::Prompt, area: Rect) -> Vec<Line<'static>> {
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let col = area.x + (prompt.label.chars().count() + prompt.input.cursor) as u16;
    frame.set_cursor_position((col.min(area.right().saturating_sub(1)), area.y));
    let mut help = vec![("Enter", "accept"), ("Esc", "cancel"), ("Ctrl+U", "clear")];
    if prompt.toggles_paths() {
        help.push(("F6", "with/without paths"));
    }
    if prompt.is_search() {
        help.push(("F2", "match case"));
        help.push(("F4", "text/hex"));
        help.push(("↑↓", "history"));
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
}

/// Linha do visualizador com o trecho encontrado destacado.
fn highlighted(line: String, mark: Option<(usize, usize)>) -> Line<'static> {
    let Some((s, e)) = mark else {
        return Line::from(line);
    };
    let chars: Vec<char> = line.chars().collect();
    let (s, e) = (s.min(chars.len()), e.min(chars.len()));
    let part = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    Line::from(vec![
        Span::raw(part(0, s)),
        Span::styled(part(s, e), highlight(true).add_modifier(Modifier::BOLD)),
        Span::raw(part(e, chars.len())),
    ])
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

/// Linha de menu no estilo do XTree: a tecla de cada palavra é a primeira
/// letra maiúscula ("Copy" → C, "eXecute" → X).
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

/// Tela de configuração: as opções da página, com a letra, a descrição e o
/// valor; com F4, a lista de temas por cima.
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

    let inner = panel("", false).inner(body);
    v.page = inner.height as usize;
    v.width = inner.width as usize;
    let lines: Vec<Line> = v
        .visible(inner.height as usize)
        .into_iter()
        .map(|(line, mark)| highlighted(line, mark))
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
    let note = v.note.clone();
    let lines = if let Some(prompt) = &app.prompt {
        prompt_lines(frame, prompt, menu)
    } else {
        let mut words = vec!["Find", "Backward", "Hex"];
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
            None => Line::from(format!("{:9}{position}  ({percent}%)", "")),
        };
        vec![first, second]
    };
    frame.render_widget(Paragraph::new(lines), menu);
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

/// Como `fit`, mas guarda o fim do texto: "…/projetos/xt".
fn fit_tail(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len <= width {
        return s.to_string();
    }
    let tail: String = s.chars().skip(len + 1 - width.max(1)).collect();
    format!("…{tail}")
}

/// Número com o separador de milhar configurado.
fn fmt_num(n: u64) -> String {
    group_digits(n, formats().number.separator())
}

/// 1234567 -> "1,234,567" (com `,`).
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
        let comma = Some(',');
        assert_eq!(group_digits(0, comma), "0");
        assert_eq!(group_digits(999, comma), "999");
        assert_eq!(group_digits(1000, comma), "1,000");
        assert_eq!(group_digits(1234567, comma), "1,234,567");
        assert_eq!(group_digits(1234567, Some('.')), "1.234.567");
        assert_eq!(group_digits(1234567, None), "1234567");
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

        // Subindo dentro da janela, ela não se move.
        state.select(Some(8));
        assert_eq!(visible_window(&mut state, 100, 10).0, 6..16);

        // A lista encolheu: a janela recua.
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
            // As letras de tecla são as partes em negrito (o tema pode
            // estar sendo trocado por outro teste ao mesmo tempo).
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
        let tmp = tempfile::tempdir().unwrap();
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
        let tmp = tempfile::tempdir().unwrap();
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
        let tmp = tempfile::tempdir().unwrap();
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
        assert!(!screen.contains("LOGGED"), "no stats panel when split");
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
