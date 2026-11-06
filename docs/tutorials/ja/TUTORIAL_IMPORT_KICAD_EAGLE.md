# KiCad／EAGLE プロジェクトを取り込んで確認する

KiCad プロジェクト全体をこのプログラムの設計モデルに読み込み、保持された内容と除外された内容を確認し、モデルを新しいファイルとして書き出し、ルールファイルで基板を検査します。続いて EAGLE の基板でも同じ確認を行います。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. プロジェクト（`.kicad_pro` と同名の `.kicad_sch`・`.kicad_pcb`）を読み込みます。ドライランで、読み込んだもの、保持した量、モデルが持たないものがレポートに出ます。

```console
# test
$ phoenix kicad-import examples/tutorials/kicad/demo.kicad_pro
~ usable: yes
~ components: 2
~ tracks: 1
~ vias: 1
~ dry run: nothing was written
```

2. 指定した名前でモデルを新しいファイルとして書き出します。既存のファイルや入力ファイルは決して上書きされません。

```console
# test
$ phoenix kicad-import --out build/demo.json examples/tutorials/kicad/demo.kicad_pro
~ written:
```

3. 取り込んだ基板を検査します。KiCad の基板は設計ルールをプログラムに渡さないため、制限はルールファイルから得ます。ファイルは 0.3 mm 以上の配線を要求し、基板の配線は 0.25 mm なので、確定した指摘が 1 件あり、終了コードは 1 です。

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
Confirmed: 1
~ PHOENIX-CHECK-001
```

4. EAGLE ファイルについて、書き直しで何が保たれるかを確認します。XML ツリーを書き出して読み直しても変化しないことを示し、設計モデルが簡略化または除外する要素を一覧にします。

```console
# test
$ phoenix eagle-check examples/tutorials/tut.brd
board (version 9.6.2)
~ rewrite of the tree: identical
```

5. 同じルールファイルを EAGLE の基板に適用します。ネット `MID` の配線幅は 0.2 mm です。

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/tut.brd
# exit 1
Confirmed: 1
```

6. EAGLE の回路図と基板を突き合わせます。不足しているネット、未接続のピン、まだ配線されていない銅箔がレポートに出ます。

```console
$ phoenix connectivity examples/tutorials/tut.sch examples/tutorials/tut.brd
```

## 関連ドキュメント

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD.md](../../KICAD.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/RULES.md](../../RULES.md)
