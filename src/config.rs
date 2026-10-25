//! Configuration file (`~/.config/xylem/config.toml`), color themes and
//! the date and number formats. The configuration screen (F12) saves here.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::RwLock;

use ratatui::style::Color;
use serde::Deserialize;

use crate::keymap::Keymap;
use crate::model::sort::{Sort, SortKey};
use crate::pane::FileDisplay;

/// Commented example, printed by `xy --default-config`. The values are the
/// defaults: a file with only this is the same as having no file.
pub const DEFAULT_CONFIG: &str = r##"# Xylem configuration: ~/.config/xylem/config.toml (or $XDG_CONFIG_HOME/xylem/)
# on Linux, %APPDATA%\xylem\config.toml on Windows. Everything is optional. The
# configuration screen (F12 inside Xylem) changes these values and keeps the comments.

# Theme: "classic" (XTree blue), "dark", "light", "mono" (the terminal's
# own colors), "norton", "solarized", "gruvbox" or "nord".
theme = "classic"

[options]
# Editor for the E key. Empty: $VISUAL, $EDITOR, editor or vi (notepad on
# Windows).
editor = ""
# Program for the O key (open with the default program). Empty: xdg-open
# (start on Windows).
opener = ""
# Dates: "yyyy-mm-dd", "dd/mm/yyyy" or "mm/dd/yyyy".
date_format = "yyyy-mm-dd"
# Numbers: "1,234", "1.234", "1 234" or "1234".
number_format = "1,234"
# Show hidden files and directories (those starting with a dot).
show_hidden = true
# Enter in the files window views the file (instead of going back to the tree).
enter_views = false
# Syntax colors for source code and scripts in the viewer (C toggles them).
syntax_highlight = true
# Initial mode of the files window (Alt+F changes it while running): "names"
# (several columns), "names_size" (columns with size), "details" (one
# column with size, date and permissions), "owner" (size, owner and group,
# permissions), "type" (what each file is, as the `file` command says) or
# "long_names" (one column, names only).
file_display = "details"
# Initial file sort order: "name", "ext", "date" or "size"
# (Alt+S changes it while running).
sort = "name"
sort_descending = false
# Ask before quitting.
confirm_quit = true
# Send deleted files to the system trash / Recycle Bin instead of deleting
# them permanently.
delete_to_trash = false
# To confirm a Prune (Alt+P), type: "prune" (the word PRUNE),
# "name" (the directory name) or "y".
prune_confirmation = "prune"
# Alt+C / Alt+M (copy/move tagged files with their paths): from where each
# file's path is recreated at the destination (F7 changes it at the prompt):
# "full" (from the root of the disk, as in ZTreeWin), "tree" (from the root
# of Xylem's tree), "current" (from the current directory, its name included)
# or "relative" (only what is below the current directory).
copy_paths = "full"
# Don't keep entries starting with "@" in the history (useful for commands
# with passwords in eXecute; the "@" is stripped before running).
history_skip_at = false
# Compare: dates up to 2 seconds apart are equal (FAT/exFAT).
compare_ignore_2s = true
# Compare: a difference of exactly 1 hour counts as equal (daylight saving).
compare_ignore_1h = false
# Re-read the current directory by itself when another program changes it.
auto_refresh = true
# Git commands (Ctrl+G) and each file's Git status in the file list. Not in
# XTree or ZTreeWin, so off by default; needs the git program.
git = false
# Colors of the built-in terminal (Alt+X): "native" (the colors of the
# terminal Xylem runs in), "xylem" (background and text of Xylem's theme),
# "solarized", "gruvbox", "nord", "dracula" or "one_dark".
terminal_theme = "native"
# The built-in terminal keeps a shell history of its own, apart from your
# other terminals' (in ~/.local/state/xylem/, e.g. bash_history; in fish, the
# "xylem" session). Takes effect when the shell starts.
terminal_history = true

# Individual colors, on top of the theme. Values: names ("yellow",
# "lightblue", "darkgray"...), "#rrggbb" or an index from 0 to 255.
# Names: background, text, border, active_border, key, dim, highlight,
# highlight_text, inactive_highlight, error, message, tagged, executable,
# symlink, special.
[colors]
# key = "yellow"
# background = "#1e1e2e"

# Keys: "key" = "command". Keys: letters, "F1".."F12", "enter",
# "esc", "tab", "space", "plus", "minus", arrows ("up"...), "pageup"...,
# with "ctrl+" and/or "alt+" in front. "none" disables the key.
# Commands: see the list with F1 inside Xylem; the names are those in the
# source file keymap.rs (view, copy, copy_tagged, copy_tagged_paths,
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

/// Key remaps: `[keys.any]` applies in both windows, `[keys.tree]` and
/// `[keys.files]` only in one of them (and take priority).
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
    /// Editor for the E key; empty uses the environment variables.
    pub editor: String,
    /// Program for the O key; empty uses xdg-open (start on Windows).
    pub opener: String,
    pub date_format: DateFormat,
    pub number_format: NumberFormat,
    pub show_hidden: bool,
    pub enter_views: bool,
    /// Syntax colors in the viewer (C toggles them while viewing).
    pub syntax_highlight: bool,
    /// Initial mode of the files window (Alt+F changes it while running).
    pub file_display: FileDisplay,
    /// Initial file sort order.
    pub sort: SortKey,
    pub sort_descending: bool,
    pub confirm_quit: bool,
    /// Send deleted files to the system trash / Recycle Bin.
    pub delete_to_trash: bool,
    /// What to type to confirm a Prune.
    pub prune_confirmation: PruneConfirm,
    /// Alt+C / Alt+M: initial path mode.
    pub copy_paths: CopyPaths,
    pub history_skip_at: bool,
    pub compare_ignore_2s: bool,
    pub compare_ignore_1h: bool,
    pub auto_refresh: bool,
    /// Git menu (Ctrl+G) and status markers in the file list.
    pub git: bool,
    /// Colors of the built-in terminal (Alt+X).
    pub terminal_theme: TerminalTheme,
    /// The built-in terminal's shell keeps its own history.
    pub terminal_history: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            editor: String::new(),
            opener: String::new(),
            date_format: DateFormat::Iso,
            number_format: NumberFormat::Comma,
            show_hidden: true,
            enter_views: false,
            syntax_highlight: true,
            file_display: FileDisplay::Details,
            sort: SortKey::Name,
            sort_descending: false,
            confirm_quit: true,
            delete_to_trash: false,
            prune_confirmation: PruneConfirm::Prune,
            copy_paths: CopyPaths::Full,
            history_skip_at: false,
            compare_ignore_2s: true,
            compare_ignore_1h: false,
            auto_refresh: true,
            git: false,
            terminal_theme: TerminalTheme::Native,
            terminal_history: true,
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

    /// The values as they are in the file, for saving.
    fn entries(&self) -> Vec<(&'static str, toml_edit::Value)> {
        use toml_edit::Value;
        let s = |v: &str| Value::from(v);
        vec![
            ("editor", s(&self.editor)),
            ("opener", s(&self.opener)),
            ("date_format", s(self.date_format.as_str())),
            ("number_format", s(self.number_format.as_str())),
            ("show_hidden", Value::from(self.show_hidden)),
            ("enter_views", Value::from(self.enter_views)),
            ("syntax_highlight", Value::from(self.syntax_highlight)),
            ("file_display", s(self.file_display.as_str())),
            ("sort", s(self.sort.as_str())),
            ("sort_descending", Value::from(self.sort_descending)),
            ("confirm_quit", Value::from(self.confirm_quit)),
            ("delete_to_trash", Value::from(self.delete_to_trash)),
            ("prune_confirmation", s(self.prune_confirmation.as_str())),
            ("copy_paths", s(self.copy_paths.as_str())),
            ("history_skip_at", Value::from(self.history_skip_at)),
            ("compare_ignore_2s", Value::from(self.compare_ignore_2s)),
            ("compare_ignore_1h", Value::from(self.compare_ignore_1h)),
            ("auto_refresh", Value::from(self.auto_refresh)),
            ("git", Value::from(self.git)),
            ("terminal_theme", s(self.terminal_theme.as_str())),
            ("terminal_history", Value::from(self.terminal_history)),
        ]
    }

    pub fn formats(&self) -> Formats {
        Formats {
            date: self.date_format,
            number: self.number_format,
        }
    }
}

/// A closed-list option: the text in the file and the next one in the list
/// (the configuration screen cycles through them).
pub trait Choice: Copy + PartialEq + 'static {
    const ALL: &'static [Self];
    fn as_str(self) -> &'static str;

    fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&c| c == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub enum DateFormat {
    #[default]
    #[serde(rename = "yyyy-mm-dd")]
    Iso,
    #[serde(rename = "dd/mm/yyyy")]
    Dmy,
    #[serde(rename = "mm/dd/yyyy")]
    Mdy,
}

impl Choice for DateFormat {
    const ALL: &'static [Self] = &[DateFormat::Iso, DateFormat::Dmy, DateFormat::Mdy];
    fn as_str(self) -> &'static str {
        match self {
            DateFormat::Iso => "yyyy-mm-dd",
            DateFormat::Dmy => "dd/mm/yyyy",
            DateFormat::Mdy => "mm/dd/yyyy",
        }
    }
}

impl DateFormat {
    /// chrono format, with time.
    pub fn pattern(self) -> &'static str {
        match self {
            DateFormat::Iso => "%Y-%m-%d %H:%M",
            DateFormat::Dmy => "%d/%m/%Y %H:%M",
            DateFormat::Mdy => "%m/%d/%Y %H:%M",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub enum NumberFormat {
    #[default]
    #[serde(rename = "1,234")]
    Comma,
    #[serde(rename = "1.234")]
    Dot,
    #[serde(rename = "1 234")]
    Space,
    #[serde(rename = "1234")]
    Plain,
}

impl Choice for NumberFormat {
    const ALL: &'static [Self] = &[
        NumberFormat::Comma,
        NumberFormat::Dot,
        NumberFormat::Space,
        NumberFormat::Plain,
    ];
    fn as_str(self) -> &'static str {
        match self {
            NumberFormat::Comma => "1,234",
            NumberFormat::Dot => "1.234",
            NumberFormat::Space => "1 234",
            NumberFormat::Plain => "1234",
        }
    }
}

impl NumberFormat {
    pub fn separator(self) -> Option<char> {
        match self {
            NumberFormat::Comma => Some(','),
            NumberFormat::Dot => Some('.'),
            NumberFormat::Space => Some(' '),
            NumberFormat::Plain => None,
        }
    }
}

/// Colors of the built-in terminal (Alt+X). The palettes themselves are in
/// `term.rs`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalTheme {
    /// The colors of the terminal Xylem runs in.
    #[default]
    Native,
    /// Background and text of Xylem's theme; the 16 colors stay the terminal's.
    /// "xt" is the name from before Xylem was called Xylem.
    #[serde(alias = "xt")]
    Xylem,
    Solarized,
    Gruvbox,
    Nord,
    Dracula,
    OneDark,
}

impl Choice for TerminalTheme {
    const ALL: &'static [Self] = &[
        TerminalTheme::Native,
        TerminalTheme::Xylem,
        TerminalTheme::Solarized,
        TerminalTheme::Gruvbox,
        TerminalTheme::Nord,
        TerminalTheme::Dracula,
        TerminalTheme::OneDark,
    ];
    fn as_str(self) -> &'static str {
        match self {
            TerminalTheme::Native => "native",
            TerminalTheme::Xylem => "xylem",
            TerminalTheme::Solarized => "solarized",
            TerminalTheme::Gruvbox => "gruvbox",
            TerminalTheme::Nord => "nord",
            TerminalTheme::Dracula => "dracula",
            TerminalTheme::OneDark => "one_dark",
        }
    }
}

/// Alt+C / Alt+M: from where each file's path is recreated at the
/// destination (F7 changes it at the prompt). The first three are
/// ZTreeWin's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyPaths {
    /// From the root of the disk: `/home/me/proj/src/a.rs` → `dest/home/me/proj/src/a.rs`.
    #[default]
    Full,
    /// From the root of Xylem's tree (not in ZTreeWin).
    Tree,
    /// From the current directory, its own name included: `dest/src/a.rs`.
    Current,
    /// Only what is below the current directory: `dest/a.rs`.
    Relative,
}

impl Choice for CopyPaths {
    const ALL: &'static [Self] = &[
        CopyPaths::Full,
        CopyPaths::Tree,
        CopyPaths::Current,
        CopyPaths::Relative,
    ];
    fn as_str(self) -> &'static str {
        match self {
            CopyPaths::Full => "full",
            CopyPaths::Tree => "tree",
            CopyPaths::Current => "current",
            CopyPaths::Relative => "relative",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PruneConfirm {
    /// The word PRUNE.
    #[default]
    Prune,
    /// The directory name.
    Name,
    /// Just Y.
    Y,
}

impl Choice for PruneConfirm {
    const ALL: &'static [Self] = &[PruneConfirm::Prune, PruneConfirm::Name, PruneConfirm::Y];
    fn as_str(self) -> &'static str {
        match self {
            PruneConfirm::Prune => "prune",
            PruneConfirm::Name => "name",
            PruneConfirm::Y => "y",
        }
    }
}

impl Choice for FileDisplay {
    const ALL: &'static [Self] = &[
        FileDisplay::Names,
        FileDisplay::NamesSize,
        FileDisplay::Details,
        FileDisplay::Owner,
        FileDisplay::Type,
        FileDisplay::LongNames,
    ];
    fn as_str(self) -> &'static str {
        match self {
            FileDisplay::Names => "names",
            FileDisplay::NamesSize => "names_size",
            FileDisplay::Details => "details",
            FileDisplay::Owner => "owner",
            FileDisplay::Type => "type",
            FileDisplay::LongNames => "long_names",
        }
    }
}

impl Choice for SortKey {
    const ALL: &'static [Self] = &[SortKey::Name, SortKey::Ext, SortKey::Date, SortKey::Size];
    fn as_str(self) -> &'static str {
        match self {
            SortKey::Name => "name",
            SortKey::Ext => "ext",
            SortKey::Date => "date",
            SortKey::Size => "size",
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        Some(crate::platform::config_dir()?.join("config.toml"))
    }

    pub fn parse(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| e.message().to_string())?;
        Theme::from_config(&config)?;
        Keymap::from_config(&config.keys, &config.options)?;
        Ok(config)
    }

    /// Reads the user's file. With no file, the defaults apply; on error,
    /// too, and the error is returned along so it can be shown.
    pub fn load() -> (Config, Option<String>) {
        let Some(path) = Self::path() else {
            return (Config::default(), None);
        };
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> (Config, Option<String>) {
        let text = match std::fs::read_to_string(path) {
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

    /// Saves the theme and options to `path`, keeping the rest of the file
    /// (comments, colors, keys) as is. With no file, starts from the
    /// commented example.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => DEFAULT_CONFIG.to_string(),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let mut doc: toml_edit::DocumentMut = text
            .parse()
            .map_err(|e| format!("cannot update {}: {e}", path.display()))?;
        doc["theme"] = toml_edit::value(self.theme.as_str());
        if !doc.contains_table("options") {
            doc["options"] = toml_edit::table();
        }
        let options = doc["options"]
            .as_table_mut()
            .ok_or("[options] is not a table")?;
        for (key, value) in self.options.entries() {
            match options.get_mut(key).and_then(|i| i.as_value_mut()) {
                // Replaces only the value: the comment above the line stays.
                Some(old) => {
                    let decor = old.decor().clone();
                    *old = value;
                    *old.decor_mut() = decor;
                }
                None => options[key] = toml_edit::Item::Value(value),
            }
        }
        write_atomic(path, doc.to_string().as_bytes())
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}

fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("toml.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp)?;
    file.write_all(data)?;
    file.sync_all()?;
    std::fs::rename(&tmp, path)
}

/// UI colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// A ready-made theme: name and how to build it.
type NamedTheme = (&'static str, fn() -> Theme);

/// The ready-made themes, in the order they appear in F4.
pub const THEMES: &[NamedTheme] = &[
    ("classic", Theme::classic),
    ("dark", Theme::dark),
    ("light", Theme::light),
    ("mono", Theme::mono),
    ("norton", Theme::norton),
    ("solarized", Theme::solarized),
    ("gruvbox", Theme::gruvbox),
    ("nord", Theme::nord),
];

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
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

    /// The terminal's own colors, with few highlights.
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

    /// Like Norton Commander: blue with cyan text.
    pub fn norton() -> Self {
        Theme {
            text: Color::LightCyan,
            border: Color::LightCyan,
            active_border: Color::White,
            dim: Color::Cyan,
            ..Self::classic()
        }
    }

    pub fn solarized() -> Self {
        Theme {
            background: rgb(0x002b36),
            text: rgb(0x839496),
            border: rgb(0x586e75),
            active_border: rgb(0xb58900),
            key: rgb(0xb58900),
            dim: rgb(0x586e75),
            highlight: rgb(0x268bd2),
            highlight_text: rgb(0xfdf6e3),
            inactive_highlight: rgb(0x073642),
            error: rgb(0xdc322f),
            message: rgb(0xcb4b16),
            tagged: rgb(0xd33682),
            executable: rgb(0x859900),
            symlink: rgb(0x2aa198),
            special: rgb(0x6c71c4),
        }
    }

    pub fn gruvbox() -> Self {
        Theme {
            background: rgb(0x282828),
            text: rgb(0xebdbb2),
            border: rgb(0x665c54),
            active_border: rgb(0xfabd2f),
            key: rgb(0xfabd2f),
            dim: rgb(0x928374),
            highlight: rgb(0x458588),
            highlight_text: rgb(0xfbf1c7),
            inactive_highlight: rgb(0x504945),
            error: rgb(0xfb4934),
            message: rgb(0xfe8019),
            tagged: rgb(0xd3869b),
            executable: rgb(0xb8bb26),
            symlink: rgb(0x8ec07c),
            special: rgb(0xd3869b),
        }
    }

    pub fn nord() -> Self {
        Theme {
            background: rgb(0x2e3440),
            text: rgb(0xd8dee9),
            border: rgb(0x4c566a),
            active_border: rgb(0x88c0d0),
            key: rgb(0xebcb8b),
            dim: rgb(0x616e88),
            highlight: rgb(0x5e81ac),
            highlight_text: rgb(0xeceff4),
            inactive_highlight: rgb(0x434c5e),
            error: rgb(0xbf616a),
            message: rgb(0xd08770),
            tagged: rgb(0xb48ead),
            executable: rgb(0xa3be8c),
            symlink: rgb(0x8fbcbb),
            special: rgb(0xb48ead),
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        THEMES.iter().find(|(n, _)| *n == name).map(|(_, f)| f())
    }

    pub fn from_config(config: &Config) -> Result<Self, String> {
        let mut theme = Self::by_name(&config.theme).ok_or_else(|| {
            let names: Vec<&str> = THEMES.iter().map(|(n, _)| *n).collect();
            format!("unknown theme '{}' ({})", config.theme, names.join(", "))
        })?;
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

/// How dates and numbers appear on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Formats {
    pub date: DateFormat,
    pub number: NumberFormat,
}

// Theme and formats can change while running (configuration screen), and are
// read on every redraw: they live in simple globals.
static THEME: RwLock<Option<Theme>> = RwLock::new(None);
static FORMATS: RwLock<Formats> = RwLock::new(Formats {
    date: DateFormat::Iso,
    number: NumberFormat::Comma,
});

pub fn set_theme(theme: Theme) {
    *THEME.write().unwrap_or_else(|e| e.into_inner()) = Some(theme);
}

/// The theme in use (the classic one, if none was set).
pub fn theme() -> Theme {
    THEME
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or_else(Theme::classic)
}

pub fn set_formats(formats: Formats) {
    *FORMATS.write().unwrap_or_else(|e| e.into_inner()) = formats;
}

pub fn formats() -> Formats {
    *FORMATS.read().unwrap_or_else(|e| e.into_inner())
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
        let t = Theme::from_config(&c).unwrap();
        assert_eq!(t.key, Color::Rgb(0xff, 0x88, 0x00));
        assert_eq!(t.border, Color::Indexed(240));
        assert_eq!(t.background, Theme::dark().background);
        for (name, _) in THEMES {
            assert!(Theme::by_name(name).is_some());
        }
        assert_eq!(Theme::nord().background, Color::Rgb(0x2e, 0x34, 0x40));
    }

    #[test]
    fn parses_option_values() {
        let c = Config::parse(
            "[options]\nsort = \"size\"\nsort_descending = true\ndate_format = \"dd/mm/yyyy\"\nnumber_format = \"1.234\"",
        )
        .unwrap();
        assert_eq!(c.options.initial_sort().label(), "Size↓");
        assert_eq!(c.options.date_format, DateFormat::Dmy);
        assert_eq!(c.options.number_format.separator(), Some('.'));
        assert!(Config::parse("[options]\nsort = \"color\"").is_err());
        assert!(Config::parse("[options]\ndate_format = \"yy\"").is_err());
    }

    #[test]
    fn choices_cycle() {
        assert_eq!(DateFormat::Mdy.next(), DateFormat::Iso);
        assert_eq!(NumberFormat::Comma.next(), NumberFormat::Dot);
        assert_eq!(SortKey::Size.next(), SortKey::Name);
        assert_eq!(PruneConfirm::Y.next().as_str(), "prune");
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
    fn save_keeps_comments_and_other_sections() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("xylem/config.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "# meu comentário\ntheme = \"dark\"\n\n[options]\n# sobre o editor\neditor = \"vim\"\n\n[colors]\nkey = \"red\"\n",
        )
        .unwrap();

        let (mut config, err) = Config::load_from(&path);
        assert!(err.is_none());
        config.theme = "nord".into();
        config.options.editor = "nano".into();
        config.options.show_hidden = false;
        config.save(&path).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# meu comentário"));
        assert!(
            text.contains("# sobre o editor\neditor = \"nano\""),
            "{text}"
        );
        assert!(text.contains("key = \"red\""));
        let (reloaded, err) = Config::load_from(&path);
        assert!(err.is_none(), "{err:?}");
        assert_eq!(reloaded.theme, "nord");
        assert!(!reloaded.options.show_hidden);
        assert_eq!(reloaded.colors["key"], "red");
    }

    #[test]
    fn save_without_file_starts_from_the_example() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("new/config.toml");
        let config = Config {
            theme: "gruvbox".into(),
            ..Config::default()
        };
        config.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# Xylem configuration"));
        assert_eq!(Config::load_from(&path).0, config);
    }

    #[test]
    fn config_path_ends_in_xt_dir() {
        // Only reads the variable; doesn't touch the environment of the other tests.
        let path = Config::path().unwrap();
        assert!(path.ends_with("xylem/config.toml"));
    }
}
