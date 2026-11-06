# Tenere più disposizioni di pannello in un solo file di impostazioni con i modelli

`examples/guides/panel_templates.cfg` contiene una disposizione di base (due schede affiancate, unite da un taglio a V, senza cornice) e due modelli con nome. Ogni modello cambia solo le righe che nomina e se ne sceglie uno con `--template`. La scheda è quella del tutorial del pannello, 30 mm per 20 mm; tutti i comandi qui sono prove a vuoto.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Senza modello si legge solo la base e le righe sotto le sezioni dei modelli sono ignorate: 60 mm per 20 mm, 2 x 1 schede.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg examples/tutorials/kicad/demo.kicad_pcb
```

2. Il modello `four` porta `columns` a 4 e nient'altro: 120 mm per 20 mm, 4 x 1 schede.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four examples/tutorials/kicad/demo.kicad_pcb
```

3. Il modello `grid` porta `rows` a 2: 60 mm per 40 mm, 2 x 2 schede.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template grid examples/tutorials/kicad/demo.kicad_pcb
```

4. `--columns` e `--rows` prevalgono ancora su un modello: con `four` e `--columns 3` il pannello è 90 mm per 20 mm, 3 x 1 schede.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --columns 3 examples/tutorials/kicad/demo.kicad_pcb
```

5. Un nome che il file non ha viene rifiutato con il codice `PHOENIX-PANEL-001` e codice di uscita 2.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template ring examples/tutorials/kicad/demo.kicad_pcb
```

6. Scrivete i file della disposizione scelta in una cartella nuova, come nel tutorial del pannello.

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --out build/panel_four examples/tutorials/kicad/demo.kicad_pcb
```

## Come leggere il risultato

* Le righe prima della prima riga `[template NOME]` sono la base. Un nome ha da 1 a 32 lettere, cifre, `_` o `-`; una sezione scritta in altro modo viene rifiutata con `PHOENIX-PANEL-001` e il numero della riga.
* La riga della dimensione è `panel W mm by H mm, C x R boards`. Con la giunzione a V e spazio 0 le schede si toccano, quindi la larghezza è il numero di colonne per 30 mm e l'altezza il numero di righe per 20 mm. Un modello è un modo per tenere una disposizione sotto un nome; non è l'impostazione di alcun produttore.

## Vedere anche

* [docs/PANEL.md](../../../../docs/PANEL.md)
* [config/panel.cfg](../../../../config/panel.cfg)
