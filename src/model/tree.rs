//! Árvore de diretórios mantida em memória, no estilo XTree.
//!
//! Os nós ficam numa arena (`Vec<DirNode>`) e se referenciam por índice,
//! o que evita ponteiros e mantém a estrutura compacta mesmo com milhões
//! de diretórios logados.

use std::collections::{HashMap, HashSet};
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
    /// Marcado com `T` para as operações em grupo.
    pub tagged: bool,
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
    /// Árvore de um arquivo compactado: tudo já vem lido e nada é relido
    /// do disco.
    is_virtual: bool,
    /// Mostra diretórios ocultos (nome começando com ponto). Só afeta o
    /// que é exibido; o que está logado continua todo aqui.
    pub show_hidden: bool,
}

/// Nome oculto no Linux: começa com ponto.
pub fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.')
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
            is_virtual: false,
            show_hidden: true,
        };
        tree.expand(ROOT);
        tree
    }

    /// Árvore virtual a partir de uma listagem: caminhos relativos a `root`
    /// e, para cada um, o arquivo (`Some`) ou um diretório (`None`).
    /// Diretórios que só aparecem no caminho de um arquivo são criados.
    pub fn from_listing(root: PathBuf, listing: Vec<(PathBuf, Option<FileEntry>)>) -> Self {
        let mut tree = Tree {
            nodes: vec![DirNode {
                name: root.into_os_string(),
                parent: None,
                children: Vec::new(),
                files: Vec::new(),
                expanded: true,
                scanned: true,
                error: None,
            }],
            is_virtual: true,
            show_hidden: true,
        };
        for (path, file) in listing {
            let dirs = match file {
                Some(_) => path.parent().unwrap_or(Path::new("")),
                None => path.as_path(),
            };
            let mut cur = ROOT;
            for name in dirs.iter() {
                let found = find_child(&tree, cur, name);
                cur = match found {
                    Some(c) => c,
                    None => {
                        let c = tree.add_node(name.to_os_string(), cur);
                        tree.nodes[c].scanned = true;
                        tree.nodes[cur].children.push(c);
                        c
                    }
                };
            }
            if let Some(file) = file {
                tree.nodes[cur].files.push(file);
            }
        }
        for node in &mut tree.nodes {
            node.files.sort_by_key(|f| sort_key(&f.name));
        }
        for i in 0..tree.nodes.len() {
            let mut children = std::mem::take(&mut tree.nodes[i].children);
            children.sort_by_key(|&c| sort_key(&tree.nodes[c].name));
            tree.nodes[i].children = children;
        }
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
        if self.nodes[id].scanned || self.is_virtual {
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
            let child = self.add_node(name, id);
            self.nodes[id].children.push(child);
        }
        self.nodes[id].files = files;
    }

    fn add_node(&mut self, name: OsString, parent: NodeId) -> NodeId {
        self.nodes.push(DirNode {
            name,
            parent: Some(parent),
            children: Vec::new(),
            files: Vec::new(),
            expanded: false,
            scanned: false,
            error: None,
        });
        self.nodes.len() - 1
    }

    /// Relê um diretório já logado (re-log). Subdiretórios que continuam
    /// existindo mantêm o que já foi lido abaixo deles; arquivos mantêm a tag.
    pub fn rescan(&mut self, id: NodeId) {
        if !self.nodes[id].scanned || self.is_virtual {
            return;
        }
        let path = self.path(id);
        let (dirs, mut files) = match read_dir_entries(&path) {
            Ok(r) => r,
            Err(e) => {
                let node = &mut self.nodes[id];
                node.error = Some(e.to_string());
                node.files.clear();
                node.children.clear();
                return;
            }
        };

        let tagged: HashSet<&OsString> = self.nodes[id]
            .files
            .iter()
            .filter(|f| f.tagged)
            .map(|f| &f.name)
            .collect();
        for f in &mut files {
            f.tagged = tagged.contains(&f.name);
        }

        let mut old: HashMap<OsString, NodeId> = self.nodes[id]
            .children
            .iter()
            .map(|&c| (self.nodes[c].name.clone(), c))
            .collect();
        // Os que sumiram ficam soltos na arena, fora da árvore.
        let children = dirs
            .into_iter()
            .map(|name| match old.remove(&name) {
                Some(c) => c,
                None => self.add_node(name, id),
            })
            .collect();

        let node = &mut self.nodes[id];
        node.children = children;
        node.files = files;
        node.error = None;
    }

    /// Nó do diretório `path`, se ele estiver na árvore.
    pub fn find(&self, path: &Path) -> Option<NodeId> {
        let rel = path.strip_prefix(&self.nodes[ROOT].name).ok()?;
        let mut cur = ROOT;
        for comp in rel.components() {
            let name = comp.as_os_str();
            cur = *self.nodes[cur]
                .children
                .iter()
                .find(|&&c| self.nodes[c].name == name)?;
        }
        Some(cur)
    }

    pub fn file_mut(&mut self, dir: NodeId, idx: usize) -> &mut FileEntry {
        &mut self.nodes[dir].files[idx]
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

    /// Subdiretórios exibidos (sem os ocultos, se for o caso).
    fn visible_children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes[id]
            .children
            .iter()
            .copied()
            .filter(|&c| self.show_hidden || !is_hidden(&self.nodes[c].name))
    }

    /// Como `subtree`, mas sem entrar em diretórios ocultos (a não ser que
    /// sejam exibidos). `id` em si sempre entra.
    pub fn visible_subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            out.push(n);
            let children: Vec<NodeId> = self.visible_children(n).collect();
            stack.extend(children.into_iter().rev());
        }
        out
    }

    fn push_children(&self, id: NodeId, indent: &str, rows: &mut Vec<VisibleRow>) {
        let children: Vec<NodeId> = self.visible_children(id).collect();
        for (i, &child) in children.iter().enumerate() {
            let last = i + 1 == children.len();
            let collapsed =
                !self.nodes[child].expanded && self.visible_children(child).next().is_some();
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
        for n in self.subtree(ROOT).into_iter().map(|id| &self.nodes[id]) {
            if !n.scanned {
                continue;
            }
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
                tagged: false,
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
            tagged: false,
        });
    }

    dirs.sort_by_key(sort_key);
    files.sort_by_key(|f| sort_key(&f.name));
    Ok((dirs, files))
}

fn find_child(tree: &Tree, parent: NodeId, name: &std::ffi::OsStr) -> Option<NodeId> {
    tree.nodes[parent]
        .children
        .iter()
        .copied()
        .find(|&c| tree.nodes[c].name == name)
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
    fn rescan_keeps_subtrees_and_tags() {
        let (tmp, mut tree) = sample();
        tree.expand_all(ROOT);
        tree.file_mut(ROOT, 1).tagged = true; // z.txt
        let b = tree.node(ROOT).children[1];

        std::fs::remove_dir(tmp.path().join("a")).unwrap();
        create_dir_all(tmp.path().join("c")).unwrap();
        write(tmp.path().join("new.txt"), "").unwrap();
        tree.rescan(ROOT);

        let root = tree.node(ROOT);
        assert_eq!(names(&tree, &root.children), ["b", "c"]);
        assert_eq!(root.children[0], b, "b keeps its node");
        assert_eq!(names(&tree, &tree.subtree(b)), ["b", "deep", "deeper"]);
        let tags: Vec<_> = root
            .files
            .iter()
            .map(|f| (f.name.to_str().unwrap(), f.tagged))
            .collect();
        assert_eq!(
            tags,
            [("A.txt", false), ("new.txt", false), ("z.txt", true)]
        );
        // "a" sumiu da árvore e dos totais.
        assert_eq!(tree.logged_totals().dirs, 4);
    }

    #[test]
    fn finds_nodes_by_path() {
        let (tmp, mut tree) = sample();
        tree.expand_all(ROOT);
        assert_eq!(tree.find(tmp.path()), Some(ROOT));
        let deep = tree.find(&tmp.path().join("b/deep")).unwrap();
        assert_eq!(tree.path(deep), tmp.path().join("b/deep"));
        assert_eq!(tree.find(&tmp.path().join("nope")), None);
        assert_eq!(tree.find(Path::new("/elsewhere")), None);
    }

    #[test]
    fn builds_virtual_tree_from_listing() {
        let file = |name: &str, size| FileEntry {
            name: name.into(),
            size,
            modified: None,
            mode: 0o644,
            kind: FileKind::File,
            tagged: false,
        };
        let listing = vec![
            (PathBuf::from("z.txt"), Some(file("z.txt", 1))),
            (PathBuf::from("src/deep/b.rs"), Some(file("b.rs", 2))),
            (PathBuf::from("src/a.rs"), Some(file("a.rs", 3))),
            (PathBuf::from("empty"), None),
        ];
        let mut tree = Tree::from_listing(PathBuf::from("/x/p.zip"), listing);
        assert_eq!(names(&tree, &tree.node(ROOT).children), ["empty", "src"]);
        let src = tree.find(Path::new("/x/p.zip/src")).unwrap();
        assert_eq!(tree.node(src).files[0].name, "a.rs");
        let deep = tree.find(Path::new("/x/p.zip/src/deep")).unwrap();
        assert_eq!(tree.path(deep), Path::new("/x/p.zip/src/deep"));
        assert_eq!(
            tree.logged_totals(),
            Totals {
                dirs: 4,
                files: 3,
                bytes: 6
            }
        );

        // Nada é lido do disco: nem scan, nem re-log.
        tree.rescan(ROOT);
        tree.expand(src);
        assert!(tree.node(ROOT).error.is_none());
        assert_eq!(tree.node(ROOT).files.len(), 1);
    }

    #[test]
    fn hides_dot_directories_when_asked() {
        let (tmp, mut tree) = sample();
        create_dir_all(tmp.path().join(".git/objects")).unwrap();
        create_dir_all(tmp.path().join("b/.cache")).unwrap();
        tree.rescan(ROOT);
        let b = tree.find(&tmp.path().join("b")).unwrap();
        tree.rescan(b);
        tree.expand_all(ROOT);
        let names_of = |t: &Tree| -> Vec<String> {
            t.visible_rows()
                .iter()
                .skip(1)
                .map(|r| t.node(r.id).name.to_string_lossy().into_owned())
                .collect()
        };
        assert!(names_of(&tree).contains(&".git".to_string()));

        tree.show_hidden = false;
        assert_eq!(names_of(&tree), ["a", "b", "deep", "deeper"]);
        assert_eq!(
            names(&tree, &tree.visible_subtree(b)),
            ["b", "deep", "deeper"]
        );
        // O que está logado não muda.
        assert_eq!(
            names(&tree, &tree.subtree(b)),
            ["b", ".cache", "deep", "deeper"]
        );
        assert!(is_hidden(std::ffi::OsStr::new(".x")) && !is_hidden(std::ffi::OsStr::new("x")));
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
