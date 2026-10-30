# Filtrar linhas

No modo texto do visualizador, `&` mostra só as linhas que combinam. As linhas
ficam escondidas, não são alteradas: com os números de linha ligados (`#`), cada
linha mantém o seu número no arquivo, e as lacunas mostram o filtro agindo. O
cabeçalho diz o resultado: `[filter error: 24 of 850 lines]`.

| Tecla no prompt | O que faz |
|---|---|
| **F1** | esta ajuda |
| **F2** | diferenciar maiúsculas ou ignorar |
| **F3** | inverter: manter as linhas que não combinam |
| **F4** | texto simples ou expressão regular |
| ↑ ↓ | filtros anteriores |
| Enter | aplicar |
| Esc | cancelar |

Aperte `&` de novo para mudar o filtro. **Enter com o prompt vazio desliga o
filtro**, e a linha no alto da tela continua lá. Um filtro que não encontra nada
ou é inválido não muda nada, e o rodapé diz o motivo.

Com um filtro ligado, a rolagem e a [busca](viewer.md) percorrem só as linhas
mantidas. `g` recebe um número de linha do arquivo; se essa linha estiver
escondida, vai para a próxima mantida. O filtro continua depois de editar o
arquivo (`E`) ou mudar a codificação (`O`), e termina quando o visualizador
fecha.

Só o que o visualizador leu é filtrado: se o cabeçalho diz `(showing first N
MiB)`, o resto do arquivo não é examinado. O filtro vê as linhas como elas
aparecem, então uma tabulação é espaços (use `\s`, não `\t`) e um caractere de
controle é `·`. No modo tabela, `&` abre o filtro SQL.

## Texto simples

As mesmas regras da busca: ignorando maiúsculas, os acentos também não contam
(`AÇÃO` encontra `ação`), e um `*` no meio combina com qualquer coisa na mesma
linha (`erro*disco`).

## Expressões regulares

O termo pode combinar em qualquer lugar da linha, a não ser que você o ancore.
Maiúsculas são ignoradas, a não ser que você aperte **F2**; aqui os acentos
contam.

| Escreva | Para combinar com |
|---|---|
| `.` | qualquer caractere |
| `\d` `\D` | um dígito, qualquer coisa menos dígito |
| `\w` `\W` | letra, dígito ou `_`, qualquer outra coisa |
| `\s` `\S` | um espaço, qualquer outra coisa |
| `[abc]` `[^abc]` | um destes, qualquer coisa menos estes |
| `[a-z0-9]` | um nestas faixas |
| `^` `$` | o início, o fim da linha |
| `\b` | a borda de uma palavra |
| `a*` `a+` `a?` | zero ou mais, um ou mais, zero ou um |
| `a{3}` `a{2,5}` | exatamente 3, de 2 a 5 |
| `a+?` | um ou mais, o mínimo possível |
| `(abc)` | um grupo, para repetir ou dar alternativas |
| `(?P<nome>abc)` | um grupo com nome |
| `(?i)` `(?-i)` | ignorar maiúsculas ou diferenciar daqui em diante |

## Expressões regulares

O termo pode combinar em qualquer lugar da linha, a não ser que você o ancore.
Maiúsculas são ignoradas, a não ser que você aperte **F2**; aqui os acentos
contam.

| Escreva | Para combinar com |
|---|---|
| `.` | qualquer caractere |
| `\d` `\D` | um dígito, qualquer coisa menos dígito |
| `\w` `\W` | letra, dígito ou `_`, qualquer outra coisa |
| `\s` `\S` | um espaço, qualquer outra coisa |
| `[abc]` `[^abc]` | um destes, qualquer coisa menos estes |
| `[a-z0-9]` | um nestas faixas |
| `^` `$` | o início, o fim da linha |
| `\b` | a borda de uma palavra |
| `a*` `a+` `a?` | zero ou mais, um ou mais, zero ou um |
| `a{3}` `a{2,5}` | exatamente 3, de 2 a 5 |
| `a+?` | um ou mais, o mínimo possível |
| `(abc)` | um grupo, para repetir ou dar alternativas |
| `(?P<nome>abc)` | um grupo com nome |
| `(?i)` `(?-i)` | ignorar maiúsculas ou diferenciar daqui em diante |

A barra vertical separa alternativas: `erro|falha|fatal`, ou
`disco (cheio|erro)` dentro de um grupo.

Para combinar com um dos caracteres especiais `. * + ? ( ) [ ] { } ^ $ \` ou com
a própria barra, ponha uma `\` antes: `\.log$`, `\(1\)`.

## Exemplos

- `^\d{4}-\d{2}-\d{2}`: linhas que começam com uma data
- `\b(erro|fatal)\b`: estas palavras inteiras, em maiúsculas ou minúsculas
- `^\s*$` com **F3**: esconde as linhas em branco
- `^\s*#` com **F3**: esconde os comentários de um script shell
- ` 50\d `: erros HTTP de 500 a 509 em um log de acesso
- `\b\d{1,3}(\.\d{1,3}){3}\b`: linhas com um endereço IPv4
- `(?-i)TODO`: só em maiúsculas, mesmo ignorando maiúsculas

Referências a grupos (`\1`) e lookaround (`(?=...)`) não existem aqui, de
propósito: sem eles, o tempo cresce só com o tamanho do texto, e nenhum arquivo
ou expressão trava o visualizador com um padrão como `(a+)+$`.

## Texto simples

As mesmas regras da busca: ignorando maiúsculas, os acentos também não contam
(`AÇÃO` encontra `ação`), e um `*` no meio combina com qualquer coisa na mesma
linha (`erro*disco`).

## Com um filtro ligado

Veja também: [o visualizador](viewer.md).
