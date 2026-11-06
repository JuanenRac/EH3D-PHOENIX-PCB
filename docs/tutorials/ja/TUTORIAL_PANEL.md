# 基板を面付けしてパネルのファイルを書き出す

30 mm × 20 mm の基板を V カットでつないで左右に 2 枚並べ、パネル全体の Gerber とドリルのファイルを書き出します。パネルの設定は `examples/tutorials/panel.cfg`（列、行、接合の種類、隙間、フレーム）にあり、設定の全一覧は `config/panel.cfg` です。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. パネルを計画します。ドライランはサイズ、接合、穴、書き出されるファイルを表示するだけで、何も書き込みません。

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ joint: V-score, 1 lines
~ dry run: nothing written
```

2. 各コピーの層、ドリルファイル、V カット線入りの外形、実装位置、部品表を新しいフォルダに書き出します。

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
panel 60.0 mm by 20.0 mm, 2 x 1 boards
~ written
```

3. 既存のフォルダには決して書き込みません。2 回目の実行はコード `PHOENIX-PANEL-008`、終了コード 2 で拒否されます。

```console
# test
$ phoenix panel --config examples/tutorials/panel.cfg --out build/panel examples/tutorials/kicad/demo.kicad_pcb
# exit 2
~~ PHOENIX-PANEL-008
```

4. パネルの Gerber ファイルの構造を検査し、機構側向けにパネルを DXF 図面として書き出します。

```console
$ phoenix gerber-check build/panel/panel_01_top.gbr
$ phoenix dxf-export panel --out build/panel.dxf --panel-config examples/tutorials/panel.cfg examples/tutorials/kicad/demo.kicad_pcb
```

## 関連ドキュメント

* [docs/PANEL.md](../../PANEL.md)
* [docs/GERBER_IMPORT.md](../../GERBER_IMPORT.md)
* [docs/DXF.md](../../DXF.md)
* [docs/FABPACK.md](../../FABPACK.md)
