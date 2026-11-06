# インピーダンス整合とフィルタ合成

RF ツールは教科書の式と理想部品で計算します。100 MHz で 100 Ω を 50 Ω に整合する L 形回路、1/4 波長トランスフォーマ、プロトタイプ表からのラダーフィルタを設計します。結果はプロトタイプで、ネットワークアナライザや電磁界ソルバとは比較していません。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. 100 Ω の負荷を 0.1 GHz で 50 Ω の信号源に整合します。ローパス形の L 形回路は直列 79.5775 nH のインダクタと並列 15.9155 pF のコンデンサで、ハイパス形はその双対です。

```console
# test
$ phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
~ 79.5775 nH
~ 15.9155 pF
~ low-pass
~ high-pass
```

2. 50 Ω と 200 Ω の負荷の間の 1/4 波長トランスフォーマのインピーダンスは √(50 × 200) = 100 Ω です。

```console
# test
$ phoenix rf match qw --load 200 --z0 50 --freq-ghz 1
~ transformer: 100.0000 ohm
```

3. ローパスフィルタを合成します。バターワース、3 次、遮断 0.1 GHz、3 点の掃引です。プロトタイプ値は 1、2、1 なので、31.8310 pF のコンデンサと、その間の 159.1549 nH のインダクタになります。

```console
# test
$ phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.2,3
~ prototype g: 1.0000 2.0000 1.0000
~ capacitor 31.8310 pF
~ inductor 159.1549 nH
~ frequency, insertion loss, return loss
```

4. バンドパスフィルタ：チェビシェフ、5 次、リップル 0.1 dB、90 MHz〜110 MHz、75 Ω 系です。

```console
# test
$ phoenix rf filter chebyshev 5 bandpass --f1-ghz 0.09 --f2-ghz 0.11 --ripple-db 0.1 --z0 75
~ Chebyshev (0.1 dB ripple) filter of order 5, band-pass
```

5. 層構成と周波数から整合回路を基板上に描きます。元に戻せる操作として適用され、レポートが付きます。構造（線路、ベンド、スタブ、フィルタ、結合器、アンテナ）の一覧は `--list` で表示します。

```console
$ phoenix rfgen --list
```

## 関連ドキュメント

* [docs/RF.md](../../RF.md)
* [docs/RFGEN.md](../../RFGEN.md)
* [docs/STACKUP.md](../../STACKUP.md)
* [docs/ORACLE.md](../../ORACLE.md)
