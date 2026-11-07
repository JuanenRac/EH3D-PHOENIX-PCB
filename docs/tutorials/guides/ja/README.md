# 機能ガイド

## ガイドの実行方法

コマンドはすべてリポジトリのルートで実行します。コマンドが書き込むのは新しいファイル、または `build/` 内の新しいフォルダだけです（先に `mkdir build` で作成します）。既存のものは置き換えません。ガイドはコマンドを示し、出力に何が入るかを説明します。`# test` で始まるブロックはリポジトリのテストが実行し、出力の先頭を比較します（`~` で始まる行は出力のどこかに現れる必要があります）。それ以外のコマンドは実行されず、出力の引用もないので、ガイドを横に置いて実際の出力を読んでください。このプログラムのすべてのツールはプロトタイプで、基準となるツールとは比較していません。

## ガイド

1. [チェックの結果を表とページに書き出し、基板をメーカーの能力プロファイルで確認する](TUTORIAL_CHECK_REPORTS_DFM.md)
1. [部品表をまとめて書き出し、別のプログラムの部品表を取り込む](TUTORIAL_BOM_GROUPING_IMPORT.md)
1. [テンプレートで複数のパネル配置を 1 つの設定ファイルにまとめる](TUTORIAL_PANEL_TEMPLATES.md)
1. [電界ソルバーで線路のインピーダンスを求め、パターンの温度上昇を見積もる](TUTORIAL_FIELD_SOLVER_HEATING.md)
1. [銅ベタの角を丸め、サーマルリリーフと島の下限を Gerber に書き出す](TUTORIAL_ZONE_CORNERS_RELIEFS.md)
1. [外形の開口とベタの切り欠きを各ファイル形式へ渡す](TUTORIAL_CUTOUTS.md)
1. [EAGLE の回路図のバスを読み込む](TUTORIAL_EAGLE_BUSES.md)
1. [内蔵オートルーターで基板を配線する：差動ペアと基板上の既存ビアの利用](TUTORIAL_AUTOROUTE_NATIVE.md)
1. [基板エディターで複数のオブジェクトを選択し、1 ステップで移動・整列・削除する](TUTORIAL_MULTIPLE_SELECTION.md)
1. [phoenix の画像モードでウィンドウの画像を撮る](TUTORIAL_SCREENSHOTS.md)
1. [制限ファイルでチェックの作業量を抑える](TUTORIAL_LIMITS.md)

## チュートリアル

* [チュートリアル](../../ja/README.md)

## 言語

* [English](../README.md)
* [Español](../es/README.md)
* [Deutsch](../de/README.md)
* [Français](../fr/README.md)
* [Italiano](../it/README.md)
* [中文](../zh/README.md)
* [Português (Portugal)](../pt/README.md)
