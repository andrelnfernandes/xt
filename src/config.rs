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

use crate::charset::Charset;
use crate::keymap::Keymap;
use crate::model::sort::{Sort, SortKey};
use crate::pane::FileDisplay;

/// Commented example, printed by `xy --default-config`. The values are the
/// defaults: a file with only this is the same as having no file.
pub const DEFAULT_CONFIG: &str = r##"# Xylem configuration: ~/.config/xylem/config.toml (or $XDG_CONFIG_HOME/xylem/)
# on Linux, %APPDATA%\xylem\config.toml on Windows. Everything is optional. The
# configuration screen (F12 inside Xylem) changes these values and keeps the comments.

# Theme: "classic" (XTree blue), "dark", "light", "mono" (the terminal's
# own colors), "norton", "solarized", "gruvbox", "nord", "slate" (Windows
# Terminal style), "graphite" (macOS dark), "daylight" (macOS light) or
# "vivid" (strong colors, for presentations and screenshots).
theme = "classic"

[options]
# Editor for the E key. Empty: $VISUAL, $EDITOR, editor or vi (notepad on
# Windows).
editor = ""
# Open the editor (E) inside Xylem's frame instead of handing over the screen.
editor_in_frame = true
# Pass Xylem's active color theme to the editor (vi/vim/nvim and framed editors).
editor_theme = true
# Language of the help screen (F1, L switches while open) and `xy --man`:
# "auto" (from $LC_ALL / $LANG), "en", "pt", "es"...
help_language = "auto"
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
# Soft wrap long lines in the viewer (W toggles it while viewing).
viewer_wrap = false
# Show line numbers in the text viewer (# or L toggles them while viewing).
viewer_line_numbers = false
# Show .gz/.xz/.zst/.bz2 files decompressed in the viewer.
viewer_decompress = true
# Charset of files that are not UTF-8 or UTF-16 (O in the viewer changes the current file's codepage):
# "auto" (guess CP850 or Windows-1252), "cp850", "cp1252" or "cp437".
legacy_charset = "auto"
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
# symlink, special, bar_low, bar_mid, bar_high (the bars of Alt+K Percent
# and Alt+W, by share: up to 10%, up to 40%, more).
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

# Editor for the E key by file name, the first match winning: name patterns
# (as in Filespec) and a command, with %f for the file (or at the end). An
# empty command is the usual editor. "binary" is for the other binary files,
# which E does not open in a text editor.
[editors]
# "*.png *.jpg *.jpeg" = "gimp %f"
# binary = "hexedit"
"##;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: String,
    pub options: Options,
    pub colors: HashMap<String, String>,
    pub keys: Keys,
    pub editors: crate::editors::Table,
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
            editors: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// Editor for the E key; empty uses the environment variables.
    pub editor: String,
    /// Open the editor (E) inside Xylem's frame.
    pub editor_in_frame: bool,
    /// Apply Xylem's active theme to the editor (vi/vim/nvim and framed editors).
    pub editor_theme: bool,
    /// Language of the help (F1) and `xy --man`: `"auto"` or an ISO code (`"en"`, `"pt"`, `"es"`, ...).
    pub help_language: String,
    /// Program for the O key; empty uses xdg-open (start on Windows).
    pub opener: String,
    pub date_format: DateFormat,
    pub number_format: NumberFormat,
    pub show_hidden: bool,
    pub enter_views: bool,
    /// Syntax colors in the viewer (C toggles them while viewing).
    pub syntax_highlight: bool,
    /// Soft wrap in the viewer (W toggles while viewing).
    pub viewer_wrap: bool,
    /// Line numbers in the text viewer (# / L toggles while viewing).
    pub viewer_line_numbers: bool,
    /// Show .gz/.xz/.zst/.bz2 files decompressed in the viewer and in
    /// Autoview (F12, J); off shows their raw bytes.
    pub viewer_decompress: bool,
    /// Charset of viewer files that are not UTF-8 or UTF-16.
    pub legacy_charset: LegacyCharset,
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
            editor_in_frame: true,
            editor_theme: true,
            help_language: "auto".into(),
            opener: String::new(),
            date_format: DateFormat::Iso,
            number_format: NumberFormat::Comma,
            show_hidden: true,
            enter_views: false,
            syntax_highlight: true,
            viewer_wrap: false,
            viewer_line_numbers: false,
            viewer_decompress: true,
            legacy_charset: LegacyCharset::Auto,
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
            ("editor_in_frame", Value::from(self.editor_in_frame)),
            ("editor_theme", Value::from(self.editor_theme)),
            ("help_language", s(&self.help_language)),
            ("opener", s(&self.opener)),
            ("date_format", s(self.date_format.as_str())),
            ("number_format", s(self.number_format.as_str())),
            ("show_hidden", Value::from(self.show_hidden)),
            ("enter_views", Value::from(self.enter_views)),
            ("syntax_highlight", Value::from(self.syntax_highlight)),
            ("viewer_wrap", Value::from(self.viewer_wrap)),
            ("viewer_line_numbers", Value::from(self.viewer_line_numbers)),
            ("viewer_decompress", Value::from(self.viewer_decompress)),
            ("legacy_charset", s(self.legacy_charset.as_str())),
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LegacyCharset {
    /// Guess between CP850 and Windows-1252.
    #[default]
    Auto,
    Cp850,
    Cp1252,
    Cp437,
}

impl LegacyCharset {
    /// The fixed charset, or `None` to guess.
    pub fn fallback(self) -> Option<Charset> {
        match self {
            LegacyCharset::Auto => None,
            LegacyCharset::Cp850 => Some(Charset::Cp850),
            LegacyCharset::Cp1252 => Some(Charset::Cp1252),
            LegacyCharset::Cp437 => Some(Charset::Cp437),
        }
    }
}

impl Choice for LegacyCharset {
    const ALL: &'static [Self] = &[
        LegacyCharset::Auto,
        LegacyCharset::Cp850,
        LegacyCharset::Cp1252,
        LegacyCharset::Cp437,
    ];
    fn as_str(self) -> &'static str {
        match self {
            LegacyCharset::Auto => "auto",
            LegacyCharset::Cp850 => "cp850",
            LegacyCharset::Cp1252 => "cp1252",
            LegacyCharset::Cp437 => "cp437",
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
        crate::editors::Editors::from_config(&config.editors)?;
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
    /// The bars of Alt+K Percent and Alt+W, by how much of the whole they
    /// take: up to 10%, up to 40%, more.
    pub bar_low: Color,
    pub bar_mid: Color,
    pub bar_high: Color,
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
    ("slate", Theme::slate),
    ("graphite", Theme::graphite),
    ("daylight", Theme::daylight),
    ("vivid", Theme::vivid),
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
            bar_low: Color::LightGreen,
            bar_mid: Color::Yellow,
            bar_high: Color::LightRed,
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
            bar_low: Color::Green,
            bar_mid: rgb(0xb36b00),
            bar_high: Color::Red,
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
            bar_low: Color::Reset,
            bar_mid: Color::Reset,
            bar_high: Color::Reset,
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
            bar_low: rgb(0x859900),
            bar_mid: rgb(0xb58900),
            bar_high: rgb(0xdc322f),
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
            bar_low: rgb(0xb8bb26),
            bar_mid: rgb(0xfabd2f),
            bar_high: rgb(0xfb4934),
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
            bar_low: rgb(0xa3be8c),
            bar_mid: rgb(0xebcb8b),
            bar_high: rgb(0xbf616a),
        }
    }

    /// In the style of Windows Terminal: the "Campbell" colors on a Windows
    /// 11 dark surface, with the accent blue for the selection.
    pub fn slate() -> Self {
        Theme {
            background: rgb(0x202020),
            text: rgb(0xcccccc),
            border: rgb(0x484848),
            active_border: rgb(0x60cdff),
            key: rgb(0x60cdff),
            dim: rgb(0x767676),
            highlight: rgb(0x0078d4),
            highlight_text: rgb(0xffffff),
            inactive_highlight: rgb(0x3a3a3a),
            error: rgb(0xe74856),
            message: rgb(0xf9f1a5),
            tagged: rgb(0xf9f1a5),
            executable: rgb(0x16c60c),
            symlink: rgb(0x61d6d6),
            special: rgb(0xb4009e),
            bar_low: rgb(0x16c60c),
            bar_mid: rgb(0xc19c00),
            bar_high: rgb(0xe74856),
        }
    }

    /// In the style of macOS in dark mode: graphite gray and the system
    /// colors, with the accent blue for the selection.
    pub fn graphite() -> Self {
        Theme {
            background: rgb(0x1e1e1e),
            text: rgb(0xdddddd),
            border: rgb(0x48484a),
            active_border: rgb(0x0a84ff),
            key: rgb(0x409cff),
            dim: rgb(0x8e8e93),
            highlight: rgb(0x0058d0),
            highlight_text: rgb(0xffffff),
            inactive_highlight: rgb(0x464646),
            error: rgb(0xff453a),
            message: rgb(0xff9f0a),
            tagged: rgb(0xffd60a),
            executable: rgb(0x30d158),
            symlink: rgb(0x64d2ff),
            special: rgb(0xbf5af2),
            bar_low: rgb(0x30d158),
            bar_mid: rgb(0xffd60a),
            bar_high: rgb(0xff453a),
        }
    }

    /// In the style of macOS in light mode.
    pub fn daylight() -> Self {
        Theme {
            background: rgb(0xffffff),
            text: rgb(0x1d1d1f),
            border: rgb(0xaeaeb2),
            active_border: rgb(0x007aff),
            key: rgb(0x0062cc),
            dim: rgb(0x8e8e93),
            highlight: rgb(0x0063e1),
            highlight_text: rgb(0xffffff),
            inactive_highlight: rgb(0x6e6e73),
            error: rgb(0xd70015),
            message: rgb(0xc93400),
            tagged: rgb(0xc93400),
            executable: rgb(0x248a3d),
            symlink: rgb(0x0071a4),
            special: rgb(0x8944ab),
            bar_low: rgb(0x248a3d),
            bar_mid: rgb(0xb36b00),
            bar_high: rgb(0xd70015),
        }
    }

    /// Strong colors on a deep indigo, for presentations, videos and
    /// screenshots: magenta selection, cyan and lime, a sun yellow for the
    /// keys; the bars of Alt+K and Alt+W go lime, yellow and coral.
    pub fn vivid() -> Self {
        Theme {
            background: rgb(0x1a1238),
            text: rgb(0xf4f1ff),
            border: rgb(0x6c5ce7),
            active_border: rgb(0x00e5ff),
            key: rgb(0xffd400),
            dim: rgb(0xa79bd8),
            highlight: rgb(0xff2e93),
            highlight_text: rgb(0xffffff),
            inactive_highlight: rgb(0x4b3a8f),
            error: rgb(0xff5e5b),
            message: rgb(0x00e5ff),
            tagged: rgb(0xff9f1c),
            executable: rgb(0x7cff6b),
            symlink: rgb(0x00e5ff),
            special: rgb(0xc77dff),
            bar_low: rgb(0x7cff6b),
            bar_mid: rgb(0xffd400),
            bar_high: rgb(0xff5e5b),
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
                "bar_low" => &mut theme.bar_low,
                "bar_mid" => &mut theme.bar_mid,
                "bar_high" => &mut theme.bar_high,
                other => return Err(format!("unknown color name '{other}'")),
            };
            *slot = color;
        }
        Ok(theme)
    }

    /// Generates a Vim/Neovim script (`-S`) that applies this theme's palette.
    /// When `in_frame` is true, normal background cells stay `NONE` (`Reset`)
    /// so `term::apply_theme` paints them with Xylem's exact background color.
    pub fn vim_script(&self, in_frame: bool) -> String {
        let is_rgb =
            matches!(self.background, Color::Rgb(..)) || matches!(self.text, Color::Rgb(..));
        let mut out = String::from(
            "\" Generated by Xylem to match the active theme.\n\
             if !exists(\"g:syntax_on\")\n\
               silent! syntax enable\n\
             endif\n",
        );
        if is_rgb {
            out.push_str(
                "if !has('nvim')\n\
                   silent! let &t_8f = \"\\<Esc>[38;2;%lu;%lu;%lum\"\n\
                   silent! let &t_8b = \"\\<Esc>[48;2;%lu;%lu;%lum\"\n\
                 endif\n\
                 silent! set termguicolors\n",
            );
        } else {
            out.push_str("silent! set notermguicolors\n");
        }
        let bg = if in_frame {
            Color::Reset
        } else {
            self.background
        };
        let normal_fg = if in_frame { Color::Reset } else { self.text };
        let hi = |out: &mut String, groups: &[&str], fg: Color, bg: Color, bold: bool| {
            let (cfg, gfg) = vim_color(fg);
            let (cbg, gbg) = vim_color(bg);
            let attr = if bold { "bold" } else { "NONE" };
            for g in groups {
                out.push_str(&format!(
                    "silent! hi {g} ctermfg={cfg} ctermbg={cbg} guifg={gfg} guibg={gbg} cterm={attr} gui={attr}\n"
                ));
            }
        };
        hi(
            &mut out,
            &["Normal", "NormalNC", "SignColumn", "FoldColumn"],
            normal_fg,
            bg,
            false,
        );
        hi(
            &mut out,
            &["EndOfBuffer", "NonText", "SpecialKey", "LineNr", "Comment"],
            self.dim,
            bg,
            false,
        );
        hi(
            &mut out,
            &[
                "CursorLineNr",
                "Statement",
                "Keyword",
                "Conditional",
                "Repeat",
                "Label",
                "Operator",
                "Title",
                "Question",
                "MoreMsg",
                "ModeMsg",
            ],
            self.key,
            bg,
            true,
        );
        hi(
            &mut out,
            &[
                "Type",
                "StorageClass",
                "Structure",
                "Typedef",
                "Identifier",
                "Function",
            ],
            self.symlink,
            bg,
            false,
        );
        hi(
            &mut out,
            &[
                "String",
                "Character",
                "Number",
                "Boolean",
                "Float",
                "Constant",
                "PreProc",
                "Include",
                "Define",
                "Macro",
            ],
            self.executable,
            bg,
            false,
        );
        hi(&mut out, &["Special", "Directory"], self.special, bg, false);
        hi(
            &mut out,
            &[
                "Visual",
                "Search",
                "IncSearch",
                "CurSearch",
                "PmenuSel",
                "WildMenu",
            ],
            self.highlight_text,
            self.highlight,
            false,
        );
        hi(
            &mut out,
            &["StatusLine"],
            self.highlight_text,
            self.highlight,
            true,
        );
        hi(
            &mut out,
            &["StatusLineNC", "Pmenu", "Folded"],
            self.text,
            self.inactive_highlight,
            false,
        );
        hi(
            &mut out,
            &["VertSplit", "WinSeparator"],
            self.border,
            bg,
            false,
        );
        hi(
            &mut out,
            &["ErrorMsg", "Error", "Todo", "WarningMsg"],
            self.error,
            bg,
            true,
        );
        out
    }
}

fn vim_color(c: Color) -> (String, String) {
    match c {
        Color::Reset => ("NONE".into(), "NONE".into()),
        Color::Black => ("0".into(), "#000000".into()),
        Color::Red => ("1".into(), "#aa0000".into()),
        Color::Green => ("2".into(), "#00aa00".into()),
        Color::Yellow => ("3".into(), "#aa5500".into()),
        Color::Blue => ("4".into(), "#0000aa".into()),
        Color::Magenta => ("5".into(), "#aa00aa".into()),
        Color::Cyan => ("6".into(), "#00aaaa".into()),
        Color::Gray => ("7".into(), "#aaaaaa".into()),
        Color::DarkGray => ("8".into(), "#555555".into()),
        Color::LightRed => ("9".into(), "#ff5555".into()),
        Color::LightGreen => ("10".into(), "#55ff55".into()),
        Color::LightYellow => ("11".into(), "#ffff55".into()),
        Color::LightBlue => ("12".into(), "#5555ff".into()),
        Color::LightMagenta => ("13".into(), "#ff55ff".into()),
        Color::LightCyan => ("14".into(), "#55ffff".into()),
        Color::White => ("15".into(), "#ffffff".into()),
        Color::Indexed(i) => (i.to_string(), "NONE".into()),
        Color::Rgb(r, g, b) => ("NONE".into(), format!("#{r:02x}{g:02x}{b:02x}")),
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
        assert_eq!(
            Theme::by_name("graphite"),
            Some(Theme::graphite()),
            "new themes are in THEMES"
        );
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
    fn legacy_charset_is_read() {
        assert_eq!(Options::default().legacy_charset, LegacyCharset::Auto);
        let c = Config::parse("[options]\nlegacy_charset = \"cp850\"").unwrap();
        assert_eq!(c.options.legacy_charset, LegacyCharset::Cp850);
        assert_eq!(
            c.options.legacy_charset.fallback(),
            Some(crate::charset::Charset::Cp850)
        );
        assert_eq!(LegacyCharset::Auto.fallback(), None);
        assert!(Config::parse("[options]\nlegacy_charset = \"latin1\"").is_err());
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

    #[test]
    fn vim_script_matches_theme_and_frame_mode() {
        let classic = Theme::by_name("classic").unwrap();
        let framed = classic.vim_script(true);
        assert!(framed.contains("hi Normal ctermfg=NONE ctermbg=NONE"));
        assert!(framed.contains("hi LineNr ctermfg=12 ctermbg=NONE"));
        assert!(!framed.contains("set termguicolors"));

        let standalone = classic.vim_script(false);
        assert!(standalone.contains("hi Normal ctermfg=15 ctermbg=4"));

        let nord = Theme::by_name("nord").unwrap();
        let rgb_framed = nord.vim_script(true);
        assert!(rgb_framed.contains("set termguicolors"));
        assert!(rgb_framed.contains("guibg=NONE"));
    }
}
