# Plano: visualizadores tabulares, planilhas, bancos de dados e formatos modernos

Origem: conversa de 2026-10-30, desdobramento do Subprojeto 2 de
[docs/plans/viewers.md](viewers.md).

## 1. Objetivo

Permitir que o visualizador do Xylem (`V` e Autoview `F7`) apresente dados
estruturados em formato tabular com grade interativa em modo texto, busca rápida
e suporte a consultas/filtros em sintaxe SQL.

O escopo abrange desde formatos clássicos até planilhas e formatos amplamente
utilizados no ecossistema moderno de desenvolvimento e dados:
1. **Planilhas:** XLSX, ODS (e XLS legado).
2. **Formatos tabulares de texto:** CSV, TSV e JSONL (JSON Lines / NDJSON).
3. **Formatos legados estruturados:** DBF (dBase III/IV/Clipper, com campos memo `.DBT`/`.FPT`) e SDF.
4. **Bancos de dados embarcados:** SQLite (`.db`, `.sqlite`, `.sqlite3`).
5. **Formatos modernos analíticos / Big Data:** Apache Parquet (`.parquet`).

---

## 2. Princípios de Segurança (Modelo de Ameaça)

Seguindo as diretrizes obrigatórias do Xylem, todo arquivo visualizado é tratado
como potencialmente hostil (pode conter payloads maliciosos, zip bombs, loops
de descompressão ou sequências de escape voltadas a sequestrar o terminal).

1. **Somente leitura estrita:**
   - Nenhum parser ou conexão tem permissão de escrita ou alteração de metadados.
   - Conexões ao SQLite usam obrigatoriamente flags defensivas:
     `mode=ro`, `immutable=1` e `SQLITE_DBCONFIG_DEFENSIVE` (bloqueando `ATTACH`,
     criação de triggers, funções externas em C e `PRAGMA writable_schema`).
2. **Defesa contra descompressão descontrolada (Zip Bomb / XML Bomb):**
   - Planilhas (XLSX, ODS) são archives ZIP contendo XMLs. O leitor impõe
     orçamento estrito de descompressão em memória, respeitando `viewer_max_size`.
   - Limite máximo de linhas e colunas carregadas por tabela/aba (ex.: 50.000 linhas
     por visualização inicial, com aviso `(showing first N rows)`).
3. **Higienização completa de células no terminal:**
   - Caracteres de controle (C0, DEL, C1) viram `·`, prevenindo injeções de escape ANSI.
   - **Trojan Source (CVE-2021-42574):** Os 12 caracteres bidi de controle são
     interceptados pelo filtro do Xylem em `ui::draw`, sendo exibidos de forma segura.
   - **Formula Injection:** Células iniciadas por `=`, `+`, `-`, `@` são tratadas
     estritamente como texto literal, sem qualquer avaliação de expressões externas.
4. **Resiliência a Pânico:**
   - Parsers e avaliadores de query cobertos por testes de propriedade (`proptest`)
     com bytes aleatórios e malformados.
5. **Arquivos temporários:**
   - Caso seja necessária extração temporária (ex.: arquivo remoto ou dentro de
     archive), usar unicamente `~/.cache/xt-dev/` ou o diretório de execução
     privado (modo 0700), nunca `/tmp`.

---

## 3. Experiência de Uso (TUI no Ratatui)

### 3.1. Grade Tabular
* Cabeçalhos de coluna com letras (`A`, `B`, `C`...) para planilhas ou nomes dos
  campos quando disponíveis (CSV com cabeçalho, DBF, SQLite, Parquet).
* Numeração de linhas à esquerda (`1`, `2`, `3`...).
* Cursor de célula ativa com destaque visual nas cores do tema atual.
* Rodapé informativo: aba atual, coordenadas da célula (ex.: `B14`), tipo de dado
  e conteúdo integral da célula ativa (permitindo ler textos longos que sofreram truncamento na grade).

### 3.2. Navegação com Teclado
* `Setas` / `H`, `J`, `K`, `L`: Movimentam a célula ativa / rolam a tabela.
* `PgUp` / `PgDn`: Rolagem por página vertical.
* `Home` / `End`: Início / fim da linha atual (ou início/fim da tabela com modificadores).
* `[` e `]`: Alternar entre abas (*sheets*) de planilhas ou tabelas de um banco SQLite.
* `T`: Alternar entre modo tabela formatado e visualização de texto puro/hex.

---

## 4. Mecanismo de Busca e Consultas SQL

Dois níveis de busca integrados:

### 4.1. Busca Rápida de Texto (`/` ou `F`)
* Busca substring case-insensitive em todas as células visíveis ou em toda a tabela.
* `Space` ou `+`: Próxima ocorrência.
* `-`: Ocorrência anterior.

### 4.2. Consultas e Filtros SQL (`\` ou `Alt+S`)
Abre prompt de comando inferior para filtragem e consulta:

1. **Para SQLite (SQL Nativo):**
   - O usuário pode digitar consultas completas de leitura:
     ```sql
     SELECT id, nome, saldo FROM clientes WHERE saldo > 1000 ORDER BY saldo DESC
     ```
   - A consulta é submetida em modo `READONLY` diretamente ao motor do SQLite e o
     conjunto retornado substitui a grade de exibição.

2. **Para Planilhas, CSV, DBF, Parquet e JSONL (Mini-avaliador SQL em Rust puro):**
   - O usuário digita condições no formato `WHERE ...`:
     ```sql
     WHERE status = 'ativo' AND (idade >= 18 OR A > 100)
     ```
     *(ou de forma simplificada: `saldo > 500 AND cidade LIKE '%São Paulo%'`)*.
   - **Gramática suportada pelo Mini-avaliador:**
     - Operadores de comparação: `=`, `!=`, `<>`, `>`, `<`, `>=`, `<=`.
     - Operadores lógicos: `AND`, `OR`, `NOT`, parênteses `( ... )`.
     - Operadores de string: `LIKE` (`%`, `_`), `ILIKE` (sem diferenciar maiúsculas/minúsculas).
     - Operadores de conjunto/nulo: `IN ('a', 'b')`, `IS NULL`, `IS NOT NULL`.
     - Referência a colunas: por nome (`cidade`) ou letra (`A`, `B`).
   - Linhas que satisfizerem a condição são exibidas na grade; a barra de status
     exibe a contagem de linhas filtradas (ex.: `[Filtro: 24/850 linhas]`). `Esc` limpa o filtro.

---

## 5. Seleção de Bibliotecas no Ecossistema Rust

| Formato | Crate / Solução | Características |
|---|---|---|
| **Planilhas (XLSX, ODS, XLS)** | [`calamine`](https://crates.io/crates/calamine) | 100% Rust puro, somente-leitura, rápida, sem macros. |
| **CSV / TSV** | [`csv`](https://crates.io/crates/csv) | Rust puro, amplamente auditado, suporte a delimitadores dinâmicos. |
| **JSONL / NDJSON** | `serde_json` | Já presente nas dependências do Xylem, robusto e seguro. |
| **DBF / Memo** | [`dbase`](https://crates.io/crates/dbase) ou parser próprio | Leitura direta do cabeçalho binário, codepages históricos e leitura de `.dbt`/`.fpt`. |
| **SQLite** | `rusqlite` (bundled, com modo defensivo obrigatório) ou `limbo` | Suporte SQL completo em modo estritamente somente-leitura e defensivo. |
| **Apache Parquet** | [`parquet`](https://crates.io/crates/parquet) (Apache Arrow) | Rust puro oficial, leitura em streaming de metadados e row groups. |
| **Mini-SQL Evaluator** | Desenho próprio em Rust puro | ~350 linhas, zero dependências extras, sem alocações descontroladas, 100% testável com `proptest`. |

---

## 6. Etapas de Implementação

1. **Etapa 1:** Motor tabular comum em `src/viewer/table.rs` (estrutura de grade `TableGrid`, sanitização de células, rolagem e destaque de cursor).
2. **Etapa 2:** Mini-avaliador de predicados SQL em Rust puro (`src/viewer/sql_filter.rs`) com testes unitários e `proptest`.
3. **Etapa 3:** Integração dos leitores de CSV/TSV e JSONL.
4. **Etapa 4:** Integração do leitor de Planilhas via `calamine` (XLSX, ODS) com suporte a navegação entre abas.
5. **Etapa 5:** Leitor de DBF (com suporte aos campos memo).
6. **Etapa 6:** Integração com SQLite (modo defensivo e execução de SELECTs).
7. **Etapa 7:** Integração com Apache Parquet.
8. **Etapa 8:** Manuais atualizados nos 6 idiomas (`docs/manual/**`) e validações obrigatórias (`cargo fmt`, `cargo clippy`, `cargo test`).
