# 迁移指南：从 EAGLE 或 KiCad 转来

本页把 EAGLE 和 KiCad 的概念与命令同本程序的对照起来，说明能读写什么、会丢失什么。本程序是原型：以下内容都没有在真实设计上与 EAGLE 或 KiCad 比较过。请先运行 `phoenix help`。

## 对应的命令

| 要做的事 | EAGLE 中 | KiCad 中 | 本程序 |
|---|---|---|---|
| 设计规则检查 | DRC | Design Rules Checker | `phoenix check`, `phoenix check --advanced` |
| 电气规则检查 | ERC | Electrical Rules Checker | `phoenix erc` |
| 电路板与原理图比对 | Forward & Back Annotation | Update PCB from Schematic | `phoenix connectivity`, `phoenix sync`, `phoenix annotate` |
| 尚未布线的连接 | RATSNEST | ratsnest | `phoenix ratsnest` |
| 布线 | ROUTE, AUTO | interactive router, Freerouting | `phoenix route`, `phoenix boardedit`, `phoenix autoroute` |
| 制造文件 | CAM Processor | Plot, Fabrication Outputs | `phoenix export`, `phoenix fabpack` |
| 物料清单与贴装坐标 | ULP | BOM and position files | `phoenix bom`, `phoenix pnp`, `phoenix stencil` |
| 拼板 | ULP | KiKit and similar | `phoenix panel` |
| 运行脚本 | SCRIPT, RUN | Python console | `phoenix script run` |
| 编辑库 | Library editor | Symbol and Footprint editors | `phoenix libedit`, `phoenix footprint`, `phoenix ipc` |
| 仿真 | SPICE export | ngspice | `phoenix netlist`, `phoenix sim` |

## 能读写什么

| 文件 | 读取 | 写出 |
|---|---|---|
| EAGLE `.sch`, `.brd`, `.lbr` (XML) | `eagle-check` 以及所有接收电路板或原理图的命令 | 以 EAGLE XML 形式的电路板和原理图: `boardedit`, `sync --apply`, `annotate --apply`; 库: `libedit`, `footprint`, `ipc` |
| EAGLE binary | 否 | 否 |
| EAGLE `.dru` | `check --rules-dru`, `eagle-convert dru-to-rules` | `eagle-convert rules-to-dru` |
| EAGLE `.cam` | `cam-import`, `eagle-convert cam-to-profile` | - |
| EAGLE `.scr` | `script run`, `script report` | - |
| EAGLE ULP | 否 | - |
| KiCad 6 - 9 `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_sym`, `.kicad_mod` | `kicad-read`, `kicad-import`; `.kicad_pcb` 和 `.kicad_sch` 也可用于接收电路板或原理图的其他命令 | KiCad 8: `kicad-export` |
| Gerber X2, Excellon | `gerber-import`, `gerber-check` | `export`, `fabpack`, `panel` |
| DXF R12 | `dxf-import` | `dxf-export` |
| Specctra DSN, SES | SES: `autoroute` | DSN: `autoroute` |

## 会丢失什么

* 设计模型不保存 KiCad 的图形、总线、无连接标记、电路板设置、铜区的填充铜、禁布区和 3D 模型；读取器把它们计为舍弃，并在报告中说明。
* EAGLE：不读取二进制文件，不运行 ULP 程序，模型简化的元素由 `phoenix eagle-check` 以八种语言的代码列出。
* 绝不会覆盖输入文件：转换总是生成新文件。

## 第一步

1. 用 `phoenix kicad-import`（KiCad）或 `phoenix eagle-check`（EAGLE）读取你的工程，并阅读报告。
2. 用 `phoenix check --rules 文件`（你的规则限值；EAGLE 规则文件可用 `phoenix eagle-convert dru-to-rules`）和 `phoenix erc` 检查。
3. 用 `phoenix export` 写出制造文件，在信任它们之前先与你常用工具的结果比较。

## 另见

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD_EXPORT.md](../../KICAD_EXPORT.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/EXPORT.md](../../EXPORT.md)
* [术语表](../../glossary/zh/GLOSSARY.md)
* [教程](../../tutorials/zh/README.md)
* [English](../../MIGRATION.md)
