# phoenix-gui の画像モードでウィンドウの画像を撮る

`phoenix-gui --shot FILE.png` はウィンドウを開き、数フレーム待ち、その画像を PNG ファイルに保存して自分で閉じます。README の画像はこうして作られます。このモードではウィンドウは設定も最近使ったファイルの一覧も読み書きしないので、実行しても設定は変わりません。`phoenix-gui` はウィンドウのプログラムで、リリースのアーカイブでは `phoenix` の隣にあり、`gui` フィーチャーでビルドします。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. スタート画面を、サイズ、テーマ、待つフレーム数を指定して撮ります。サイズは `幅x高さ` のピクセルで各辺 320〜8192、テーマは `dark`、`light`、`contrast` です。

```console
phoenix-gui --shot build/start.png --size 1280x720 --theme dark --wait 6
```

2. 基板のウィンドウを撮ります。`--open` でファイルを指定し、表示されるウィンドウは既定で `document` です。基板には `--open` が必要で、スタート画面には付けられません。

```console
phoenix-gui --shot build/board.png --open examples/tutorials/led_board.brd --theme light --wait 12
```

3. `--window` で別のウィンドウを撮ります。`3d`、`panel`、`gerber`、`search`、`measures`、`layers`、`history`、`appearance`、`shortcuts`、`help` から選びます（`3d`、`panel`、`gerber` には `--open` が必要）。`--lang` でウィンドウの言語を指定します。

```console
phoenix-gui --shot build/help_pt.png --window help --lang pt --size 1280x720
```

4. リポジトリのツールで、1 バージョン分の画像一式を撮ります。`--dry-run` はコマンドラインを表示するだけで何も実行しません。付けなければ、ツールはウィンドウごとにプログラムを 1 回起動し、各 PNG を検査し（空や単色でなく、400 KB 未満であること）、`phoenix-pcb-NN-ウィンドウ名.png` を書き出します。

```console
py -3.14 -I tools/make_shots.py --exe PATH/phoenix-gui --out build/screens --dry-run
```

## 結果の読み方

* 画像はウィンドウが描いた内容を物理ピクセルで保存したものなので、拡大表示の画面では `--size` より大きくなることがあります。終了コードは、画像が書けたら 0、書けなかったら 1、引数の誤りは 2 です。既存のファイルは `--replace` を付けない限り上書きされず、付けなければウィンドウを開く前に停止します。
* ツールの各実行は一時フォルダー内に専用の設定フォルダーを持つため、あなたの設定や最近使ったファイルは読み書きされません。画像の撮影そのものはまだ実際の画面で実行されておらず、あなたが見るまで、どの画像も正しく見えるとは主張しません。

## 関連ドキュメント

* [docs/VIEWER.md](../../../../docs/VIEWER.md)
* [tools/make_shots.py](../../../../tools/make_shots.py)
