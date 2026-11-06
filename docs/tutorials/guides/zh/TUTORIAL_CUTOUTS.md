# 让外形开口和铜区挖空穿过各种文件格式

`examples/guides/cutout.brd` 是一块 EAGLE 电路板，有两种开口：外形内部第 20 层上的一个闭合方框（电路板的开口，即槽或窗口），以及 `Top` 层铜区内部第 41 层上的一个多边形（铜区的挖空，此处不得有铜）。你把电路板写成 DXF，再读回来，再写出 KiCad 和 Gerber 文件，看看每种开口去了哪里。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 列出该电路板的 DXF 图纸将包含什么。外形和开口是图层 `OUTLINE` 上的两条闭合轮廓；报告显示 `layer OUTLINE: 2`，并说明没有写入任何文件。

```console
$ phoenix dxf-export board examples/guides/cutout.brd
```

2. 把图纸写入一个新文件。

```console
$ phoenix dxf-export board examples/guides/cutout.brd --out build/cutout.dxf
```

3. 把图纸读回电路板。最大的外部轮廓成为电路板外形，其内部的轮廓作为外形的开口保留（`PHOENIX-DXF-015`，信息）。该操作是试运行，前后都会用设计规则检查，所以要给出规则文件。

```console
$ phoenix dxf-import build/cutout.dxf --board examples/guides/cutout.brd --layer OUTLINE --rules examples/tutorials/rules_relaxed.json
```

4. 写出 KiCad 电路板。外形和开口放在 `Edge.Cuts` 上，铜区的挖空变成其所在层的规则区域：不允许铜区进入，但允许走线、过孔和焊盘。报告会列出写出的内容以及该格式无法表达的内容。

```console
$ phoenix kicad-export --out build/cutout.kicad_pcb board examples/guides/cutout.brd
```

5. 写出制造文件。轮廓文件先是外形，然后是开口，各是一条闭合轮廓；在铜层文件中，铜区的挖空被写成铜区之后的清除极性区域。

```console
$ phoenix export --out build/cutout_fab examples/guides/cutout.brd
```

## 如何阅读结果

* 开口的来源：在 EAGLE 电路板中，第 20 层上首尾相接并闭合的线，位于同类最大环内部的，就是电路板的开口；位于该层铜区内部的第 41 层（`tRestrict`，顶层）或第 42 层（底层）上的多边形是铜区的挖空。在 KiCad 电路板中，`Edge.Cuts` 上位于最大闭合轮廓内部的闭合轮廓是开口；在 DXF 中则是最大外部轮廓内部的轮廓。
* 每种格式只表达它能表达的内容。铜区的挖空在 KiCad 中是规则区域，在 EAGLE 中是禁布区，在自动布线器的 Specctra 文件中是禁入区（Specctra 的平面不能有孔，所以走线和过孔也被挡在外面）。拼板会把两种开口带到每个副本中，并随副本一起移动、旋转和镜像。

## 另见

* [docs/DXF.md](../../../../docs/DXF.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/PANEL.md](../../../../docs/PANEL.md)
