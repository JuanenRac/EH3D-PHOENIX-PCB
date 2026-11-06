# Guia de migração: se vem do EAGLE ou do KiCad

Esta página coloca as ideias e os comandos do EAGLE e do KiCad ao lado dos deste programa, diz o que se lê e se escreve, e o que se perde. O programa é um protótipo: nada do que se segue foi comparado com o EAGLE ou o KiCad em projetos reais. Comece por `phoenix help`.

## Comandos equivalentes

| O que faz | No EAGLE | No KiCad | Aqui |
|---|---|---|---|
| Verificação das regras de projeto | DRC | Design Rules Checker | `phoenix check`, `phoenix check --advanced` |
| Verificação das regras elétricas | ERC | Electrical Rules Checker | `phoenix erc` |
| Placa face ao esquema | Forward & Back Annotation | Update PCB from Schematic | `phoenix connectivity`, `phoenix sync`, `phoenix annotate` |
| Ligações por encaminhar | RATSNEST | ratsnest | `phoenix ratsnest` |
| Encaminhar | ROUTE, AUTO | interactive router, Freerouting | `phoenix route`, `phoenix boardedit`, `phoenix autoroute` |
| Ficheiros de fabrico | CAM Processor | Plot, Fabrication Outputs | `phoenix export`, `phoenix fabpack` |
| Lista de materiais e posicionamento | ULP | BOM and position files | `phoenix bom`, `phoenix pnp`, `phoenix stencil` |
| Painel | ULP | KiKit and similar | `phoenix panel` |
| Executar um script | SCRIPT, RUN | Python console | `phoenix script run` |
| Editar uma biblioteca | Library editor | Symbol and Footprint editors | `phoenix libedit`, `phoenix footprint`, `phoenix ipc` |
| Simular | SPICE export | ngspice | `phoenix netlist`, `phoenix sim` |

## O que se lê e se escreve

| Ficheiro | Leitura | Escrita |
|---|---|---|
| EAGLE `.sch`, `.brd`, `.lbr` (XML) | `eagle-check` e todos os comandos que recebem uma placa ou um esquema | placas e esquemas como XML do EAGLE: `boardedit`, `sync --apply`, `annotate --apply`; bibliotecas: `libedit`, `footprint`, `ipc` |
| EAGLE binary | não | não |
| EAGLE `.dru` | `check --rules-dru`, `eagle-convert dru-to-rules` | `eagle-convert rules-to-dru` |
| EAGLE `.cam` | `cam-import`, `eagle-convert cam-to-profile` | - |
| EAGLE `.scr` | `script run`, `script report` | - |
| EAGLE ULP | não | - |
| KiCad 6 - 9 `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_sym`, `.kicad_mod` | `kicad-read`, `kicad-import`; `.kicad_pcb` e `.kicad_sch` também nos comandos que recebem uma placa ou um esquema | KiCad 8: `kicad-export` |
| Gerber X2, Excellon | `gerber-import`, `gerber-check` | `export`, `fabpack`, `panel` |
| DXF R12 | `dxf-import` | `dxf-export` |
| Specctra DSN, SES | SES: `autoroute` | DSN: `autoroute` |

## O que se perde

* O modelo de projeto não guarda gráficos do KiCad, barramentos, marcas de não ligação, a configuração da placa, o cobre preenchido das zonas, as zonas de exclusão nem os modelos 3D; os leitores contam-nos como deixados de fora e dizem-no no relatório.
* EAGLE: os ficheiros binários não são lidos, os programas ULP não correm, e o `phoenix eagle-check` lista os elementos que o modelo simplifica, com códigos em oito idiomas.
* Nada é escrito sobre uma entrada: uma conversão cria sempre um ficheiro novo.

## Primeiros passos

1. Leia o seu projeto com `phoenix kicad-import` (KiCad) ou `phoenix eagle-check` (EAGLE) e leia o relatório.
2. Verifique-o com `phoenix check --rules FICHEIRO` (os limites das suas regras, ou `phoenix eagle-convert dru-to-rules` para um ficheiro de regras EAGLE) e `phoenix erc`.
3. Escreva os ficheiros de fabrico com `phoenix export` e compare-os com os da sua ferramenta habitual antes de confiar neles.

## Ver também

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD_EXPORT.md](../../KICAD_EXPORT.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/EXPORT.md](../../EXPORT.md)
* [Glossário](../../glossary/pt/GLOSSARY.md)
* [Tutoriais](../../tutorials/pt/README.md)
* [English](../../MIGRATION.md)
