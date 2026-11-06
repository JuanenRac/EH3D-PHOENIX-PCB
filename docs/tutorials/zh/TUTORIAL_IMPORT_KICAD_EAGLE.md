# 导入 KiCad 或 EAGLE 工程并审查

把整个 KiCad 工程读入本程序的设计模型，查看保留和舍弃了什么，把模型写成新文件，并用规则文件检查电路板。然后对一块 EAGLE 板做同样的审查。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. 读取工程（`.kicad_pro` 及同名的 `.kicad_sch` 和 `.kicad_pcb`）。这是试运行：报告会说明读取了什么、保留了多少，以及模型不包含什么。

```console
# test
$ phoenix kicad-import examples/tutorials/kicad/demo.kicad_pro
~ usable: yes
~ components: 2
~ tracks: 1
~ vias: 1
~ dry run: nothing was written
```

2. 按你给出的名称把模型写成新文件。已存在的文件或输入文件绝不会被覆盖。

```console
# test
$ phoenix kicad-import --out build/demo.json examples/tutorials/kicad/demo.kicad_pro
~ written:
```

3. 检查导入的电路板。KiCad 板不向程序提供设计规则，因此限值来自规则文件。文件要求走线至少 0.3 mm，而板上的走线是 0.25 mm：有 1 条已确认的结果，退出码为 1。

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
Confirmed: 1
~ PHOENIX-CHECK-001
```

4. 对一个 EAGLE 文件，查看重写会保留什么。报告会证明把 XML 树写出再读回不会有任何变化，并列出设计模型简化或舍弃的内容。

```console
# test
$ phoenix eagle-check examples/tutorials/tut.brd
board (version 9.6.2)
~ rewrite of the tree: identical
```

5. 对 EAGLE 板运行同一个规则文件。网络 `MID` 的走线宽 0.2 mm。

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/tut.brd
# exit 1
Confirmed: 1
```

6. 比对 EAGLE 原理图与 EAGLE 电路板：报告会列出缺失的网络、未连接的引脚以及尚未布线的铜。

```console
$ phoenix connectivity examples/tutorials/tut.sch examples/tutorials/tut.brd
```

## 另见

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD.md](../../KICAD.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/RULES.md](../../RULES.md)
