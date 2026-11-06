# 銅ベタの角を丸め、サーマルリリーフと島の下限を Gerber に書き出す

基板 `examples/guides/pour.brd` には、`Top` 層に 2 つの抵抗と 1 本のパターンを囲むグランドのベタがあります。角を尖らせたまま、次に丸めて塗りつぶして比べ、続いて、ベタの四角いパッドにサーマルリリーフを付け、小さな島を除くプロファイルで Gerber を書き出します。設計ルールのない EAGLE 基板には固有の上限がないため、ルールファイル `rules_relaxed.json` を指定します。

コマンドはリポジトリのルートで実行します。[ガイドの実行方法](README.md) を参照してください。

1. リポジトリの設定（`corner_radius_nm = 0`、角はすべて尖ったまま）でゾーンを塗りつぶします。レポートにはポリゴン数と銅の平方ナノメートルが出ます。

```console
$ phoenix zones fill --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

2. `corner_radius_nm = 1000000`（半径 1 mm の円弧）を設定する `zones_round.cfg` でもう一度塗りつぶします。外向きのベタの角とサーマルリリーフの隙間の角が丸められるので、銅の面積を 1 回目と比べてください。

```console
$ phoenix zones fill --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

3. 丸めた塗りつぶしを基板とルールに照らして測定します。丸めはベタから銅を取り除くか、サーマルリリーフの隙間の内側にだけ銅を加えるので、他のネットの銅に近づくことはなく、チェックでクリアランスのエラーは見つかりません。

```console
$ phoenix zones check --config examples/guides/zones_round.cfg --rules examples/tutorials/rules_relaxed.json examples/guides/pour.brd
```

4. まず汎用プロファイルで Gerber を書き出します。グランドのネットのパッド `R2.2` はベタの中にあり、リリーフなしでつながったままです。

```console
$ phoenix export --out build/pour_plain examples/guides/pour.brd
```

5. 次に `export_pour.json` のプロファイル `pour` を使います。スポーク 4 本（`gerber.zone_thermal_spokes`）、隙間とスポークは 0.25 mm、四角や長円のパッドにも同じリリーフ（`gerber.zone_thermal_non_round`）、1 平方ミリメートル未満の島は出力しません（`gerber.zone_island_min_area_nm2`）。2 つのフォルダの `Top` 層の銅ファイルを比べると、後者にはリリーフの隙間がパッドの周りのクリア極性の領域として書かれています。

```console
$ phoenix export --profiles examples/guides/export_pour.json --profile pour --out build/pour_relief examples/guides/pour.brd
```

## 結果の読み方

* 塗りつぶしの設定では、`corner_radius_nm` が円弧の半径（0 なら角はすべて尖ったまま）、`corner_tolerance_nm` が円弧と弦のずれの許容値です。円弧は角に接する各辺の半分までしか使えないので、収まらない半径は小さくされます。他の銅がベタに残す穴の角は丸めません。
* 出力では、リリーフのスポークは 4 本か 2 本です（`gerber.zone_thermal_spokes` は 0、2、4 のいずれかで、0 ならパッドはベタにつながったまま）。直角でない角度に回転したパッドはそのまま接続され、その旨が報告されます（`PHOENIX-EXPORT-022`）。`gerber.zone_island_min_area_nm2` より小さい島は書き出さず、レポートに出ます（`PHOENIX-EXPORT-021`）。0 ならすべてのゾーンを書きます。

## 関連ドキュメント

* [docs/ZONES.md](../../../../docs/ZONES.md)
* [docs/EXPORT.md](../../../../docs/EXPORT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
