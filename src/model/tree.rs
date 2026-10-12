//! Árvore de diretórios mantida em memória, no estilo XTree.
//!
//! Os nós ficam numa arena (`Vec<DirNode>`) e se referenciam por índice,
//! o que evita ponteiros e mantém a estrutura compacta mesmo com milhões
//! de diretórios logados.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

pub type NodeId = usize;

pub const ROOT: NodeId = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Symlink,
    Other,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: OsString,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub mode: u32,
    pub kind: FileKind,
}

#[derive(Debug)]
pub struct DirNode {
    /// Nome do diretório; na raiz é o caminho completo.
    pub name: OsString,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub files: Vec<FileEntry>,
    pub expanded: bool,
    /// Já foi lido do disco ("logado").
    pub scanned: bool,
    /// Erro ao ler o diretório (ex.: sem permissão).
    pub error: Option<String>,
}

/// Uma linha visível no painel da árvore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRow {
    pub id: NodeId,
    /// Conectores desenhados antes do nome (`│  ├──`).
    pub prefix: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub dirs: u64,
    pub files: u64,
    pub bytes: u64,
}

pub struct Tree {
    nodes: Vec<DirNode>,
}

/// Leitura de um ramo inteiro feita aos poucos, para a interface poder
/// mostrar o progresso e o usuário poder cancelar.
pub struct LogJob {
    stack: Vec<NodeId>,
    /// Além de ler, expande os nós na árvore (tecla `*`).
    expand: bool,
    /// Diretórios processados até agora.
    pub dirs: u64,
}

impl LogJob {
    pub fn new(root: NodeId, expand: bool) -> Self {
        LogJob {
            stack: vec![root],
            expand,
            dirs: 0,
        }
    }

    pub fn is_done(&self) -> bool {
        self.stack.is_empty()
    }
}

impl Tree {
    /// Cria a árvore com `root` já logado e expandido.
    pub fn new(root: PathBuf) -> Self {
        let mut tree = Tree {
            nodes: vec![DirNode {
                name: root.into_os_string(),
                parent: None,
                children: Vec::new(),
                files: Vec::new(),
                expanded: false,
                scanned: false,
                error: None,
            }],
        };
        tree.expand(ROOT);
        tree
    }

    pub fn node(&self, id: NodeId) -> &DirNode {
        &self.nodes[id]
    }

    /// Caminho completo do diretório.
    pub fn path(&self, id: NodeId) -> PathBuf {
        let mut parts = Vec::new();
        let mut cur = Some(id);
        while let Some(n) = cur {
            parts.push(&self.nodes[n].name);
            cur = self.nodes[n].parent;
        }
        parts.iter().rev().collect()
    }

    /// Lê o diretório do disco, se ainda não foi lido.
    pub fn scan(&mut self, id: NodeId) {
        if self.nodes[id].scanned {
            return;
        }
        self.nodes[id].scanned = true;

        let path = self.path(id);
        let (dirs, files) = match read_dir_entries(&path) {
            Ok(r) => r,
            Err(e) => {
                self.nodes[id].error = Some(e.to_string());
                return;
            }
        };

        for name in dirs {
            let child = self.nodes.len();
            self.nodes.push(DirNode {
                name,
                parent: Some(id),
                children: Vec::new(),
                files: Vec::new(),
                expanded: false,
                scanned: false,
                error: None,
            });
            self.nodes[id].children.push(child);
        }
        self.nodes[id].files = files;
    }

    /// Expande um nó. Os filhos também são lidos para sabermos se
    /// eles têm subdiretórios (e mostrar o indicador `+`).
    pub fn expand(&mut self, id: NodeId) {
        self.scan(id);
        self.nodes[id].expanded = true;
        for i in 0..self.nodes[id].children.len() {
            let child = self.nodes[id].children[i];
            self.scan(child);
        }
    }

    pub fn collapse(&mut self, id: NodeId) {
        self.nodes[id].expanded = false;
    }

    /// Expande o ramo inteiro abaixo de `id` de uma vez.
    #[cfg(test)]
    pub fn expand_all(&mut self, id: NodeId) {
        let mut job = LogJob::new(id, true);
        self.run_job(&mut job, Duration::MAX);
    }

    /// Avança um [`LogJob`] até terminar ou estourar `budget`.
    pub fn run_job(&mut self, job: &mut LogJob, budget: Duration) {
        let start = Instant::now();
        while let Some(n) = job.stack.pop() {
            if job.expand {
                self.expand(n);
            } else {
                self.scan(n);
            }
            job.dirs += 1;
            // Ordem reversa para visitar os filhos em ordem alfabética.
            job.stack
                .extend(self.nodes[n].children.iter().rev().copied());
            if start.elapsed() >= budget {
                break;
            }
        }
    }

    /// `id` e todos os seus descendentes já criados, em pré-ordem.
    pub fn subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            out.push(n);
            stack.extend(self.nodes[n].children.iter().rev().copied());
        }
        out
    }

    /// Expande os ancestrais de `id` para que ele apareça na árvore.
    pub fn reveal(&mut self, id: NodeId) {
        let mut cur = self.nodes[id].parent;
        while let Some(n) = cur {
            self.nodes[n].expanded = true;
            cur = self.nodes[n].parent;
        }
    }

    pub fn has_children(&self, id: NodeId) -> bool {
        !self.nodes[id].children.is_empty()
    }

    /// Linhas visíveis, em ordem, respeitando os nós recolhidos.
    pub fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut rows = vec![VisibleRow {
            id: ROOT,
            prefix: String::new(),
        }];
        if self.nodes[ROOT].expanded {
            self.push_children(ROOT, "", &mut rows);
        }
        rows
    }

    fn push_children(&self, id: NodeId, indent: &str, rows: &mut Vec<VisibleRow>) {
        let children = &self.nodes[id].children;
        for (i, &child) in children.iter().enumerate() {
            let last = i + 1 == children.len();
            let collapsed = !self.nodes[child].expanded && self.has_children(child);
            let connector = match (last, collapsed) {
                (false, false) => "├──",
                (false, true) => "├─+",
                (true, false) => "└──",
                (true, true) => "└─+",
            };
            rows.push(VisibleRow {
                id: child,
                prefix: format!("{indent}{connector}"),
            });
            if self.nodes[child].expanded {
                let next = format!("{indent}{}", if last { "   " } else { "│  " });
                self.push_children(child, &next, rows);
            }
        }
    }

    /// Totais de tudo o que já foi logado.
    pub fn logged_totals(&self) -> Totals {
        let mut t = Totals::default();
        for n in self.nodes.iter().filter(|n| n.scanned) {
            t.dirs += 1;
            t.files += n.files.len() as u64;
            t.bytes += n.files.iter().map(|f| f.size).sum::<u64>();
        }
        t
    }
}

/// Lê um diretório e devolve (subdiretórios, arquivos), ambos ordenados.
/// Links simbólicos nunca são seguidos: aparecem como arquivos.
fn read_dir_entries(path: &Path) -> std::io::Result<(Vec<OsString>, Vec<FileEntry>)> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name();
        let Ok(meta) = entry.metadata() else {
            // Entrada que sumiu ou sem permissão de stat: mostra mesmo assim.
            files.push(FileEntry {
                name,
                size: 0,
                modified: None,
                mode: 0,
                kind: FileKind::Other,
            });
            continue;
        };
        let ft = meta.file_type();
        if ft.is_dir() {
            dirs.push(name);
            continue;
        }
        let kind = if ft.is_symlink() {
            FileKind::Symlink
        } else if ft.is_file() {
            FileKind::File
        } else {
            FileKind::Other
        };
        files.push(FileEntry {
            name,
            size: meta.len(),
            modified: meta.modified().ok(),
            mode: meta.permissions().mode(),
            kind,
        });
    }

    dirs.sort_by_key(sort_key);
    files.sort_by_key(|f| sort_key(&f.name));
    Ok((dirs, files))
}

fn sort_key(name: &OsString) -> String {
    name.to_string_lossy().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};

    fn sample() -> (tempfile::TempDir, Tree) {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path();
        create_dir_all(p.join("b/deep/deeper")).unwrap();
        create_dir_all(p.join("a")).unwrap();
        write(p.join("z.txt"), "12345").unwrap();
        write(p.join("A.txt"), "1").unwrap();
        write(p.join("b/inside"), "xyz").unwrap();
        let tree = Tree::new(p.to_path_buf());
        (tmp, tree)
    }

    fn names(tree: &Tree, ids: &[NodeId]) -> Vec<String> {
        ids.iter()
            .map(|&i| tree.node(i).name.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn logs_root_sorted() {
        let (_tmp, tree) = sample();
        let root = tree.node(ROOT);
        assert_eq!(names(&tree, &root.children), ["a", "b"]);
        let files: Vec<_> = root
            .files
            .iter()
            .map(|f| f.name.to_str().unwrap())
            .collect();
        assert_eq!(files, ["A.txt", "z.txt"]);
    }

    #[test]
    fn path_is_rebuilt_from_parents() {
        let (tmp, mut tree) = sample();
        let b = tree.node(ROOT).children[1];
        tree.expand(b);
        let deep = tree.node(b).children[0];
        assert_eq!(tree.path(deep), tmp.path().join("b/deep"));
    }

    #[test]
    fn visible_rows_follow_expansion() {
        let (_tmp, mut tree) = sample();
        let prefixes: Vec<_> = tree.visible_rows().into_iter().map(|r| r.prefix).collect();
        // "b" tem filhos e está recolhido: indicador "+".
        assert_eq!(prefixes, ["", "├──", "└─+"]);

        tree.expand_all(ROOT);
        let prefixes: Vec<_> = tree.visible_rows().into_iter().map(|r| r.prefix).collect();
        assert_eq!(prefixes, ["", "├──", "└──", "   └──", "      └──"]);

        tree.collapse(ROOT);
        assert_eq!(tree.visible_rows().len(), 1);
    }

    #[test]
    fn totals_count_only_logged_dirs() {
        let (_tmp, mut tree) = sample();
        // raiz + a + b (filhos lidos ao expandir a raiz)
        assert_eq!(
            tree.logged_totals(),
            Totals {
                dirs: 3,
                files: 3,
                bytes: 9
            }
        );
        tree.expand_all(ROOT);
        assert_eq!(tree.logged_totals().dirs, 5);
    }

    #[test]
    fn log_job_can_run_in_slices() {
        let (_tmp, mut tree) = sample();
        let mut job = LogJob::new(ROOT, false);
        // Orçamento zero: processa um diretório por chamada.
        let mut calls = 0;
        while !job.is_done() {
            tree.run_job(&mut job, Duration::ZERO);
            calls += 1;
        }
        assert_eq!(calls, 5);
        assert_eq!(job.dirs, 5);
        assert_eq!(tree.logged_totals().dirs, 5);
        // Só leu, não expandiu.
        assert_eq!(tree.visible_rows().len(), 3);
    }

    #[test]
    fn subtree_and_reveal() {
        let (_tmp, mut tree) = sample();
        tree.expand_all(ROOT);
        let b = tree.node(ROOT).children[1];
        assert_eq!(names(&tree, &tree.subtree(b)), ["b", "deep", "deeper"]);

        let deeper = tree.subtree(b)[2];
        tree.collapse(ROOT);
        tree.collapse(b);
        tree.reveal(deeper);
        assert!(tree.visible_rows().iter().any(|r| r.id == deeper));
    }

    #[test]
    fn symlinks_are_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        create_dir_all(tmp.path().join("real")).unwrap();
        std::os::unix::fs::symlink(tmp.path(), tmp.path().join("loop")).unwrap();
        let mut tree = Tree::new(tmp.path().to_path_buf());
        tree.expand_all(ROOT);
        assert_eq!(names(&tree, &tree.node(ROOT).children), ["real"]);
        assert_eq!(tree.node(ROOT).files[0].kind, FileKind::Symlink);
    }

    #[test]
    fn unreadable_dir_records_error() {
        let mut tree = Tree::new(PathBuf::from("/caminho/que/nao/existe"));
        tree.scan(ROOT);
        assert!(tree.node(ROOT).error.is_some());
    }
}
