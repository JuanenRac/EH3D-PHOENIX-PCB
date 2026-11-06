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

## 另见

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
