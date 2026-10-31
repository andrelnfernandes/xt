# Plano: Markdown renderizado no viewer

Objetivo: `.md` e `.markdown` abrem como documento, com o arquivo tratado como
hostil. Aprovado pelo autor em 2026-10-30, sem ícones Nerd Fonts.

## Decisões

- **Parser:** `pulldown-cmark`, sem as features padrão (sem SIMD). O parser do
  help não serve para arquivo hostil: é quadrático (`inline()` monta o resto do
  texto a cada caractere) e recursivo. O help fica como está, porque a página
  man depende dos blocos dele.
- **Duas etapas** (`src/viewer/markdown.rs`): `parse` gera *linhas lógicas*
  (parágrafo, item, linha de código, célula) e `Doc::layout` as quebra em linhas
  de tela para uma largura. A busca olha as linhas lógicas, então acha uma frase
  quebrada entre duas linhas da tela. A quebra conta células, não caracteres.
- **Modos:** `H` faz Rendered → Text → Hex; `T` alterna renderizado e fonte
  mantendo o lugar. `g`, `#` e `Ctrl+B` usam as linhas da fonte. `&` passa para
  o modo texto, porque o filtro é de linhas. O filtro não vale no renderizado.
- **Só por `Viewer::from_file`:** a saída do Git e as telas internas usam
  `from_bytes` e continuam texto.
- **Sem thread:** o tempo do parser cresceu de forma linear até 16 MiB nas
  entradas hostis medidas. O teto de tamanho limita a primeira passada, que não
  pode ser interrompida. Um caso quadrático não coberto pelas medições travaria
  a tela até terminar; uma thread seria a defesa, ao custo de CPU órfã.
- **Sem ícones:** Nerd Fonts não muda a renderização em si.

## Segurança

- Todo texto passa por `Line::push` **depois** do parser, porque `&#27;` e
  `&#x202E;` só viram caractere real ali. O contador bidi do cabeçalho conta
  esses casos.
- Um link sempre mostra o endereço real, inteiro. Nada é baixado nem aberto.
- HTML e comentários ficam visíveis. O bloco YAML do início fica visível.
- Limites: 8 MiB de entrada (acima disso, a fonte), prazo entre eventos (3 s no
  `V`, 100 ms no Autoview), memória de 16 × a entrada para as linhas e de novo
  para as linhas de tela, 32 contêineres com recuo, 256 colunas. Pânico vira
  modo texto, por `panic_guard::catch`.
- Sem recursão no código novo: a pilha é um `Vec`, limitado pela profundidade.

## Medições (release, 8 MiB, entradas hostis)

| Entrada | Parser | Layout |
|---|---|---|
| `>` repetido | 0,85 s | 0 |
| `[a](` repetido | 0,88 s | 75 ms |
| `*_` repetido | 0,74 s | 26 ms |
| tabela de 1,4 milhão de linhas | 0,38 s | 80 ms |
| parágrafos de uma letra | 0,54 s | 79 ms |
| código de uma linha só de tabs | 0,17 s | 0,62 s |
| texto comum | 0,10 s | 80 ms |

## Limite conhecido

O próprio Markdown esconde definições de link sem uso e células além das colunas
do cabeçalho. Não há como detectar isso de forma confiável, então a moldura diz
`RENDERED` e o manual explica que `T` mostra a fonte.

## Para depois

- Migrar o help para o mesmo parser.
- Notas de rodapé, hoje em texto literal.
