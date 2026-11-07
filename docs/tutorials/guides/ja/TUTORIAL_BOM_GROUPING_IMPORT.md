# 部品表をまとめて書き出し、別のプログラムの部品表を取り込む

基板 `examples/guides/bom_four.brd` には 10 k の抵抗が 3 個、4k7 が 1 個あります。部品のまとめ方と CSV の列を設定ファイルで選び、続いて別のプログラムが書いた部品表を、その列の対応表で読み込みます。ネットワークからは何も取得せず、設定の料金表は見積りの形を示す例にすぎません。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. 既定のまとめ方（値、フットプリント、メーカー、型番）と `bom_parts.csv` のメーカー情報で部品表を出します。R1 と R2 はメーカーと型番が同じで R3 は別のメーカーなので、行は 3 つです。価格ファイルがないため、各行に `PHOENIX-BOM-005` が付きます。

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

2. 同じ基板で `bom_costs_joined.cfg` の `group_by = value,footprint` を使うと、メーカーと型番がまとめ方から外れ、R1・R2・R3 はどちらも持たない数量 3 の 1 行になります。

```console
$ phoenix bom --config examples/guides/bom_costs_joined.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

3. CSV ファイルを書き出します。列は設定の `csv_columns` の順（ここでは `references,quantity,value,footprint,manufacturer,mpn,status`）で、HTML ページと JSON ファイルはすべての列を保ちます。既存のファイルは置き換えません。

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv --csv build/bom.csv examples/guides/bom_four.brd
```

4. 別のプログラムの部品表を取り込みます。`bom_other_program.csv` はセミコロン区切りで独自の列名を使い、`bom_import_map.cfg` がその読み方を指定します。レポートは部品表と基板を比較します。R5 は部品表にあって基板になく（`PHOENIX-BOM-021`）、R4 は実装しない印が付いているので部品表から外れ、情報として一覧されます（`PHOENIX-BOM-009`）。`--parts-out` は部品表にあるメーカーと型番をパーツデータのファイルとして保存します。

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --import examples/guides/bom_other_program.csv --import-map examples/guides/bom_import_map.cfg --parts-out build/parts_from_bill.csv examples/guides/bom_four.brd
```

5. 以降の実行では、元の部品表の代わりに保存したパーツデータのファイルを使います。

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts build/parts_from_bill.csv examples/guides/bom_four.brd
```

## 結果の読み方

* 既定の設定ファイルは `config/costs.cfg`、既定の対応表は `config/bom_import.cfg` です（ファイルがある場合）。対応表は区切り文字（カンマ、セミコロン、タブ）と、各列がファイル内でとりうる名前を指定し、必須なのは参照だけです。付属の名前は推測であり、特定のプログラムの形式ではありません。ファイルをコピーして、自分のファイルの列名を書いてください。
* コード `PHOENIX-BOM-019`、`PHOENIX-BOM-020`、`PHOENIX-BOM-021` は警告で、何も変えません。行の参照数と一致しない数量、2 行にある参照（最初のものを使用）、片方にしかない部品や値が異なる部品が対象です。`R1-R5` のような範囲は個々の部品に展開されます。形式が正しくない範囲、逆順の範囲、`max_range` を超える範囲はインポートを止めます（`PHOENIX-BOM-022` から `PHOENIX-BOM-024`）。`--import-format` では EAGLE のパートリストや KiCad の部品表も読み込めます（`docs/BOM.md`）。`--parts` の内容は取り込んだ部品表より優先されます。
* `--csv`、`--json-file`、`--html` を付けなければ、コマンドは何も書き込みません。終了コードは、正常終了が 0、部品が基板にないフットプリントを指しているときが 1、引数や入力が使えないときが 2 です。

## 関連ドキュメント

* [docs/BOM.md](../../../../docs/BOM.md)
* [docs/PARTS.md](../../../../docs/PARTS.md)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
