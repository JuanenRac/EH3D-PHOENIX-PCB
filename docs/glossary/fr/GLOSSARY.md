# Glossaire

Les termes du domaine tels que ce programme les emploie. Le tableau à la fin de la page anglaise donne chaque terme dans les huit langues.

| Terme | Sens |
|---|---|
| **nm (nanomètre)** | L'unité de toute longueur dans le modèle, gardée comme entier : 1 mm vaut 1 000 000 nm, donc sommes et comparaisons n'ont pas d'erreur d'arrondi. |
| **réseau (net)** | L'ensemble des pastilles, pistes et vias qui doivent être reliés électriquement ; il porte un nom comme GND. |
| **classe de réseau** | Un groupe de réseaux qui partagent des règles : largeur de piste, dégagement et taille de via. Le fichier de règles du second format contient les classes. |
| **via** | Un trou métallisé qui relie le cuivre de deux couches. |
| **anneau annulaire** | L'anneau de cuivre qui reste autour d'un trou percé ; trop mince, la mèche peut le rompre. |
| **empreinte** | Les pastilles de cuivre, trous et dessins d'un composant sur la carte. |
| **zone d'exclusion (courtyard)** | La zone autour d'une empreinte où aucune autre pièce ne doit entrer. |
| **zone de cuivre** | Une zone de cuivre remplie, souvent un plan de masse, qui garde son dégagement avec les autres réseaux. |
| **empilage** | L'ordre, l'épaisseur et le matériau des couches de cuivre et d'isolant d'une carte ; l'impédance d'une ligne en dépend. |
| **DRC (vérification des règles de conception)** | La vérification d'une carte par rapport à des limites comme la piste la plus étroite, le dégagement, le perçage et l'anneau annulaire. Commande : `phoenix check`. |
| **ERC (vérification des règles électriques)** | La vérification d'un schéma : broches sur aucun réseau, entrées que rien ne pilote, alimentations sans source, etc. Commande : `phoenix erc`. |
| **chevelu (ratsnest)** | Les lignes droites qui montrent quelles connexions restent à router. Commande : `phoenix ratsnest`. |
| **Gerber X2** | Le format des fichiers de cuivre, vernis, pâte et sérigraphie que lit le fabricant ; X2 ajoute des attributs comme la fonction de la couche. |
| **Excellon** | Le format des fichiers de perçage : les outils et les positions des trous, métallisés ou non. |
| **DSN / SES** | Les fichiers Specctra pour un autorouteur externe : DSN est la conception qui part, SES la session avec les routes qui revient. Commande : `phoenix autoroute`. |
| **rainure en V (V-score)** | Une rainure en ligne droite entre deux cartes d'un panneau, pour pouvoir les séparer en les cassant. |
| **panneau** | Plusieurs cartes disposées ensemble sur une feuille, avec cadre, ponts, mires et trous d'outillage, pour les fabriquer et les assembler d'un coup. Commande : `phoenix panel`. |
| **ponts à trous (mouse bites)** | Un pont entre deux cartes traversé par une rangée de petits trous, pour qu'il casse proprement. |
| **mire de repérage (fiducial)** | Une petite marque ronde de cuivre nu que la machine d'assemblage voit pour trouver la position de la carte. |
| **fichier de placement (pick and place)** | Le fichier avec la référence, la position, la rotation et la face de chaque pièce, pour la machine d'assemblage. Commande : `phoenix pnp`. |
| **pochoir (stencil)** | La feuille mince percée d'ouvertures par lesquelles on étale la pâte à braser sur les pastilles. Commande : `phoenix stencil`. |
| **BOM (nomenclature)** | La liste des pièces d'une carte, groupées par valeur et empreinte, avec quantités et prix. Commande : `phoenix bom`. |
| **paire différentielle** | Deux pistes qui portent un signal comme différence de leurs tensions ; elles doivent garder un écart, des longueurs égales et une symétrie. Commande : `phoenix pairs`. |
| **diaphonie (NEXT, FEXT)** | La part d'un signal qui apparaît sur une piste voisine ; proche (NEXT) côté émetteur, lointaine (FEXT) à l'autre bout. Commande : `phoenix si crosstalk`. |
| **paramètres S / Touchstone** | La réflexion et la transmission d'un réseau à chaque fréquence ; Touchstone (.s1p, .s2p) est le fichier qui les stocke. Commande : `phoenix rf touchstone`. |
| **TOS (taux d'onde stationnaire)** | Un nombre à partir de 1 qui dit à quel point une charge est mal adaptée à une ligne : 1 est une adaptation parfaite. |
| **Monte Carlo** | De nombreuses exécutions d'un circuit où les valeurs varient au hasard dans leurs tolérances ; une graine fixe donne toujours le même tableau. |
| **netlist (liste de connexions)** | Le texte qui liste les éléments d'un circuit et les nœuds qu'ils relient ; c'est ce que lit le simulateur. Commande : `phoenix netlist`. |
| **essai à blanc** | Une exécution qui fait tout sauf écrire : elle montre ce qui serait écrit ou changé. Presque toute commande est un essai à blanc sauf si `--out` nomme un nouveau fichier. |

* [English](../../GLOSSARY.md)
