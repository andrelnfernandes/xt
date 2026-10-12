//! xt — gerenciador de arquivos em modo texto inspirado no XTree/ZTreeWin.
//!
//! Uso: `xt [diretório]` (padrão: diretório atual); `xt --help` para mais.

mod app;
mod archive;
mod config;
mod input;
mod keymap;
mod model;
mod ops;
mod pane;
mod shell;
mod state;
mod ui;
mod viewer;
mod watch;

use std::io::{Write, stdout};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::crossterm::{cursor, execute, terminal};

use app::App;

const USAGE: &str = "\
xt - text-mode file manager inspired by XTree/ZTreeWin

Usage: xt [DIRECTORY]

Options:
  --default-config   print a commented configuration file
  -h, --help         show this help
  -V, --version      show the version

Configuration: ~/.config/xt/config.toml (or $XDG_CONFIG_HOME/xt/config.toml).
Inside xt, press F1 for the list of keys.";

fn main() -> anyhow::Result<()> {
    let mut root = None;
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("-h" | "--help") => {
                println!("{USAGE}");
                return Ok(());
            }
            Some("-V" | "--version") => {
                println!("xt {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Some("--default-config") => {
                print!("{}", config::DEFAULT_CONFIG);
                return Ok(());
            }
            Some(s) if s.starts_with('-') => bail!("unknown option {s}\n\n{USAGE}"),
            _ if root.is_some() => bail!("only one directory can be given\n\n{USAGE}"),
            _ => root = Some(PathBuf::from(arg)),
        }
    }
    let root = match root {
        Some(path) => path,
        None => std::env::current_dir().context("cannot read current directory")?,
    };
    let root =
        std::fs::canonicalize(&root).with_context(|| format!("cannot open {}", root.display()))?;
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let (config, config_error) = config::Config::load();
    if let Ok(theme) = config::Theme::from_config(&config) {
        config::set_theme(theme);
    }
    let mut app = App::new(root);
    app.set_sort(config.options.initial_sort());
    app.options = config.options;
    app.keymap = keymap::Keymap::from_config(&config.keys).unwrap_or_default();
    app.message = config_error;
    app.history_file = state::history_path();
    if let Some(path) = &app.history_file {
        app.history = state::load_history(path);
    }
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
        self.full = false;
    }

    /// Volta ao modo simples depois de um `restore` (ex.: na volta do editor).
    fn resume(&mut self) {
        if self.supported {
            let _ = execute!(stdout(), PushKeyboardEnhancementFlags(Self::SIMPLE));
        }
    }
}

/// Entrega o terminal a um programa externo (editor, comando) e o pega de
/// volta quando ele termina.
fn run_external(
    terminal: &mut DefaultTerminal,
    keyboard: &mut Keyboard,
    ext: &shell::External,
) -> std::io::Result<std::process::ExitStatus> {
    keyboard.restore();
    terminal::disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, cursor::Show)?;

    let status = std::process::Command::new(&ext.program)
        .args(&ext.args)
        .current_dir(&ext.cwd)
        .status();
    if ext.wait {
        print!("\n[xt] Press Enter to return to xt...");
        let _ = stdout().flush();
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    execute!(stdout(), EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    keyboard.resume();
    terminal.clear()?;
    status
}

fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    keyboard: &mut Keyboard,
) -> anyhow::Result<()> {
    // Sem inotify (limite esgotado, sistema sem suporte), só não há
    // atualização automática.
    let mut watcher = watch::Watcher::new().ok();
    while !app.should_quit {
        keyboard.sync(app)?;
        if let Some(w) = &mut watcher {
            w.watch_only(app.watched_dirs());
        }
        terminal.draw(|frame| ui::draw(frame, app))?;
        // Com trabalho em andamento (log, cópia...), não bloqueia esperando tecla:
        // processa um pedaço e redesenha o progresso.
        if app.busy() && !event::poll(Duration::ZERO)? {
            app.tick(Duration::from_millis(50));
            continue;
        }
        // Parado, espera uma tecla por até meio segundo; nesse meio-tempo,
        // aplica as mudanças vistas no disco (agrupadas, para uma cópia
        // grande feita por outro programa não reler a cada arquivo).
        if !event::poll(Duration::from_millis(500))? {
            if let Some(w) = &watcher {
                app.fs_changed(w.changed_dirs());
            }
            continue;
        }
        // Pressionar, repetir e soltar: o App decide o que usar.
        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }
        if let Some(ext) = app.external.take() {
            let result = run_external(terminal, keyboard, &ext);
            app.after_external(result);
        }
    }
    Ok(())
}
