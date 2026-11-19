# 从电路板到制造文件

`examples/tutorials/kicad/` 中的 KiCad 工程在 30 mm × 20 mm 的电路板上有两个电阻，由网络 `SIG` 相连。你要比对原理图与电路板、运行设计规则，并写出带校验清单的制造文件。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. 运行原理图的电气规则检查。每条结果都有代码，已确认的与无法判定的分开列出。

```console
$ phoenix erc examples/tutorials/kicad/demo.kicad_sch
```

2. 按元件身份比对原理图与电路板。`sync` 是试运行：只列出同步会改动的内容，除非加上 `--apply --out 文件`，否则不写入任何内容。

```console
$ phoenix sync examples/tutorials/kicad/demo.kicad_sch examples/tutorials/kicad/demo.kicad_pcb
```

3. 用规则文件中的限值运行设计规则。走线宽 0.25 mm，而文件要求至少 0.2 mm，因此没有错误，退出码为 0。

```console
# test
$ phoenix check --rules examples/tutorials/rules_relaxed.json examples/tutorials/kicad/demo.kicad_pcb
Confirmed: 0
Inconclusive: 0
```

4. 把制造文件写入新文件夹：每层一个 Gerber X2 文件、钻孔文件、贴装坐标文件、物料清单，以及记录每个文件 SHA-256 的 `manifest.json`。写出之前会把文件读回并与电路板比对。

```console
# test
$ phoenix export --out build/fab examples/tutorials/kicad/demo.kicad_pcb
~ files written: build/fab
```

5. 把包含钢网和各面贴装坐标的完整制造包打成一个确定性 zip：同一块板得到相同的字节。不加 `--out` 时是列出文件的试运行。

```console
$ phoenix fabpack --out build/demo_pack.zip examples/tutorials/kicad/demo.kicad_pcb
```

6. 发送之前先用数字查看电路板：尺寸、各项数量、每层的铜、钻孔和密度。不会写入任何内容。

```console
# test
$ phoenix stats examples/tutorials/kicad/demo.kicad_pcb
Board statistics:
Board: 30.000000 x 20.000000 mm; 600.000000 mm2; Cutouts: 0.000000 mm2
Components: 2 (top: 2; bottom: 0)
```

7. 用电路板编辑器的操作把一个焊盘标记为测试点（`examples/tutorials/testpoint_ops.json` 标记 R1 的第 2 个焊盘，在网络 `MID` 上），并生成测试点和没有探针能到达的网络的报告。输入文件不会被覆盖。

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/testpoint_ops.json --out build/with_test_point.json examples/tutorials/tut.brd
~ written: build/with_test_point.json
$ phoenix testpoints build/with_test_point.json
Test points of the board:
Test points: 1 (probe side: both)
Nets asked: 1; covered: 1; coverage: 100.00 %
```

8. 给带有文档层的电路板添加两个尺寸标注（`examples/tutorials/dimension_ops.json`：板的宽度和高度，尺寸线在外侧 3 mm），并写入 DXF 图纸；在窗口中用尺寸标注工具完成同样的事，*编辑尺寸标注* 可移动其中的点。在 `--out` 指定新文件之前只是试运行。

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/dimension_ops.json --out build/with_dimensions.json examples/tutorials/notes_board.json
~ written: build/with_dimensions.json
$ phoenix dxf-export board --dimensions build/with_dimensions.json
entities: 21
layer DIMENSIONS: 16
```

9. 请供应商助手告诉您到哪里采购物料清单。它读取价格文件（`examples/tutorials/prices.csv`，每个供应商和数量档位一行），以及设置文件（`examples/tutorials/sourcing.cfg`）中的供应商、运费和最低订单，并按每个供应商以及分拆采购为物料清单报价。价格取自文件：不会从网络获取任何内容。

```console
# test
$ phoenix bom --sourcing --config examples/tutorials/sourcing.cfg --prices examples/tutorials/prices.csv examples/tutorials/kicad/demo.kicad_pcb
~ suppliers: 2 considered, 1 lines, EUR
~ cheapest single supplier: Shop A for 2.210000
~ split purchase: 1 orders for 2.210000
```

## 另见

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
* [docs/STATS.md](../../STATS.md)
* [docs/TESTPOINTS.md](../../TESTPOINTS.md)
* [docs/DIMENSIONS.md](../../DIMENSIONS.md)
* [docs/BOM.md](../../BOM.md)
