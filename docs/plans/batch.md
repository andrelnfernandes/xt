# Plano: Batch (Ctrl+B) sobre arquivos e registros

Origem: conversa de 2026-10-30, depois dos visualizadores tabulares
([viewer-tables.md](viewer-tables.md), [viewer-parquet.md](viewer-parquet.md)).
Estado: etapa 1 (filtro de linhas, tecla `&`, ajuda em F1) implementada; o
restante é proposta.

## 1. Objetivo

Levar o Batch além do que XTree e ZTreeWin oferecem:

1. continuar funcionando sobre os arquivos marcados, como hoje;
2. atuar também sobre os registros lidos pelo viewer, filtrados ou não (por
   exemplo, o resultado de uma query em uma tabela SQLite);
3. gerar, além de texto (`txt`, `sh`, `bat`), saídas estruturadas (JSON, XML,
   HTML) e binárias;
4. integrar o filtro de linhas do viewer de texto (simples ou regex), que
   ainda não existe.

## 2. Ponto de partida

O Ctrl+B já existe, no contexto de arquivos ([actions.rs](../../src/app/actions.rs),
`write_batch`; [shell.rs](../../src/shell.rs), `batch_line`):

- pede o arquivo e a máscara, e grava uma linha por arquivo marcado;
- parâmetros `%f %d %n %b %e %s`: em minúsculas saem com aspas para o shell,
  em maiúsculas como texto puro;
- `.sh` ganha `#!/bin/sh` e permissão de execução; `.bat`/`.cmd` ganham
  `@echo off`;
- arquivo existente não é sobrescrito (`create_new`).

Esse comportamento é mantido. No contexto da árvore, Ctrl+B continua sendo
"marcar diretório como bookmark".

## 3. Arquitetura

Um único fluxo, em vez de um Batch de arquivos e outro de registros:

**fonte de registros → filtro → codificador → destino**

| Fonte | Campos de cada registro |
|---|---|
| Arquivos marcados | path, dir, nome, base, ext, tamanho, data |
| Grade tabular | as colunas, com ou sem filtro |
| Texto | número da linha, a linha e os grupos de captura da regex |

O filtro de texto é o que mais amplia o recurso: com grupos nomeados, como
`(?P<ip>\S+) .* (?P<status>\d{3})`, um log vira uma tabela.

## 4. Pré-requisitos no código atual

1. **A grade não serve para exportar.** `TableGrid::sanitize_cell` troca
   caracteres de controle e bidi por `·` e corta em 2048 caracteres já no
   carregamento. No SQLite, texto acima de 8 KiB é cortado e blob vira
   `<Blob N bytes>`. A grade só guarda strings, e `Value::infer` transformaria
   `007` em `7` (corrompendo um CEP no JSON). Correção: sanear só ao desenhar,
   ou manter o valor bruto e tipado separado do texto de exibição.
2. **O limite de tela truncaria a saída.** `MAX_TABLE_ROWS` (50.000) é limite
   de exibição e o export não pode herdá-lo. Os loaders precisam virar um
   iterador de linhas, do qual a grade é só um consumidor. É o maior refactor
   do projeto.
3. **Consultas não são canceláveis.** Não há `progress_handler` nem interrupção
   no viewer; uma CTE recursiva pode travar a tela. O export roda em segundo
   plano e é cancelável, como `archive::Extractor`. A trava já existe hoje na
   tela, independentemente do Batch.

## 5. Segurança

Os valores deixam de ser só nomes de arquivo: passam a ser conteúdo arbitrário
de um arquivo possivelmente hostil, gravado em um script executável.

Exemplo do problema: `curl "https://api/x?id=%{id}"`. Com o quoting de shell
atual sai `"...id='abc'"`, valor errado; com o texto puro atual, `$(...)` abre
injeção de comando.

Regras:

- **Escape por contexto, explícito:** `%{id:url}`, `%{id:sh}`, `%{id:json}`,
  `%{id:xml}`, `%{id:sql}`, `%{id:csv}`.
- **Padrão seguro:** o padrão vem do tipo de saída; `:raw` precisa ser pedido
  por escrito.
- **Quebras de linha:** um valor com newline em `:raw` não pode criar uma linha
  nova em um `.sh`.
- **Windows:** o `quote` só põe aspas duplas e `%VAR%` ainda expande (item 3 de
  Security no [BACKLOG](../../BACKLOG.md)). Registros em `.bat` só depois de
  resolver isso.
- **Prévia:** mostrar as primeiras linhas antes de gravar, com a contagem de
  valores com caracteres suspeitos.
- **Execução:** sem `chmod +x` automático quando a fonte for um registro.
- **CSV:** opção para neutralizar células iniciadas por `=`, `+`, `-` ou `@`
  (injeção de fórmula em planilhas).
- **Regex:** crate `regex`, de tempo linear, nunca `fancy-regex`, que faz
  backtracking. O `regex` já está no `Cargo.lock` de forma transitiva.

## 6. Tipos de saída

- **Templates, para texto livre:** a máscara de uma linha atual, estendida com
  `%{coluna:filtro}`, e arquivos de template com cabeçalho, linha e rodapé,
  escolhidos por nome.
- **Codificadores nativos, para formatos estruturados:** JSON, JSONL, CSV, XML
  e HTML. Basta escolher o formato e as colunas; vírgulas e escape ficam
  garantidos pelo codificador.
- **Binário, só por codificador, nunca por template:** SQLite primeiro
  (`rusqlite` já é dependência); Parquet depois (o crate já está no projeto,
  mas exige os tipos do item 1 da seção 4); XLSX por último (dependência nova).

Sem motor de templates completo (`minijinja` e afins) por enquanto: laços e
condicionais só pagam o custo quando máscara, cabeçalho e rodapé não bastarem.

## 7. Etapas

1. **Filtro de linhas no viewer de texto (feito):** substring e regex, tecla
   `&` (a mesma do `less`; no modo tabela abre o filtro SQL). O filtro não altera `Viewer::lines`: guarda à parte os índices das
   linhas visíveis, e o desenho, a rolagem, o Goto e a busca passam a percorrer
   esses índices. Assim a margem continua mostrando o número original da linha,
   com lacunas, e as cores de sintaxe seguem calculadas sobre o texto inteiro.
2. **Valor bruto separado da exibição e iterador de registros:** cada formato
   vira um fluxo em `src/viewer/records.rs`, com valores tipados (`Cell`: nulo,
   booleano, inteiro, real, texto, bytes), sem cortes nem saneamento e sem limite
   de linhas. Quem recebe (`RecordSink`) pode mandar parar. A grade da tela vira
   um consumidor: saneia, corta e para em 50.000 linhas, avisando no cabeçalho.
   O limite de tempo do viewer vale para as tabelas (`Budget`), com
   `progress_handler` no SQLite. O filtro SQL continua avaliando a grade, com as
   mesmas limitações; muda na etapa 3, que precisa filtrar tudo.
3. **Ctrl+B no viewer com máscara e escape por contexto:** inclui a prévia, o
   cancelamento e a saída sem o limite de linhas.
4. **Codificadores de texto:** JSON, JSONL, CSV, XML e HTML.
5. **Arquivos de template.**
6. **Saídas binárias:** SQLite primeiro.
7. **Registros marcados na grade:** o equivalente ao Tag para registros. O `T`
   já alterna entre tabela e texto, então precisa de outra tecla.

## 8. Decisões

- O Batch sempre processa o conjunto inteiro da fonte com o filtro ativo, nunca
  só o que está na tela, e informa o total processado.
- `create_new` é mantido: nunca sobrescreve um arquivo existente.
- Saída para `| comando` fica fora do início: gerar um arquivo que se revisa e
  depois se executa é mais seguro com dados hostis.
- A sintaxe `%x` existente continua valendo; `%{...}` é uma extensão dela.
- Dois números distintos para registros de texto: o número da linha no arquivo
  (o da margem, com lacunas, que permite conferir o filtro e voltar à origem) e
  o sequencial do registro no resultado filtrado (1, 2, 3...), que o Batch
  oferece como campo próprio. O rodapé mostra a contagem, como `24/850 linhas`.
