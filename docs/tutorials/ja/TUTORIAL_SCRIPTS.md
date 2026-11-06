# スクリプトを使う：LED マトリクスとパネル

言語 `phoenix-script/2` のスクリプトには、変数、回数制限付きのループ、基板への問い合わせ、元に戻せる 1 ステップとして実行される編集があります。実行はサンドボックス内で行われ、ネットワーク、プログラム、設定のデータフォルダ以外のファイルにはアクセスしません。`--out` が新しいファイルを指さない限りドライランです。例は `examples/scripts/` にあります。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. プログラムに付属のサンプルスクリプトを一覧にし、1 つを表示します。

```console
# test
$ phoenix script list
~ led_matrix
~ panelize_matrix
$ phoenix script list --show led_matrix
phoenix-script/2 board
```

2. スクリプトを実行せずに検査します。構文、名前、引数の数、各関数が対象とするドキュメント、ステップ数の見積りを確認します。

```console
# test
$ phoenix script check examples/scripts/led_matrix.phs
```

3. フットプリントが `LED0603` の基板で LED マトリクスを実行します。ピッチ 2.5 mm で LED を 6 個配置し、各 LED の 2 番パッドを 1 つのネットにつなぎます。基板は変更されないドライランです。

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --board examples/tutorials/led_board.brd
~ placed 6 LEDs on net LEDCOM
~ pads on the net: 6
~ dry run: nothing was written
```

4. パネル化スクリプトは、基板の 2 つの部品を 2 × 2 の行列にコピーします。部品数は 2 から 8 になります。

```console
# test
$ phoenix script run examples/scripts/panelize_matrix.phs --board examples/tutorials/tut.brd
~ copies: 4
~ components: 2 -> 8
```

5. `--out` を付けると、結果が新しいファイルに 1 回だけ書き出されます。既存のファイルは決して置き換えられません。

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
~ written
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
# exit 2
~~ never replaced
```

## 関連ドキュメント

* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/SCRIPT_API.md](../../SCRIPT_API.md)
* [docs/SCRIPT_EXAMPLES.md](../../SCRIPT_EXAMPLES.md)
