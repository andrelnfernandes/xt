# Plano: novos visualizadores de arquivo

Origem: conversa de 2026-10-27, logo depois da integração com editores hex
(commit 5cb7ff6, branch `binary-editor`). Este arquivo é o plano geral; cada
subprojeto tem o seu spec, nesta mesma pasta.

## Objetivo

Permitir que o viewer (`V` e o Autoview, F7) mostre mais do que texto UTF-8 e
hex: codificações antigas, arquivos comprimidos, tabelas (DBF, CSV, SDF),
logs, SQLite e, como opção, conversores externos. **Requisito principal:** um
arquivo hostil não pode comprometer a máquina. Um viewer comprometido
permitiria invadi-la.

## Princípios de segurança (valem para todos os subprojetos)

Todo arquivo é tratado como hostil: pode ter vindo de download, de dentro de
um archive, de um servidor SFTP ou de um repositório clonado. Quem o criou
controla os bytes, o nome e o tamanho.

1. **Nada roda sozinho em arquivo hostil.** O Autoview só usa parsers
   embutidos, em Rust. Programa externo só roda com uma ação explícita do
   usuário.
2. **Filtros externos não passam pelo shell.** O comando é executado direto
   (argv), e `%f` vira um único argumento. Isso também evita o `%VAR%` do
   `cmd` no Windows.
3. **Limite em tudo:** bytes lidos, saída, tempo e memória.
4. **Somente leitura.** Nenhum viewer escreve no arquivo nem ao lado dele.
   Descompressão e decodificação acontecem em memória. A exceção é o
   subprojeto 6: archive e remoto exigem uma cópia temporária, que fica no
   `run_dir` (0700).
5. **Configuração só do `config.toml` do usuário**, nunca de um arquivo
   dentro da pasta que está sendo vista.
6. **A tela também é um vetor.** Caracteres de controle (C0, DEL, C1) viram
   `·`, como o viewer já faz hoje. Isso vale para texto, para células de
   tabela e para saída de filtro. Cor ANSI em log só aceita SGR de cor; todo
   o resto é descartado (OSC 52, por exemplo, escreve no clipboard).
   **Trojan Source (CVE-2021-42574), cuidado especial pedido pelo autor:**
   os 12 caracteres bidi nunca chegam ao terminal. Há um filtro único no fim
   de `ui::draw`, e o que vem de fora (conteúdo, nomes) aparece marcado e com
   aviso. Detalhes na seção 2b de [viewer-encoding.md](viewer-encoding.md).
   Todo subprojeto novo que mostre texto de fora herda isso.
7. **Parsers próprios:**
   - todo tamanho declarado no arquivo é conferido contra o tamanho real;
   - nenhum `panic`, testado com proptest sobre bytes aleatórios (o
     proptest já está nas dev-deps);
   - nenhuma dependência nova com `unsafe` entra sem revisão.

## Subprojetos

Abordagem escolhida (A): primeiro o que é embutido, em Rust; os programas
externos vêm por último, como opção.

| # | Subprojeto | Quem lê o arquivo | Depende de | Status |
|---|---|---|---|---|
| 1 | Codificações (UTF-8, UTF-16, CP850, CP437, 1252), `.gz/.xz/.zst/.bz2` soltos, limite de tamanho configurável | Rust, dentro do Xylem | — | **spec em andamento:** [viewer-encoding.md](viewer-encoding.md) |
| 2 | Modo tabela + DBF (com memo `.DBT`/`.FPT`) e CSV; SDF como extra | Rust, dentro do Xylem | 1 | não iniciado |
| 3 | Viewer em janelas, como o `less`: lê sob demanda, monta o índice de linhas em segundo plano, carrega de forma assíncrona e Esc cancela | — | 1 | não iniciado |
| 4 | Logs: follow, ir para o fim, filtro de linhas, cor por nível, ANSI só SGR | Rust, dentro do Xylem | 3 | não iniciado |
| 5 | SQLite | a decidir (ver abaixo) | 2 | não iniciado |
| 6 | `[viewers]`: programas externos (`pdftotext`, `exiftool`, `ffprobe`...) | programa externo | — | não iniciado |

Notas de cada subprojeto:

- **2 — SDF:** é o SDF do dBase/Clipper, a exportação de largura fixa
  (`COPY TO ... SDF`), não o SQL Server Compact. Não é mais tão comum, por
  isso entra como extra. Precisa de um layout, que pode vir da estrutura de
  um DBF ou de um arquivo de layout. O mesmo mecanismo serviria depois para
  CNAB 240/400.
- **2 — DBF:** o codepage vem do byte 29 do cabeçalho (LDID). Arquivos de
  Clipper costumam ser CP850.
- **3:** a ideia surgiu porque o limite fixo de 16 MiB é pequeno para
  máquinas de hoje. Com janelas, o limite deixa de existir para arquivo sem
  compressão. Comprimido e remoto continuam com teto, porque não dá para
  pular dentro deles. A mudança mexe no coração do viewer: hoje busca,
  quebra de linha, "ir para a linha" e cores dependem de `lines:
  Vec<String>`.
- **4:** "ir para o fim" num arquivo comprimido exige descomprimir tudo,
  jogando fora o que não se mostra. Isso roda em segundo plano, com Esc,
  limite de tempo e limite de bytes descartados, como o `archive::Extractor`.
- **5:** há três caminhos, a decidir no spec dele:
  - `rusqlite` com `bundled`: C dentro do processo. Exige o modo defensivo:
    somente leitura, `trusted_schema=OFF`, `SQLITE_DBCONFIG_DEFENSIVE` e
    tempo limite para views (o guia "Defense Against the Dark Arts" do
    sqlite.org).
  - `sqlite3 -readonly -safe` como filtro isolado.
  - um leitor próprio do formato de arquivo.
- **6:**
  - desligado por padrão e nunca no Autoview;
  - no Linux, roda isolado: Landlock (só leitura do arquivo e das
    bibliotecas, nenhuma escrita, sem rede no kernel 6.7 ou mais novo),
    `no_new_privs`, rlimits de CPU e memória, timeout que mata o grupo de
    processos, stdin em `/dev/null` e saída limitada;
  - no Windows não há equivalente simples, então fica desligado.

  Exemplo real do risco: o CVE-2021-22204 do ExifTool executava código a
  partir de uma imagem.

Opções descartadas:

- **B (programas externos primeiro):** cobriria mais formatos logo, mas
  começaria pela parte mais arriscada.
- **C (nunca ter programa externo):** é o mais seguro, mas `O` e `X` já
  cobrem esse caso de forma explícita. Fica valendo como plano de reserva se
  o subprojeto 6 não convencer.

## Processo

Cada subprojeto segue o mesmo caminho:

1. Spec nesta pasta, em português, apresentado e aprovado seção por seção.
2. Revisão do spec escrito.
3. Plano de implementação.
4. PR.

A documentação do usuário existe em seis línguas (`docs/manual/` em, pt,
es, de, fr e zh) e precisa acompanhar cada mudança.

## Como retomar (para quem continuar o trabalho)

- **Onde estamos:** subprojeto 1. O spec
  ([viewer-encoding.md](viewer-encoding.md)) foi aprovado pelo autor e o
  plano de implementação ([viewer-encoding-plan.md](viewer-encoding-plan.md))
  está em execução.
  - **PR 1 (guarda bidi, Tarefas 1 a 4): implementado** no branch
    `bidi-guard`, ainda sem push nem PR.
  - **PR 2 (módulo `decompress` e correção do Alt+F5, Tarefas 5 a 7):
    implementado** no branch `decompress`, ainda sem push nem PR.
  - **PR 3 (viewer mostra comprimidos e opção `viewer_decompress`, Tarefas
    8 e 9): implementado** no branch `viewer-decompress`, ainda sem push nem
    PR.
  - **PR 4 (codificações e a tecla `O`, Tarefas 10 e 11): implementado** no
    branch `viewer-charset`, ainda sem push nem PR.
  - **PR 5 (`viewer_max_size` e a RAM, Tarefa 12): implementado** no branch
    `viewer-max-size`, ainda sem push nem PR.
  - **PR 6 (`cargo audit` no CI, Tarefa 13): implementado** no branch
    `ci-audit`, ainda sem push nem PR.
  - **Subprojeto 1 implementado** nos PRs 1 a 6, em branches encadeados
    (`bidi-guard` → `decompress` → `viewer-decompress` → `viewer-charset` →
    `viewer-max-size` → `ci-audit`), nada enviado. **Próximo passo:** a
    revisão do branch inteiro, depois push e PRs.
- **Correções feitas no spec ao escrever o plano** (já incorporadas):
  - o `ruzstd` 0.9.1 já limita a janela a 128 MiB por padrão
    (`new_with_max_window_size`);
  - o limite do xz se calcula com `lzma_rust2::lzma2_get_memory_usage`
    (40 + 64 + dict/1024 KiB);
  - o `catch_unwind` precisa de um hook que não restaure o terminal dentro
    da área protegida (`src/panic_guard.rs`);
  - as letras do F12 seguem a ordem dos PRs (J, K, L).
- **Medições:** ficam em `~/.cache/xt-dev/viewer-mem/`:
  - `data/app.log`: log de 16 MiB;
  - `scripts/gen.py`: gera o log;
  - `scripts/measure.py`: roda o `xy` num pty com HOME isolado e mede RAM
    e CPU ao abrir com V.

  Resultado com o binário release de 2026-10-26: 16 MiB custam cerca de
  +40 MiB de RAM (2,5× o arquivo) e cerca de 75 ms de CPU.
- **Preferências do autor:**
  - segurança primeiro, com modelo de ameaça em todo desenho;
  - arquivos temporários só em `~/.cache/xt-dev/`, nunca em `/tmp`;
  - planos registrados aqui a cada passo aprovado;
  - commit só quando o autor pedir.
