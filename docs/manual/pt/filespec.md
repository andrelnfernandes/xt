# Filespec: o filtro de arquivos

`F` define quais arquivos aparecem na janela de arquivos (cada janela tem o seu
próprio filtro). O filespec atual aparece no título da janela. As
especificações são separadas por espaços, `;` ou `,`, de três tipos, como no
ZTreeWin:

- **nome:** um padrão glob (`*.rs`, `a?c*`, `[abc]*`), sem diferenciar
  maiúsculas e minúsculas; entre aspas, um nome pode conter espaços
  (`"meu arquivo*"`);
- **data:** `<`, `=`, `>`, `<=`, `>=` ou `<>` e uma data na ordem do formato
  configurado, com `/`, `.` ou `-` e ano de 2 ou 4 dígitos (`>01/05/2026`;
  `2026-05-01` funciona sempre), ou `TODAY` e `TODAY-n` (`>TODAY-7`: os últimos
  7 dias);
- **tamanho:** o mesmo com um `s` após o operador (`>=s200000`); `K`, `M` e `G`
  também funcionam (`>s100M`).

Um `-` no início nega a condição (`-*.bak`; `-<s1K` é o mesmo que `>=s1K`).
Dentro de cada tipo, especificações positivas são combinadas com OU (OR),
exceto `>a` seguido imediatamente de `<b`, que representa o intervalo entre os
dois (`>=01/05/2026,<=31/05/2026`: maio); `<>` exclui. Entre tipos diferentes,
aplica-se E (AND): `*.txt,=TODAY,<s1000` são os arquivos `.txt` de hoje menores
que 1000 bytes.

`*` (ou um prompt vazio) mostra tudo. **↑** traz filespecs anteriores.

Veja também: [a janela de arquivos](files.md), [marcações](tagging.md) (Ctrl+T
na árvore marca o que corresponde ao filespec).
