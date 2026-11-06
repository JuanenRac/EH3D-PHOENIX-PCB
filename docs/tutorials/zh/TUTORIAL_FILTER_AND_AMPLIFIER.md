# 仿真 RC 滤波器和反相放大器

用 SPICE 风格网表写成的两个电路：1 kΩ、1 µF 的 RC 低通（`rc_step.cir` 和 `rc_ac.cir`），以及用运放构成、增益为 −R2/R1 的反相放大器（`inv_amp.cir`）。你将用 `--measure` 取测量值，用 `--step` 遍历一组数值，求截止频率和极点，并以固定种子做蒙特卡洛分析。

请在仓库根目录运行命令；参见[如何运行教程](README.md)。

1. 测量阶跃响应从 10 % 到 90 % 的上升时间。表中有 `run` 列和一个测量列。对 RC 电路，它等于 ln 9 · R · C，约 2.197 ms。

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 10u 10m --measure "tran rise10 rise v(out)"
run,rise10
~ nominal,
```

2. 用 `--step` 对电阻的每个取值各运行一次电路。阻值为 2 kΩ 时上升时间加倍。表中每个取值占一行。

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 20u 40m --step "R1 list 1k 2k" --measure "tran rise10 rise v(out)"
r1,run,rise10
```

3. 扫描频率并求取响应的性能指标。低通的截止频率为 1 / (2π R C)，约 159 Hz。

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 100 1 100k --figures "v(out)"
name,value,unit
~ cutoff_3db,
```

4. 把极点和零点拟合到交流响应上。该电路的极点是实数，位于 −1 / (R C) = −1000 rad/s。

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 20 10 100k --poles "v(out)"
~ pole,
```

5. 接着是反相放大器。R1 = 1 kΩ、R2 = 10 kΩ 时增益为 −10；输入 1 V 得到 −9.999 V，绝对值略小于 10 V，因为运放模型的增益有限（1e5）。

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --op --probe "v(out)"
name,value
~ v(out),-9.99
```

6. 让 R2 取三个值，并在直流扫描的唯一一个点上测量输出。增益接近 −2、−10 和 −20。

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --dc V1 1 1 1 --step "R2 list 2k 10k 20k" --measure "dc vout at v(out) 1"
r2,run,vout
```

7. 蒙特卡洛：电阻容差为 1 % 的分压电路（网表中的 `tol=1%`）。相同的种子在任何计算机上得到相同的表。各次运行之后是 `count`、`mean`、`std`、`min`、`max` 和百分位数各行。

```console
# test
$ phoenix sim examples/tutorials/div_tol.cir --mc 200 --seed 5
run,vb
```

## 另见

* [docs/SIM.md](../../SIM.md)
* [docs/SIM_ADVANCED.md](../../SIM_ADVANCED.md)
* [docs/SIM_NONLINEAR.md](../../SIM_NONLINEAR.md)
* [docs/ORACLE.md](../../ORACLE.md)
