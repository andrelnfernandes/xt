# Especificação Técnica: Visualizador Tabular de Apache Parquet (.parquet)

## 1. Visão Geral
Adiciona suporte nativo à visualização e filtragem do formato analítico colunar **Apache Parquet (`.parquet`)** dentro do modo tabela (`TableGrid`) do Xylem.

Ao abrir um arquivo `.parquet` com `V` (View) ou inspecioná-lo com `F7` (Autoview):
- Os metadados e esquema do arquivo são lidos via streaming seguro do Apache Arrow (`parquet` e `arrow-cast` em Rust 100% puro).
- A grade tabular TUI é construída com suporte a navegação por células, cabeçalhos de coluna, status detalhado e busca.
- O mini-avaliador SQL (`\`) funciona imediatamente sobre o grid em memória.
- A tecla `T` alterna ciclicamente entre modo Tabela, Texto e Hexadecimal.

## 2. Modelo de Ameaça e Segurança
1. **Validação de Assinatura Magic Bytes:**
   O arquivo deve iniciar e terminar com a assinatura de 4 bytes `PAR1`. Arquivos arbitrários ou truncados são rejeitados de imediato antes de acionar parsers de descompressão ou metadados.
2. **Defesa contra Decompression Bombs:**
   Leitura em streaming por lotes (`RecordBatch` de 1.024 registros), parando a leitura no teto de `MAX_TABLE_ROWS = 50.000`. Arquivos com bilhões de registros abrem com consumo de memória controlado e sinalização visual de truncamento.
3. **Esquemas Maliciosos / Schema Flooding:**
   Teto de `MAX_TABLE_COLS = 256` colunas. Colunas excedentes são descartadas de forma segura.
4. **Sanitização de Células:**
   Todas as células e nomes de colunas passam por `table::sanitize_cell`:
   - Remoção de códigos de controle C0/C1 (prevenção contra injeção ANSI).
   - Sanitização de caracteres de controle bidirecional Unicode (prevenção contra Trojan Source CVE-2021-42574).
   - Truncamento em 2.048 caracteres por célula com elipse `…`.
5. **Pure Rust Safe Code:**
   Uso exclusivo de crates oficiais Apache Arrow compilados em safe Rust com codecs nativos (`snap`, `flate2-zlib-rs`, `lz4_flex`, `zstd`), sem bibliotecas C externas adicionadas.
