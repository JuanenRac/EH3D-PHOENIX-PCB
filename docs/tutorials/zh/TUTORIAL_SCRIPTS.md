# 使用脚本：LED 矩阵和拼板

语言 `phoenix-script/2` 的脚本有变量、带上限的循环、对电路板的查询，以及合为一个可撤销步骤的编辑，在沙箱中运行：无网络、无程序、不能访问设置的数据文件夹之外的文件。除非 `--out` 指定新文件，否则运行就是试运行。示例在 `examples/scripts/`。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. 列出程序自带的示例脚本，并打印其中一个。

```console
# test
$ phoenix script list
~ led_matrix
~ panelize_matrix
$ phoenix script list --show led_matrix
phoenix-script/2 board
```

2. 不运行脚本而检查它：语法、名称、参数个数、每个函数作用的文档，以及步骤数估计。

```console
# test
$ phoenix script check examples/scripts/led_matrix.phs
```

3. 在封装为 `LED0603` 的电路板上运行 LED 矩阵：间距 2.5 mm 的六个 LED，每个的第 2 个焊盘接到同一个网络。电路板不会改变：这是试运行。

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --board examples/tutorials/led_board.brd
~ placed 6 LEDs on net LEDCOM
~ pads on the net: 6
~ dry run: nothing was written
```

4. 拼板脚本把电路板上的两个元件复制成 2 × 2 的矩阵：元件数从 2 变成 8。

```console
# test
$ phoenix script run examples/scripts/panelize_matrix.phs --board examples/tutorials/tut.brd
~ copies: 4
~ components: 2 -> 8
```

5. 加上 `--out` 后，结果会一次性写入新文件。已存在的文件绝不会被替换。

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
~ written
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
# exit 2
~~ never replaced
```

## 另见

* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/SCRIPT_API.md](../../SCRIPT_API.md)
* [docs/SCRIPT_EXAMPLES.md](../../SCRIPT_EXAMPLES.md)
