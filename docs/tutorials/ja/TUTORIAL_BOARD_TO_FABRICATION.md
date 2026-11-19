# 基板から製造ファイルまで

`examples/tutorials/kicad/` の KiCad プロジェクトは、30 mm × 20 mm の基板に 2 本の抵抗があり、ネット `SIG` で接続されています。回路図と基板を突き合わせ、設計ルールを実行し、チェックサム一覧付きで製造ファイルを書き出します。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. 回路図の電気ルールチェックを実行します。指摘にはそれぞれコードが付き、確定したものと判定不能のものは分けて表示されます。

```console
$ phoenix erc examples/tutorials/kicad/demo.kicad_sch
```

2. 部品の同一性で回路図と基板を突き合わせます。`sync` はドライランで、反映で変わる内容を一覧にするだけです。`--apply --out FILE` を付けない限り何も書き込みません。

```console
$ phoenix sync examples/tutorials/kicad/demo.kicad_sch examples/tutorials/kicad/demo.kicad_pcb
```

3. ルールファイルの制限で設計ルールを実行します。配線幅は 0.25 mm、ファイルの要求は 0.2 mm 以上なので問題はなく、終了コードは 0 です。

```console
# test
$ phoenix check --rules examples/tutorials/rules_relaxed.json examples/tutorials/kicad/demo.kicad_pcb
Confirmed: 0
Inconclusive: 0
```

4. 製造ファイルを新しいフォルダに書き出します。層ごとの Gerber X2、ドリルファイル、実装位置ファイル、部品表、各ファイルの SHA-256 を記した `manifest.json` です。書き出す前にファイルを読み戻して基板と照合します。

```console
# test
$ phoenix export --out build/fab examples/tutorials/kicad/demo.kicad_pcb
~ files written: build/fab
```

5. ステンシルと各面の実装位置を含むパッケージ全体を、決定的な zip 1 つにまとめます。同じ基板なら同じバイト列になります。`--out` なしではファイル一覧を出すだけのドライランです。

```console
$ phoenix fabpack --out build/demo_pack.zip examples/tutorials/kicad/demo.kicad_pcb
```

6. 送る前に基板を数値で確認します。寸法、各種の個数、各層の銅、ドリル、密度です。何も書き込まれません。

```console
# test
$ phoenix stats examples/tutorials/kicad/demo.kicad_pcb
Board statistics:
Board: 30.000000 x 20.000000 mm; 600.000000 mm2; Cutouts: 0.000000 mm2
Components: 2 (top: 2; bottom: 0)
```

7. 基板エディタの操作でパッドをテストポイントにし（`examples/tutorials/testpoint_ops.json` は R1 の 2 番パッド、ネット `MID` を印付けします）、テストポイントとどのプローブも届かないネットのレポートを出します。入力ファイルは上書きされません。

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/testpoint_ops.json --out build/with_test_point.json examples/tutorials/tut.brd
~ written: build/with_test_point.json
$ phoenix testpoints build/with_test_point.json
Test points of the board:
Test points: 1 (probe side: both)
Nets asked: 1; covered: 1; coverage: 100.00 %
```

8. ドキュメント層のある基板に寸法を 2 つ追加し（`examples/tutorials/dimension_ops.json`：基板の幅と高さ、寸法線は外側 3 mm）、DXF 図面に書き出します。ウィンドウでは寸法ツールで同じことができ、*寸法を編集* でその点を動かせます。`--out` に新しいファイルを指定するまではドライランです。

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/dimension_ops.json --out build/with_dimensions.json examples/tutorials/notes_board.json
~ written: build/with_dimensions.json
$ phoenix dxf-export board --dimensions build/with_dimensions.json
entities: 21
layer DIMENSIONS: 16
```

9. 仕入先アシスタントに部品表をどこで買うか尋ねます。価格ファイル（`examples/tutorials/prices.csv`、仕入先と数量区分ごとに 1 行）と、設定ファイル（`examples/tutorials/sourcing.cfg`）の仕入先、送料、最低注文額を読み、各仕入先での金額と分割購入の金額を出します。価格はファイルのものです。ネットワークからは何も取得しません。

```console
# test
$ phoenix bom --sourcing --config examples/tutorials/sourcing.cfg --prices examples/tutorials/prices.csv examples/tutorials/kicad/demo.kicad_pcb
~ suppliers: 2 considered, 1 lines, EUR
~ cheapest single supplier: Shop A for 2.210000
~ split purchase: 1 orders for 2.210000
```

## 関連ドキュメント

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
* [docs/STATS.md](../../STATS.md)
* [docs/TESTPOINTS.md](../../TESTPOINTS.md)
* [docs/DIMENSIONS.md](../../DIMENSIONS.md)
* [docs/BOM.md](../../BOM.md)
