# 编辑库并生成 IPC 封装

检查一个 EAGLE 库，用操作文件向其中添加封装（要么全部成功要么不改动，并保留原文件副本），再按 IPC-7351B 方法生成带规范名称的封装。生成封装的尺寸只是原型，未与数据手册核对。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. 检查库。不加 `--out` 不会写入任何内容；报告会统计封装、符号和器件集的数量并列出结果。

```console
# test
$ phoenix libedit examples/tutorials/parts.lbr
~ 1 footprints, 1 symbols, 1 device sets
~ 0 errors, 0 warnings
~ dry run: nothing was written
```

2. 应用一个操作文件，它创建封装 `HDR1X2` 并为其添加两个焊盘。新库写入新文件，读回后比对；原文件的副本保存在旁边。

```console
# test
$ phoenix libedit --apply examples/tutorials/lib_ops.json --out build/parts_hdr.lbr examples/tutorials/parts.lbr
~ operation 3: add pad to HDR1X2
~ the file was read back and is the same library
```

3. 按密度等级 B（标称）生成 0603 贴片电阻的封装。规范名称为 `RESC1608X55N`：本体 1.6 mm × 0.8 mm，高 0.55 mm。不加 `--out` 时是列出焊盘的试运行。

```console
# test
$ phoenix ipc generate chip 0603 --density B
RESC1608X55N (density B,
~ dry run: nothing was written
```

4. 写入新的 `.lbr` 文件（若文件名为 `.kicad_mod`，则改为写出 KiCad 封装）。

```console
# test
$ phoenix ipc generate chip 0603 --density B --out build/RESC1608X55N.lbr
~ written: build/RESC1608X55N.lbr
```

5. 更简单的生成器 `footprint` 根据参数生成封装（系列：贴片、SOIC、TSSOP、QFN、QFP、DIP 和排针）。

```console
# test
$ phoenix footprint chip --param size=0603 --out build/chip.lbr
~ footprint CHIP_0603
~ written:
```

6. 检查你生成的库：每个库都有一个封装。

```console
# test
$ phoenix libedit build/chip.lbr
~ 1 footprints
$ phoenix libedit build/RESC1608X55N.lbr
~ 1 footprints
```

7. 把自己库中的封装与生成的封装逐焊盘比较（焊盘不同时退出码为 1），并列出库中的系列。

```console
$ phoenix ipc compare build/chip.lbr chip 0603
$ phoenix ipc list
```

## 另见

* [docs/LIBRARY_EDIT.md](../../LIBRARY_EDIT.md)
* [docs/IPC_LIBRARY.md](../../IPC_LIBRARY.md)
* [docs/LIBINDEX.md](../../LIBINDEX.md)
* [docs/SANITIZER.md](../../SANITIZER.md)
