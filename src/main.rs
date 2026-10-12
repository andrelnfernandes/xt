//! xt — gerenciador de arquivos em modo texto inspirado no XTree/ZTreeWin.
//!
//! Uso: `xt [diretório]` (padrão: diretório atual)

mod app;
mod input;
mod model;
mod ops;
mod pane;
mod ui;
mod viewer;

use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
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
    let mut keyboard = Keyboard::detect();
    let result = run(&mut terminal, &mut app, &mut keyboard);
    keyboard.restore();
    ratatui::restore();
    result
}

/// Protocolo de teclado do kitty, quando o terminal o suporta (kitty,
/// WezTerm, foot, Ghostty, Alacritty...).
///
/// - Modo simples: Ctrl+M chega separado de Enter e Esc deixa de ser
///   ambíguo. Usado enquanto um prompt está aberto, para a digitação
///   (acentos, teclas mortas) continuar como sempre.
/// - Modo completo: o terminal também avisa quando cada tecla é pressionada
///   e solta, inclusive Ctrl e Alt sozinhos; é o que permite trocar o menu
///   enquanto o Ctrl/Alt está pressionado, como no ZTree.
///
/// Sem suporte (ex.: terminais VTE como o Terminator), nada disso é ligado.
struct Keyboard {
    supported: bool,
    full: bool,
}

impl Keyboard {
    const SIMPLE: KeyboardEnhancementFlags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES;
    const FULL: KeyboardEnhancementFlags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        .union(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        .union(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
        .union(KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS);

    fn detect() -> Self {
        let supported = terminal::supports_keyboard_enhancement().unwrap_or(false)
            && execute!(stdout(), PushKeyboardEnhancementFlags(Self::SIMPLE)).is_ok();
        if supported {
            // Se o programa entrar em pânico, desliga o protocolo antes de o
            // hook do ratatui restaurar o terminal; senão o shell ficaria
            // recebendo códigos de tecla estranhos. Desempilhar a mais é
            // inofensivo.
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
                let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
                previous(info);
            }));
        }
        Keyboard {
            supported,
            full: false,
        }
    }

    /// Liga o modo completo fora dos prompts e o desliga dentro deles.
    fn sync(&mut self, app: &mut App) -> std::io::Result<()> {
        let want_full = self.supported && app.prompt.is_none();
        if want_full == self.full {
            return Ok(());
        }
        if want_full {
            execute!(stdout(), PushKeyboardEnhancementFlags(Self::FULL))?;
        } else {
            execute!(stdout(), PopKeyboardEnhancementFlags)?;
        }
        self.full = want_full;
        // No modo simples a soltura do Ctrl/Alt não é avisada.
        app.held = None;
        Ok(())
    }

    fn restore(&mut self) {
        if self.full {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
        if self.supported {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
    }
}

fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    keyboard: &mut Keyboard,
) -> anyhow::Result<()> {
    while !app.should_quit {
        keyboard.sync(app)?;
        terminal.draw(|frame| ui::draw(frame, app))?;
        // Com trabalho em andamento (log, cópia...), não bloqueia esperando tecla:
        // processa um pedaço e redesenha o progresso.
        if app.busy() && !event::poll(Duration::ZERO)? {
            app.tick(Duration::from_millis(50));
            continue;
        }
        // Pressionar, repetir e soltar: o App decide o que usar.
        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }
    }
    Ok(())
}
