# 对物料清单分组并导出，并导入其他程序的物料清单

电路板 `examples/guides/bom_four.brd` 上有三个 10 k 电阻和一个 4k7 电阻。你在设置文件中选择零件如何分组以及 CSV 文件有哪些列，然后用列映射表读取其他程序写出的物料清单。不会从网络获取任何内容，设置中的价格表只是报价形式的示例。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 按默认分组（值、封装、制造商和料号）以及 `bom_parts.csv` 中的制造商数据列出清单。R1 和 R2 的制造商与料号相同，R3 的制造商不同，所以有三行。没有价格文件，因此每一行都带有 `PHOENIX-BOM-005`。

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

2. 同一块电路板，在 `bom_costs_joined.cfg` 中使用 `group_by = value,footprint`：制造商和料号不再参与分组，R1、R2、R3 合并为数量为 3 的一行，且这一行没有制造商和料号。

```console
$ phoenix bom --config examples/guides/bom_costs_joined.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

3. 写出 CSV 文件。它的列就是设置中的 `csv_columns`，按该顺序排列（这里是 `references,quantity,value,footprint,manufacturer,mpn,status`）；HTML 网页和 JSON 文件保留全部列。已存在的文件不会被替换。

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv --csv build/bom.csv examples/guides/bom_four.brd
```

4. 导入其他程序的物料清单。`bom_other_program.csv` 使用分号和它自己的列名，`bom_import_map.cfg` 说明如何读取它。报告会把清单与电路板比较：R5 在清单中但不在电路板上（`PHOENIX-BOM-021`），R4 被标记为不贴装，所以从清单中去掉并作为信息列出（`PHOENIX-BOM-009`）。`--parts-out` 把清单中关于制造商和料号的内容保存为零件数据文件。

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --import examples/guides/bom_other_program.csv --import-map examples/guides/bom_import_map.cfg --parts-out build/parts_from_bill.csv examples/guides/bom_four.brd
```

5. 以后的运行中，用保存下来的零件数据文件代替原来的清单。

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts build/parts_from_bill.csv examples/guides/bom_four.brd
```

## 如何阅读结果

* 默认的设置文件是 `config/costs.cfg`，默认的映射表是 `config/bom_import.cfg`（如果存在）。映射表指定分隔符（逗号、分号或制表符），并为每一列给出它在文件中可能使用的名称；只有位号是必需的。程序自带的名称只是猜测，不是任何程序的格式：请复制该文件并写入你自己文件中的列名。
* 代码 `PHOENIX-BOM-019`、`PHOENIX-BOM-020` 和 `PHOENIX-BOM-021` 是警告，不会改变任何内容：数量与所在行的位号数不符、同一位号出现在两行（采用第一行）、只有一侧有的零件或值不同的零件。`R1-R5` 这样的范围会被展开为单个元件；格式错误、顺序颠倒或超过 `max_range` 的范围会中止导入（`PHOENIX-BOM-022` 至 `PHOENIX-BOM-024`）。`--import-format` 还能读取 EAGLE 的元件清单和 KiCad 的物料清单（`docs/BOM.md`）。`--parts` 的内容优先于导入的清单。
* 不带 `--csv`、`--json-file` 和 `--html` 时，命令不写任何文件。退出码：正常完成为 0，某个元件引用了电路板上没有的封装为 1，参数或输入无法使用为 2。

## 另见

* [docs/BOM.md](../../../../docs/BOM.md)
* [docs/PARTS.md](../../../../docs/PARTS.md)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
