# Excluir, lixeira e Prune

## Excluir

`D` ou Del exclui o arquivo sob o cursor (na árvore, o diretório, somente se
estiver vazio); Ctrl+D exclui os arquivos marcados. O Xylem sempre pergunta
antes.

Com `delete_to_trash = true` na [configuração](configuration.md), arquivos e
diretórios vazios vão para a lixeira do sistema (FreeDesktop Trash no Linux, a
Lixeira no Windows) em vez de serem removidos. Na confirmação, **T** envia para
a lixeira e **P** exclui permanentemente, qualquer que seja a configuração.
**Shift+Del** sempre exclui permanentemente.

## Oops

**Oops** (`O` na árvore, ou Ctrl+Z em ambas as janelas) restaura para o lugar
original o último item enviado para a lixeira. Sem nada enviado para a lixeira
nesta sessão, oferece o item mais recente da lixeira do sistema.

## Prune (Alt+P na árvore)

Exclui o diretório com tudo o que há nele. O Xylem lê todo o ramo, mostra
quantos arquivos, diretórios e bytes serão removidos e pede para você **digitar
PRUNE** (ou o nome do diretório, ou Y, dependendo de `prune_confirmation`). Ele
exclui os arquivos e depois os diretórios, do mais profundo para cima, com
progresso e `Esc` para cancelar. Links simbólicos são removidos sem serem
seguidos, e **outros pontos de montagem dentro do ramo são ignorados** (um
pendrive montado ali dentro não é excluído). A raiz da árvore não pode sofrer
Prune. O Prune sempre exclui permanentemente.
