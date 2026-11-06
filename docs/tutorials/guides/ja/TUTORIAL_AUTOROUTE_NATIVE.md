# 内蔵オートルーターで基板を配線する：差動ペアと基板上の既存ビアの利用

`phoenix autoroute native` は独自の探索で基板の未接続部分を配線し、外部プログラムを必要としません。`examples/guides/open.brd` には 2 つのパッドがつながっていないネットが 1 つ、`examples/guides/open_pair.brd` には未接続のペア `D_P` と `D_N` があります。計画が追加するのはパターンとビアだけで、前後に設計ルールが検査されます。例の EAGLE 基板には固有の設計ルールがないため、ルールファイルを指定します。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. 配線を計画します。ドライランで、レポートには配線できた接続、追加されたパターン・ビア・長さ、まだ開いている接続が並び、何も書き込まれていないことがエラー出力に出ます。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json examples/guides/open.brd
```

2. 配線した基板を新しいファイルに書き出します。ファイルは名前に関係なく設計モデルの JSON で、既存のファイルは置き換えません。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --out build/open_routed.json examples/guides/open.brd
```

3. 書き出した基板をもう一度配線します。未接続の接続は残っていないので、追加するものはありません。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json build/open_routed.json
```

4. ペアを指定します。`pair_dp_dn.json` は、`D_P` と `D_N` を幅 0.2 mm、間隔 0.3 mm のペアとして宣言します。余地があれば 2 本のパターンは 1 つの層に並べて敷かれ、レポートに `coupled pair: p` と出ます。なければ 2 つのネットは普通のネットとして配線され、コード `PHOENIX-AROUTE-017` の注記が理由を示します。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json examples/guides/open_pair.brd
```

5. ペアの 2 つのネットだけを配線し、接続が残る計画は拒否します。`--require-complete` を付けると、接続が残ればコマンドは終了コード 2 とコード `PHOENIX-AROUTE-016` で終わります。

```console
$ phoenix autoroute native --rules examples/tutorials/rules_relaxed.json --pairs examples/guides/pair_dp_dn.json --nets D_P,D_N --require-complete examples/guides/open_pair.brd
```

## 結果の読み方

* 基板にすでにあるそのネットのビアがグリッドのノード上にあれば、層の切り替えは無料です。経路はそこで層を変え、新しいビアは置きません。`config/autoroute_native.cfg` の `via_snap_nm` が 0 より大きければ、ノードからその距離以内にあるビアも使われ、注記 `PHOENIX-AROUTE-019` が付きます。これがビアの再利用で、計画は既存のパターン・ビア・パッドを変えません。
* ペアが結合配線されるのは、各ネットの未接続の接続がちょうど 1 つで、4 つのパッドが同じ層を共有しているときだけです。2 本のパターンは同じ長さで、ペアの間隔を保ちます。`--pairs` も `--pair-config` もなければ、ペアのネットは 2 つの普通のネットとして配線され、結果は後で基板エディターのペアチェックが判定します。
* プロトタイプで、手作りの小さな基板とテストで試したものです。探索、そのコストと上限（ノード数を数え、時間は数えません）は `config/autoroute_native.cfg` にあります。あるネットの配線をまとめるとルールエラーが増える場合、そのネットは除外されて `PHOENIX-AROUTE-015` で報告されます。計画は基板になかったエラーを決して増やしません。

## 関連ドキュメント

* [docs/AUTOROUTE_NATIVE.md](../../../../docs/AUTOROUTE_NATIVE.md)
* [docs/DIFFPAIRS.md](../../../../docs/DIFFPAIRS.md)
* [docs/AUTOROUTE.md](../../../../docs/AUTOROUTE.md)
