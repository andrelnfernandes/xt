# Filtrer les lignes

Dans le mode texte de la visionneuse, `&` n'affiche que les lignes qui
correspondent. Les lignes sont masquées, pas modifiées : avec les numéros de
ligne activés (`#`), chaque ligne garde son numéro dans le fichier, et les
sauts montrent le filtre à l'œuvre. L'en-tête donne le résultat :
`[filter error: 24 of 850 lines]`.

| Touche dans l'invite | Ce qu'elle fait |
|---|---|
| **F1** | cette aide |
| **F2** | respecter la casse ou l'ignorer |
| **F3** | inverser : garder les lignes qui ne correspondent pas |
| **F4** | texte simple ou expression régulière |
| ↑ ↓ | filtres précédents |
| Entrée | appliquer |
| Échap | annuler |

## Expressions régulières

Le terme peut correspondre n'importe où dans la ligne, sauf si vous l'ancrez. La
casse est ignorée sauf si vous appuyez sur **F2** ; ici, les accents comptent.

| Écrivez | Pour trouver |
|---|---|
| `.` | n'importe quel caractère |
| `\d` `\D` | un chiffre, tout sauf un chiffre |
| `\w` `\W` | une lettre, un chiffre ou `_`, tout le reste |
| `\s` `\S` | un espace, tout le reste |
| `[abc]` `[^abc]` | l'un de ceux-ci, tout sauf ceux-ci |
| `[a-z0-9]` | un caractère dans ces plages |
| `^` `$` | le début, la fin de la ligne |
| `\b` | la limite d'un mot |
| `a*` `a+` `a?` | zéro ou plus, un ou plus, zéro ou un |
| `a{3}` `a{2,5}` | exactement 3, de 2 à 5 |
| `a+?` | un ou plus, le moins possible |
| `(abc)` | un groupe, pour répéter ou donner des variantes |
| `(?P<nom>abc)` | un groupe nommé |
| `(?i)` `(?-i)` | ignorer la casse ou la respecter à partir d'ici |

La barre verticale sépare les variantes : `erreur|échec|fatal`, ou
`disque (plein|erreur)` dans un groupe.

Pour trouver l'un des caractères spéciaux `. * + ? ( ) [ ] { } ^ $ \` ou la barre
elle-même, placez un `\` devant : `\.log$`, `\(1\)`.

## Exemples

- `^\d{4}-\d{2}-\d{2}` : les lignes qui commencent par une date
- `\b(erreur|fatal)\b` : ces mots entiers, quelle que soit la casse
- `^\s*$` avec **F3** : masque les lignes vides
- `^\s*#` avec **F3** : masque les commentaires d'un script shell
- ` 50\d ` : les erreurs HTTP 500 à 509 dans un journal d'accès
- `\b\d{1,3}(\.\d{1,3}){3}\b` : les lignes avec une adresse IPv4
- `(?-i)TODO` : seulement en majuscules, même si la casse est ignorée

Les références arrière (`\1`) et le lookaround (`(?=...)`) n'existent pas ici, à
dessein : sans eux, le temps ne croît qu'avec la taille du texte, et aucun
fichier ni expression ne peut bloquer la visionneuse avec un motif comme
`(a+)+$`.

## Texte simple

Les mêmes règles que la recherche : en ignorant la casse, les accents ne comptent
pas non plus (`AÇÃO` trouve `ação`), et un `*` au milieu correspond à n'importe
quoi sur la même ligne (`erreur*disque`).

## Avec un filtre actif

Appuyez de nouveau sur `&` pour changer le filtre. **Entrée avec une invite vide
le désactive**, et la ligne en haut de l'écran y reste. Un filtre qui ne trouve
rien ou qui n'est pas valide ne change rien, et le pied de page dit pourquoi.

Avec un filtre actif, le défilement et la [recherche](viewer.md) ne parcourent
que les lignes gardées. `g` prend un numéro de ligne du fichier ; si cette ligne
est masquée, il va à la suivante gardée. Le filtre reste après l'édition du
fichier (`E`) ou un changement d'encodage (`O`), et prend fin à la fermeture de
la visionneuse.

Seul ce que la visionneuse a lu est filtré : si l'en-tête indique `(showing first
N MiB)`, le reste du fichier n'est pas examiné. Le filtre voit les lignes telles
qu'elles sont affichées : une tabulation est faite d'espaces (utilisez `\s`, pas
`\t`) et un caractère de contrôle est un `·`. En mode tableau, `&` ouvre le
filtre SQL.

Voir aussi : [la visionneuse](viewer.md).
