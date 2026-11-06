# Guida alla migrazione: se arrivate da EAGLE o da KiCad

Questa pagina affianca le idee e i comandi di EAGLE e KiCad a quelli di questo programma, dice cosa si legge e cosa si scrive, e cosa si perde. Il programma è un prototipo: nulla di quanto segue è stato confrontato con EAGLE o KiCad su progetti reali. Iniziate con `phoenix help`.

## Comandi equivalenti

| Cosa fate | In EAGLE | In KiCad | Qui |
|---|---|---|---|
| Controllo delle regole di progetto | DRC | Design Rules Checker | `phoenix check`, `phoenix check --advanced` |
| Controllo delle regole elettriche | ERC | Electrical Rules Checker | `phoenix erc` |
| Scheda rispetto allo schema | Forward & Back Annotation | Update PCB from Schematic | `phoenix connectivity`, `phoenix sync`, `phoenix annotate` |
| Collegamenti da instradare | RATSNEST | ratsnest | `phoenix ratsnest` |
| Instradare | ROUTE, AUTO | interactive router, Freerouting | `phoenix route`, `phoenix boardedit`, `phoenix autoroute` |
| File di fabbricazione | CAM Processor | Plot, Fabrication Outputs | `phoenix export`, `phoenix fabpack` |
| Distinta base e posizionamento | ULP | BOM and position files | `phoenix bom`, `phoenix pnp`, `phoenix stencil` |
| Pannello | ULP | KiKit and similar | `phoenix panel` |
| Eseguire uno script | SCRIPT, RUN | Python console | `phoenix script run` |
| Modificare una libreria | Library editor | Symbol and Footprint editors | `phoenix libedit`, `phoenix footprint`, `phoenix ipc` |
| Simulare | SPICE export | ngspice | `phoenix netlist`, `phoenix sim` |

## Cosa si legge e cosa si scrive

| File | Lettura | Scrittura |
|---|---|---|
| EAGLE `.sch`, `.brd`, `.lbr` (XML) | `eagle-check` e ogni comando che prende una scheda o uno schema | schede e schemi come XML EAGLE: `boardedit`, `sync --apply`, `annotate --apply`; librerie: `libedit`, `footprint`, `ipc` |
| EAGLE binary | no | no |
| EAGLE `.dru` | `check --rules-dru`, `eagle-convert dru-to-rules` | `eagle-convert rules-to-dru` |
| EAGLE `.cam` | `cam-import`, `eagle-convert cam-to-profile` | - |
| EAGLE `.scr` | `script run`, `script report` | - |
| EAGLE ULP | no | - |
| KiCad 6 - 9 `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_sym`, `.kicad_mod` | `kicad-read`, `kicad-import`; `.kicad_pcb` e `.kicad_sch` anche nei comandi che prendono una scheda o uno schema | KiCad 8: `kicad-export` |
| Gerber X2, Excellon | `gerber-import`, `gerber-check` | `export`, `fabpack`, `panel` |
| DXF R12 | `dxf-import` | `dxf-export` |
| Specctra DSN, SES | SES: `autoroute` | DSN: `autoroute` |

## Cosa si perde

* Il modello di progetto non contiene grafica KiCad, bus, marcatori di non connessione, la configurazione della scheda, il rame riempito delle zone, le zone di esclusione né i modelli 3D; i lettori li contano come lasciati fuori e lo dicono nel rapporto.
* EAGLE: i file binari non si leggono, i programmi ULP non si eseguono, e `phoenix eagle-check` elenca gli elementi che il modello semplifica, con codici in otto lingue.
* Nulla viene scritto su un ingresso: una conversione crea sempre un file nuovo.

## Primi passi

1. Leggete il vostro progetto con `phoenix kicad-import` (KiCad) o `phoenix eagle-check` (EAGLE) e leggete il rapporto.
2. Controllatelo con `phoenix check --rules FILE` (i limiti delle vostre regole, o `phoenix eagle-convert dru-to-rules` per un file di regole EAGLE) e `phoenix erc`.
3. Scrivete i file di fabbricazione con `phoenix export` e confrontateli con quelli del vostro strumento abituale prima di fidarvi.

## Vedere anche

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD_EXPORT.md](../../KICAD_EXPORT.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/EXPORT.md](../../EXPORT.md)
* [Glossario](../../glossary/it/GLOSSARY.md)
* [Tutorial](../../tutorials/it/README.md)
* [English](../../MIGRATION.md)
