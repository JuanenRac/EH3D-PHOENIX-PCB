# 把电路板拼板并写出拼板文件

把 30 mm × 20 mm 的电路板并排排两块，用 V 形槽相连，并写出整个拼板的 Gerber 和钻孔文件。拼板设置在 `examples/tutorials/panel.cfg`（列、行、连接方式、间隙和边框）；完整的设置列表见 `config/panel.cfg`。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. 规划拼板。试运行会打印尺寸、连接方式、孔和将要写出的文件，但不写入任何内容。

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ joint: V-score, 1 lines
~ dry run: nothing written
```

2. 把每个副本的各层、钻孔文件、带 V 形槽线的外形、贴装坐标和物料清单写入新文件夹。

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ written
```

3. 绝不会向已存在的文件夹写入：第二次运行会被拒绝，代码为 `PHOENIX-PANEL-008`，退出码为 2。

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
# exit 2
~~ PHOENIX-PANEL-008
```

4. 检查拼板某个 Gerber 文件的结构，并把拼板画成 DXF 图纸供机械部分使用。

```console
$ phoenix gerber-check build/panel/panel_01_top.gbr
$ phoenix dxf-export panel --out build/panel.dxf --panel-config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
```

## 另见

* [docs/PANEL.md](../../PANEL.md)
* [docs/GERBER_IMPORT.md](../../GERBER_IMPORT.md)
* [docs/DXF.md](../../DXF.md)
* [docs/FABPACK.md](../../FABPACK.md)
