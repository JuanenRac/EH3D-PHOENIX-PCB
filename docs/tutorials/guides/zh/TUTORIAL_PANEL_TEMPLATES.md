# 用模板在一个设置文件中保存多种拼板布局

`examples/guides/panel_templates.cfg` 中有一个基础布局（两块板并排，用 V 形槽相连，无边框）和两个命名模板。每个模板只改动它所列出的行，用 `--template` 选择其中一个。电路板与拼板教程中的相同，为 30 mm × 20 mm；这里的所有命令都是试运行。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 不使用模板时只读取基础部分，模板各节下的行被忽略：60 mm × 20 mm，2 x 1 块板。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg examples/tutorials/kicad/demo.kicad_pcb
```

2. 模板 `four` 只把 `columns` 改为 4：120 mm × 20 mm，4 x 1 块板。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four examples/tutorials/kicad/demo.kicad_pcb
```

3. 模板 `grid` 把 `rows` 改为 2：60 mm × 40 mm，2 x 2 块板。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template grid examples/tutorials/kicad/demo.kicad_pcb
```

4. `--columns` 和 `--rows` 仍然优先于模板：使用 `four` 并加上 `--columns 3` 时，拼板为 90 mm × 20 mm，3 x 1 块板。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --columns 3 examples/tutorials/kicad/demo.kicad_pcb
```

5. 文件中没有的名称会被拒绝，代码为 `PHOENIX-PANEL-001`，退出码为 2。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template ring examples/tutorials/kicad/demo.kicad_pcb
```

6. 像拼板教程那样，把所选布局的文件写入一个新文件夹。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --out build/panel_four examples/tutorials/kicad/demo.kicad_pcb
```

## 如何阅读结果

* 第一个 `[template 名称]` 行之前的行是基础部分。名称由 1 到 32 个字母、数字、`_` 或 `-` 组成；写法不同的节会被拒绝，并给出 `PHOENIX-PANEL-001` 和行号。
* 尺寸行的形式是 `panel W mm by H mm, C x R boards`。V 形槽连接且间隙为 0 时板与板相接，所以宽度是列数乘 30 mm，高度是行数乘 20 mm。模板只是给一种布局起名保存的方法，不是任何制造商的设置。

## 另见

* [docs/PANEL.md](../../../../docs/PANEL.md)
* [config/panel.cfg](../../../../config/panel.cfg)
