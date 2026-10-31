# Uso do disco (Alt+W)

Para onde vai o espaço. **Alt+W** lê todo o ramo do diretório atual e lista
suas pastas e arquivos pelo espaço que ocupam, do maior para o menor, com uma
barra e sua fração do diretório. Enter entra em uma pasta; de pasta em pasta,
você chega ao que está enchendo o disco.

```
       32.9 GiB ████████████████   45%   wsl-ubuntu-backup.tar
       18.9 GiB █████████▏         26%  /projetos
 .      5.5 GiB ██▋               7.6%  /.local
 >    154.6 MiB                      -  /boot
```

- **A barra** é desenhada em relação ao maior item do diretório, para que as
  diferenças entre os itens fiquem visíveis em uma trilha da largura do maior;
  **a porcentagem** é a fração que o item ocupa no diretório. A cor indica o
  tamanho dessa fração: até 10% (verde, na maioria dos temas), até 40% (âmbar),
  ou mais (vermelho). Cada tema tem as suas (`bar_low`, `bar_mid` e `bar_high`
  em `[colors]`).
- **Pastas e arquivos ficam misturados**, por tamanho: um único arquivo grande
  (um backup antigo, uma imagem ISO) aparece entre as pastas. As pastas começam
  com `/`.
- **A linha na parte inferior** traz os totais do diretório: no disco,
  aparente, o número de arquivos e quanto eles representam do espaço usado do
  disco e do disco inteiro. No topo de um disco, ela informa quanto do espaço
  usado foi encontrado: o que falta está em pastas que não puderam ser lidas,
  nos próprios metadados do sistema de arquivos ou em arquivos excluídos que
  ainda estão abertos.
- **O painel à direita** mostra o item sob o cursor: o espaço no disco e o
  aparente, arquivos, data e sua fração do espaço usado e do disco.

## No disco ou aparente

Por padrão, os tamanhos refletem **o espaço no disco**, como o `du` conta: um
arquivo ocupa blocos inteiros (4 KiB, geralmente), de modo que um arquivo
pequeno ocupa mais do que seu tamanho, e um arquivo esparso (o disco de uma
máquina virtual) ou comprimido ocupa menos. **A** alterna para o tamanho
**aparente**, os tamanhos próprios dos arquivos. A linha inferior sempre mostra
ambos.

## O que a análise contabiliza

- **Um disco por vez.** Outro sistema de arquivos montado dentro do ramo (outro
  disco, `/boot`, um pendrive, uma pasta montada pelo rclone, `/proc`) não é
  lido nem somado: aparece com `>` e o espaço que aquele sistema de arquivos
  usa, e Enter inicia uma análise dele.
- **Hard links contam uma só vez.** Dois nomes para o mesmo arquivo ocupam seu
  espaço apenas uma vez: o primeiro encontrado é somado, e os demais aparecem
  com `H` e sem tamanho.
- **Arquivos ocultos são contados**, mesmo quando a árvore não os exibe, para
  que os totais fechem.
- Links (`@`) contam pelo próprio link; eles não são seguidos.

## Marcações

| Marca | Significado |
|---|---|
| `>` | outro disco, não contabilizado |
| `!` | a pasta não pôde ser lida (geralmente por permissão) |
| `.` | algo lá dentro não pôde ser lido: pode ocupar mais espaço |
| `H` | outro hard link de um arquivo já contabilizado |
| `@` | um link simbólico ou arquivo especial |
| `e` | uma pasta vazia |

## Teclas

| Tecla | O que faz |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move o cursor |
| Enter ou → | entra na pasta (em `>`: analisa aquele disco) |
| Backspace ou ← | volta para a pasta acima |
| `\` | fecha a visualização, posicionando a árvore na pasta listada |
| `A` | alterna entre espaço no disco e aparente |
| `C` | mostra ou oculta o número de arquivos |
| `S` | ordena por tamanho, nome, número de arquivos ou data |
| `D` | procura [duplicados](duplicates.md) na pasta listada |
| F3 | relê a pasta listada |
| Esc ou Alt+W | fecha |

Tab, F8, Alt+X (o terminal) e F12 funcionam normalmente com a visualização
aberta.

Analisar um disco grande lê cada pasta para a memória: leva algum tempo (o
rodapé mostra o progresso e Esc cancela) e consome alguma memória, cerca de
algumas centenas de megabytes para milhões de arquivos. Como usuário comum,
algumas pastas do sistema não podem ser lidas (`!`), por isso os totais de `/`
ficam abaixo do total usado: a linha inferior informa quantas pastas falharam.
