# Leggere i bus di uno schema EAGLE

`examples/guides/bus.sch` è lo schema del primo tutorial con un bus `D[0..1]` disegnato sul foglio e due reti, `D0` e `D1`, i cui fili terminano sul filo del bus. Il lettore trasforma il nome del bus in un oggetto bus con i suoi membri e mette un ingresso dove ogni rete incontra il bus. Un bus è un modo di dare nomi: i suoi membri sono reti normali.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Leggete la gerarchia come JSON. La chiave `buses` conta i bus del foglio: 1. L'elenco dei problemi non ha alcun codice della famiglia `PHOENIX-BUS`, perché entrambi i membri sono usati da un ingresso.

```console
$ phoenix hier --json examples/guides/bus.sch
```

2. Lo stesso come testo: le reti `D0` e `D1` compaiono per nome accanto a `MID` e `GND`, e l'ultima riga dice se ci sono problemi.

```console
$ phoenix hier examples/guides/bus.sch
```

3. Scrivete lo schema come file KiCad. Il bus è scritto come la sua etichetta e una linea dal suo ancoraggio attraverso le estremità dei suoi ingressi, e ogni ingresso come un `bus_entry` con l'etichetta del suo membro.

```console
$ phoenix kicad-export --out build/bus.kicad_sch schematic examples/guides/bus.sch
```

## Come leggere il risultato

* Un vettore `prefisso[primo..ultimo]` dà i membri `prefisso<primo>` fino a `prefisso<ultimo>` (`D[0..1]` dà `D0` e `D1`; un vettore può scendere, come `A[2..0]`). Il lettore accetta anche le liste di EAGLE (`A[0..3],B[0..3]`) e un nome con i due punti (`BUS:A[0..3]`), dove ciò che precede i due punti dà il nome al bus stesso.
* Il bus è ancorato dove inizia il suo primo filo. Si crea un ingresso all'estremità di ogni rete del foglio che ha il nome di un membro e termina su un filo del bus. I fili e le etichette del bus, i nomi che non sono vettori e un bus senza filo non si leggono e si contano come parti omesse. I codici da `PHOENIX-BUS-001` a `PHOENIX-BUS-007` dicono cosa non va in un bus, e `phoenix hier` li elenca.

## Vedere anche

* [docs/BUSES.md](../../../../docs/BUSES.md)
* [docs/HIERARCHY.md](../../../../docs/HIERARCHY.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
