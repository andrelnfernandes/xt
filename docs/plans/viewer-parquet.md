# Especificação Técnica: Visualizador Tabular de Apache Parquet (.parquet)

## 1. Visão Geral
Adiciona suporte nativo à visualização e filtragem do formato analítico colunar **Apache Parquet (`.parquet`)** dentro do modo tabela (`TableGrid`) do Xylem.

Ao abrir um arquivo `.parquet` com `V` (View); o Autoview (`F7`) não usa este leitor e mostra o arquivo em hexadecimal:
- Os metadados e esquema do arquivo são lidos via streaming seguro do Apache Arrow (`parquet` e `arrow-cast`, em Rust).
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
5. **Código em C isolado da navegação:**
   Os crates do Apache Arrow e os codecs `snap`, `flate2-zlib-rs` e `lz4_flex` são em Rust, mas o codec `zstd` usa `zstd-sys`, que compila a biblioteca em C. Por isso o Parquet só é lido com `V`, uma ação explícita, nunca no Autoview. Ver "Código em C" em [viewers.md](viewers.md).
