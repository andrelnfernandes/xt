# Máquinas remotas (SSH)

O Xylem pode exibir os arquivos de outra máquina, acessada via SSH, como uma
árvore: sozinha, ou com esta máquina ao lado na [tela dividida](split.md).

## Como abrir

Digite o endereço em `L` ([Log](treespec.md)), ou passe-o para `xy` na linha de
comando:

- `user@host:` — o diretório pessoal (*home*); `user@host:/srv/www` — aquele
  diretório;
- `ssh://[user@]host[:port][/path]` — com porta, ou sem usuário (como no seu
  `~/.ssh/config`).

O Xylem executa o seu próprio `ssh`, portanto tudo o que você usa no terminal
funciona da mesma forma: `~/.ssh/config` (apelidos de host, portas, ProxyJump),
chaves, o agente e `known_hosts`. Não é preciso instalar nada na outra máquina:
o Xylem fala SFTP, que todo servidor SSH já possui (Linux, BSD, macOS,
Windows).

## Senhas e perguntas

Quando o ssh pergunta algo, a pergunta aparece no rodapé:

- uma **senha** ou a **frase secreta** (*passphrase*) de uma chave: digitada de
  forma oculta; Enter envia, Esc desiste da conexão. O Xylem a repassa ao ssh e
  não guarda nada.
- um **host desconhecido**: o host e a impressão digital (*fingerprint*) de sua
  chave, para conferir antes de responder `yes` (o ssh então o memoriza em
  `known_hosts`, como de costume).

`Esc` durante a conexão cancela. Com chave e agente, nada é perguntado.

## O que funciona

A árvore, a janela de arquivos, [Filespec](filespec.md), Branch, Showall,
marcações, [o visualizador](viewer.md), [Ctrl+S e Ctrl+V](search.md), Autoview,
Info (Alt+I), os modos de exibição (owner mostra os IDs numéricos de usuário e
grupo, que é o que o SFTP envia), [Comparar](compare.md) e Abrir (`O` baixa uma
cópia e a abre aqui).

**Copiar** traz os arquivos para cá, com suas permissões e datas (com precisão
de segundos, como o SFTP mantém); com a tela dividida, sugere a janela desta
máquina como destino. Alt+C (com caminhos) também funciona, com os caminhos
começando na raiz da janela remota.

Por enquanto, o lado remoto é **somente leitura**: Mover, Excluir, Renomear,
Criar diretório, Atributos, datas, Edit, eXecute e copiar *para* a máquina
remota ainda não estão disponíveis. F3 relê um diretório; não há atualização
automática.

**F8** em uma janela remota abre uma janela nesta máquina (na pasta em que o
Xylem começou), permitindo ter ambas lado a lado.

## O terminal na máquina remota

**Alt+X** em uma janela remota abre um shell naquela máquina (`ssh -t`),
começando no diretório da janela; em uma janela local, abre o shell local, como
antes (cada um continua rodando enquanto o outro está na tela). No Linux e no
macOS, o shell remoto reutiliza a conexão já estabelecida, de modo que a senha
não é pedida novamente; no Windows, o ssh a pede no terminal.

A árvore acompanha o `cd` do shell remoto quando o shell informa seu diretório
com a sequência OSC 7. Muitos sistemas já fazem isso; para o bash, adicione
isto ao `~/.bashrc` **na máquina remota**:

```bash
PROMPT_COMMAND='printf "\033]7;file://%s%s\007" "$HOSTNAME" "$PWD"'
```

O Xylem nunca digita comandos em um shell remoto (ele não consegue ver se há um
programa rodando lá), e Alt+O e `$XY_TAGGED` funcionam apenas em terminais
locais. Sair com um terminal remoto aberto pede confirmação antes.

## tmux ou screen iniciados no login

Algumas pessoas iniciam o tmux (ou o screen) automaticamente ao entrar via SSH,
a partir do `~/.bashrc` na máquina remota, para que uma queda de conexão (Wi-Fi
instável, VPN que reconecta) não encerre o que estava rodando: ao conectar
novamente, a mesma sessão é retomada. Por exemplo:

```bash
if [ -n "$SSH_CONNECTION" ] && [ -z "$TMUX" ]; then
    tmux attach-session -t remote 2>/dev/null || tmux new-session -s remote
fi
```

Nesse caso, o terminal remoto do Xylem também abre dentro do tmux. Tudo
funciona, exceto que **a árvore não acompanha o `cd` do shell**: o tmux fica
entre o shell e o Xylem, retém a sequência OSC 7 para si e não a repassa.

O shell remoto do Xylem inicia com `XY_TERMINAL=1` definido, assim como o
local, para que o login possa diferenciar os dois. Adicionar esse teste à
condição mantém o tmux para o seu `ssh` habitual e o deixa de fora quando o
Xylem se conecta:

```bash
if [ -n "$SSH_CONNECTION" ] && [ -z "$TMUX" ] && [ -z "$XY_TERMINAL" ]; then
    tmux attach-session -t remote 2>/dev/null || tmux new-session -s remote
fi
```

Dentro do Xylem, o shell não precisa do tmux para sobreviver: ele continua
rodando enquanto o Xylem está aberto e, se a conexão cair, a janela remota cai
junto de qualquer maneira.

Para Google Drive, S3 e outros serviços de nuvem, veja
[Armazenamento na nuvem](cloud.md).
