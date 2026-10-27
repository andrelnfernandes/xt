# Machines distantes (SSH)

Xylem peut afficher les fichiers d'une autre machine, jointe par SSH, sous forme
d'arborescence : seule, ou avec votre machine à côté en [écran
partagé](split.md).

## En ouvrir une

Tapez l'adresse dans `L` ([Log](treespec.md)), ou passez-la à `xy` sur la ligne
de commande :

- `user@host:` — le répertoire personnel ; `user@host:/srv/www` — ce répertoire ;
- `ssh://[user@]host[:port][/path]` — avec un port, ou sans utilisateur (comme
  dans votre `~/.ssh/config`).

Xylem lance votre propre `ssh` ; tout ce que vous utilisez dans un terminal
fonctionne donc de la même manière : `~/.ssh/config` (alias d'hôtes, ports,
ProxyJump), clés, agent et `known_hosts`. Il n'y a rien à installer sur l'autre
machine : Xylem parle SFTP, que tout serveur SSH intègre (Linux, BSD, macOS,
Windows).

## Mots de passe et questions

Lorsque ssh demande quelque chose, la question s'affiche dans le pied de page :

- un **mot de passe** ou la **phrase secrète** d'une clé : saisi à l'aveugle ;
  Enter l'envoie, Esc abandonne la connexion. Xylem le transmet à ssh et ne
  conserve rien.
- un **hôte inconnu** : l'hôte et l'empreinte de sa clé, à vérifier avant de
  répondre `yes` (ssh l'enregistre ensuite dans `known_hosts`, comme
  d'habitude).

`Esc` pendant la connexion annule. Avec une clé et l'agent, rien n'est demandé.

## Ce qui fonctionne

L'arborescence, la fenêtre de fichiers, [Filespec](filespec.md), Branch,
Showall, le marquage, [la visionneuse](viewer.md), [Ctrl+S et Ctrl+V](search.md),
Autoview, Info (Alt+I), les modes d'affichage (le mode propriétaire affiche les
identifiants numériques d'utilisateur et de groupe envoyés par SFTP),
[Comparer](compare.md) et Ouvrir (`O` télécharge une copie et l'ouvre ici).

**Copier** rapatrie les fichiers ici, avec leurs permissions et leurs dates (à la
seconde près, telles que SFTP les conserve) ; avec l'écran partagé, la fenêtre
de votre machine est proposée comme destination. Alt+C (avec chemins) fonctionne
aussi, avec des chemins partant de la racine de la fenêtre distante.

Pour l'instant, le côté distant est en **lecture seule** : Déplacer, Supprimer,
Renommer, Créer un répertoire, Attributs, dates, Éditer, eXécuter et la copie
*vers* la machine distante ne sont pas encore disponibles. F3 relit un
répertoire ; il n'y a pas de rafraîchissement automatique.

**F8** dans une fenêtre distante ouvre une fenêtre sur votre machine (dans le
dossier où Xylem a démarré), afin d'avoir les deux côte à côte.

## Le terminal sur la machine distante

**Alt+X** dans une fenêtre distante ouvre un shell sur cette machine (`ssh -t`),
en démarrant dans le répertoire de la fenêtre ; dans une fenêtre locale, le
shell local, comme avant (chacun continue de tourner pendant que l'autre est à
l'écran). Sous Linux et macOS, le shell distant réutilise la connexion déjà
établie, de sorte que le mot de passe n'est pas redemandé ; sous Windows, ssh le
demande dans le terminal.

L'arborescence suit les `cd` du shell distant lorsque celui-ci signale son
répertoire à l'aide de la séquence OSC 7. Beaucoup de systèmes le font déjà ;
pour bash, ajoutez ceci au `~/.bashrc` **sur la machine distante** :

```bash
PROMPT_COMMAND='printf "\033]7;file://%s%s\007" "$HOSTNAME" "$PWD"'
```

Xylem ne tape jamais rien dans un shell distant (il ne peut pas voir si un
programme y tourne), et Alt+O ainsi que `$XY_TAGGED` sont réservés aux terminaux
locaux. Quitter alors qu'un terminal distant est ouvert demande d'abord
confirmation.

## tmux ou screen lancé à la connexion

Certaines personnes lancent tmux (ou screen) automatiquement lorsqu'elles se
connectent en SSH, depuis le `~/.bashrc` de la machine distante, afin qu'une
coupure de connexion (Wi-Fi instable, VPN qui se reconnecte) n'interrompe pas ce
qui tournait : se reconnecter reprend la même session. Par exemple :

```bash
if [ -n "$SSH_CONNECTION" ] && [ -z "$TMUX" ]; then
    tmux attach-session -t remote 2>/dev/null || tmux new-session -s remote
fi
```

Dans ce cas, le terminal distant de Xylem s'ouvre lui aussi dans tmux. Tout
fonctionne, sauf que **l'arborescence ne suit pas les `cd` du shell** : tmux
s'interpose entre le shell et Xylem, garde la séquence OSC 7 pour lui et ne la
retransmet pas.

Le shell distant de Xylem démarre avec `XY_TERMINAL=1`, comme le shell local, ce
qui permet à la connexion de les distinguer. Ajouter ce test à la condition
conserve tmux pour vos connexions `ssh` habituelles et l'exclut lorsque Xylem se
connecte :

```bash
if [ -n "$SSH_CONNECTION" ] && [ -z "$TMUX" ] && [ -z "$XY_TERMINAL" ]; then
    tmux attach-session -t remote 2>/dev/null || tmux new-session -s remote
fi
```

Dans Xylem, le shell n'a pas besoin de tmux pour survivre : il continue de
tourner tant que Xylem est ouvert, et si la connexion coupe, la fenêtre distante
tombe avec elle de toute façon.

Pour Google Drive, S3 et les autres services de stockage cloud, voir [Stockage
cloud](cloud.md).
