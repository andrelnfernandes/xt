//! xt — gerenciador de arquivos em modo texto inspirado no XTree/ZTreeWin.
//!
//! Uso: `xt [diretório]` (padrão: diretório atual)

mod app;
mod input;
mod model;
mod ops;
mod ui;
mod viewer;

use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, Event, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{execute, terminal};

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
    let enhanced = enable_keyboard_enhancement();
    let result = run(&mut terminal, &mut app);
    if enhanced {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

/// Liga o protocolo de teclado do kitty, quando o terminal o suporta
/// (kitty, WezTerm, foot, Ghostty, Alacritty...). Com ele, Ctrl+M chega
/// separado de Enter (e Esc deixa de ser ambíguo); sem ele, Ctrl+M é Enter
/// e o "mover marcados" fica só no Alt+M.
fn enable_keyboard_enhancement() -> bool {
    let supported = terminal::supports_keyboard_enhancement().unwrap_or(false);
    if !supported {
        return false;
    }
    let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES;
    if execute!(stdout(), PushKeyboardEnhancementFlags(flags)).is_err() {
        return false;
    }
    // Se o programa entrar em pânico, desliga o protocolo antes de o hook
    // do ratatui restaurar o terminal; senão o shell ficaria recebendo
    // códigos de tecla estranhos.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        previous(info);
    }));
    true
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
