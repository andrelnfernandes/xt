# Comparar

`C` na árvore, com a tela [dividida](split.md), compara os arquivos do
diretório atual com os do diretório da outra janela e marca, conforme sua
escolha:

- **U**nique (únicos): não presentes do outro lado;
- **N**ewer (mais novos) e **O**lder (mais antigos);
- **I**dentical (idênticos): mesmo tamanho e data;
- **D**ifferent (diferentes);
- **A**ll new (todos os novos): únicos + mais novos, o que falta para atualizar
  o outro lado.

**B** (ou Alt+C na árvore, como no ZTreeWin) muda o escopo para todo o ramo: o
Xylem lê o ramo, compara seguindo os caminhos relativos
(`src/a.rs` ↔ `<outro>/src/a.rs`) e abre o Branch com os arquivos marcados.

Datas com até 2 segundos de diferença contam como iguais (FAT e exFAT gravam
horários em passos de 2 segundos); `compare_ignore_1h` também ignora diferenças
de exatamente uma hora (horário de verão). Ambos ficam na página 4 da
[configuração](configuration.md).

Para sincronizar: `C` `B` `A` e depois Alt+C ([copiar com caminhos](copy-move.md)),
que já sugere a outra janela como destino.
