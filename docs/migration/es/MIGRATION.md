# Guía de migración: si viene de EAGLE o de KiCad

Esta página pone las ideas y comandos de EAGLE y KiCad junto a los de este programa, dice qué se lee y qué se escribe, y qué se pierde. El programa es un prototipo: nada de lo que sigue se ha comparado con EAGLE ni con KiCad en diseños reales. Empiece por `phoenix help`.

## Comandos equivalentes

| Lo que hace | En EAGLE | En KiCad | Aquí |
|---|---|---|---|
| Comprobación de reglas de diseño | DRC | Design Rules Checker | `phoenix check`, `phoenix check --advanced` |
| Comprobación de reglas eléctricas | ERC | Electrical Rules Checker | `phoenix erc` |
| Placa frente a esquema | Forward & Back Annotation | Update PCB from Schematic | `phoenix connectivity`, `phoenix sync`, `phoenix annotate` |
| Conexiones por rutar | RATSNEST | ratsnest | `phoenix ratsnest` |
| Rutear | ROUTE, AUTO | interactive router, Freerouting | `phoenix route`, `phoenix boardedit`, `phoenix autoroute` |
| Ficheros de fabricación | CAM Processor | Plot, Fabrication Outputs | `phoenix export`, `phoenix fabpack` |
| Lista de materiales y posición | ULP | BOM and position files | `phoenix bom`, `phoenix pnp`, `phoenix stencil` |
| Panel | ULP | KiKit and similar | `phoenix panel` |
| Ejecutar un script | SCRIPT, RUN | Python console | `phoenix script run` |
| Editar una biblioteca | Library editor | Symbol and Footprint editors | `phoenix libedit`, `phoenix footprint`, `phoenix ipc` |
| Simular | SPICE export | ngspice | `phoenix netlist`, `phoenix sim` |

## Qué se lee y qué se escribe

| Fichero | Lectura | Escritura |
|---|---|---|
| EAGLE `.sch`, `.brd`, `.lbr` (XML) | `eagle-check` y todos los comandos que toman una placa o un esquema | placas y esquemas como XML de EAGLE: `boardedit`, `sync --apply`, `annotate --apply`; bibliotecas: `libedit`, `footprint`, `ipc` |
| EAGLE binary | no | no |
| EAGLE `.dru` | `check --rules-dru`, `eagle-convert dru-to-rules` | `eagle-convert rules-to-dru` |
| EAGLE `.cam` | `cam-import`, `eagle-convert cam-to-profile` | - |
| EAGLE `.scr` | `script run`, `script report` | - |
| EAGLE ULP | no | - |
| KiCad 6 - 9 `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_sym`, `.kicad_mod` | `kicad-read`, `kicad-import`; `.kicad_pcb` y `.kicad_sch` también en los comandos que toman una placa o un esquema | KiCad 8: `kicad-export` |
| Gerber X2, Excellon | `gerber-import`, `gerber-check` | `export`, `fabpack`, `panel` |
| DXF R12 | `dxf-import` | `dxf-export` |
| Specctra DSN, SES | SES: `autoroute` | DSN: `autoroute` |

## Qué se pierde

* El modelo de diseño no guarda gráficos de KiCad, buses, marcas de no conexión, la configuración de la placa, el cobre relleno de las zonas, las zonas de exclusión ni los modelos 3D; los lectores los cuentan como omitidos y lo dicen en el informe.
* EAGLE: los ficheros binarios no se leen, los programas ULP no se ejecutan, y `phoenix eagle-check` lista los elementos que el modelo simplifica, con códigos en ocho idiomas.
* Nada se escribe sobre una entrada: una conversión siempre crea un fichero nuevo.

## Primeros pasos

1. Lea su proyecto con `phoenix kicad-import` (KiCad) o `phoenix eagle-check` (EAGLE) y lea el informe.
2. Compruébelo con `phoenix check --rules FICHERO` (los límites de sus reglas, o `phoenix eagle-convert dru-to-rules` para un fichero de reglas de EAGLE) y `phoenix erc`.
3. Escriba los ficheros de fabricación con `phoenix export` y compárelos con los de su herramienta habitual antes de fiarse de ellos.

## Véase también

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD_EXPORT.md](../../KICAD_EXPORT.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/EXPORT.md](../../EXPORT.md)
* [Glosario](../../glossary/es/GLOSSARY.md)
* [Tutoriales](../../tutorials/es/README.md)
* [English](../../MIGRATION.md)
