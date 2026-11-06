# チュートリアル

## チュートリアルの実行方法

コマンドはすべてリポジトリのルートで実行します。コマンドが書き込むのは新しいファイル、または `build/` 内の新しいフォルダだけです（先に `mkdir build` で作成します）。既存のものは置き換えません。表示している出力は各コマンドが出力する内容の先頭部分です。`# test` で始まるブロックはリポジトリのテストが実行し、出力の先頭を比較します（`~` で始まる行は出力のどこかに含まれている必要があります）。このプログラムの各ツールはプロトタイプで、基準となるツールとの比較は行っていません。

ヘルプはプログラム自体にあります。`phoenix help` でトピックとコマンドを一覧にし、`phoenix help sim` や `phoenix help --lang es sim` でトピックまたはコマンドを説明します。

## チュートリアル

1. [回路図からネットリストとシミュレーションまで](TUTORIAL_NETLIST_AND_SIMULATION.md)
1. [基板から製造ファイルまで](TUTORIAL_BOARD_TO_FABRICATION.md)
1. [KiCad／EAGLE プロジェクトを取り込んで確認する](TUTORIAL_IMPORT_KICAD_EAGLE.md)
1. [基板を面付けしてパネルのファイルを書き出す](TUTORIAL_PANEL.md)
1. [RC フィルタと反転増幅器をシミュレーションする](TUTORIAL_FILTER_AND_AMPLIFIER.md)
1. [ライブラリを編集して IPC フットプリントを生成する](TUTORIAL_LIBRARY_AND_FOOTPRINTS.md)
1. [インピーダンス整合とフィルタ合成](TUTORIAL_RF_MATCHING_FILTER.md)
1. [差動ペアのシグナルインテグリティを確認する](TUTORIAL_DIFFERENTIAL_PAIR.md)
1. [スクリプトを使う：LED マトリクスとパネル](TUTORIAL_SCRIPTS.md)

## 機能ガイド

* [新しい機能のガイド](../guides/ja/README.md)：チェックレポートと能力プロファイル、部品表、パネルのテンプレート、電界ソルバーと発熱、銅ベタ、開口、バス、オートルーター、複数選択、ウィンドウの画像。八つの言語で提供

## 用語集と移行ガイド

* [用語集](../../glossary/ja/GLOSSARY.md)
* [移行ガイド：EAGLE や KiCad から来た方へ](../../migration/ja/MIGRATION.md)

* [English](../README.md)
