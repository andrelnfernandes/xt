# Git

O Git não existe no XTree nem no ZTreeWin, por isso começa desativado: ative-o
com F12 (página 1, "Git commands") ou `git = true` sob `[options]` na
[configuração](configuration.md). Requer o programa `git`.

## O menu Git (Ctrl+G)

Ctrl+G abre o menu Git no rodapé; a próxima letra escolhe:

- **S**tatus, **L**og, **D**iff (contra HEAD: staged e unstaged juntos, com
  cores) e **B**lame abrem no [visualizador](viewer.md), com busca;
- **G**raph: `git log --graph --decorate --oneline --all`, os
  últimos 1000 commits de todos os ramos, com as cores do próprio Git (uma por
  linha de ramo), também no visualizador;
- **A**dd e **U**nstage;
- **C**ommit abre o próprio editor do Git, como faz `git commit`.

Os comandos atuam nos arquivos marcados da lista ou, se não houver nenhum, no
arquivo atual; na árvore, no diretório atual. O Status e o Graph sempre cobrem todo
o repositório. Para push, pull e outros comandos que usam a rede, use
[eXecute](execute.md) (`X`), por exemplo `git push`.

## Status na lista de arquivos

A lista de arquivos mostra o status de cada arquivo antes do nome, com as duas
letras de `git status --short`: `M ` em stage, ` M` alterado mas fora de stage,
`A ` adicionado, `??` não rastreado, `R ` renomeado. Arquivos sem alterações
mostram espaços em branco. O status é relido após as próprias operações do
Xylem, após F3 e a cada 10 segundos (para detectar commits feitos em outro
terminal).

## Diff no visualizador

No visualizador, **D** mostra o que mudou no arquivo desde o último commit (seu
`git diff HEAD`, com cores) e **D** novamente volta para o arquivo, na mesma
posição. "Diff" aparece no rodapé apenas para arquivos com alterações; **E**
continua editando o arquivo, e o diff é atualizado ao retornar.

## Repositórios que você não criou

O `.git` de um repositório pode nomear comandos para o Git executar, e uma pasta baixada ou extraída vem com o seu `.git` (um `git clone` não o traz). Por isso, os marcadores de arquivo e Status, Log, Graph, Diff e Blame não executam nenhum deles: nem ganchos, `core.fsmonitor`, diffs externos, conversões de texto ou verificação de assinaturas. O custo:

- Um repositório que define filtros próprios, como faz o `git-crypt`, fica sem marcadores, Status, Diff e Blame aqui, com uma mensagem explicando o motivo. Log e Graph funcionam. Filtros da sua configuração global, como o Git LFS, são confiáveis.
- Um submódulo com arquivos alterados, mas sem commit novo, não aparece como alterado.

Add, Unstage e Commit são pedidos seus e seguem a configuração do repositório: podem executar o que ela nomeia.

### Pastas confiáveis

Por padrão, nada é confiável: todo repositório é tratado como hostil. Para os repositórios de uma pasta funcionarem com toda a configuração, como `git-crypt` ou uma ferramenta de diff sua, adicione a pasta em **Trusted Git folders** (**K** na página 1 de **F12**), com `;` entre as pastas:

```
~/projetos; /trabalho/codigo
```

No arquivo, é uma lista: `git_trusted = ["~/projetos", "/trabalho/codigo"]`. Uma pasta é um caminho completo ou abaixo de `~`, e vale para tudo o que está dentro dela. Um link que aponta para fora não leva a confiança junto.

**Perigo:** dentro de uma pasta confiável, basta entrar em um repositório para o Git executar o que ele nomeia, com as suas permissões. Nunca extraia nem baixe nada de outras pessoas nela, e não confie em `~` nem em `/`, que contêm o que você baixa.
