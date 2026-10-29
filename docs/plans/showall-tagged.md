# Plano: Ctrl+S na árvore, Showall só dos marcados (Tags-Only)

Origem: conversa de 2026-10-29. O autor usa muito o Ctrl+S do ZTreeWin na
árvore ("Ctrl-Showall"), que o Xylem não tinha. Referência: manual do
ZTreeWin v2.2, seção 1.7.1 "Tags-Only Mode".

## Como o ZTreeWin faz

- Mostra só os arquivos marcados que atendem ao Filespec; um losango no
  indicador de filtro avisa que há arquivos fora da lista.
- Só entra no modo se houver arquivos marcados que atendam ao Filespec; senão
  não faz nada.
- Arquivos desmarcados no modo continuam na lista até ela ser relida (F3 ou
  atualização automática); se não sobrar nenhum marcado, a lista volta a
  mostrar todos os arquivos.
- Entra-se no modo pela árvore com Ctrl+Enter, Ctrl+Branch, Ctrl+Showall e
  Ctrl+Global, e pela janela de arquivos com Ctrl+Enter ou Ctrl+F4.

## O que foi feito (aprovado)

- `Pane::tags_only`: com ele ligado, `refresh_files` só lista os marcados; se
  não sobrar nenhum, desliga e lista todos. `set_view` e `back_to_tree`
  desligam; `set_view_tagged` liga.
- Comando `showall_tagged`, em Ctrl+S na árvore (na janela de arquivos o
  Ctrl+S continua sendo a busca nos marcados, como no ZTreeWin). Sem marcados
  que atendam ao Filespec, mostra "No tagged files match the filespec" e fica
  na árvore.
- Título da janela de arquivos: `Showall (tagged): ...` no lugar do losango.
- Menu Ctrl (F9) da árvore: "Showall".
- Manual (en, pt, es, fr, de, zh): `tree.md`, `tagging.md` e `ztreewin.md`.

## Depois (não aprovado ainda)

- Tags-Only no diretório e no Branch: Ctrl+Enter (difícil de distinguir em
  muitos terminais) e Ctrl+F4 na janela de arquivos; Ctrl+B na árvore já é
  "marcar favorito" no Xylem, então o Branch só dos marcados precisaria de
  outra tecla.
