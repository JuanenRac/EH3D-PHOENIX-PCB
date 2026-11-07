# チェックの結果を表とページに書き出し、基板をメーカーの能力プロファイルで確認する

設計ルールを実行し、結果を CSV の表とカテゴリ別の HTML ページとして保存し、続いて小さな EAGLE 基板を基板メーカーの能力プロファイルで確認します。例のプロファイルは架空のもので、ファイルの形を示すだけで、どのメーカーの能力でもありません。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. ルールファイルのルールを実行し、2 つのレポートを書き出します。各オプションは新しいファイルを書き、既存のファイルは置き換えず、出力のどれかが拒否されたときは何も書きません（終了コード 2）。基板のパターンはファイルの要求より細いので、確定した指摘が 1 件あり、終了コードは 1 です。

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json --csv build/check.csv --html build/check.html examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. 同じレポートを別の言語で書き出します。CSV のタイトル列とページの見出しは `--lang` に従い、列・カテゴリ・重大度のキーは英語のままなので、スクリプトはどの言語でも読めます。

```console
$ phoenix check --lang es --rules examples/tutorials/rules_strict.json --csv build/check_es.csv --html build/check_es.html examples/tutorials/kicad/demo.kicad_pcb
```

3. 回路図と基板の比較にも同じ 2 つのオプションがあります。

```console
$ phoenix connectivity --csv build/connectivity.csv --html build/connectivity.html examples/tutorials/tut.sch examples/tutorials/tut.brd
```

4. EAGLE 基板をプロファイル `examples/guides/dfm_small_shop.cfg` で確認します。`--dfm` を使うとプロファイルの上限が基板のルールの代わりになるので、`--rules` は指定しません。基板は 30 mm × 20 mm で、プロファイルは短辺 10 mm までなので、コード `PHOENIX-DFM-003` のエラーになり、終了コードは 1 です。銅層が 1 層を超える基板には `PHOENIX-DFM-001` も付き、情報 `PHOENIX-DFM-006` はプロファイルが未検証であることを知らせます。

```console
$ phoenix check --dfm examples/guides/dfm_small_shop.cfg --csv build/dfm.csv --html build/dfm.html examples/tutorials/tut.brd
```

5. 次はリポジトリの例のプロファイル `config/dfm/example.cfg` です。基板はすべての上限の範囲内なので、能力カテゴリの指摘はプロファイルが未検証だという情報だけです。

```console
$ phoenix check --dfm config/dfm/example.cfg --html build/dfm_example.html examples/tutorials/tut.brd
```

## 結果の読み方

* CSV のヘッダーは `list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail` で、指摘ごとに 1 行あり、証明されたものが先頭です（`list` は `confirmed` か `inconclusive`）。表計算が数式として実行しそうなセルには先頭に引用符が付きます。
* HTML ページにはスクリプトも外部ファイルもなく、基板由来のテキストはすべてエスケープされるので、ブラウザで安全に開けます。先頭にカテゴリごとの証明済み・未確定の件数があり、指摘のあるカテゴリごとに表が続きます。プロファイルの指摘はカテゴリ `capability` にあります。
* プロファイルは `キー = 値` の行からなるテキストファイルで、単位はナノメートルです。行がない、または 0 の項目は無効です。`config/dfm/example.cfg` をコピーし、メーカーの資料で読み取った数値を入れ、どこで読んだかを `source` に書き、そのあとで初めて `verified = true` にします。ファイルをメーカーと照合する仕組みはありません。

## 関連ドキュメント

* [docs/DRC_ADVANCED.md](../../../../docs/DRC_ADVANCED.md)
* [docs/DFM.md](../../../../docs/DFM.md)
* [docs/RULES.md](../../../../docs/RULES.md)
