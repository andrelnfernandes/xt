# Informações

- **Info (Alt+I):** tipo, tamanho, permissões, dono e grupo, datas
  (modificação, acesso e alteração), inode, links e dispositivo do arquivo ou
  diretório; para um link, para onde ele aponta; para um diretório, também o
  que foi lido nele e em seu ramo.
- **Avail (`A` na árvore):** os sistemas de arquivos montados com total, usado,
  livre e % usado, como o `df`.
- **Estatísticas (`?`):** o que a janela leu na memória, a lista, os arquivos
  marcados e o disco (ponto de montagem, livre e usado); logo abaixo, a máquina
  (sistema e kernel, CPU, tempo de atividade, carga, memória e swap, shell,
  terminal), a rede (endereços de cada interface, gateway e DNS) e o próprio
  Xylem (versão, PID, memória, executável e arquivo de configuração).

## Tamanhos de diretórios (Alt+K)

Mostra à direita de cada diretório na árvore o **S**ize (tamanho total em K),
o **C**ount (contagem de arquivos) ou o **P**ercent (porcentagem); **N**one
desativa. Antes de escolher, **A**ll ou **T**agged contam todos os arquivos ou
apenas os marcados, e **B**ranch ou **D**irectory somam todo o ramo ou apenas o
diretório. Diretórios ainda não lidos exibem `?`. Cada janela tem sua própria
configuração.

**Percent** mostra quanto do diretório acima cada ramo ocupa, junto com uma
barra: o maior diretório entre os irmãos a preenche por completo, e os demais
são desenhados proporcionalmente, para que o que ocupa mais espaço se destaque.
Sua cor indica a fração ocupada do diretório acima: até 10% (verde, na maioria
dos temas), até 40% (âmbar) ou mais (vermelho). Esse modo sempre soma ramos
inteiros. Os arquivos do diretório acima contam no total dele, por isso a soma
das frações de seus subdiretórios pode ser menor que 100%. Para descobrir o que
enche um disco, leia o ramo com `*` primeiro: um diretório ainda não lido conta
como vazio. Com o Google Drive ou o S3 montados via rclone, é assim que se
encontra o que está ocupando espaço neles: veja
[Armazenamento na nuvem](cloud.md). Para um disco inteiro, a
[visualização de Uso](usage.md) (Alt+W) faz ainda mais: arquivos e pastas
juntos, o espaço real no disco e hard links contados uma única vez.
