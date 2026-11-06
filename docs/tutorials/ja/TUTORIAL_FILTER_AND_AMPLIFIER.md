# RC フィルタと反転増幅器をシミュレーションする

SPICE 形式のネットリストで書いた 2 つの回路を使います。1 kΩ と 1 µF の RC ローパス（`rc_step.cir` と `rc_ac.cir`）と、オペアンプによる利得 −R2/R1 の反転増幅器（`inv_amp.cir`）です。`--measure` で測定し、`--step` で値のリストを順に実行し、遮断周波数と極を求め、シード固定のモンテカルロ解析を行います。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. ステップ応答の 10 %〜90 % 立ち上がり時間を測定します。表には `run` 列と測定値の列があります。RC 回路では ln 9 · R · C で、約 2.197 ms です。

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 10u 10m --measure "tran rise10 rise v(out)"
run,rise10
~ nominal,
```

2. `--step` で抵抗の値ごとに回路を 1 回ずつ実行します。2 kΩ では立ち上がり時間が 2 倍になります。表には値ごとに 1 行あります。

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 20u 40m --step "R1 list 1k 2k" --measure "tran rise10 rise v(out)"
r1,run,rise10
```

3. 周波数を掃引し、応答の性能指標を求めます。ローパスの遮断周波数は 1 / (2π R C) で、約 159 Hz です。

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 100 1 100k --figures "v(out)"
name,value,unit
~ cutoff_3db,
```

4. AC 応答に極と零点をフィッティングします。この回路の極は実数で、−1 / (R C) = −1000 rad/s にあります。

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 20 10 100k --poles "v(out)"
~ pole,
```

5. 次に反転増幅器です。R1 = 1 kΩ、R2 = 10 kΩ のとき利得は −10 で、入力 1 V に対する出力は −9.999 V です。オペアンプモデルの利得が有限（1e5）のため、絶対値は 10 V よりわずかに小さくなります。

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --op --probe "v(out)"
name,value
~ v(out),-9.99
```

6. R2 の 3 つの値を順に実行し、DC 掃引の 1 点で出力を測定します。利得は −2、−10、−20 に近い値になります。

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --dc V1 1 1 1 --step "R2 list 2k 10k 20k" --measure "dc vout at v(out) 1"
r2,run,vout
```

7. モンテカルロ：抵抗の許容差が 1 %（ネットリストの `tol=1%`）の分圧回路です。同じシードなら、どのコンピュータでも同じ表になります。実行行の後に `count`、`mean`、`std`、`min`、`max` と百分位の行が続きます。

```console
# test
$ phoenix sim examples/tutorials/div_tol.cir --mc 200 --seed 5
run,vb
```

## 関連ドキュメント

* [docs/SIM.md](../../SIM.md)
* [docs/SIM_ADVANCED.md](../../SIM_ADVANCED.md)
* [docs/SIM_NONLINEAR.md](../../SIM_NONLINEAR.md)
* [docs/ORACLE.md](../../ORACLE.md)
