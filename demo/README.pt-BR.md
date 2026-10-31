# Arquivos de demonstração

Arquivos para testar o [visualizador](../docs/manual/pt/viewer.md) e o
[Batch no visualizador](../docs/manual/pt/execute.md) (Ctrl+B): os mesmos nove clientes
fictícios em todos os formatos, e templates que os transformam em outros arquivos.

## Os dados

| Arquivo | Como abre |
|---|---|
| `data/customers.csv`, `.tsv` | tabela |
| `data/customers.jsonl` | tabela |
| `data/customers.dbf` | tabela |
| `data/customers.db` (SQLite, com as tabelas `customers` e `orders`; `[` `]` trocam) | tabela, com `V` |
| `data/customers.parquet` | tabela, com `V` |
| `data/customers.xlsx` (com as abas `customers` e `orders`; `[` `]` trocam) | tabela, com colunas A, B, C; a primeira linha é uma linha como as outras |
| `data/customers.json`, `.xml` | texto, com cores |
| `data/access.log` | texto, para o filtro de linhas |

Os quatro últimos clientes são **hostis de propósito**: um nome como
`<script>alert(1)</script>` ou `Robert'); DROP TABLE customers;--`. Eles estão ali para
mostrar cada escape funcionando, em uma página, um script ou SQL. Abri-los não executa nada.

## Os templates

Copie-os para onde o Xylem procura templates:

```bash
mkdir -p ~/.config/xylem/batch && cp demo/templates/*.tpl ~/.config/xylem/batch/
```

(No Windows, `%APPDATA%\xylem\batch\`.)

Depois abra os dados, não o template: uma tabela de `demo/data` com `V`. Aperte **Ctrl+B**, informe o nome do arquivo a gravar e
digite `:nome` como máscara, ou escolha com **↑↓** (**F3** mostra o template inteiro). A extensão desse arquivo escolhe o escape, então use a da
tabela. A prévia mostra as primeiras linhas: **Enter** grava e **Esc** desiste.

| Máscara | Gravar em | O que sai |
|---|---|---|
| `:report` | `report.html` | uma página com uma tabela e o total |
| `:table` | `customers.md` | uma tabela em Markdown |
| `:update` | `update.sql` | um `UPDATE` por registro, em uma transação deixada aberta |
| `:bulk_fix` | `bulk_fix.sql` | muitas correções em um só `UPDATE`, com uma tabela temporária |
| `:undo` | `undo.sql` | o caminho de volta de uma correção, gerado antes dela |
| `:missing` | `missing.sql` | `SELECT`s que listam o que um banco não tem |
| `:requests` | `requests.sh` | um `curl` por registro, com escape para o endereço |
| `:one_file_each` | `split.sh` | um script que grava um arquivo por registro |
| `:contacts` | `contacts.vcf` | contatos para o celular ou o e-mail |
| `:elastic_bulk` | `bulk.ndjson` | duas linhas por registro, para o Elasticsearch |
| `:log_to_sql` | `log.sql` | o log como banco: veja abaixo |

Teste o mesmo template com `customers.csv` e `customers.parquet`: o texto é o mesmo, mas
`id` é texto em um e número no outro, então o `.sql` recebe `'1'` ou `1`.

Esses templates usam nomes de coluna, então não funcionam com `customers.xlsx`, que tem
letras: nele, experimente a máscara `%{B};%{C}` em um `.txt`.

## Outras coisas para testar

**Máscaras, sem template.** Em `out.txt`: `%{#} %{name:url}`. Em `out.sh`: `echo %{name}`.
Em `out.json`, uma máscara vazia grava registros inteiros, e `@id, name` só esses campos.

**O filtro SQL.** Em uma tabela, `&` filtra as linhas, e o Ctrl+B grava só essas:
`plan = 'pro'` ou `balance > 100`.

**Filtrar linhas.** Em um texto, `&` deixa só as linhas que batem: abra `data/access.log`
e digite `500`. Um arquivo que abre como tabela tem o filtro SQL no `&`: aperte `T` antes
para ver o texto.

**Um log como tabela.** Abra `data/access.log`, aperte `&`, depois **F4** para expressão
regular, e filtre com:

```
^(?P<ip>\S+) .*"(?P<method>[A-Z]+) (?P<path>.*) HTTP/[\d.]+" (?P<status>\d{3})
```

Depois, Ctrl+B com a máscara `:log_to_sql` em `log.sql`, e:

```bash
sqlite3 log.db < log.sql
```

```bash
sqlite3 log.db 'SELECT status, count(*) FROM hits GROUP BY status'
```

**Os scripts SQL.** Foram feitos para uma tabela chamada `customers`, como a de
`customers.db`. Para testar, trabalhe em uma cópia:

```bash
cp demo/data/customers.db copia.db
```

```bash
sqlite3 copia.db < update.sql
```

Os scripts saem sem permissão de execução: leia-os antes. O `requests.tpl` precisa de
`API_URL` definido ao rodar, e não há servidor real por trás da demonstração.

## Como gerar de novo

Quem os grava é o [batch_demo.rs](../src/viewer/batch_demo.rs), cujos testes também rodam
cada template com cada tabela, e executam de verdade o SQL e os scripts:

```bash
cargo test --bin xy write_demo_files -- --ignored
```
