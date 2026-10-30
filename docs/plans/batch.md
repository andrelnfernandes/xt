# Plano: Batch (Ctrl+B) sobre arquivos e registros

Origem: conversa de 2026-10-30, depois dos visualizadores tabulares
([viewer-tables.md](viewer-tables.md), [viewer-parquet.md](viewer-parquet.md)).
Estado: etapas 1 a 5 implementadas; o restante é proposta.

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
   cancelamento e a saída sem o limite de linhas. Em `src/viewer/batch.rs`.
   Campos `%{nome}`, `%{A}`, `%{2}` (tabela), `%{line}`, `%{0}`, `%{ip}` (texto),
   `%{#}` e `%{#line}`; escapes `:sh :url :json :xml :sql :csv :csvsafe :line
   :raw`, com padrão pela extensão. Decisões aprovadas: o filtro SQL vale para
   todas as linhas, com a regra da tela (`007` = 7, mas sai `007`); a tabela é
   relida na hora; o texto sai como o viewer leu, com a linha original; `.bat` e
   `.cmd` recusados para registros; `.sh` sem permissão de execução. A prévia
   olha até 1.000 registros (e por 3 s no máximo): o total só vem no fim, e uma
   coluna de JSON Lines que só apareça depois disso é recusada.
4. **Codificadores de texto (feito):** JSON, JSONL, CSV, TSV, XML e HTML. Em
   arquivos desses formatos, máscara vazia grava os registros inteiros e `@a,b`
   só esses campos; qualquer outra coisa continua sendo máscara. Os campos vêm de
   uma primeira leitura de até 1.000 registros; coluna de JSON Lines que surja
   depois fica de fora, com aviso no fim. Nomes de coluna não viram nomes de
   elemento XML (`<field name="...">`), pois vêm de um arquivo hostil.
5. **Arquivos de template (feito):** `~/.config/xylem/batch/nome.tpl`, escolhido
   com `:nome` na máscara. Só um nome simples vale, sem caminho: `: %{a}` continua
   sendo máscara. Seções `%%header`, `%%record` e `%%footer`, e comentários `%%#`;
   sem seções, o arquivo inteiro é o registro, que pode ter várias linhas. Cabeçalho
   e rodapé não aceitam campos de registro. Campos novos `%{#path}`, `%{#dir}`,
   `%{#name}`, `%{#base}`, `%{#ext}` e `%{#size}`, do arquivo aberto, valem também
   nas máscaras: um `%N` em uma página seria injeção, pois não tem escape. Sem
   separador entre registros: um array JSON sai pelo codificador nativo. Vale só
   para o Batch de registros, não para o de arquivos marcados.
   Depois, o escape `:mysql`: em MySQL e MariaDB a barra invertida também escapa,
   então um valor com `\'` fecharia um literal de `:sql`. `.sql` continua com `:sql`
   por padrão, pois a extensão não diz o banco; a prévia avisa sobre esses valores.

   **5b. Templates com ambiente, macros, totais e condições (proposta):** em três
   passos, nesta ordem.

   1. **`count`, ambiente e macros:**
      - `%{count}` no rodapé. Totais só no rodapé, pois o cabeçalho é gravado antes
        de ler os registros.
      - `%{$NOME}` lê qualquer variável do processo, com escape. Variável inexistente
        é erro na prévia, nunca valor vazio. O valor é o da geração; um `$NOME`
        escrito em um `.sh` continua sendo o da execução. A prévia lista as variáveis
        lidas: um template de terceiros poderia mandar um segredo para fora quando o
        script rodasse, e deve ser tratado como um script dessa pessoa.
      - Macros com o prefixo `#`: `#date`, `#time`, `#datetime`, `#utc`, `#weekday`
        (ISO, segunda = 1), `#weekday_name` (sempre em inglês), `#year`, `#month`,
        `#day`, `#epoch`, `#user`, `#host`, `#template` e `#output`. A hora é fixa, a
        de quando o template é lido, igual em todos os registros; é local, com o fuso
        em `#datetime`. O `chrono` já é dependência.
   2. **`if` e `stop`:** `%%if`, `%%else` e `%%end` dentro do registro, com a linguagem
      do filtro SQL, que já tem parser e avaliador. `%%stop if expressão` para a
      leitura e cobre o `until`. Como a prévia olha só 1.000 registros, ela avisa
      quais ramos não foram vistos.
   3. **`sum`, depois `min`, `max` e `avg`:** em decimal exato, nunca ponto flutuante,
      pois pode ser dinheiro. O formato do número é explícito (`%{sum:valor:br}`), já
      que `1.234,56` e `1,234.56` são ambíguos em um CSV. Valor inválido não é ignorado
      em silêncio: a mensagem final diz quantos ficaram de fora.

   Fora de escopo: `while` genérico, que exigiria variáveis que mudam e limites contra
   laços infinitos com dados hostis; atribuição de variáveis; laços aninhados; totais
   por grupo; `%{#date:formato}` e `%{#uuid}` até fazerem falta. Se um dia pedir `each`
   e grupos, trocar pelo `minijinja` em vez de crescer uma linguagem própria.
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
