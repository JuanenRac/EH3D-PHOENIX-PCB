# 移行ガイド：EAGLE や KiCad から来た方へ

このページでは、EAGLE や KiCad の概念とコマンドをこのプログラムのものと並べ、読み書きできるものと失われるものを示します。このプログラムはプロトタイプで、実際の設計で EAGLE や KiCad と比較したものはありません。まず `phoenix help` を実行してください。

## 対応するコマンド

| やりたいこと | EAGLE | KiCad | このプログラム |
|---|---|---|---|
| デザインルールチェック | DRC | Design Rules Checker | `phoenix check`, `phoenix check --advanced` |
| 電気ルールチェック | ERC | Electrical Rules Checker | `phoenix erc` |
| 基板と回路図の突き合わせ | Forward & Back Annotation | Update PCB from Schematic | `phoenix connectivity`, `phoenix sync`, `phoenix annotate` |
| 未配線の接続 | RATSNEST | ratsnest | `phoenix ratsnest` |
| 配線 | ROUTE, AUTO | interactive router, Freerouting | `phoenix route`, `phoenix boardedit`, `phoenix autoroute` |
| 製造ファイル | CAM Processor | Plot, Fabrication Outputs | `phoenix export`, `phoenix fabpack` |
| 部品表と実装位置 | ULP | BOM and position files | `phoenix bom`, `phoenix pnp`, `phoenix stencil` |
| パネル | ULP | KiKit and similar | `phoenix panel` |
| スクリプトの実行 | SCRIPT, RUN | Python console | `phoenix script run` |
| ライブラリの編集 | Library editor | Symbol and Footprint editors | `phoenix libedit`, `phoenix footprint`, `phoenix ipc` |
| シミュレーション | SPICE export | ngspice | `phoenix netlist`, `phoenix sim` |

## 読み書きできるもの

| ファイル | 読み込み | 書き出し |
|---|---|---|
| EAGLE `.sch`, `.brd`, `.lbr` (XML) | `eagle-check` および、基板や回路図を受け取るすべてのコマンド | EAGLE XML としての基板と回路図: `boardedit`, `sync --apply`, `annotate --apply`; ライブラリ: `libedit`, `footprint`, `ipc` |
| EAGLE binary | いいえ | いいえ |
| EAGLE `.dru` | `check --rules-dru`, `eagle-convert dru-to-rules` | `eagle-convert rules-to-dru` |
| EAGLE `.cam` | `cam-import`, `eagle-convert cam-to-profile` | - |
| EAGLE `.scr` | `script run`, `script report` | - |
| EAGLE ULP | いいえ | - |
| KiCad 6 - 9 `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_sym`, `.kicad_mod` | `kicad-read`, `kicad-import`; `.kicad_pcb` と `.kicad_sch` は、基板や回路図を受け取る他のコマンドでも使えます | KiCad 8: `kicad-export` |
| Gerber X2, Excellon | `gerber-import`, `gerber-check` | `export`, `fabpack`, `panel` |
| DXF R12 | `dxf-import` | `dxf-export` |
| Specctra DSN, SES | SES: `autoroute` | DSN: `autoroute` |

## 失われるもの

* 設計モデルは、KiCad の図形、バス、ノーコネクト記号、基板のセットアップ、ゾーンの塗りつぶし銅箔、禁止領域、3D モデルを保持しません。リーダはこれらを「除外」として数え、レポートに記します。
* EAGLE：バイナリファイルは読めず、ULP プログラムは実行できません。モデルが簡略化する要素は `phoenix eagle-check` が 8 言語のコード付きで一覧にします。
* 入力ファイルは決して上書きされません。変換は常に新しいファイルを作ります。

## 最初の手順

1. `phoenix kicad-import`（KiCad）または `phoenix eagle-check`（EAGLE）でプロジェクトを読み込み、レポートを読みます。
2. `phoenix check --rules FILE`（自分のルールの制限。EAGLE のルールファイルなら `phoenix eagle-convert dru-to-rules`）と `phoenix erc` で検査します。
3. `phoenix export` で製造ファイルを書き出し、信頼する前にいつものツールのものと比較します。

## 関連ドキュメント

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD_EXPORT.md](../../KICAD_EXPORT.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/EXPORT.md](../../EXPORT.md)
* [用語集](../../glossary/ja/GLOSSARY.md)
* [チュートリアル](../../tutorials/ja/README.md)
* [English](../../MIGRATION.md)
