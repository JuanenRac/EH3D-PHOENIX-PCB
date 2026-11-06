# 用场求解器求线路阻抗，并估算走线的温升

`examples/guides/stackup_job.json` 描述了一个带一条线路的两层叠层。你先用二维场求解器求解截面并与封闭公式比较，再从电路板中取出线路，最后针对两种电流估算某个网络走线的温升。这两个工具都是准静态的原型，没有与其他求解器或实测比较过。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 求解作业文件中的线路。报告给出网格、迭代次数和残差，然后是阻抗、有效介电常数、电感、电容、延迟和介质损耗，旁边是封闭公式及以百分数表示的差值。这是试运行：不写任何文件。

```console
$ phoenix field2d examples/guides/stackup_job.json
```

2. 把网格加密 2 倍，看结果变化多少；变化很小就说明已收敛。未达到容差的解会给出 `PHOENIX-FIELD-004`，且不给数值。

```console
$ phoenix field2d examples/guides/stackup_job.json --refine 2
```

3. 从电路板中取出线路。网络 `MID` 的大部分铜所在的层和宽度（`Top` 层上 0.2 mm 的走线）决定一条微带线，并在作业文件的叠层上求解。

```console
$ phoenix field2d examples/tutorials/tut.brd --stackup examples/guides/stackup_job.json --net MID
```

4. 把场求解器加入 `stackup` 的报告，使封闭公式和场解出现在同一份报告中。

```console
$ phoenix stackup examples/guides/stackup_job.json --freq-ghz 1 --field2d
```

5. 估算网络 `MID` 通过 0.5 A 时的发热。报告按层、宽度和铜厚列出走线，并给出按 IPC-2221 拟合和按对流模型得到的温升。温升低于设置中的限值，退出码为 0。

```console
$ phoenix thermal --currents examples/guides/currents_low.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

6. 同一网络通过 3 A。温升超过限值，走线被标为 `[above the limit]` 并给出警告 `PHOENIX-HEAT-004`，退出码为 1。

```console
$ phoenix thermal --currents examples/guides/currents_high.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

## 如何阅读结果

* 求解器把线路当作准 TEM，在壁面电位为零的盒子里用矩形网格求解，所以阻抗比开放区域略低。导体是矩形，阻焊层和铜损耗被省略。与封闭公式的差值超过 `compare_warn_percent` 时给出警告 `PHOENIX-FIELD-009`。
* 温升估算让网络的全部电流流过每一段走线和每个过孔，所以把电流分流的网络按安全一侧来判断。IPC-2152 没有被应用，每份报告都会说明这一点（`PHOENIX-HEAT-012`）。叠层中没有列出的层采用设置中的默认值（`PHOENIX-HEAT-008`）。
* 两个命令都是试运行，只有在用 `--out` 指定一个新文件时才写 JSON 报告。常数和限值放在 `config/field2d.cfg` 和 `config/heating.cfg` 中，而不在代码里。退出码：一切正常为 0，有线路无法求解或有走线超过限值为 1，参数或输入无法使用为 2。

## 另见

* [docs/FIELD2D.md](../../../../docs/FIELD2D.md)
* [docs/THERMAL.md](../../../../docs/THERMAL.md)
* [docs/STACKUP.md](../../../../docs/STACKUP.md)
