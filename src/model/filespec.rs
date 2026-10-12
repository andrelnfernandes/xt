//! Filespec: o filtro de arquivos do XTree (tecla `F`).
//!
//! Especificações separadas por espaço, `;` ou `,` (entre aspas, um nome
//! pode ter espaços). Três tipos, como no ZTreeWin:
//!
//! - **nome**: glob (`*.rs`, `a?c*`, `[abc]*`), sem diferenciar maiúsculas;
//! - **data**: `<`, `=`, `>`, `<=`, `>=` ou `<>` e uma data (`>01/05/2026`,
//!   `=TODAY`, `>TODAY-7`);
//! - **tamanho**: o mesmo com `s` (`>=s200000`, `<s10K`, `>s1G`).
//!
//! Um `-` na frente nega a especificação. Dentro de cada tipo, as positivas
//! valem com "ou" (exceto `>a` logo seguido de `<b`, que forma a faixa entre
//! as duas) e as negativas com "e não"; entre tipos, vale "e".

use std::ffi::OsStr;
use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDate};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use super::tree::FileEntry;

/// Ordem de dia, mês e ano nas datas digitadas (a do formato de data
/// configurado). `aaaa-mm-dd` é sempre aceito.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DateOrder {
    #[default]
    Ymd,
    Dmy,
    Mdy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Less,
    LessEq,
    Equal,
    GreaterEq,
    Greater,
    NotEqual,
}

impl Op {
    fn holds(self, a: i64, b: i64) -> bool {
        match self {
            Op::Less => a < b,
            Op::LessEq => a <= b,
            Op::Equal => a == b,
            Op::GreaterEq => a >= b,
            Op::Greater => a > b,
            Op::NotEqual => a != b,
        }
    }

    /// `-<` é `>=`, `-=` é `<>`, e assim por diante.
    fn negated(self) -> Op {
        match self {
            Op::Less => Op::GreaterEq,
            Op::LessEq => Op::Greater,
            Op::Equal => Op::NotEqual,
            Op::GreaterEq => Op::Less,
            Op::Greater => Op::LessEq,
            Op::NotEqual => Op::Equal,
        }
    }
}

/// Uma condição de data (dias desde 1970, no fuso local) ou de tamanho.
#[derive(Debug, Clone, Copy)]
struct Cond {
    op: Op,
    value: i64,
}

/// Condições de um tipo (data ou tamanho), já agrupadas.
#[derive(Debug, Default)]
struct Conds {
    /// Positivas: basta uma (cada uma pode ser uma faixa de duas).
    any: Vec<Vec<Cond>>,
    /// `<>`: nenhuma pode valer igualdade.
    none: Vec<Cond>,
}

impl Conds {
    fn from_list(list: Vec<Cond>) -> Self {
        let mut conds = Conds::default();
        let mut iter = list.into_iter().peekable();
        while let Some(c) = iter.next() {
            if c.op == Op::NotEqual {
                conds.none.push(c);
                continue;
            }
            // ">a" seguido de "<b" (b > a): faixa, as duas valem juntas.
            let lower = matches!(c.op, Op::Greater | Op::GreaterEq);
            if lower
                && let Some(next) = iter.peek()
                && matches!(next.op, Op::Less | Op::LessEq)
                && next.value > c.value
            {
                let next = iter.next().unwrap();
                conds.any.push(vec![c, next]);
            } else {
                conds.any.push(vec![c]);
            }
        }
        conds
    }

    fn matches(&self, value: i64) -> bool {
        let positive = self.any.is_empty()
            || self
                .any
                .iter()
                .any(|g| g.iter().all(|c| c.op.holds(value, c.value)));
        positive && self.none.iter().all(|c| c.op.holds(value, c.value))
    }

    fn is_empty(&self) -> bool {
        self.any.is_empty() && self.none.is_empty()
    }
}

pub struct Filespec {
    text: String,
    include: Option<GlobSet>,
    exclude: GlobSet,
    dates: Conds,
    sizes: Conds,
}

impl Default for Filespec {
    fn default() -> Self {
        Filespec::parse("*").expect("'*' is a valid filespec")
    }
}

impl Filespec {
    pub fn parse(text: &str) -> Result<Self, String> {
        Self::parse_with(text, DateOrder::default())
    }

    pub fn parse_with(text: &str, order: DateOrder) -> Result<Self, String> {
        let mut include = GlobSetBuilder::new();
        let mut exclude = GlobSetBuilder::new();
        let mut has_include = false;
        let (mut dates, mut sizes) = (Vec::new(), Vec::new());
        let mut parts = Vec::new();

        for Token {
            text: part,
            quoted,
            negated,
        } in tokenize(text)
        {
            let rest = part.as_str();
            if !quoted && rest.starts_with(['<', '=', '>']) {
                let (op, value) = split_op(rest).ok_or_else(|| format!("invalid spec '{part}'"))?;
                let op = if negated { op.negated() } else { op };
                match value.strip_prefix(['s', 'S']) {
                    Some(size) => sizes.push(Cond {
                        op,
                        value: parse_size(size).ok_or_else(|| format!("invalid size '{size}'"))?,
                    }),
                    None => dates.push(Cond {
                        op,
                        value: parse_date(value, order)
                            .ok_or_else(|| format!("invalid date '{value}'"))?,
                    }),
                }
                parts.push(format!("{}{part}", if negated { "-" } else { "" }));
                continue;
            }
            let set = if negated {
                &mut exclude
            } else {
                has_include = true;
                &mut include
            };
            let glob = GlobBuilder::new(rest)
                .case_insensitive(true)
                .literal_separator(true)
                .build()
                .map_err(|e| format!("invalid pattern '{rest}': {}", e.kind()))?;
            set.add(glob);
            let minus = if negated { "-" } else { "" };
            parts.push(if quoted {
                format!("{minus}\"{rest}\"")
            } else {
                format!("{minus}{rest}")
            });
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
            dates: Conds::from_list(dates),
            sizes: Conds::from_list(sizes),
        })
    }

    /// Texto normalizado, para mostrar na tela.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Só a parte dos nomes.
    pub fn matches(&self, name: &OsStr) -> bool {
        self.include.as_ref().is_none_or(|s| s.is_match(name)) && !self.exclude.is_match(name)
    }

    /// Nome, data e tamanho.
    pub fn matches_entry(&self, f: &FileEntry) -> bool {
        if !self.matches(&f.name) {
            return false;
        }
        if !self.dates.is_empty() {
            let Some(day) = f.modified.map(local_day) else {
                return false;
            };
            if !self.dates.matches(day) {
                return false;
            }
        }
        self.sizes.matches(f.size as i64)
    }
}

/// Uma especificação: o texto (sem o `-` de negação nem as aspas).
struct Token {
    text: String,
    quoted: bool,
    negated: bool,
}

/// Divide o texto em especificações. Entre aspas, separadores e um `-` no
/// começo são parte do nome; o `-` só nega fora delas (`-"a b*"`).
fn tokenize(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut in_quotes, mut quoted, mut negated) = (false, false, false);
    let mut flush = |cur: &mut String, quoted: &mut bool, negated: &mut bool| {
        if !cur.is_empty() {
            out.push(Token {
                text: std::mem::take(cur),
                quoted: *quoted,
                negated: *negated,
            });
        }
        (*quoted, *negated) = (false, false);
    };
    for c in text.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                quoted = true;
            }
            ' ' | ';' | ',' if !in_quotes => flush(&mut cur, &mut quoted, &mut negated),
            '-' if !in_quotes && !quoted && cur.is_empty() && !negated => negated = true,
            c => cur.push(c),
        }
    }
    flush(&mut cur, &mut quoted, &mut negated);
    out
}

fn split_op(s: &str) -> Option<(Op, &str)> {
    let ops = [
        ("<>", Op::NotEqual),
        ("<=", Op::LessEq),
        ("=<", Op::LessEq),
        (">=", Op::GreaterEq),
        ("=>", Op::GreaterEq),
        ("<", Op::Less),
        (">", Op::Greater),
        ("=", Op::Equal),
    ];
    ops.iter()
        .find_map(|(t, op)| s.strip_prefix(t).map(|rest| (*op, rest)))
        .filter(|(_, rest)| !rest.is_empty())
}

/// "200000", "10K", "1.5M", "2G" (K = 1024).
fn parse_size(s: &str) -> Option<i64> {
    let s = s.trim();
    let (num, mult) = match s.chars().last()?.to_ascii_uppercase() {
        'K' => (&s[..s.len() - 1], 1024.0),
        'M' => (&s[..s.len() - 1], 1024.0 * 1024.0),
        'G' => (&s[..s.len() - 1], 1024.0 * 1024.0 * 1024.0),
        _ => (s, 1.0),
    };
    let n: f64 = num.parse().ok().filter(|n: &f64| *n >= 0.0)?;
    Some((n * mult) as i64)
}

/// Dia (desde 1970) de uma data digitada: `TODAY`, `TODAY-n`, ou dia,
/// mês e ano na ordem configurada, separados por `/`, `.` ou `-`.
fn parse_date(s: &str, order: DateOrder) -> Option<i64> {
    let upper = s.to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix("TODAY") {
        let back: i64 = match rest.strip_prefix('-') {
            Some(n) => n.parse().ok()?,
            None if rest.is_empty() => 0,
            None => return None,
        };
        return Some(day_number(Local::now().date_naive()) - back);
    }
    let nums: Vec<&str> = s.split(['/', '.', '-']).collect();
    let [a, b, c] = nums[..] else {
        return None;
    };
    let (y, m, d) = if a.len() == 4 {
        (a, b, c)
    } else {
        match order {
            DateOrder::Ymd => (a, b, c),
            DateOrder::Dmy => (c, b, a),
            DateOrder::Mdy => (c, a, b),
        }
    };
    let mut year: i32 = y.parse().ok()?;
    match y.len() {
        2 => year += if year < 70 { 2000 } else { 1900 },
        4 => {}
        _ => return None,
    }
    let date = NaiveDate::from_ymd_opt(year, m.parse().ok()?, d.parse().ok()?)?;
    Some(day_number(date))
}

fn day_number(d: NaiveDate) -> i64 {
    d.signed_duration_since(NaiveDate::default()).num_days()
}

/// Dia local de uma data de modificação.
fn local_day(t: SystemTime) -> i64 {
    day_number(DateTime::<Local>::from(t).date_naive())
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

    fn file(name: &str, size: u64, date: &str) -> FileEntry {
        let day = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        let local = day
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap();
        FileEntry {
            name: name.into(),
            size,
            modified: Some(local.into()),
            mode: 0o644,
            kind: crate::model::tree::FileKind::File,
            tagged: false,
        }
    }

    fn me(spec: &str, f: &FileEntry) -> bool {
        Filespec::parse_with(spec, DateOrder::Dmy)
            .unwrap()
            .matches_entry(f)
    }

    #[test]
    fn size_specs() {
        let small = file("a", 100, "2026-05-10");
        let big = file("b", 300_000, "2026-05-10");
        assert!(me("=>s200000", &big) && !me("=>s200000", &small));
        assert!(me("<s1K", &small) && !me("<s1K", &big));
        assert!(me(">s0.1M", &big));
        assert!(me("-<s200000", &big), "-< is >=");
        assert!(!me("<>s100", &small));
        // Faixa: >a seguido de <b.
        assert!(me(">s50,<s200", &small) && !me(">s50,<s200", &big));
        assert!(Filespec::parse("<sabc").is_err());
    }

    #[test]
    fn date_specs_follow_configured_order() {
        let may10 = file("x.txt", 1, "2026-05-10");
        let june = file("y.txt", 1, "2026-06-02");
        // Maio de 2026, em dd/mm/aaaa (e aaaa-mm-dd sempre vale).
        assert!(me("=>01/05/2026,=<31/05/2026", &may10));
        assert!(!me("=>01/05/2026,=<31/05/2026", &june));
        assert!(me(">2026-05-31", &june));
        assert!(me("=10.05.26", &may10));
        // Fora de maio: "<1/5" ou ">31/5".
        assert!(me("<01/05/2026,>31/05/2026", &june));
        assert!(!me("<01/05/2026,>31/05/2026", &may10));
        let mdy = Filespec::parse_with("=05/10/2026", DateOrder::Mdy).unwrap();
        assert!(mdy.matches_entry(&may10));
        assert!(Filespec::parse("=31/02/2026").is_err());
    }

    #[test]
    fn today_and_combined_types() {
        let now = FileEntry {
            modified: Some(SystemTime::now()),
            ..file("n.txt", 10, "2026-01-01")
        };
        let old = file("o.txt", 10, "2001-01-01");
        assert!(me("=TODAY", &now) && !me("=TODAY", &old));
        assert!(me(">TODAY-7", &now) && me("<TODAY-7", &old));
        // Nome, data e tamanho juntos: "e".
        assert!(me("*.txt,=TODAY,<s100", &now));
        assert!(!me("*.rs,=TODAY", &now));
        assert!(!me("*.txt,=TODAY,>s100", &now));
    }

    #[test]
    fn quoted_names_keep_spaces() {
        assert!(m("\"meu arquivo*\"", "meu arquivo.txt"));
        assert!(!m("\"meu arquivo*\"", "meu.txt"));
        assert!(m("-\"a b*\"", "c.txt"));
        assert!(!m("-\"a b*\"", "a b.txt"));
        // Entre aspas, o hífen é parte do nome.
        assert!(m("\"-*.txt\"", "-notas.txt"));
        assert!(!m("\"-*.txt\"", "notas.txt"));
        assert_eq!(
            Filespec::parse("-\"a b*\" -*.o").unwrap().text(),
            "-\"a b*\" -*.o"
        );
    }

    #[test]
    fn works_with_non_utf8_names() {
        use std::os::unix::ffi::OsStrExt;
        let name = OsStr::from_bytes(b"caf\xe9.txt");
        assert!(Filespec::parse("*.txt").unwrap().matches(name));
    }
}
