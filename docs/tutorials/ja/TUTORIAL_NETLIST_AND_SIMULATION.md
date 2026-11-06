# 回路図からネットリストとシミュレーションまで

`examples/tutorials/divider.json` は、10 V の電源と 1 kΩ・3 kΩ の直列抵抗からなる回路図です。ネットリストを手で書かずに作成してシミュレーションし、分圧の式 10 V × 3 / (1 + 3) = 7.5 V と結果を照合します。

コマンドはリポジトリのルートで実行します。[チュートリアルの実行方法](README.md) を参照してください。

1. 回路図のネットリストを表示します。何も書き込まれず（ドライラン）、先に電気ルールチェックが実行されます。1 行目はタイトルで、続いて素子が、シミュレータの表記の値で並びます。

```console
# test
$ phoenix netlist examples/tutorials/divider.json
divider
~ R1 in out 1e3
~ R2 out 0 3e3
~ V1 in 0 1e1
```

2. 新しいファイルに書き出します。既存のファイルは決して上書きされません。

```console
# test
$ phoenix netlist examples/tutorials/divider.json --out build/divider.cir
~ written:
```

3. 動作点を求めます。分圧の式ではノード `out` は 7.5 V になります。

```console
# test
$ phoenix sim build/divider.cir --op --probe "v(out)"
name,value
v(out),7.5e0
```

4. ファイルを介さず、`--schematic` で回路図からネットリストを作り、そのままソルバに渡します。

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --op --probe "v(out)"
name,value
v(out),7.5e0
```

5. 電源を 0 V から 10 V まで 5 V 刻みで掃引します。出力は常に入力の 3/4 です。

```console
# test
$ phoenix sim --schematic examples/tutorials/divider.json --dc V1 0 10 5 --probe "v(out)"
~ 5e0,3.75e0
~ 1e1,7.5e0
```

## 関連ドキュメント

* [docs/SIM.md](../../SIM.md)
* [docs/NETLIST.md](../../NETLIST.md)
* [docs/ERC.md](../../ERC.md)
