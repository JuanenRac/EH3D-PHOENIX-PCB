# 外形の開口とベタの切り欠きを各ファイル形式へ渡す

`examples/guides/cutout.brd` は 2 種類の開口を持つ EAGLE 基板です。外形の内側の層 20 にある閉じた四角形（基板の開口、スロットや窓）と、`Top` 層のベタの内側の層 41 にあるポリゴン（銅を置いてはいけないベタの切り欠き）です。基板を DXF に書き出して読み戻し、KiCad と Gerber のファイルを書いて、各開口がどこへ行くかを見ます。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. 基板の DXF 図面に何が入るかを一覧します。外形と開口はレイヤー `OUTLINE` の 2 つの閉じた輪郭で、レポートには `layer OUTLINE: 2` と、何も書き込まれていないことが出ます。

```console
$ phoenix dxf-export board examples/guides/cutout.brd
```

2. 図面を新しいファイルに書き出します。

```console
$ phoenix dxf-export board examples/guides/cutout.brd --out build/cutout.dxf
```

3. 図面を基板に読み戻します。最大の外側の輪郭が基板の外形になり、その内側の輪郭は外形の開口として残ります（`PHOENIX-DXF-015`、情報）。操作はドライランで、前後に設計ルールで検査されるため、ルールファイルを指定します。

```console
$ phoenix dxf-import build/cutout.dxf --board examples/guides/cutout.brd --layer OUTLINE --rules examples/tutorials/rules_relaxed.json
```

4. KiCad の基板を書き出します。外形と開口は `Edge.Cuts` に入り、ベタの切り欠きはその層のルールエリアになって、ベタを寄せ付けず、パターン・ビア・パッドは許可します。レポートには書き出した内容と、その形式で表せないことが並びます。

```console
$ phoenix kicad-export --out build/cutout.kicad_pcb board examples/guides/cutout.brd
```

5. 製造ファイルを書き出します。外形ファイルには外形に続いて開口があり、どちらも閉じた輪郭です。銅のファイルでは、ベタの切り欠きがベタのあとにクリア極性の領域として書かれます。

```console
$ phoenix export --out build/cutout_fab examples/guides/cutout.brd
```

## 結果の読み方

* 開口の出どころ：EAGLE 基板では、層 20 の連続して閉じる線のうち、最大のリングの内側にあるものが基板の開口です。その層のベタの内側にある層 41（`tRestrict`、上面）または 42（下面）のポリゴンはベタの切り欠きです。KiCad 基板では `Edge.Cuts` の最大の閉じた輪郭の内側にある閉じた輪郭、DXF では最大の外側の輪郭の内側にある輪郭が開口です。
* 各形式はできることだけを表します。ベタの切り欠きは、KiCad ではルールエリア、EAGLE では禁止エリア、オートルーターの Specctra ファイルでは進入禁止エリアになります（Specctra には面の穴がないため、パターンとビアも入れません）。パネルは両方の開口を各コピーへ、コピーと一緒に移動・回転・反転して運びます。

## 関連ドキュメント

* [docs/DXF.md](../../../../docs/DXF.md)
* [docs/KICAD_EXPORT.md](../../../../docs/KICAD_EXPORT.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/PANEL.md](../../../../docs/PANEL.md)
