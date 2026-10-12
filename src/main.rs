//! xt — gerenciador de arquivos em modo texto inspirado no XTree/ZTreeWin.
//!
//! Uso: `xt [diretório]` (padrão: diretório atual)

mod app;
mod input;
mod model;
mod ops;
mod ui;
mod viewer;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use app::App;

fn main() -> anyhow::Result<()> {
    let root = match std::env::args_os().nth(1) {
        Some(arg) => PathBuf::from(arg),
        None => std::env::current_dir().context("cannot read current directory")?,
    };
    let root =
        std::fs::canonicalize(&root).with_context(|| format!("cannot open {}", root.display()))?;
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let mut app = App::new(root);
    // `init` também instala um panic hook que restaura o terminal.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> anyhow::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        // Com trabalho em andamento (log, cópia...), não bloqueia esperando tecla:
        // processa um pedaço e redesenha o progresso.
        if app.busy() && !event::poll(Duration::ZERO)? {
            app.tick(Duration::from_millis(50));
            continue;
        }
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key);
        }
    }
    Ok(())
}
