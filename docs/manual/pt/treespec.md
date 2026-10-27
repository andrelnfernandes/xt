# Treespec e Log

## Treespec (`\` ou `/`)

Abre um prompt com o caminho atual. Conforme você digita, o cursor da árvore
vai para o primeiro diretório que começa com o que foi digitado (sem
diferenciar maiúsculas e minúsculas):

- `/` ou `\` completam o nome e descem um nível;
- Tab e Shift+Tab completam e movem para o próximo diretório ou para o anterior
  no mesmo nível;
- Backspace sobe níveis;
- Enter confirma e Esc retorna para onde você estava;
- ↑ traz caminhos anteriores (salvos entre sessões).

Um caminho fora da árvore se torna um Log ao ser confirmado. Em Branch e
Showall, `\` vai direto para o diretório do arquivo selecionado, com ele
destacado.

## Log (L)

Abre outro diretório na janela, substituindo a árvore (um diretório dentro da
árvore atual apenas move o cursor até lá). Um endereço como `user@host:` abre
uma [máquina remota](remote.md). O caminho pode ser absoluto, relativo ao
diretório atual ou começar com `~`; **F2** [escolhe-o em uma árvore](picker.md)
e ↑ traz os anteriores. Novas janelas mantêm a [ordem de classificação](sort.md)
e o modo de exibição atuais.

Veja também: [a janela da árvore](tree.md), [favoritos](bookmarks.md).
