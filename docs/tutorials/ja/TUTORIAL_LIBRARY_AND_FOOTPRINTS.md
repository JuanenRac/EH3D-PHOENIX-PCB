# ライブラリを編集して IPC フットプリントを生成する

EAGLE ライブラリを検査し、操作ファイルでフットプリントを追加し（すべて成功か何も変更しないかのいずれかで、元の複製が残ります）、IPC-7351B の方法で正式名称付きのフットプリントを生成します。生成されるフットプリントの寸法はプロトタイプで、データシートとは照合していません。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. ライブラリを検査します。`--out` がなければ何も書き込みません。レポートはフットプリント、シンボル、デバイスセットの数を数え、指摘を一覧にします。

```console
# test
$ phoenix libedit examples/tutorials/parts.lbr
~ 1 footprints, 1 symbols, 1 device sets
~ 0 errors, 0 warnings
~ dry run: nothing was written
```

2. パッケージ `HDR1X2` を作成してパッドを 2 つ追加する操作ファイルを適用します。新しいライブラリは新規ファイルに書き出され、読み戻して比較されます。元の複製がその隣に残ります。

```console
# test
$ phoenix libedit --apply examples/tutorials/lib_ops.json --out build/parts_hdr.lbr examples/tutorials/parts.lbr
~ operation 3: add pad to HDR1X2
~ the file was read back and is the same library
```

3. チップ抵抗 0603 のフットプリントを密度レベル B（標準）で生成します。正式名称は `RESC1608X55N` で、本体は 1.6 mm × 0.8 mm、高さ 0.55 mm です。`--out` なしではパッドを一覧にするドライランです。

```console
# test
$ phoenix ipc generate chip 0603 --density B
RESC1608X55N (density B,
~ dry run: nothing was written
```

4. 新しい `.lbr` ファイルに書き出します（`.kicad_mod` の名前を指定すると、代わりに KiCad のフットプリントを書き出します）。

```console
# test
$ phoenix ipc generate chip 0603 --density B --out build/RESC1608X55N.lbr
~ written: build/RESC1608X55N.lbr
```

5. より簡単なジェネレータ `footprint` は、パラメータからフットプリントを作ります（チップ、SOIC、TSSOP、QFN、QFP、DIP、ヘッダのファミリ）。

```console
# test
$ phoenix footprint chip --param size=0603 --out build/chip.lbr
~ footprint CHIP_0603
~ written:
```

6. 作成したライブラリを検査します。どれもフットプリントを 1 つ持っています。

```console
# test
$ phoenix libedit build/chip.lbr
~ 1 footprints
$ phoenix libedit build/RESC1608X55N.lbr
~ 1 footprints
```

7. 自分のライブラリのフットプリントを生成したものとパッド単位で比較し（差異があれば終了コード 1）、ライブラリのファミリを一覧にします。

```console
$ phoenix ipc compare build/chip.lbr chip 0603
$ phoenix ipc list
```

## 関連ドキュメント

* [docs/LIBRARY_EDIT.md](../../LIBRARY_EDIT.md)
* [docs/IPC_LIBRARY.md](../../IPC_LIBRARY.md)
* [docs/LIBINDEX.md](../../LIBINDEX.md)
* [docs/SANITIZER.md](../../SANITIZER.md)
