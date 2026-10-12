//! Arquivo de configuração (`~/.config/xt/config.toml`) e temas de cores.

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::OnceLock;

use ratatui::style::Color;
use serde::Deserialize;

use crate::keymap::Keymap;
use crate::model::sort::{Sort, SortKey};

/// Exemplo comentado, impresso por `xt --default-config`. Os valores são os
/// padrões: um arquivo com só isto equivale a não ter arquivo.
pub const DEFAULT_CONFIG: &str = r##"# Configuração do xt: ~/.config/xt/config.toml
# (ou $XDG_CONFIG_HOME/xt/config.toml). Tudo é opcional.

# Tema: "classic" (azul do XTree), "dark", "light" ou "mono" (cores do
# próprio terminal).
theme = "classic"

[options]
# Perguntar antes de sair.
confirm_quit = true
# Ordenação inicial dos arquivos: "name", "ext", "date" ou "size"
# (Alt+S muda durante o uso).
sort = "name"
sort_descending = false
# Para confirmar um Prune (Alt+P), digitar: "prune" (a palavra PRUNE),
# "name" (o nome do diretório) ou "y".
prune_confirmation = "prune"

# Cores individuais, por cima do tema. Valores: nomes ("yellow",
# "lightblue", "darkgray"...), "#rrggbb" ou um índice de 0 a 255.
# Nomes: background, text, border, active_border, key, dim, highlight,
# highlight_text, inactive_highlight, error, message, tagged, executable,
# symlink, special.
[colors]
# key = "yellow"
# background = "#1e1e2e"

# Teclas: "tecla" = "comando". Teclas: letras, "F1".."F12", "enter",
# "esc", "tab", "space", "plus", "minus", setas ("up"...), "pageup"...,
# com "ctrl+" e/ou "alt+" na frente. "none" desliga a tecla.
# Comandos: veja a lista com F1 dentro do xt; os nomes são os do
# arquivo-fonte keymap.rs (view, copy, copy_tagged, copy_tagged_paths,
# move_tagged, relog, compare, execute...).
[keys.any]
# "ctrl+r" = "relog"
# "f3" = "view"

[keys.tree]

[keys.files]
# "enter" = "view"
"##;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: String,
    pub options: Options,
    pub colors: HashMap<String, String>,
    pub keys: Keys,
}

/// Trocas de teclas: `[keys.any]` vale nas duas janelas, `[keys.tree]` e
/// `[keys.files]` só numa delas (e têm prioridade).
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Keys {
    pub any: HashMap<String, String>,
    pub tree: HashMap<String, String>,
    pub files: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: "classic".into(),
            options: Options::default(),
            colors: HashMap::new(),
            keys: Keys::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    pub confirm_quit: bool,
    /// Ordenação inicial dos arquivos.
    pub sort: SortKey,
    pub sort_descending: bool,
    /// O que digitar para confirmar um Prune.
    pub prune_confirmation: PruneConfirm,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PruneConfirm {
    /// A palavra PRUNE.
    #[default]
    Prune,
    /// O nome do diretório.
    Name,
    /// Só Y.
    Y,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            confirm_quit: true,
            sort: SortKey::Name,
            sort_descending: false,
            prune_confirmation: PruneConfirm::Prune,
        }
    }
}

impl Options {
    pub fn initial_sort(&self) -> Sort {
        Sort {
            key: self.sort,
            descending: self.sort_descending,
            path_first: false,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("xt").join("config.toml"))
    }

    pub fn parse(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| e.message().to_string())?;
        Theme::from_config(&config)?;
        Keymap::from_config(&config.keys)?;
        Ok(config)
    }

    /// Lê o arquivo do usuário. Sem arquivo, valem os padrões; com erro,
    /// também, e o erro volta junto para ser mostrado.
    pub fn load() -> (Config, Option<String>) {
        let Some(path) = Self::path() else {
            return (Config::default(), None);
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (Config::default(), None);
            }
            Err(e) => return (Config::default(), Some(format!("{}: {e}", path.display()))),
        };
        match Self::parse(&text) {
            Ok(c) => (c, None),
            Err(e) => (
                Config::default(),
                Some(format!("Config error in {}: {e}", path.display())),
            ),
        }
    }
}

/// Cores da interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub text: Color,
    pub border: Color,
    pub active_border: Color,
    pub key: Color,
    pub dim: Color,
    pub highlight: Color,
    pub highlight_text: Color,
    pub inactive_highlight: Color,
    pub error: Color,
    pub message: Color,
    pub tagged: Color,
    pub executable: Color,
    pub symlink: Color,
    pub special: Color,
}

impl Theme {
    pub fn classic() -> Self {
        Theme {
            background: Color::Blue,
            text: Color::White,
            border: Color::Cyan,
            active_border: Color::Yellow,
            key: Color::Yellow,
            dim: Color::LightBlue,
            highlight: Color::Cyan,
            highlight_text: Color::Black,
            inactive_highlight: Color::Gray,
            error: Color::LightRed,
            message: Color::LightYellow,
            tagged: Color::Yellow,
            executable: Color::LightGreen,
            symlink: Color::LightCyan,
            special: Color::LightMagenta,
        }
    }

    pub fn dark() -> Self {
        Theme {
            background: Color::Black,
            text: Color::Gray,
            border: Color::DarkGray,
            active_border: Color::LightBlue,
            dim: Color::DarkGray,
            highlight: Color::Blue,
            highlight_text: Color::White,
            inactive_highlight: Color::DarkGray,
            ..Self::classic()
        }
    }

    pub fn light() -> Self {
        Theme {
            background: Color::White,
            text: Color::Black,
            border: Color::Blue,
            active_border: Color::Red,
            key: Color::Red,
            dim: Color::DarkGray,
            highlight: Color::LightCyan,
            highlight_text: Color::Black,
            inactive_highlight: Color::Gray,
            error: Color::Red,
            message: Color::Blue,
            tagged: Color::Magenta,
            executable: Color::Green,
            symlink: Color::Cyan,
            special: Color::Magenta,
        }
    }

    /// As cores do próprio terminal, com poucos destaques.
    pub fn mono() -> Self {
        Theme {
            background: Color::Reset,
            text: Color::Reset,
            border: Color::Reset,
            active_border: Color::Reset,
            key: Color::Reset,
            dim: Color::Reset,
            highlight: Color::Gray,
            highlight_text: Color::Black,
            inactive_highlight: Color::DarkGray,
            error: Color::Reset,
            message: Color::Reset,
            tagged: Color::Reset,
            executable: Color::Reset,
            symlink: Color::Reset,
            special: Color::Reset,
        }
    }

    pub fn from_config(config: &Config) -> Result<Self, String> {
        let mut theme = match config.theme.as_str() {
            "classic" => Self::classic(),
            "dark" => Self::dark(),
            "light" => Self::light(),
            "mono" => Self::mono(),
            other => {
                return Err(format!(
                    "unknown theme '{other}' (classic, dark, light, mono)"
                ));
            }
        };
        let mut names: Vec<_> = config.colors.iter().collect();
        names.sort();
        for (name, value) in names {
            let color = Color::from_str(value)
                .map_err(|_| format!("invalid color '{value}' for {name}"))?;
            let slot = match name.as_str() {
                "background" => &mut theme.background,
                "text" => &mut theme.text,
                "border" => &mut theme.border,
                "active_border" => &mut theme.active_border,
                "key" => &mut theme.key,
                "dim" => &mut theme.dim,
                "highlight" => &mut theme.highlight,
                "highlight_text" => &mut theme.highlight_text,
                "inactive_highlight" => &mut theme.inactive_highlight,
                "error" => &mut theme.error,
                "message" => &mut theme.message,
                "tagged" => &mut theme.tagged,
                "executable" => &mut theme.executable,
                "symlink" => &mut theme.symlink,
                "special" => &mut theme.special,
                other => return Err(format!("unknown color name '{other}'")),
            };
            *slot = color;
        }
        Ok(theme)
    }
}

static THEME: OnceLock<Theme> = OnceLock::new();

/// Define o tema, uma vez, na inicialização.
pub fn set_theme(theme: Theme) {
    let _ = THEME.set(theme);
}

/// O tema em uso (o clássico, se nenhum foi definido).
pub fn theme() -> &'static Theme {
    THEME.get_or_init(Theme::classic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_text_means_defaults() {
        assert_eq!(Config::parse(DEFAULT_CONFIG).unwrap(), Config::default());
        assert_eq!(Config::parse("").unwrap(), Config::default());
    }

    #[test]
    fn themes_and_color_overrides() {
        let c = Config::parse(
            "theme = \"dark\"\n[options]\nconfirm_quit = false\n[colors]\nkey = \"#ff8800\"\nborder = \"240\"",
        )
        .unwrap();
        assert!(!c.options.confirm_quit);
        let sorted = Config::parse("[options]\nsort = \"size\"\nsort_descending = true").unwrap();
        assert_eq!(sorted.options.initial_sort().label(), "Size↓");
        assert!(Config::parse("[options]\nsort = \"color\"").is_err());
        let t = Theme::from_config(&c).unwrap();
        assert_eq!(t.key, Color::Rgb(0xff, 0x88, 0x00));
        assert_eq!(t.border, Color::Indexed(240));
        assert_eq!(t.background, Theme::dark().background);
    }

    #[test]
    fn reports_mistakes() {
        assert!(
            Config::parse("theme = \"pink\"")
                .unwrap_err()
                .contains("unknown theme")
        );
        assert!(
            Config::parse("[colors]\nkey = \"nope\"")
                .unwrap_err()
                .contains("invalid color")
        );
        assert!(
            Config::parse("[colors]\nkeys = \"red\"")
                .unwrap_err()
                .contains("unknown color")
        );
        assert!(Config::parse("tema = \"dark\"").is_err(), "unknown field");
        assert!(Config::parse("theme = ").is_err(), "syntax");
        let bad_key = Config::parse("[keys.any]\n\"f3\" = \"explode\"").unwrap_err();
        assert!(bad_key.contains("unknown command"), "{bad_key}");
    }

    #[test]
    fn config_path_ends_in_xt_dir() {
        // Só lê a variável; não mexe no ambiente dos outros testes.
        let path = Config::path().unwrap();
        assert!(path.ends_with("xt/config.toml"));
    }
}
