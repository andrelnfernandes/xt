# Armazenamento na nuvem (rclone)

O Xylem não tem um cliente próprio para Google Drive ou S3, mas não precisa de
um: o [rclone](https://rclone.org) monta esses serviços (e cerca de 70 outros:
OneDrive, Dropbox, Backblaze B2, Cloudflare R2, MinIO...) como uma pasta, e o
Xylem trabalha nessa pasta como em qualquer outra. Tudo funciona ali: leitura
de diretórios, Branch, marcações, visualizador, Copiar e Mover nas duas
direções, Renomear, Criar diretório e Excluir.

## Por que rclone

Optamos por trabalhar com o rclone em vez de conversar diretamente com o Google
Drive e o Amazon S3. Código próprio para cada serviço seria mais um lugar para
erros, e o Google Drive também traria junto as políticas do Google: um
aplicativo que lê todos os arquivos de alguém precisa de verificação do Google
e de uma avaliação de segurança anual, ou então exibe um aviso de "aplicativo
não verificado" e fica limitado a poucos usuários. Para o S3, a biblioteca
oficial é muito grande e traria muita complexidade, o que o Xylem, por projeto,
evita sempre que pode. O rclone já faz tudo isso muito bem, para vários
serviços de uma só vez.

## Configuração inicial (uma única vez)

Instale o rclone (`apt install rclone`, `dnf install rclone`, `brew install
rclone` ou em [rclone.org](https://rclone.org/downloads/)) e crie um *remote*
com `rclone config`. Você dá um nome a ele, que é o que vai antes dos dois
pontos abaixo.

- **Google Drive:** tipo de armazenamento `drive`; os padrões servem. O rclone
  abre o navegador para você entrar com sua conta do Google e autorizar. Em uma
  máquina sem navegador (via SSH), responda *no* à pergunta sobre o navegador:
  o rclone mostra um comando para rodar com `rclone authorize` em uma máquina
  que tenha navegador.
- **Amazon S3:** tipo de armazenamento `s3`, provedor `AWS`, e depois a chave
  de acesso e o segredo, ou *env_auth* para usar o que as ferramentas da AWS já
  usam (`~/.aws/credentials`, variáveis de ambiente, a *role* da instância).
  Outros serviços S3 (R2, MinIO, Wasabi...) aparecem como outros provedores na
  mesma lista.

`rclone lsd gdrive:` (ou `rclone lsd s3:`) lista as pastas da raiz (os
*buckets*, no S3): se funcionar, o remote está pronto.

## Montar e abrir

No Linux, com um remote chamado `gdrive`:

```bash
mkdir -p ~/gdrive
rclone mount gdrive: ~/gdrive --vfs-cache-mode writes --daemon
xy ~/gdrive
```

Para o S3, faça o mesmo com `s3:` (todos os buckets) ou `s3:meu-bucket` (apenas
um). `fusermount -u ~/gdrive` desmonta.

- `--vfs-cache-mode writes` mantém uma cópia local dos arquivos que estão sendo
  gravados, o que editores (`E`) e alguns programas exigem.
- `--dir-cache-time 1h` mantém as listagens de diretórios por mais tempo que os
  5 minutos padrão: reler (F3) fica mais rápido, mas alterações feitas fora dali
  demoram mais para aparecer.

No **macOS**, `rclone mount` precisa do [macFUSE](https://osxfuse.github.io);
`rclone nfsmount gdrive: ~/gdrive` não precisa de nada extra. No **Windows**,
precisa do [WinFsp](https://winfsp.dev) e monta em uma letra de unidade:
`rclone mount gdrive: X:`, e depois `xy X:\`.

Para ter a pasta sempre à mão, salve-a como um [favorito](bookmarks.md)
(Alt+B).

No Linux, o Xylem reconhece as pastas montadas pelo rclone: o painel DISK
mostra o serviço (Google Drive, Amazon S3, OneDrive...) e, abaixo dele, o
remote (`rclone gdrive:`). Info (Alt+I) e Avail (`A`) também os mostram. O
serviço vem do arquivo de configuração do rclone
(`~/.config/rclone/rclone.conf`, ou `$RCLONE_CONFIG`); quando ele está
criptografado, o painel diz apenas rclone.

## O que muda lá

- **Tudo passa pela rede.** Cada leitura de diretório é uma requisição: em um
  Drive ou bucket grande, `*` (ler todo o ramo) e Branch (`B`) podem demorar.
  Abrir apenas os diretórios de que você precisa (→ ou `+`) é rápido, pois eles
  são lidos conforme você os abre.
- **Alterações feitas em outro lugar** (no navegador, no celular) só aparecem
  após F3 e depois que o cache de diretórios expira (`--dir-cache-time`).
- **Excluir:** responda `P` (permanentemente) em vez de `T`: no Linux, a
  lixeira de uma pasta montada é uma pasta `.Trash-` (com o número do seu
  usuário) na raiz dela, de modo que o arquivo continuaria no remote ocupando
  espaço. O Google Drive já guarda o que o rclone exclui na própria lixeira do
  Drive; no S3, um arquivo excluído some de vez, a menos que o bucket tenha
  versionamento.
- **Permissões e donos** não são mantidos: a montagem exibe os mesmos para
  todos os arquivos, e o comando de Atributos não pode alterá-los. As datas são
  mantidas.
- **Google Docs, Planilhas e Apresentações** não são arquivos comuns: o rclone
  os mostra exportados (`.docx`, `.xlsx`, `.pptx`), com tamanho 0. A opção
  `--drive-skip-gdocs` os oculta.
- **O S3 não tem diretórios reais**, apenas nomes com `/` neles: um diretório
  vazio pode desaparecer quando desmontado, e renomear ou mover um diretório
  copia e exclui cada arquivo dentro dele.

## Descobrir o que está enchendo o seu Google Drive

Quando o Drive vive pedindo um plano de armazenamento maior, o Xylem pode
mostrar para onde está indo o espaço antes de você pagar por mais. Monte o
Drive, vá até a raiz dele, leia tudo com `*` (em um Drive grande isso leva um
tempo) e escolha Alt+K → **Percent** ([Tamanhos de diretórios](info.md)): cada
pasta mostra quanto ocupa da pasta acima, com uma barra proporcional à maior
entre as pastas irmãs. Siga as maiores barras até chegar aos backups antigos,
vídeos e arquivos duplicados.

A [visualização de Uso](usage.md) (Alt+W, na raiz do Drive) vai além: junta
arquivos e pastas, do maior para o menor, para que um único arquivo enorme se
destaque tanto quanto uma pasta cheia. E o Branch (`B`) lista os arquivos de
uma pasta inteira: [ordenados](sort.md) por tamanho, do maior para o menor
(Alt+S, depois `O` para decrescente e `S` para Size), os maiores ficam no topo.

Algumas coisas contam na cota de armazenamento sem aparecer na montagem:

- **A lixeira do Drive** ocupa espaço até ser esvaziada (ou até passarem 30
  dias). `rclone about gdrive:` mostra o total, o usado e o que está na
  lixeira; montar com `--drive-trashed-only` mostra apenas o que está na
  lixeira.
- **Gmail e Google Fotos** compartilham o mesmo armazenamento, e somente os
  arquivos do Drive aparecem na montagem. Se `rclone about` mostrar muito mais
  espaço usado do que a soma da montagem, o restante provavelmente está lá.
- **Google Docs, Planilhas e Apresentações** aparecem com tamanho 0, mas os
  mais recentes contam no armazenamento.
- Arquivos **compartilhados com você** não contam: eles consomem o
  armazenamento do dono.
