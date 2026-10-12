//! Comparação de um arquivo com o seu correspondente em outro diretório.

use std::fs::Metadata;
use std::time::SystemTime;

/// Quais diferenças de data contam como "mesma data".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tolerance {
    /// Até 2 segundos: FAT/exFAT guardam a hora com essa precisão.
    pub two_seconds: bool,
    /// Exatamente 1 hora: mudança de horário de verão.
    pub one_hour: bool,
}

impl Tolerance {
    fn same_time(self, a: u64, b: u64) -> bool {
        let diff = a.abs_diff(b);
        let close = |d: u64| diff == d || (self.two_seconds && diff.abs_diff(d) <= 2);
        close(0) || (self.one_hour && close(3600))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// Não existe do outro lado.
    Unique,
    Newer,
    Older,
    /// Mesmo tamanho e mesma data.
    Identical,
    /// Mesma data, tamanho diferente (ou o outro lado é um diretório).
    Different,
}

/// O que marcar numa comparação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareMode {
    Unique,
    Newer,
    Older,
    Identical,
    /// Tudo o que existe dos dois lados e não é idêntico.
    Different,
    /// Únicos + mais novos: o que falta copiar para atualizar o outro lado.
    AllNew,
}

impl CompareMode {
    pub fn from_key(c: char) -> Option<Self> {
        Some(match c.to_ascii_lowercase() {
            'u' => CompareMode::Unique,
            'n' => CompareMode::Newer,
            'o' => CompareMode::Older,
            'i' => CompareMode::Identical,
            'd' => CompareMode::Different,
            'a' => CompareMode::AllNew,
            _ => return None,
        })
    }

    pub fn describe(self) -> &'static str {
        match self {
            CompareMode::Unique => "unique",
            CompareMode::Newer => "newer",
            CompareMode::Older => "older",
            CompareMode::Identical => "identical",
            CompareMode::Different => "different",
            CompareMode::AllNew => "unique or newer",
        }
    }

    pub fn matches(self, r: Relation) -> bool {
        use Relation as R;
        match self {
            CompareMode::Unique => r == R::Unique,
            CompareMode::Newer => r == R::Newer,
            CompareMode::Older => r == R::Older,
            CompareMode::Identical => r == R::Identical,
            CompareMode::Different => matches!(r, R::Newer | R::Older | R::Different),
            CompareMode::AllNew => matches!(r, R::Unique | R::Newer),
        }
    }
}

/// Relação entre o arquivo de origem e o do outro lado (`other`).
pub fn relation(
    size: u64,
    modified: Option<SystemTime>,
    other: Option<&Metadata>,
    tolerance: Tolerance,
) -> Relation {
    let Some(other) = other else {
        return Relation::Unique;
    };
    if other.is_dir() {
        return Relation::Different;
    }
    let secs = |t: Option<SystemTime>| {
        t.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs())
    };
    let (mine, theirs) = (secs(modified), secs(other.modified().ok()));
    if tolerance.same_time(mine, theirs) {
        if size == other.len() {
            Relation::Identical
        } else {
            Relation::Different
        }
    } else if mine > theirs {
        Relation::Newer
    } else {
        Relation::Older
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::time::Duration;

    fn file_at(dir: &std::path::Path, name: &str, content: &str, secs: u64) -> Metadata {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(t)
            .unwrap();
        fs::metadata(path).unwrap()
    }

    fn at(secs: u64) -> Option<SystemTime> {
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
    }

    const FAT: Tolerance = Tolerance {
        two_seconds: true,
        one_hour: false,
    };

    #[test]
    fn tolerances() {
        let exact = Tolerance {
            two_seconds: false,
            one_hour: false,
        };
        let dst = Tolerance {
            two_seconds: true,
            one_hour: true,
        };
        assert!(exact.same_time(10, 10));
        assert!(!exact.same_time(10, 12));
        assert!(FAT.same_time(10, 12));
        assert!(!FAT.same_time(10, 3610));
        assert!(dst.same_time(10, 3610));
        assert!(dst.same_time(3611, 10));
        assert!(!dst.same_time(10, 1810));
    }

    #[test]
    fn relations() {
        let tmp = tempfile::tempdir().unwrap();
        let other = file_at(tmp.path(), "x", "abc", 1_000_000);
        assert_eq!(relation(3, at(1_000_000), None, FAT), Relation::Unique);
        assert_eq!(
            relation(3, at(1_000_000), Some(&other), FAT),
            Relation::Identical
        );
        assert_eq!(
            relation(3, at(1_000_001), Some(&other), FAT),
            Relation::Identical,
            "tolerance"
        );
        assert_eq!(
            relation(9, at(1_000_000), Some(&other), FAT),
            Relation::Different
        );
        assert_eq!(
            relation(3, at(1_000_100), Some(&other), FAT),
            Relation::Newer
        );
        assert_eq!(relation(3, at(999_000), Some(&other), FAT), Relation::Older);
        let dir = fs::metadata(tmp.path()).unwrap();
        assert_eq!(relation(3, at(1), Some(&dir), FAT), Relation::Different);
    }

    #[test]
    fn modes_select_relations() {
        use Relation as R;
        let all = [R::Unique, R::Newer, R::Older, R::Identical, R::Different];
        let pick =
            |m: CompareMode| -> Vec<R> { all.into_iter().filter(|&r| m.matches(r)).collect() };
        assert_eq!(pick(CompareMode::Unique), [R::Unique]);
        assert_eq!(
            pick(CompareMode::Different),
            [R::Newer, R::Older, R::Different]
        );
        assert_eq!(pick(CompareMode::AllNew), [R::Unique, R::Newer]);
        assert_eq!(CompareMode::from_key('A'), Some(CompareMode::AllNew));
        assert_eq!(CompareMode::from_key('x'), None);
    }
}
