# Busca nos arquivos marcados

## Ctrl+S

Busca um texto em todos os arquivos marcados da lista e **desmarca os que não o
contêm**, com barra de progresso e `Esc` para interromper. No prompt, **F2**
alterna a diferenciação de maiúsculas e minúsculas (ao ignorar
maiúsculas/minúsculas, os acentos também são considerados: `AÇÃO` encontra
`ação`) e **F4** alterna entre texto e hexadecimal (pares de bytes como
`4d 5a`). Um `*` no meio do termo corresponde a qualquer trecho na mesma linha
(`error*disk`); um termo nunca atravessa uma quebra de linha. Também funciona
dentro de [arquivos compactados](archives.md).

## Ctrl+V

Abre os arquivos marcados no [visualizador](viewer.md), um após o outro:
**N**ext e **P**rev alternam de arquivo (Alt+Home e Alt+End: primeiro e último)
e **U** desmarca o arquivo atual. O termo é compartilhado com o Ctrl+S: após um
Ctrl+S, **Space** no Ctrl+V vai direto para a próxima ocorrência, passando para
o próximo arquivo marcado quando o arquivo atual termina.

Um fluxo comum: marque arquivos (`T`, ou Ctrl+T em um Branch), use Ctrl+S com
um termo, abra com Ctrl+V e pressione Space para percorrer as ocorrências entre
os arquivos.
