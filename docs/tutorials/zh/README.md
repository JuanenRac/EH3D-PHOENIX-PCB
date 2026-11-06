# 教程

## 如何运行教程

请在仓库根目录运行每条命令。命令只写入新文件，或写入 `build/` 内的新文件夹（先用 `mkdir build` 创建）；不会替换任何已有内容。所示输出是每条命令输出的开头部分。以 `# test` 开头的代码块由仓库的测试运行，测试会比较输出的开头（以 `~` 开头的行必须出现在输出的某处）。本程序的所有工具都是原型，尚未与参考工具比较过。

帮助内置在程序中：`phoenix help` 列出主题和命令，`phoenix help sim` 和 `phoenix help --lang es sim` 解释某个主题或命令。

## 教程

1. [从原理图到网表再到仿真](TUTORIAL_NETLIST_AND_SIMULATION.md)
1. [从电路板到制造文件](TUTORIAL_BOARD_TO_FABRICATION.md)
1. [导入 KiCad 或 EAGLE 工程并审查](TUTORIAL_IMPORT_KICAD_EAGLE.md)
1. [把电路板拼板并写出拼板文件](TUTORIAL_PANEL.md)
1. [仿真 RC 滤波器和反相放大器](TUTORIAL_FILTER_AND_AMPLIFIER.md)
1. [编辑库并生成 IPC 封装](TUTORIAL_LIBRARY_AND_FOOTPRINTS.md)
1. [阻抗匹配与滤波器综合](TUTORIAL_RF_MATCHING_FILTER.md)
1. [审查差分对的信号完整性](TUTORIAL_DIFFERENTIAL_PAIR.md)
1. [使用脚本：LED 矩阵和拼板](TUTORIAL_SCRIPTS.md)

## 术语表与迁移指南

* [术语表](../../glossary/zh/GLOSSARY.md)
* [迁移指南：从 EAGLE 或 KiCad 转来](../../migration/zh/MIGRATION.md)

* [English](../README.md)
