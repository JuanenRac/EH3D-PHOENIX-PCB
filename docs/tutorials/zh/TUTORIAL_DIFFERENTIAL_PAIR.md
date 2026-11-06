# 审查差分对的信号完整性

电路板 `examples/tutorials/diffpair.kicad_pcb` 上有两条 0.25 mm 的走线 `D_P` 和 `D_N`，并排 20 mm，边缘间距 0.75 mm。你将用一个说明差分对应有何要求的文件来检查它，查看间距不对时的结果，用西班牙语阅读该结果，并估算串扰。信号完整性工具是基于闭式公式的准静态估算，不是场求解器。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. `pairs.json` 指明差分对是 `D_P` 和 `D_N`，走线 0.25 mm，间距 0.75 mm。电路板与之相符：没有错误，也没有警告。

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs.json examples/tutorials/diffpair.kicad_pcb
~ pairs: 1, errors: 0, warnings: 0
~ pair p (D_P / D_N)
```

2. 现在文件要求间距 0.3 mm，而电路板上是 0.75 mm：间距超出容差，代码为 `PHOENIX-PAIR-003`，退出码为 1。

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ PHOENIX-PAIR-003
```

3. 同一份报告的西班牙语版本。`--lang` 接受 en、es、de、fr、it、ja、zh 和 pt。

```console
# test
$ phoenix pairs --lang es --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ La separación del par está fuera de tolerancia
```

4. 根据并行长度、中心距和叠层估算两条走线之间的串扰。估算值以攻击线阶跃的百分比表示。

```console
# test
$ phoenix si crosstalk examples/tutorials/diffpair.kicad_pcb
~ crosstalk: 1 coupled pairs of nets
```

5. 其他分析：网络的反射与端接、简化的眼图，以及包含回流路径的全部分析（超过 `config/si.cfg` 中的阈值时退出码为 1）。

```console
$ phoenix si reflect --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si eye --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si check examples/tutorials/diffpair.kicad_pcb
```

## 另见

* [docs/DIFFPAIRS.md](../../DIFFPAIRS.md)
* [docs/SI.md](../../SI.md)
* [docs/LENGTHS.md](../../LENGTHS.md)
* [docs/STACKUP.md](../../STACKUP.md)
