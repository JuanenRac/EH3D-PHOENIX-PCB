# Pannellizzare una scheda e scrivere i file del pannello

Si dispone due volte la scheda di 30 mm per 20 mm una accanto all'altra, unite da un taglio a V, e si scrivono i file Gerber e di foratura dell'intero pannello. Le impostazioni sono in `examples/tutorials/panel.cfg` (colonne, righe, tipo di giunzione, spazio e cornice); l'elenco completo è `config/panel.cfg`.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Pianificate il pannello. Una prova a secco stampa dimensioni, giunzione, fori e file che verrebbero scritti, e non scrive nulla.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ joint: V-score, 1 lines
~ dry run: nothing written
```

2. Scrivete i livelli di ogni copia, i file di foratura, il contorno con il taglio a V, il posizionamento e la distinta base in una nuova cartella.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ written
```

3. In una cartella esistente non si scrive mai: la seconda esecuzione viene rifiutata con il codice `PHOENIX-PANEL-008` e codice di uscita 2.

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
# exit 2
~~ PHOENIX-PANEL-008
```

4. Controllate la struttura di un file Gerber del pannello e disegnate il pannello come disegno DXF per la parte meccanica.

```console
$ phoenix gerber-check build/panel/panel_01_top.gbr
$ phoenix dxf-export panel --out build/panel.dxf --panel-config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
```

## Vedere anche

* [docs/PANEL.md](../../PANEL.md)
* [docs/GERBER_IMPORT.md](../../GERBER_IMPORT.md)
* [docs/DXF.md](../../DXF.md)
* [docs/FABPACK.md](../../FABPACK.md)
