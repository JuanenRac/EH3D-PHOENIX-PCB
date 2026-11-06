# 从原理图到网表再到仿真

`examples/tutorials/divider.json` 是一张原理图：一个 10 V 电源和串联的 1 kΩ、3 kΩ 两个电阻。你无需手写网表即可生成仿真网表，运行仿真，并用分压公式 10 V × 3 / (1 + 3) = 7.5 V 核对结果。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. 打印原理图的网表。不会写入任何内容（试运行），并且会先运行电气规则检查。第一行是标题，随后是各元件，数值采用仿真器的写法。

```console
# test
$ phoenix netlist examples/tutorials/divider.json
divider
~ R1 in out 1e3
~ R2 out 0 3e3
~ V1 in 0 1e1
```

2. 把它写入一个新文件。已存在的文件绝不会被覆盖。

```console
# test
$ phoenix netlist examples/tutorials/divider.json --out build/divider.cir
~ written:
```

3. 求工作点。分压公式给出节点 `out` 为 7.5 V。

```console
# test
$ phoenix sim build/divider.cir --op --probe "v(out)"
name,value
v(out),7.5e0
```

4. 无需文件：`--schematic` 直接由原理图生成网表并交给求解器。

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --op --probe "v(out)"
name,value
v(out),7.5e0
```

5. 把电源从 0 V 扫描到 10 V，步长 5 V：输出始终是输入的四分之三。

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --dc V1 0 10 5 --probe "v(out)"
~ 5e0,3.75e0
~ 1e1,7.5e0
```

## 另见

* [docs/SIM.md](../../SIM.md)
* [docs/NETLIST.md](../../NETLIST.md)
* [docs/ERC.md](../../ERC.md)
