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

## 関連ドキュメント

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
