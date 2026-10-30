# Plano: correções de segurança

Origem: auditoria externa (9 achados), conferida contra o código em
2026-10-25. Cada item diz se o achado procede, o risco real e a correção.

## Veredito por achado

| # | Achado | Procede? | Risco real |
|---|--------|----------|------------|
| 0 | *(não estava na auditoria)* Nomes do SFTP com `/` ou `..` | **Sim** | **Alto**: escrita fora do destino |
| 1a | Symlink baixado aponta para arquivo local sensível | Não como vulnerabilidade | Mesmo comportamento de `cp -a`/`rsync -l`: o link é inerte |
| 1b | Escrita através de symlink criado na mesma cópia | Sim, só com servidor malicioso | Médio |
| 2 | Nome começando com `-` vira opção | Parcial | Baixo: só Alt+O (caminho relativo); E/O usam caminho absoluto |
| 3 | Temporário `.xy-<pid>.tmp` sem `O_EXCL` | Sim | Baixo/Médio: `protected_symlinks=1` cobre `/tmp`; pastas de grupo não |
| 4a | Prompt do servidor imita "(yes/no)" e desmascara a senha | Sim | Baixo: a senha vai para esse servidor de qualquer jeito; o dano é exibição/confusão |
| 4b | ssh traduzido por locale | **Não** | OpenSSH não tem tradução; `LC_ALL=C` não muda nada |
| 5 | `%VAR%` expandido pelo `cmd` (Windows) | Sim | Baixo: troca o nome; só quebra a linha se a variável tiver `"` |
| 6 | Zip bomb trava a interface | Sim | Baixo (DoS na própria ação do usuário), mas incômodo |
| 7 | `git status` executa `core.fsmonitor` de repositório baixado | Sim | Médio, atenuado: `git = false` por padrão |
| 8 | SUID/SGID preservado ao extrair/baixar | Sim | Baixo como usuário; real com `sudo xy` |
| 9a | Senha do askpass em arquivo no Windows | Sim | Baixo: fica em `%LOCALAPPDATA%` (ACL só do usuário) por instantes |
| 9b | Senha sem `zeroize` na memória | Sim | Muito baixo: ssh e o pipe também a têm |

Notas:

- **0** é o mais grave e foi o que a auditoria deixou passar. `Sftp::list_dir`
  (`src/remote/sftp.rs:349`) só descarta `.` e `..` exatos. Um servidor
  malicioso pode devolver o nome `../../.bashrc` ou `/home/x/.ssh/authorized_keys`;
  `PathBuf::join` mantém o `..` (e um nome absoluto substitui o caminho
  inteiro), e a cópia escreve onde ele quiser. É o Zip-Slip do SFTP — o
  mesmo que `archive.rs` já bloqueia para pacotes.
- **1b** precisa que a mesma listagem traga `sub` como symlink e `sub/x`
  como arquivo, o que um servidor honesto não faz; com o item 0 corrigido
  continua possível (nome repetido como link e como diretório).
- **2**: `editor()` e `open_detached()` recebem `file_path`, sempre absoluto.
  `%n`/`%b` no Ctrl+B são escolha explícita do usuário no próprio modelo; o
  `%f` dá o caminho inteiro. Só o Alt+O gera relativo sem ele pedir.
- **7**: além de `fsmonitor`, um `filter.<x>.clean` no `.git/config` +
  `.gitattributes` também roda em `status` para arquivos "racily clean". Não
  há chave única que desligue filtros; ver passo 2.

## Passos

**Status:** concluído (PRs #37 a #40, 2026-10-25). O que sobrou está em
[BACKLOG.md](../../BACKLOG.md), seção Security.

Em ordem de prioridade; cada um é um commit pequeno com teste.

### 1. Nomes vindos do servidor (achado 0) — P1

- `Sftp::list_dir`: descartar (ou devolver erro de protocolo) entradas cujo
  nome tenha `/` ou `\0`, seja vazio, `.` ou `..`. Em Windows local, também
  `\` e `:` (seriam separador/stream ao virar `PathBuf`).
- Mesma checagem onde um nome remoto vira caminho local: `virtual_path`
  e `read_link` não precisam (alvo de link não é caminho de escrita), mas
  qualquer outro `bytes_os(name)` sim — conferir com `grep bytes_os`.
- Teste em `sftp.rs`: servidor de teste não ajuda (o OpenSSH não cria esses
  nomes), então testar a função de filtro isolada.

### 2. Git automático (achado 7) — P1

- `git()` em `src/git.rs:72`: acrescentar `-c core.fsmonitor=false`.
- Filtros: não há opção do git que os desligue todos. Opção viável para o
  `status` automático (`read_status`): não rodá-lo em repositório cujo
  `.git/config` tenha seção `filter.` — mostrar o diretório sem cores de
  git e uma mensagem; os comandos manuais do menu Git continuam. Decidir no
  PR se vale; o mínimo é o `fsmonitor=false`.
- Teste: repositório com `core.fsmonitor = touch marker` → depois do
  refresh, `marker` não existe.

### 3. Symlinks numa cópia em lote (achado 1b) — P1

- `Operation` guarda os symlinks que ela mesma criou (`HashSet<PathBuf>` de
  destinos). Em `start_item`, antes de `create_dir_all`, falhar o item se
  algum ancestro de `item.dst` estiver nesse conjunto ("would write through
  a link created by this copy").
- Vale para cópia local também (não muda nada lá, mas fica uma regra só).
- Não validar o alvo do link (1a): copiar links como links é o
  comportamento documentado; teste `copies_symlinks_as_links` continua.
- Teste: itens `[sub → ../fora (link), sub/x]` com `Overwrite::Always` →
  `x` não aparece em `fora`, erro no resumo.

### 4. Temporários exclusivos (achado 3) — P2

- Função `create_tmp(dst) -> io::Result<(File, PathBuf)>` em `ops/mod.rs`:
  `OpenOptions::new().write(true).create_new(true)`; em `AlreadyExists`,
  `remove_file` (apaga o link, não o alvo) e tenta de novo uma vez; se
  falhar de novo, erro.
- Usar em `start_item`, `download_item`, `extract_item` e em
  `ArchiveJob::new` (`archive.rs:374`, que duplica `tmp_path`: passar a usar
  a mesma função).
- `extract_for_opening`/`download_for_opening` escrevem em `run_dir`
  (0700, nosso): não precisam.
- Teste (Unix): pré-criar `.<nome>.xy-<pid>.tmp` como symlink para um
  arquivo-alvo; copiar; alvo intacto, cópia correta.

### 5. Extração em partes e limites (achado 6) — P2

- `extract_item`: em vez de `extract(..., u64::MAX)` num passo, extrair via
  `Transfer` como as cópias locais/SFTP, com progresso e Esc. Exige que o
  `Reader` ofereça um `Read` para a entrada (`entry_reader`), o que é
  natural para ZIP/TAR e, no 7z, só com uma thread que alimente um pipe.
  Alternativa mais barata: rodar a extração numa thread e o `step` só
  acompanhar bytes escritos/cancelar por flag.
- `extract_for_opening` (O dentro de pacote): mesma thread + mensagem de
  progresso; Esc cancela e apaga o parcial.
- `content_opener` em `view.rs:111` (busca dentro de pacote) lê a entrada
  inteira para um `Vec`: passar um teto (p.ex. o mesmo da busca em arquivo
  grande) ou ler em streaming.
- 7z sólido (`archive.rs:166`): quando `limit` não é `u64::MAX`, limitar o
  total descartado no `sink` (p.ex. 256 MiB) e devolver erro "entry is deep
  inside a solid block" — o preview mostra a mensagem.

### 6. Modos SUID/SGID (achado 8) — P2

- `download_item`, `extract_item`: `& 0o777` em vez de `& 0o7777`.
- Leitura de pacotes (`archive.rs:225, 267, 317`) pode continuar com
  `0o7777` (o Alt+I mostra o modo real); só a escrita mascara.
- Cópia local continua preservando (como `cp -p`); decidir no PR.
- Teste: entrada TAR com modo `0o4755` → arquivo extraído `0o755`.

### 7. Askpass mais estrito (achado 4a) — P3

- `is_secret`: só é pergunta de sim/não se começar com
  `The authenticity of host '` ou contiver o aviso de chave mudada do
  OpenSSH (`WARNING: REMOTE HOST IDENTIFICATION`, se chegar ao askpass —
  conferir). Senão, mascarar.
- Conferir se o ssh passa `SSH_ASKPASS_PROMPT=confirm` para a confirmação
  de host (OpenSSH ≥ 8.4): se sim, usar a variável em vez do texto, que é
  imune a spoofing. `xy --askpass` repassaria a dica no arquivo de pergunta.
- Não adicionar `LC_ALL=C` (4b não procede).
- Testes: prompt keyboard-interactive com "(yes/no)" no meio → secreto.

### 8. Alt+O com nomes começando em `-` (achado 2) — P3

- `type_tagged` (`app/terminal.rs:133`): se o relativo começar com `-`,
  prefixar `./`.
- Não mexer em `%n`/`%b` do Ctrl+B nem em `with_file`.

### 9. Windows: `%` e senha em disco (achados 5 e 9a) — P3

- `%`: passar o arquivo por variável de ambiente — `with_file` gera
  `program "%XY_FILE%"` e o `Command` recebe `.env("XY_FILE", file)`. O `cmd`
  expande uma vez só e não reexpande o resultado, então `%PATH%` no nome
  fica literal. Mesmo truque para `%f`/`%t` do `expand` (variáveis
  `XY_F`, `XY_T1`…) se valer a complexidade; no mínimo E e O.
- Senha: trocar o arquivo por named pipe do Windows, ou manter o arquivo e
  só documentar que `run_dir` está em `%LOCALAPPDATA%`. Baixa prioridade.
- Checar com `cargo clippy --target x86_64-pc-windows-gnu`.

### Fora do plano

- 9b (`zeroize`): custo de dependência sem ganho real; registrar como
  descartado.
- 1a: documentar no README/help que links remotos são copiados como links.

## Ordem sugerida de PRs

1. Passos 1–3 (escrita fora do destino e execução de código).
2. Passos 4 e 6 (pequenos, mesmo arquivo).
3. Passo 5 (o maior; mexe em `Operation` e `archive::Reader`).
4. Passos 7–9.

## Depois: leituras do Git (2026-10-30)

O passo 2 tinha desligado só o `core.fsmonitor`, e só nos marcadores. Testes com
repositórios hostis mostraram que os marcadores ainda executavam ganchos, filtros
via `include.path`, `config.worktree` e de submódulos, e que o menu Ctrl+G executava
também `diff.external`, `textconv` e o programa de assinatura. Toda leitura agora
passa por `git::reading`, e os filtros próprios são procurados com `--show-scope`.
Add, Unstage e Commit continuam seguindo a configuração do repositório.

Quem precisa de filtros próprios ou diffs externos usa `git_trusted`, vazio por
padrão: uma lista de pastas, resolvidas antes de comparar e por nomes inteiros, para
um link não levar a confiança junto e `~/work` não valer para `~/work-downloads`. É
por pasta, e não uma chave geral, porque uma chave ficaria ligada por causa de um
projeto e valeria para o próximo `.zip`.
