# Eine Bibliothek bearbeiten und IPC-Footprints erzeugen

Sie prüfen eine EAGLE-Bibliothek, fügen ihr mit einer Operationsdatei einen Footprint hinzu (ganz oder gar nicht, eine Kopie des Originals bleibt erhalten) und erzeugen Footprints nach IPC-7351B mit ihren kanonischen Namen. Die Maße der erzeugten Footprints sind ein Prototyp und wurden an keinem Datenblatt geprüft.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Prüfen Sie die Bibliothek. Ohne `--out` wird nichts geschrieben; der Bericht zählt Footprints, Symbole und Bauteilsätze und listet die Befunde.

```console
# test
$ phoenix libedit examples/tutorials/parts.lbr
~ 1 footprints, 1 symbols, 1 device sets
~ 0 errors, 0 warnings
~ dry run: nothing was written
```

2. Wenden Sie eine Operationsdatei an, die das Gehäuse `HDR1X2` anlegt und zwei Pads hinzufügt. Die neue Bibliothek wird in eine neue Datei geschrieben, zurückgelesen und verglichen; daneben bleibt eine Kopie des Originals.

```console
# test
$ phoenix libedit --apply examples/tutorials/lib_ops.json --out build/parts_hdr.lbr examples/tutorials/parts.lbr
~ operation 3: add pad to HDR1X2
~ the file was read back and is the same library
```

3. Erzeugen Sie den Footprint eines Chip-Widerstands 0603 in Dichtestufe B (nominal). Sein kanonischer Name ist `RESC1608X55N`: Der Körper ist 1,6 mm × 0,8 mm und 0,55 mm hoch. Ohne `--out` ist es ein Probelauf, der die Pads auflistet.

```console
# test
$ phoenix ipc generate chip 0603 --density B
RESC1608X55N (density B,
~ dry run: nothing was written
```

4. Schreiben Sie ihn in eine neue `.lbr`-Datei (mit einem Namen `.kicad_mod` wird stattdessen ein KiCad-Footprint geschrieben).

```console
# test
$ phoenix ipc generate chip 0603 --density B --out build/RESC1608X55N.lbr
~ written: build/RESC1608X55N.lbr
```

5. Der einfachere Generator `footprint` erzeugt einen Footprint aus Parametern (Familien Chip, SOIC, TSSOP, QFN, QFP, DIP und Stiftleiste).

```console
# test
$ phoenix footprint chip --param size=0603 --out build/chip.lbr
~ footprint CHIP_0603
~ written:
```

6. Prüfen Sie die erzeugten Bibliotheken: Jede hat einen Footprint.

```console
# test
$ phoenix libedit build/chip.lbr
~ 1 footprints
$ phoenix libedit build/RESC1608X55N.lbr
~ 1 footprints
```

7. Vergleichen Sie einen Footprint Ihrer Bibliothek mit dem erzeugten, Pad für Pad (Exit-Code 1, wenn Pads abweichen), und listen Sie die Familien der Bibliothek auf.

```console
$ phoenix ipc compare build/chip.lbr chip 0603
$ phoenix ipc list
```

## Siehe auch

* [docs/LIBRARY_EDIT.md](../../LIBRARY_EDIT.md)
* [docs/IPC_LIBRARY.md](../../IPC_LIBRARY.md)
* [docs/LIBINDEX.md](../../LIBINDEX.md)
* [docs/SANITIZER.md](../../SANITIZER.md)
