//! Programas externos: o editor (tecla `E`) e comandos do shell (tecla `X`).
//!
//! Tudo passa por `sh -c`, com os nomes de arquivo sempre entre aspas
//! simples. Os nomes são tratados como bytes: no Linux eles não precisam
//! ser UTF-8 válido.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

/// Um programa a rodar com o terminal entregue a ele.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct External {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    /// Pede Enter antes de voltar ao xt, para dar tempo de ler a saída.
    pub wait: bool,
}

/// Abre `file` no editor: o configurado, `$VISUAL`, `$EDITOR`, o `editor`
/// do Debian/Ubuntu ou `vi`. Pode ter argumentos (`code -w`).
pub fn editor(file: &Path, cwd: &Path, configured: &str) -> External {
    let env = |name| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
    let editor = Some(configured.trim().to_string())
        .filter(|e| !e.is_empty())
        .or_else(|| env("VISUAL"))
        .or_else(|| env("EDITOR"))
        .unwrap_or_else(|| if in_path("editor") { "editor" } else { "vi" }.into());
    External {
        program: "sh".into(),
        args: vec![
            "-c".into(),
            format!("{editor} \"$1\"").into(),
            "sh".into(),
            file.as_os_str().to_os_string(),
        ],
        cwd: cwd.to_path_buf(),
        wait: false,
    }
}

/// Abre `file` no programa padrão do sistema (`xdg-open`, ou `opener`),
/// sem entregar o terminal: o programa roda em segundo plano, separado.
pub fn open_detached(file: &Path, opener: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let opener = match opener.trim() {
        // Nos testes, nunca abre programas de verdade na área de trabalho.
        "" if cfg!(test) => "true",
        "" => "xdg-open",
        other => other,
    };
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(format!("{opener} \"$1\""))
        .arg("sh")
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Recolhe o processo quando terminar, sem travar o xt.
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Roda uma linha de comando do usuário.
pub fn command(line: OsString, cwd: &Path) -> External {
    External {
        program: "sh".into(),
        args: vec!["-c".into(), line],
        cwd: cwd.to_path_buf(),
        wait: true,
    }
}

/// Troca, na linha de comando, `%f` pelo arquivo sob o cursor, `%t` pelos
/// arquivos marcados e `%d` pelo diretório atual, todos entre aspas.
/// `%%` vira `%`.
pub fn expand(template: &str, file: Option<&Path>, tagged: &[PathBuf], dir: &Path) -> OsString {
    let mut out: Vec<u8> = Vec::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            let mut buf = [0; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match chars.peek() {
            Some('f') => {
                chars.next();
                if let Some(f) = file {
                    out.extend(quote(f.as_os_str()));
                }
            }
            Some('t') => {
                chars.next();
                let quoted: Vec<Vec<u8>> = tagged.iter().map(|p| quote(p.as_os_str())).collect();
                out.extend(quoted.join(&b' '));
            }
            Some('d') => {
                chars.next();
                out.extend(quote(dir.as_os_str()));
            }
            Some('%') => {
                chars.next();
                out.push(b'%');
            }
            _ => out.push(b'%'),
        }
    }
    OsString::from_vec(out)
}

/// Uma linha do Batch (Ctrl+B) para `file`: `%f` caminho, `%d` diretório,
/// `%n` nome, `%b` nome sem extensão, `%e` extensão, `%s` tamanho. Em
/// minúsculas saem entre aspas (para scripts); em maiúsculas, como texto
/// puro (para listas). `%%` é um `%`.
pub fn batch_line(mask: &str, file: &Path, size: u64) -> Vec<u8> {
    let name = file.file_name().unwrap_or_default();
    let (base, ext) = split_ext(name.as_bytes());
    let dir = file.parent().unwrap_or(Path::new(""));
    let size = size.to_string();
    let mut out = Vec::new();
    let mut chars = mask.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            let mut buf = [0; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        let Some(&k) = chars.peek() else {
            out.push(b'%');
            continue;
        };
        let value: &[u8] = match k.to_ascii_lowercase() {
            'f' => file.as_os_str().as_bytes(),
            'd' => dir.as_os_str().as_bytes(),
            'n' => name.as_bytes(),
            'b' => base,
            'e' => ext,
            's' => size.as_bytes(),
            '%' => {
                chars.next();
                out.push(b'%');
                continue;
            }
            _ => {
                out.push(b'%');
                continue;
            }
        };
        chars.next();
        if k.is_ascii_lowercase() {
            out.extend(quote(OsStr::from_bytes(value)));
        } else {
            out.extend_from_slice(value);
        }
    }
    out
}

/// "foto.tar.gz" → ("foto.tar", "gz"); ".bashrc" e "Makefile" não têm
/// extensão.
fn split_ext(name: &[u8]) -> (&[u8], &[u8]) {
    match name.iter().rposition(|&b| b == b'.') {
        Some(0) | None => (name, b""),
        Some(i) => (&name[..i], &name[i + 1..]),
    }
}

/// Aspas simples do shell: `it's` → `'it'\''s'`.
fn quote(s: &OsStr) -> Vec<u8> {
    let mut out = vec![b'\''];
    for &b in s.as_bytes() {
        if b == b'\'' {
            out.extend_from_slice(b"'\\''");
        } else {
            out.push(b);
        }
    }
    out.push(b'\'');
    out
}

fn in_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Roda pelo shell de verdade e devolve os argumentos que chegaram.
    fn args_seen_by_shell(line: &OsStr) -> Vec<Vec<u8>> {
        let mut script = OsString::from("for a in ");
        script.push(line);
        script.push("; do printf '%s\\0' \"$a\"; done");
        let out = Command::new("sh").arg("-c").arg(script).output().unwrap();
        out.stdout
            .split(|&b| b == 0)
            .filter(|a| !a.is_empty())
            .map(<[u8]>::to_vec)
            .collect()
    }

    #[test]
    fn expands_placeholders() {
        let line = expand(
            "wc -l %f %% %d",
            Some(Path::new("/a/b c.txt")),
            &[],
            Path::new("/a"),
        );
        assert_eq!(line, "wc -l '/a/b c.txt' % '/a'");
        let tagged = [PathBuf::from("/x/1"), PathBuf::from("/x/2")];
        assert_eq!(
            expand("tar cf t.tar %t", None, &tagged, Path::new("/")),
            "tar cf t.tar '/x/1' '/x/2'"
        );
        assert_eq!(expand("echo %f%z", None, &[], Path::new("/")), "echo %z");
    }

    #[test]
    fn batch_lines() {
        let f = Path::new("/fotos/it's.JPG");
        let line = |m| String::from_utf8(batch_line(m, f, 1234)).unwrap();
        assert_eq!(
            line("convert %f %b.png"),
            "convert '/fotos/it'\\''s.JPG' 'it'\\''s'.png"
        );
        assert_eq!(line("%N;%S;%E;%D"), "it's.JPG;1234;JPG;/fotos");
        assert_eq!(line("100%% %x"), "100% %x");
        assert_eq!(split_ext(b".bashrc"), (&b".bashrc"[..], &b""[..]));
        assert_eq!(split_ext(b"a.tar.gz"), (&b"a.tar"[..], &b"gz"[..]));
    }

    #[test]
    fn quoting_survives_the_real_shell() {
        let tricky = [
            PathBuf::from("it's here.txt"),
            PathBuf::from("$(rm -rf nada) `x` \"q\""),
            PathBuf::from(OsStr::from_bytes(b"caf\xe9")),
        ];
        let line = expand("%t", None, &tricky, Path::new("/"));
        let seen = args_seen_by_shell(&line);
        let expected: Vec<Vec<u8>> = tricky
            .iter()
            .map(|p| p.as_os_str().as_bytes().to_vec())
            .collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn opens_detached_with_configured_program() {
        let tmp = tempfile::tempdir().unwrap();
        let marker = tmp.path().join("opened");
        let opener = format!("cp {} -- ", tmp.path().join("src").display());
        std::fs::write(tmp.path().join("src"), "x").unwrap();
        // "cp src -- <arquivo>": prova que o programa recebeu o caminho.
        open_detached(&marker, &opener).unwrap();
        for _ in 0..100 {
            if marker.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("opener did not run");
    }

    #[test]
    fn editor_uses_env_and_passes_file_as_argument() {
        let ext = editor(Path::new("/tmp/a b.txt"), Path::new("/tmp"), "");
        assert_eq!(ext.program, "sh");
        assert_eq!(ext.args[3], "/tmp/a b.txt");
        assert!(ext.args[1].to_string_lossy().ends_with(" \"$1\""));
        assert!(!ext.wait);
        let ext = editor(Path::new("/x"), Path::new("/"), "nano -l");
        assert_eq!(ext.args[1], "nano -l \"$1\"");
    }
}
