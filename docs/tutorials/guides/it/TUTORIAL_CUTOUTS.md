# Portare le aperture del contorno e i ritagli di un riempimento attraverso i formati di file

`examples/guides/cutout.brd` è una scheda EAGLE con due tipi di apertura: un quadrato chiuso di segmenti sullo strato 20 dentro il contorno (un'apertura della scheda, una fessura o finestra) e un poligono sullo strato 41 dentro il riempimento dello strato `Top` (un ritaglio del riempimento, dove non deve esserci rame). Si scrive la scheda in DXF, si rilegge il DXF, si scrivono file KiCad e Gerber, e si vede dove va ogni apertura.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Elencate ciò che conterrebbe un disegno DXF della scheda. Il contorno e l'apertura sono due contorni chiusi sul livello `OUTLINE`; il rapporto dice `layer OUTLINE: 2` e che non si è scritto nulla.

```console
# test
$ phoenix dxf-export board examples/guides/cutout.brd
~ layer OUTLINE: 2
~ dry run: nothing was written
```

2. Scrivete il disegno in un file nuovo.

```console
# test
$ phoenix dxf-export board examples/guides/cutout.brd --out build/cutout.dxf
~ written: build/cutout.dxf
```

3. Rileggete il disegno nella scheda. Il contorno esterno più grande è il contorno della scheda e quello al suo interno viene conservato come ritaglio del contorno (`PHOENIX-DXF-015`, informazione). L'operazione è una prova a vuoto, verificata con le regole di progetto prima e dopo, quindi si dà il file di regole.

```console
$ phoenix dxf-import build/cutout.dxf --board examples/guides/cutout.brd --layer OUTLINE --rules examples/tutorials/rules_relaxed.json
```

4. Scrivete una scheda KiCad. Il contorno e l'apertura vanno su `Edge.Cuts`, e il ritaglio del riempimento diventa un'area di regole sul suo strato che tiene fuori il riempimento e consente piste, via e pad. Il rapporto elenca ciò che è stato scritto e ciò che il formato non può dire.

```console
$ phoenix kicad-export --out build/cutout.kicad_pcb board examples/guides/cutout.brd
```

5. Scrivete i file di fabbricazione. Il file del profilo ha il contorno e poi l'apertura, ciascuno un contorno chiuso, e nel file del rame il ritaglio del riempimento è scritto come regione a polarità chiara dopo il riempimento.

```console
$ phoenix export --out build/cutout_fab examples/guides/cutout.brd
```

## Come leggere il risultato

* Da dove viene un'apertura: in una scheda EAGLE, i segmenti dello strato 20 che si susseguono e si chiudono, dentro il più grande anello di questo tipo, sono aperture della scheda; un poligono sullo strato 41 (`tRestrict`, il lato superiore) o 42 (quello inferiore) dentro un riempimento di quello strato è un ritaglio del riempimento. In una scheda KiCad, i contorni chiusi di `Edge.Cuts` dentro il più grande sono le aperture, e in un DXF i contorni dentro il più grande contorno esterno.
* Ogni formato dice ciò che può. I ritagli di un riempimento sono aree di regole in KiCad, aree di restrizione in EAGLE e aree di esclusione nel file Specctra dell'autorouter (che non ha un buco in un piano, quindi si escludono anche piste e via). Il pannello porta entrambi i tipi di apertura in ogni copia, spostate, ruotate e specchiate con essa.

## Vedere anche

* [docs/DXF.md](../../../../docs/DXF.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/PANEL.md](../../../../docs/PANEL.md)
