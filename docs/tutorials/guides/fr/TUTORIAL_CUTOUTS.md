# Faire passer les ouvertures du contour et les découpes d'un plan par les formats de fichier

`examples/guides/cutout.brd` est une carte EAGLE avec deux sortes d'ouverture : un carré fermé de traits sur la couche 20 à l'intérieur du contour (une ouverture de la carte, une fente ou une fenêtre) et un polygone sur la couche 41 à l'intérieur du plan de la couche `Top` (une découpe du plan, où il ne doit pas y avoir de cuivre). Vous écrivez la carte en DXF, relisez le DXF, écrivez des fichiers KiCad et Gerber, et voyez où va chaque ouverture.

Exécutez les commandes depuis la racine du dépôt ; voir [comment exécuter les guides](README.md).

1. Listez ce que contiendrait un dessin DXF de la carte. Le contour et l'ouverture sont deux contours fermés sur le calque `OUTLINE` ; le rapport indique `layer OUTLINE: 2` et que rien n'a été écrit.

```console
# test
$ phoenix dxf-export board examples/guides/cutout.brd
~ layer OUTLINE: 2
~ dry run: nothing was written
```

2. Écrivez le dessin dans un nouveau fichier.

```console
# test
$ phoenix dxf-export board examples/guides/cutout.brd --out build/cutout.dxf
~ written: build/cutout.dxf
```

3. Relisez le dessin dans la carte. Le plus grand contour extérieur est le contour de la carte et celui qui est à l'intérieur est gardé comme découpe du contour (`PHOENIX-DXF-015`, information). L'opération est un essai à blanc, vérifié par les règles de conception avant et après ; le fichier de règles est donc donné.

```console
$ phoenix dxf-import build/cutout.dxf --board examples/guides/cutout.brd --layer OUTLINE --rules examples/tutorials/rules_relaxed.json
```

4. Écrivez une carte KiCad. Le contour et l'ouverture vont sur `Edge.Cuts`, et la découpe du plan devient une zone de règles sur sa couche qui tient le plan à l'écart et autorise pistes, vias et pastilles. Le rapport liste ce qui a été écrit et ce que le format ne peut pas dire.

```console
$ phoenix kicad-export --out build/cutout.kicad_pcb board examples/guides/cutout.brd
```

5. Écrivez les fichiers de fabrication. Le fichier de profil contient le contour puis l'ouverture, chacun un contour fermé, et dans le fichier de cuivre la découpe du plan est écrite comme une région de polarité claire après le plan.

```console
$ phoenix export --out build/cutout_fab examples/guides/cutout.brd
```

## Comment lire le résultat

* D'où vient une ouverture : dans une carte EAGLE, des traits de la couche 20 qui se suivent et se ferment, à l'intérieur du plus grand anneau de ce type, sont des ouvertures de la carte ; un polygone sur la couche 41 (`tRestrict`, le dessus) ou 42 (le dessous) à l'intérieur d'un plan de cette couche est une découpe du plan. Dans une carte KiCad, les contours fermés de `Edge.Cuts` à l'intérieur du plus grand sont les ouvertures, et dans un DXF les contours à l'intérieur du plus grand contour extérieur.
* Chaque format dit ce qu'il peut. Les découpes d'un plan sont des zones de règles dans KiCad, des zones d'interdiction dans EAGLE et des zones d'exclusion dans le fichier Specctra de l'autorouteur (qui n'a pas de trou dans un plan, de sorte que pistes et vias en sont aussi écartés). Le panneau reporte les deux sortes d'ouverture dans chaque copie, déplacées, tournées et miroitées avec elle.

## Voir aussi

* [docs/DXF.md](../../../../docs/DXF.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/PANEL.md](../../../../docs/PANEL.md)
