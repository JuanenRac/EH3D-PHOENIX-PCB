# 差動ペアのシグナルインテグリティを確認する

基板 `examples/tutorials/diffpair.kicad_pcb` には幅 0.25 mm の配線 `D_P` と `D_N` が 20 mm にわたって並び、縁の間隔は 0.75 mm です。ペアがどうあるべきかを記したファイルで検査し、間隔が誤っているときの指摘を確認し、スペイン語で読み、クロストークを見積ります。シグナルインテグリティのツールは閉じた式による準静的な見積りで、電磁界ソルバではありません。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. `pairs.json` は、ペアが `D_P` と `D_N` で、配線幅 0.25 mm、間隔 0.75 mm であることを示しています。基板は一致しており、エラーも警告もありません。

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs.json examples/tutorials/diffpair.kicad_pcb
~ pairs: 1, errors: 0, warnings: 0
~ pair p (D_P / D_N)
```

2. 今度はファイルが間隔 0.3 mm を要求しますが、基板は 0.75 mm です。間隔が許容外で、コードは `PHOENIX-PAIR-003`、終了コードは 1 です。

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ PHOENIX-PAIR-003
```

3. 同じレポートをスペイン語で表示します。`--lang` は en、es、de、fr、it、ja、zh、pt を受け付けます。

```console
# test
$ phoenix pairs --lang es --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ La separación del par está fuera de tolerancia
```

4. 並走する長さ、中心間距離、層構成から、2 本の配線間のクロストークを見積ります。見積りはアグレッサのステップに対する百分率です。

```console
# test
$ phoenix si crosstalk examples/tutorials/diffpair.kicad_pcb
~ crosstalk: 1 coupled pairs of nets
```

5. その他の解析：ネットの反射と終端、簡易アイダイアグラム、リターンパスを含むすべての解析（`config/si.cfg` のしきい値を超えると終了コード 1）。

```console
$ phoenix si reflect --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si eye --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si check examples/tutorials/diffpair.kicad_pcb
```

## 関連ドキュメント

* [docs/DIFFPAIRS.md](../../DIFFPAIRS.md)
* [docs/SI.md](../../SI.md)
* [docs/LENGTHS.md](../../LENGTHS.md)
* [docs/STACKUP.md](../../STACKUP.md)
