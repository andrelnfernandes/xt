//! Filespec: o filtro de arquivos do XTree (tecla `F`).
//!
//! Sintaxe: padrões glob separados por espaço, `;` ou `,`.
//! Um `-` na frente exclui: `*.rs *.toml -build*`.
//! Sem nenhum padrão de inclusão, vale `*`. A comparação ignora maiúsculas.

use std::ffi::OsStr;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

pub struct Filespec {
    text: String,
    include: Option<GlobSet>,
    exclude: GlobSet,
}

impl Default for Filespec {
    fn default() -> Self {
        Filespec::parse("*").expect("'*' is a valid filespec")
    }
}

impl Filespec {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut include = GlobSetBuilder::new();
        let mut exclude = GlobSetBuilder::new();
        let mut has_include = false;
        let mut parts = Vec::new();

        for part in text.split([' ', ';', ',']).filter(|p| !p.is_empty()) {
            let (pattern, set) = match part.strip_prefix('-') {
                Some(p) => (p, &mut exclude),
                None => {
                    has_include = true;
                    (part, &mut include)
                }
            };
            let glob = GlobBuilder::new(pattern)
                .case_insensitive(true)
                .literal_separator(true)
                .build()
                .map_err(|e| format!("invalid pattern '{pattern}': {}", e.kind()))?;
            set.add(glob);
            parts.push(part);
        }

        let build = |b: GlobSetBuilder| b.build().map_err(|e| e.to_string());
        Ok(Filespec {
            text: if parts.is_empty() {
                "*".into()
            } else {
                parts.join(" ")
            },
            include: if has_include {
                Some(build(include)?)
            } else {
                None
            },
            exclude: build(exclude)?,
        })
    }

    /// Texto normalizado, para mostrar na tela.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn matches(&self, name: &OsStr) -> bool {
        self.include.as_ref().is_none_or(|s| s.is_match(name)) && !self.exclude.is_match(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(spec: &str, name: &str) -> bool {
        Filespec::parse(spec).unwrap().matches(OsStr::new(name))
    }

    #[test]
    fn default_matches_everything() {
        let f = Filespec::default();
        assert!(f.matches(OsStr::new("anything.txt")));
        assert!(f.matches(OsStr::new(".hidden")));
        assert_eq!(f.text(), "*");
    }

    #[test]
    fn includes_and_excludes() {
        assert!(m("*.rs *.toml", "main.rs"));
        assert!(m("*.rs;*.toml", "Cargo.toml"));
        assert!(!m("*.rs,*.toml", "README.md"));
        assert!(m("-*.o", "main.c"));
        assert!(!m("-*.o", "main.o"));
        assert!(!m("*.c -test*", "test_main.c"));
    }

    #[test]
    fn ignores_case() {
        assert!(m("*.JPG", "photo.jpg"));
        assert!(m("readme*", "README.md"));
    }

    #[test]
    fn normalizes_text() {
        assert_eq!(
            Filespec::parse("  *.rs;;*.md ").unwrap().text(),
            "*.rs *.md"
        );
        assert_eq!(Filespec::parse("").unwrap().text(), "*");
    }

    #[test]
    fn rejects_bad_pattern() {
        assert!(Filespec::parse("[abc").is_err());
    }

    #[test]
    fn works_with_non_utf8_names() {
        use std::os::unix::ffi::OsStrExt;
        let name = OsStr::from_bytes(b"caf\xe9.txt");
        assert!(Filespec::parse("*.txt").unwrap().matches(name));
    }
}
