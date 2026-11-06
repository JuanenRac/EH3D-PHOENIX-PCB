# 把铜区的拐角倒圆，并在 Gerber 文件中写出热焊盘和孤岛下限

电路板 `examples/guides/pour.brd` 的 `Top` 层上有一块围绕两个电阻和一条走线的接地铜区。你先用尖角、再用圆角填充并比较，然后用一个档案写出 Gerber 文件：该档案给铜区中的矩形焊盘加上热焊盘，并去掉铜区的小孤岛。这里给出规则文件 `rules_relaxed.json`，因为没有设计规则的 EAGLE 电路板本身没有限值。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 用仓库的设置填充该区域，所有拐角保持尖角（`corner_radius_nm = 0`）。报告给出多边形数和铜的平方纳米数。

```console
$ phoenix zones fill --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

2. 再用 `zones_round.cfg` 填充一次，它设置 `corner_radius_nm = 1000000`（半径 1 mm 的圆弧）。铜区朝外的拐角和热焊盘缺口的拐角会被倒圆；请把铜的面积与第一次运行比较。

```console
$ phoenix zones fill --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

3. 对照电路板和规则测量圆角填充。倒圆只会从铜区上去掉铜，或仅在热焊盘缺口内部增加铜，因此填充不会更靠近其他网络的铜，检查不会发现间距错误。

```console
$ phoenix zones check --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

4. 先用通用档案写出 Gerber 文件。接地网络的焊盘 `R2.2` 位于铜区内，不带热焊盘直接与铜区相连。

```console
$ phoenix export --out build/pour_plain examples/guides/pour.brd
```

5. 现在使用 `export_pour.json` 中的档案 `pour`：四根辐条（`gerber.zone_thermal_spokes`）、间隙和辐条均为 0.25 mm、对矩形和长圆焊盘使用同样的热焊盘（`gerber.zone_thermal_non_round`），并且不写出小于 1 平方毫米的孤岛（`gerber.zone_island_min_area_nm2`）。比较两个文件夹中 `Top` 层的铜文件：后者把热焊盘的缺口写成焊盘周围的清除极性区域。

```console
$ phoenix export --profiles examples/guides/export_pour.json --profile pour --out build/pour_relief examples/guides/pour.brd
```

## 如何阅读结果

* 在填充设置中，`corner_radius_nm` 是圆弧半径（为零则所有拐角保持尖角），`corner_tolerance_nm` 是圆弧的弦允许偏离圆弧的距离。圆弧最多占用拐角两侧每条边的一半，放不下的半径会被减小。其他铜在铜区中留下的孔洞的拐角不会被倒圆。
* 导出时，热焊盘有四根或两根辐条（`gerber.zone_thermal_spokes` 取 0、2 或 4；0 表示焊盘与铜区实心相连）。旋转角度不是直角的焊盘保持实心相连并给出说明（`PHOENIX-EXPORT-022`）。小于 `gerber.zone_island_min_area_nm2` 的孤岛不会被写出，报告会说明（`PHOENIX-EXPORT-021`）；为零则写出所有区域。

## 另见

* [docs/ZONES.md](../../../../docs/ZONES.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
