# 基板エディターで複数のオブジェクトを選択し、1 ステップで移動・整列・削除する

基板エディターの選択は、部品、パターン、ビア、ゾーン、シルク要素の順序付きの集合です。画面なしでは `phoenix boardedit` と操作ファイルで扱います。`select` が選択を変え、`move_selection`、`delete_selection`、`align_selection` が選択に作用して、正確な逆操作を持つ履歴の 1 ステップになります。`examples/guides/select_ops.json` は矩形で完全に内側にあるもの（ここでは抵抗 R1 で、パターンは含まれません）を選び、2 mm 上へ移動します。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. 操作をドライランで適用します。レポートには 3 つの操作と、前後のルールエラーが出て、何も書き込まれません。EAGLE 基板には固有の設計ルールがないため、ルールファイルを指定します。

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json examples/tutorials/tut.brd
```

2. 編集した基板を新しいファイルに書き出します（設計モデルは `.json`、EAGLE は `.brd`）。入力は上書きされません。終了コードは、結果にルールエラーが証明されなければ 0、証明されれば 1 です。

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --out build/moved.json examples/tutorials/tut.brd
```

3. レポートを JSON で出力します。スクリプトから読め、キー `rule_errors` は結果にルールが証明したエラーの数です。

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --json examples/tutorials/tut.brd
```

## 結果の読み方

* `select` のステップは、`add`、`remove`、`toggle`（`items` のオブジェクト、`{"kind": "component", "id": "..."}` の形）、`net`（ネットの ID によるそのネットの全オブジェクト）、`rect`（`from` と `to` の間の矩形、座標は `x_nm`、`y_nm` のナノメートル。`mode` は `contained` か、矩形に触れるものも含む `crossed`）、`all`、`invert`、`clear` です。順に、全部成功するか全部失敗するかで適用され、選択は履歴に含まれません。
* `move_selection` は `dx_nm`、`dy_nm`（上が正）だけ移動し、`snap` が false でなければグリッドに丸めます。選択していないパターンは部品に追従しないので、銅も運ぶならまずネットを選びます。`delete_selection` は選択を削除します。`align_selection` は `left`、`right`、`top`、`bottom`、`center_x`、`center_y`（2 個以上）と `distribute_x`、`distribute_y`（3 個以上）を取ります。ロックされた部品があると操作全体が拒否され（`PHOENIX-BEDIT-006`）、空の選択は `PHOENIX-BEDIT-009` です。
* ウィンドウで選択ツールを使うとき：Shift または Ctrl+クリックでオブジェクトを切り替え、空きの場所での Shift または Ctrl+ドラッグで矩形に完全に入るもの（ゾーンとシルク要素も）を追加し、Alt+ドラッグは矩形が触れるものも追加します。`N` はポインター下のネットを選択、Ctrl+A はすべて選択、Esc は選択解除、矢印キーは選択を 1 グリッド移動（Shift で 10 グリッド）、Delete は削除、Alt+左・右・上・下と Alt+C、Alt+M は整列、Alt+H と Alt+V は等間隔に配置（3 つ以上）、Ctrl+I は選択の反転です。シルク要素やゾーンの縁をクリックするとそれが選択され、選択したオブジェクトをドラッグすると選択全体がグリッド単位で動き、ステータスバーには選択数とその範囲・寸法が表示されます。どれも履歴の 1 ステップで、Ctrl+Z で同じバイト列に戻ります。ウィンドウは画面で見られたことも、マウスで使われたこともありません。

## 関連ドキュメント

* [docs/BOARD_EDIT.md](../../../../docs/BOARD_EDIT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
* [docs/VIEWER.md](../../../../docs/VIEWER.md)
