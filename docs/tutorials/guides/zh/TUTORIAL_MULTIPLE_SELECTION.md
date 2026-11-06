# 在电路板编辑器中选择多个对象，并作为一步来移动、对齐或删除

电路板编辑器的选择是由元件、走线、过孔、区域和丝印项组成的有序集合。没有屏幕时，用 `phoenix boardedit` 和一个操作文件来操作：`select` 改变选择，`move_selection`、`delete_selection` 和 `align_selection` 对选择起作用，作为历史记录的一步，并有精确的逆操作。`examples/guides/select_ops.json` 用矩形选中完全位于其内的对象（这里是电阻 R1，不含走线），并把它向上移动 2 mm。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 以试运行方式应用这些操作。报告列出三个操作以及前后的规则错误数；不写任何文件。这里给出规则文件，因为这块 EAGLE 电路板本身没有设计规则。

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json examples/tutorials/tut.brd
```

2. 把编辑后的电路板写入新文件（设计模型用 `.json`，EAGLE 用 `.brd`）。输入文件绝不会被覆盖。若规则在结果上没有证明任何错误，退出码为 0，否则为 1。

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --out build/moved.json examples/tutorials/tut.brd
```

3. 以 JSON 格式输出报告，脚本可以读取：键 `rule_errors` 是规则在结果上证明的错误数。

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --json examples/tutorials/tut.brd
```

## 如何阅读结果

* `select` 的步骤有：`add`、`remove` 和 `toggle`（`items` 中的对象，写成 `{"kind": "component", "id": "..."}`）、`net`（某个网络的所有对象，按其 id）、`rect`（`from` 与 `to` 之间的矩形，坐标为以纳米计的 `x_nm` 和 `y_nm`；`mode` 为 `contained`，或同时包含矩形触及对象的 `crossed`）、`all`、`invert` 和 `clear`。这些步骤按顺序、要么全部成功要么全部不生效地应用，选择不属于历史记录。
* `move_selection` 按 `dx_nm` 和 `dy_nm` 移动（向上为正），除非 `snap` 为 false，否则取整到网格；未被选中的走线不会跟随元件，所以要带着铜一起移动就要先选中网络。`delete_selection` 删除所选对象。`align_selection` 支持 `left`、`right`、`top`、`bottom`、`center_x`、`center_y`（至少两个对象）以及 `distribute_x`、`distribute_y`（至少三个）。被锁定的元件会使整个操作被拒绝（`PHOENIX-BEDIT-006`），空选择是 `PHOENIX-BEDIT-009`。
* 在窗口中使用选择工具：Shift 或 Ctrl+点击切换一个对象；在空白处按 Shift 或 Ctrl 拖动，加入完全位于矩形内的对象（包括区域和丝印项）；Alt+拖动还会加入矩形触及的对象；`N` 选中指针下的网络；Ctrl+A 全选；Esc 取消选择；方向键把选择移动一格网格（Shift：十格）；Delete 删除；Alt+左、右、上、下以及 Alt+C、Alt+M 进行对齐。每一项都是历史记录的一步，Ctrl+Z 会还原成相同的字节。该窗口从未在屏幕上看过，也没有用鼠标试过。

## 另见

* [docs/BOARD_EDIT.md](../../../../docs/BOARD_EDIT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
* [docs/VIEWER.md](../../../../docs/VIEWER.md)
