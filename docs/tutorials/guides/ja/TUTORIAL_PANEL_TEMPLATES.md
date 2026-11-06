# テンプレートで複数のパネル配置を 1 つの設定ファイルにまとめる

`examples/guides/panel_templates.cfg` には基本の配置（V カットでつないだ 2 枚の基板を並べ、フレームなし）と、名前付きの 2 つのテンプレートがあります。テンプレートは名前を挙げた行だけを変え、`--template` で選びます。基板はパネルのチュートリアルと同じ 30 mm × 20 mm で、ここのコマンドはすべてドライランです。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. テンプレートなしでは基本部分だけを読み、テンプレートの節の行は無視されます。60 mm × 20 mm、2 x 1 枚です。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg examples/tutorials/kicad/demo.kicad_pcb
```

2. テンプレート `four` は `columns` を 4 にするだけです。120 mm × 20 mm、4 x 1 枚になります。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four examples/tutorials/kicad/demo.kicad_pcb
```

3. テンプレート `grid` は `rows` を 2 にします。60 mm × 40 mm、2 x 2 枚です。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template grid examples/tutorials/kicad/demo.kicad_pcb
```

4. `--columns` と `--rows` は引き続きテンプレートより優先されます。`four` に `--columns 3` を付けると、90 mm × 20 mm、3 x 1 枚です。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --columns 3 examples/tutorials/kicad/demo.kicad_pcb
```

5. ファイルにない名前はコード `PHOENIX-PANEL-001` で拒否され、終了コードは 2 です。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template ring examples/tutorials/kicad/demo.kicad_pcb
```

6. 選んだ配置のファイルを、パネルのチュートリアルと同様に新しいフォルダに書き出します。

```console
$ phoenix panel --config examples/guides/panel_templates.cfg --template four --out build/panel_four examples/tutorials/kicad/demo.kicad_pcb
```

## 結果の読み方

* 最初の `[template 名前]` 行より前の行が基本部分です。名前は英数字、`_`、`-` の 1〜32 文字で、別の書き方の節は `PHOENIX-PANEL-001` と行番号で拒否されます。
* サイズの行は `panel W mm by H mm, C x R boards` です。V カットで隙間 0 なら基板同士が接するので、幅は列数 × 30 mm、高さは行数 × 20 mm です。テンプレートは配置に名前を付けて保管する方法であり、どの基板メーカーの設定でもありません。

## 関連ドキュメント

* [docs/PANEL.md](../../../../docs/PANEL.md)
* [config/panel.cfg](../../../../config/panel.cfg)
