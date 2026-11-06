# 阻抗匹配与滤波器综合

射频工具基于教科书公式和理想元件。你将设计在 100 MHz 把 100 Ω 匹配到 50 Ω 的 L 形网络、四分之一波长变换器，以及由原型表得到的梯形滤波器。结果只是原型，没有与网络分析仪或场求解器比较过。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. 在 0.1 GHz 把 100 Ω 负载匹配到 50 Ω 信号源。低通形式的 L 网络是 79.5775 nH 串联电感加 15.9155 pF 并联电容，高通形式则是其对偶。

```console
# test
$ phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
~ 79.5775 nH
~ 15.9155 pF
~ low-pass
~ high-pass
```

2. 50 Ω 与 200 Ω 负载之间的四分之一波长变换器，阻抗为 √(50 × 200) = 100 Ω。

```console
# test
$ phoenix rf match qw --load 200 --z0 50 --freq-ghz 1
~ transformer: 100.0000 ohm
```

3. 综合一个低通滤波器：巴特沃思、3 阶、截止 0.1 GHz，扫描三个点。原型值为 1、2、1，因此滤波器有 31.8310 pF 的电容，中间是 159.1549 nH 的电感。

```console
# test
$ phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.2,3
~ prototype g: 1.0000 2.0000 1.0000
~ capacitor 31.8310 pF
~ inductor 159.1549 nH
~ frequency, insertion loss, return loss
```

4. 带通滤波器：切比雪夫、5 阶、0.1 dB 纹波，90 MHz 至 110 MHz，75 Ω 系统。

```console
# test
$ phoenix rf filter chebyshev 5 bandpass --f1-ghz 0.09 --f2-ghz 0.11 --ripple-db 0.1 --z0 75
~ Chebyshev (0.1 dB ripple) filter of order 5, band-pass
```

5. 根据叠层和频率在电路板上绘制匹配网络，作为可撤销的操作并附报告。结构列表（线路、弯角、短截线、滤波器、耦合器、天线）用 `--list` 查看。

```console
$ phoenix rfgen --list
```

## 另见

* [docs/RF.md](../../RF.md)
* [docs/RFGEN.md](../../RFGEN.md)
* [docs/STACKUP.md](../../STACKUP.md)
* [docs/ORACLE.md](../../ORACLE.md)
