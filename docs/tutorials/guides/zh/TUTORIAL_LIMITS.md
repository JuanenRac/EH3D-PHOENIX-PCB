# 用限制文件约束检查的工作量

有些检查所需时间与某个数量的平方成正比，例如检查多边形的边是否相交。限制可以约束这个时间：超过限制的多边形不做相交测试，报告会以信息的形式说明。先用默认值检查一块板，再用一个低得多的限制文件检查，并查看这条说明。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 在 KiCad 板上用默认限制运行规则文件。板上的走线比文件要求的更细，所以有一条已确认的发现，退出码为 1。

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. 再用 `--limits examples/guides/limits_small.cfg` 运行一次，其中写着 `max_polygon_points = 3`。板的轮廓有四个点，所以它以代码 `PHOENIX-BOARD-017` 作为信息列出；退出码仍为 1，因为信息从不改变退出码。

```console
# test
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ PHOENIX-BOARD-017
~ max_polygon_points = 3
~ PHOENIX-CHECK-001
```

3. 同一条说明换一种语言，并输出 JSON 供脚本使用。代码保持英文；标题跟随 `--lang`。

```console
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --lang pt examples/tutorials/kicad/demo.kicad_pcb
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --json examples/tutorials/kicad/demo.kicad_pcb
```

## 如何阅读结果

* 每条命令都接受 `--limits 文件`，可放在命令名之前或之后。文件包含 `config/board.cfg` 和 `config/eagle_board.cfg` 中 `键 = 值` 形式的行，顺序不限；未知的行或超出范围的值会被拒绝，并给出行号和退出码 2。 `config/outline_arcs.cfg` 的两个键（`tolerance_nm` 和 `max_pieces`，决定轮廓圆弧如何切成弦）也可以写在同一个文件中。
* 限制只改变它所约束的部分。使用默认值时任何输出都不变，超过 `max_polygon_points` 的多边形仍会检查重复点。

## 另请参阅

* [config/board.cfg](../../../../config/board.cfg)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
* [docs/RULES.md](../../../../docs/RULES.md)
