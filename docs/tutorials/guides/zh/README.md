# 功能指南

## 如何运行指南

请在仓库根目录运行每条命令。命令只写入新文件，或写入 `build/` 内的新文件夹（先用 `mkdir build` 创建）；不会替换任何已有内容。指南展示命令并说明输出包含什么。以 `# test` 开头的块由仓库的测试运行，测试会比较输出的开头（以 `~` 开头的行必须出现在输出的某个位置）；其余命令不会被运行，也不引用任何输出，所以请对照指南阅读实际输出。本程序的所有工具都是原型，尚未与参考工具比较过。

## 指南

1. [把检查结果写成表格和网页，并按制造商的能力档案检查电路板](TUTORIAL_CHECK_REPORTS_DFM.md)
1. [对物料清单分组并导出，并导入其他程序的物料清单](TUTORIAL_BOM_GROUPING_IMPORT.md)
1. [用模板在一个设置文件中保存多种拼板布局](TUTORIAL_PANEL_TEMPLATES.md)
1. [用场求解器求线路阻抗，并估算走线的温升](TUTORIAL_FIELD_SOLVER_HEATING.md)
1. [把铜区的拐角倒圆，并在 Gerber 文件中写出热焊盘和孤岛下限](TUTORIAL_ZONE_CORNERS_RELIEFS.md)
1. [让外形开口和铜区挖空穿过各种文件格式](TUTORIAL_CUTOUTS.md)
1. [读取 EAGLE 原理图中的总线](TUTORIAL_EAGLE_BUSES.md)
1. [用内置自动布线器为电路板布线：差分对和复用板上已有的过孔](TUTORIAL_AUTOROUTE_NATIVE.md)
1. [在电路板编辑器中选择多个对象，并作为一步来移动、对齐或删除](TUTORIAL_MULTIPLE_SELECTION.md)
1. [用 phoenix 的截图模式为窗口截图](TUTORIAL_SCREENSHOTS.md)
1. [用限制文件约束检查的工作量](TUTORIAL_LIMITS.md)

## 教程

* [教程](../../zh/README.md)

## 语言

* [English](../README.md)
* [Español](../es/README.md)
* [Deutsch](../de/README.md)
* [Français](../fr/README.md)
* [Italiano](../it/README.md)
* [日本語](../ja/README.md)
* [Português (Portugal)](../pt/README.md)
