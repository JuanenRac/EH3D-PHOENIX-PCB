# 電界ソルバーで線路のインピーダンスを求め、パターンの温度上昇を見積もる

`examples/guides/stackup_job.json` は 1 本の線路を持つ 2 層のスタックアップを記述しています。2D 電界ソルバーで断面を解いて閉じた式と比較し、次に基板から線路を取り出し、最後に 2 つの電流でネットのパターンの温度上昇を見積もります。どちらも準静的なプロトタイプで、他のソルバーや実測との比較はしていません。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. ジョブの線路を解きます。レポートにはメッシュ、反復回数、残差に続いて、インピーダンス、実効比誘電率、インダクタンス、容量、遅延、誘電体損失が出て、その横に閉じた式とパーセントでの差が並びます。ドライランなので何も書き込みません。

```console
$ phoenix field2d examples/guides/stackup_job.json
```

2. メッシュを 2 倍に細かくして、答えがどれだけ動くかを見ます。あまり変わらなければ収束しています。許容値に達しない解は `PHOENIX-FIELD-004` になり、数値は出ません。

```console
$ phoenix field2d examples/guides/stackup_job.json --refine 2
```

3. 基板から線路を取り出します。ネット `MID` の銅の大部分が通る層と幅（`Top` 上の 0.2 mm のパターン）からマイクロストリップが決まり、ジョブのスタックアップ上で解かれます。

```console
$ phoenix field2d examples/tutorials/tut.brd --stackup examples/guides/stackup_job.json --net MID
```

4. `stackup` のレポートに電界ソルバーを加え、閉じた式と電界解を 1 つのレポートに並べます。

```console
$ phoenix stackup examples/guides/stackup_job.json --freq-ghz 1 --field2d
```

5. ネット `MID` に 0.5 A を流したときの発熱を見積もります。レポートはパターンを層・幅・銅厚ごとに列挙し、IPC-2221 の近似式による上昇温度（幅 0.2 mm、厚さ 35 µm のパターンで約 4 K）を示します。対流モデルは数値を出しません。設定の冷却条件では、これほど細いパターンは 125 C を超えるためです（`PHOENIX-HEAT-005`、情報）。上昇は設定の上限より低く、終了コードは 0 です。

```console
$ phoenix thermal --currents examples/guides/currents_low.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

6. 同じネットに 3 A を流します。上昇は上限を超え、パターンには `[above the limit]` と警告 `PHOENIX-HEAT-004` が付き、終了コードは 1 です。

```console
$ phoenix thermal --currents examples/guides/currents_high.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

## 結果の読み方

* ソルバーは線路を準 TEM とみなし、壁の電位が 0 の箱の中の矩形メッシュで解くので、開放空間よりインピーダンスがやや低くなります。導体は矩形で、ソルダーマスクと銅損は含みません。閉じた式との差が `compare_warn_percent` を超えると警告 `PHOENIX-FIELD-009` になります。
* 発熱の見積もりは、ネットの全電流をすべてのパターンとビアに流すものとして扱うので、電流が分流するネットは安全側で判定されます。IPC-2152 は適用されず、どのレポートもそれを明記します（`PHOENIX-HEAT-012`）。スタックアップに名前のない層には設定の既定値が使われます（`PHOENIX-HEAT-008`）。
* どちらのコマンドもドライランで、JSON レポートは `--out` で指定した新しいファイルにだけ書きます。定数と上限は `config/field2d.cfg` と `config/heating.cfg` にあり、コードの中にはありません。終了コードは、正常が 0、線路を解けなかった場合かパターンが上限を超えた場合が 1、引数や入力が使えない場合が 2 です。

## 関連ドキュメント

* [docs/FIELD2D.md](../../../../docs/FIELD2D.md)
* [docs/THERMAL.md](../../../../docs/THERMAL.md)
* [docs/STACKUP.md](../../../../docs/STACKUP.md)
