# 用内置自动布线器为电路板布线：差分对和复用板上已有的过孔

`phoenix autoroute native` 用自带的搜索算法为电路板上未连接的部分布线，不需要任何外部程序。`examples/guides/open.brd` 有一个两个焊盘未相连的网络；`examples/guides/open_pair.brd` 有一对差分网络 `D_P` 和 `D_N`，同样未连接。方案只会添加走线和过孔，并在前后用规则检查。这里给出规则文件，因为示例中的 EAGLE 电路板自身没有设计规则。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 规划布线。这是试运行：报告列出已布通的连接、新增的走线、过孔和长度，以及仍未连通的连接，并在错误流中说明没有写入任何文件。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json examples/guides/open.brd
```

2. 把带有布线的电路板写入新文件。无论文件名是什么，它都是设计模型的 JSON，已存在的文件不会被替换。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --out build/open_routed.json examples/guides/open.brd
```

3. 对写出的电路板再布一次线：已经没有未连通的连接，所以没有需要添加的内容。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json build/open_routed.json
```

4. 指定差分对。`pair_dp_dn.json` 声明 `D_P` 和 `D_N` 是一对，线宽 0.2 mm，间距 0.3 mm。有空间时，两条走线并排敷设在同一层，报告显示 `coupled pair: p`；否则两个网络按普通网络布线，并有一条代码为 `PHOENIX-AROUTE-017` 的说明给出原因。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json examples/guides/open_pair.brd
```

5. 只给这一对的两个网络布线，并拒绝仍留有未连通连接的方案：加上 `--require-complete` 后，若还有连接未布通，命令以退出码 2 和代码 `PHOENIX-AROUTE-016` 结束。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json --nets D_P,D_N --require-complete examples/guides/open_pair.brd
```

## 如何阅读结果

* 板上已有的本网络过孔如果恰好在网格节点上，换层就是免费的：走线在那里换层，不再放新的过孔。在 `config/autoroute_native.cfg` 中把 `via_snap_nm` 设为大于 0 时，离节点不超过该距离的过孔也会被使用，并给出说明 `PHOENIX-AROUTE-019`。这就是过孔的复用；方案绝不会改动原有的走线、过孔或焊盘。
* 只有当每个网络恰好有一条未连通的连接，并且四个焊盘位于同一层时，差分对才会被耦合布线。两条走线等长，并保持差分对的间距。没有 `--pairs` 或 `--pair-config` 时，一对网络按两个普通网络布线，之后由电路板编辑器的差分对检查来评判结果。
* 这是一个原型，只在手工制作的小电路板和测试上试过。搜索、其代价和限制（按节点数计，从不按时间）都在 `config/autoroute_native.cfg` 中。如果某个网络的所有走线合起来会新增规则错误，该网络会被排除并以 `PHOENIX-AROUTE-015` 报告：方案绝不会增加电路板原本没有的错误。

## 另见

* [docs/AUTOROUTE_NATIVE.md](../../../../docs/AUTOROUTE_NATIVE.md)
* [docs/DIFFPAIRS.md](../../../../docs/DIFFPAIRS.md)
* [docs/AUTOROUTE.md](../../../../docs/AUTOROUTE.md)
