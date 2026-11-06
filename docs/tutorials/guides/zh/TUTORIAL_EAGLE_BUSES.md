# 读取 EAGLE 原理图中的总线

`examples/guides/bus.sch` 是第一个教程中的原理图，图页上多了一条总线 `D[0..1]` 和两个网络 `D0`、`D1`，它们的导线终止在总线的导线上。读取器把总线的名称变成带成员的总线对象，并在每个网络与总线相遇处放置一个总线入口。总线只是一种命名方式：它的成员就是普通网络。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 以 JSON 形式读取层次结构。键 `buses` 统计图页上的总线数：1。两个成员都被总线入口使用，所以问题列表中没有 `PHOENIX-BUS` 系列的代码。

```console
$ phoenix hier --json examples/guides/bus.sch
```

2. 以文本形式查看同样的内容：网络 `D0` 和 `D1` 与 `MID`、`GND` 一起按名称列出，最后一行说明是否有问题。

```console
$ phoenix hier examples/guides/bus.sch
```

3. 把原理图写成 KiCad 文件。总线被写成它的标签加上一条从锚点穿过各入口端点的线，每个入口写成带有其成员标签的 `bus_entry`。

```console
$ phoenix kicad-export --out build/bus.kicad_sch schematic examples/guides/bus.sch
```

## 如何阅读结果

* 向量 `前缀[起始..结束]` 给出成员 `前缀<起始>` 到 `前缀<结束>`（`D[0..1]` 得到 `D0` 和 `D1`；向量可以递减，如 `A[2..0]`）。读取器也接受 EAGLE 的列表（`A[0..3],B[0..3]`）和带冒号的名称（`BUS:A[0..3]`），冒号之前的部分是总线自身的名称。
* 总线锚定在它第一条导线的起点。图页上每个名称与某成员同名、且终止于总线导线上的网络，在其端点处生成一个入口。总线自身的导线和标签、不是向量的名称以及没有导线的总线都不会被读取，而是计为被省略的部分。代码 `PHOENIX-BUS-001` 到 `PHOENIX-BUS-007` 说明总线的问题，`phoenix hier` 会列出它们。

## 另见

* [docs/BUSES.md](../../../../docs/BUSES.md)
* [docs/HIERARCHY.md](../../../../docs/HIERARCHY.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
