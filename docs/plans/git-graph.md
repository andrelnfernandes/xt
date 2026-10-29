# Plano: Graph no menu Git (`git log --graph --decorate --oneline --all`)

Origem: conversa de 2026-10-29, olhando o [serie](https://github.com/lusingander/serie)
(grafo de commits desenhado como imagem, pelos protocolos do kitty e do
iTerm2). Descartados: desenhar o grafo como imagem no Xylem (dependência,
mais código lendo dados de fora, só alguns terminais, nada no Windows) e
integrar lazygit/gitui/tig (cada um com seu jeito). O autor preferiu o bom e
velho `git log --graph`.

## Etapa 1 (aprovada)

- **Ctrl+G → G (Graph):** `git log --graph --decorate --oneline --all -n 1000`
  do repositório inteiro, no visualizador, com busca. O Log continua como
  está (age sobre os marcados).
- **Cores do próprio git** (`--color=always`): ele já dá uma cor a cada ramo
  do grafo e separa ramo local, remoto, tag e HEAD, e segue o `color.*` do
  usuário. O Xylem lê só as sequências SGR de cor de texto (`ESC [ ... m`:
  16 cores, 256 e RGB) e descarta qualquer outra sequência; o que sobrar de
  controle o visualizador já troca por `·`. As cores passam pelo mesmo ajuste
  de contraste da sintaxe (o tema azul clássico incluído).
- `--no-show-signature`: o grafo não chama o gpg mesmo com
  `log.showSignature` ligado no repositório.
- Limite de 1000 commits, como no Log.
- Manual (en, pt, es, fr, de, zh): `git.md`.

## Etapa 2 (não aprovada ainda)

- Enter numa linha do Graph (ou do Log) abre o `git show` daquele commit, com
  as cores de diff; Esc volta para a mesma linha (como o D do visualizador).
