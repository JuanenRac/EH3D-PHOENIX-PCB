# 制限ファイルでチェックの作業量を抑える

チェックの中には、数の二乗に比例して時間がかかるものがあります。たとえば、多角形の辺が交差しないことのテストです。制限はその時間を抑えます。制限を超えた多角形は交差のテストを行わず、レポートは情報としてそれを伝えます。基板のチェックを既定値で実行し、次に大幅に低い制限ファイルで実行して、そのメモを確認します。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md)を参照してください。

1. KiCad の基板に対して、既定の制限でルールファイルを実行します。基板の配線はファイルが求める幅より細いので、確定した指摘が 1 件あり、終了コードは 1 です。

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. `max_polygon_points = 3` を含む `--limits examples/guides/limits_small.cfg` を付けてもう一度実行します。基板の外形は 4 点なので、コード `PHOENIX-BOARD-017` の情報として列挙されます。情報は終了コードを変えないので、終了コードは 1 のままです。

```console
# test
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ PHOENIX-BOARD-017
~ max_polygon_points = 3
~ PHOENIX-CHECK-001
```

3. 同じメモを別の言語で、またスクリプト向けに JSON で出します。コードは英語のままで、タイトルは `--lang` に従います。

```console
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --lang pt examples/tutorials/kicad/demo.kicad_pcb
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --json examples/tutorials/kicad/demo.kicad_pcb
```

## 結果の読み方

* `--limits ファイル` はすべてのコマンドが受け付け、コマンド名の前でも後でも使えます。ファイルには `config/board.cfg` と `config/eagle_board.cfg` の `キー = 値` の行を任意の順で書きます。未知の行や範囲外の値は、行番号と終了コード 2 で拒否されます。 `config/outline_arcs.cfg` の 2 つのキー（外形の円弧を弦に分割する方法を決める `tolerance_nm` と `max_pieces`）も同じファイルに書けます。
* 制限が変えるのは、制限した部分だけです。既定値ではどの出力も変わらず、`max_polygon_points` を超えた多角形でも、重複した点のチェックは行われます。

## 関連項目

* [config/board.cfg](../../../../config/board.cfg)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
* [docs/RULES.md](../../../../docs/RULES.md)
