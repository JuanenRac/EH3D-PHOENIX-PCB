<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-PCB
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB：回路図入力、PCB レイアウト、検証ツール、回路シミュレーション、スイッチング電源と RF 向けの専用モジュールを備えた EDA 環境として計画中。現時点で書かれているのはコーパスの棚卸しだけです。

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-223-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 今日実際に動くもの：** コマンド `phoenix inventory` を持つ Rust ワークスペースで、非公開フォルダー内の設計ファイルを、名前や値を一切表示せずに数えます（拡張子・種類・宣言されたバージョンごとのファイル数と、内容の指紋 1 つ）。独自のテストが 220 件あり、コンピューター上で通っています。実行したのは自身のテストが作ったフォルダーだけで、実際の設計ファイルでは一度もありません。設計モデルの土台（範囲を検査するナノメートルとマイクロ度の整数値、オブジェクト ID、正規化 JSON）、プロジェクトファイル `.phoenixproj`（マニフェスト、ドメイン、移行）、元に戻せる操作を持つ基板モデル（層、ネット、部品、配線、ビア、ゾーン）もありますが、生成したデータでしか試していません。基板ライブラリは、どの銅がどの銅に接しているかの判定と、ルールファイルに基づくサイズと間隔の検査も行います。5 つ目のライブラリは EAGLE XML のライブラリ・基板・回路図を要約として読み、XML ツリーを同一のまま書き戻します。すべて生成または手作りのファイルでしか試していません。回路図モデルと基板との整合性検査もあります。他の設計のモデルへの取り込みとエディターはなく、ルールも予定した検査の一部しかカバーしておらず、EDA の機能の大半は計画段階のままです。基板モデルには、ルールチェックと Gerber 出力が使うシルク文字とシルク線（内蔵のストロークフォント）も含まれます。回路図はシートに配置した部品と配線で描くことができ、ネットリストおよび基板と照合されます。基板は EAGLE の XML ファイルとして書き出して読み戻せますが、確認したのはこのコードが書いたファイルだけで、EAGLE で開いたことはありません。EAGLE のバイナリファイルとその XML の組を扱う取り決めは書かれていますが、バイナリファイルの読み取り部はありません。

---

> **安全性とセキュリティの現状：** このリポジトリは最初の段階で、安全装置ではありません。ここに書かれたことが、機械とそのコントローラーの上限、インターロック、非常停止の代わりになることはありません。

## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.2` |
| 成熟度 | 足場 |
| テスト | 223 |
| 技術スタック | Rust, Python 3.10+ |
| 実機での動作 | まだハードウェアでは実行していません |

**移動先：** [概要](#-概要) · [アーキテクチャ](#-アーキテクチャ) · [クイックスタート](#-クイックスタート) · [テスト](#-テスト) · [リポジトリの構成](#-リポジトリの構成) · [関連プロジェクト](#-関連プロジェクト)

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| リポジトリ チェック | 🟡 | リポジトリ チェック、画像のチェック、基盤の 3 件のテストは CI で実行され、通っています。 |
| テスト | 🟡 | Rust コードの 220 件のテストと基盤の 3 件のテストは、コンピューター上で通っています。テスト自身が作ったフォルダーだけを使っており、実際の設計ファイルで実行したものはありません。 |
| 実機 | 🟠 | どの機械やボードでも実行していません。ここにあるものは、ハードウェア上で試していません。 |
| 計画中の機能 | 🟠 | 作成済み：コーパスの伏せ字モードによる棚卸し、モデルの土台（単位、ID、正規化 JSON）、プロジェクトファイル `.phoenixproj`、元に戻す／やり直しを持つ基板モデルとその接続性、最初のルール検査、EAGLE XML の要約リーダー。未着手：回路図エディター、他形式のモデルへの取り込み、他形式の書き出し、ルールの大半、ビューアーとエディター、シミュレーション、その他の計画内容すべて。基板モデルには、ルールチェックと Gerber 出力が使うシルク文字とシルク線（内蔵のストロークフォント）も含まれます。回路図はシートに配置した部品と配線で描くことができ、ネットリストおよび基板と照合されます。基板は EAGLE の XML ファイルとして書き出して読み戻せますが、確認したのはこのコードが書いたファイルだけで、EAGLE で開いたことはありません。EAGLE のバイナリファイルとその XML の組を扱う取り決めは書かれていますが、バイナリファイルの読み取り部はありません。 |
| 要件 | 🟠 | 30 件の要件のうち 5 件（伏せ字モード、モデルの土台、元に戻せる操作、プロジェクトファイル、診断コード）は試作段階、25 件は計画のみで、検証済みのものはありません。 |

## 🎯 概要

* **現在あるもの：** 設計ファイルの非公開コーパスを伏せ字モードで数えるライブラリ（件数、種類、宣言されたバージョン、指紋 1 つだけで、名前や値は決して出さない）を含むワークスペースと、コマンドライン `phoenix inventory`。ファイルの先頭バイトだけを見て、設計内容は読みません。2 つ目のライブラリには設計モデルの土台があります（範囲を検査した整数の長さと角度、オブジェクト ID（UUID v7）、正規化 JSON）。3 つ目のライブラリは EH3D-COMMON の共有コンテナ上で `.phoenixproj` プロジェクトを読み書きします（ドメイン、リンク、移行を持つマニフェスト、理解できないドメインはそのまま保存、`must_understand` の規則）。4 つ目のライブラリは基板モデルを持ちます（層、ネット、フットプリント、部品、直線と円弧の配線、ビア、ゾーンを安定した ID で結び、元に戻せる操作と元に戻す／やり直しの履歴）。すべての失敗に安定したコード `PHOENIX-<領域>-<NNN>` があります。基板ライブラリは各ネットの島も計算し、幅・ドリル・アニュラリング・間隔をミリメートル単位のルールファイルで検査して、確定した結果と判定できなかった結果を分けて示します。5 つ目のライブラリは EAGLE XML のライブラリ・基板・回路図を要約として読み、XML ツリーを同一のまま書き戻し、SHA-256 で確認したコーパスのコピーで読み取り率を測ります（元のファイルは変更しません）。コマンド `phoenix eagle-rate` で実行できます。6 つ目のライブラリは、EH3D-CORE-LIBS と EH3D-COMMON を使って基板の Gerber X2・Excellon・`.eh3dproj` 1.0.0 ファイルを書き出し、同じ基板なら同じバイト列になります。書き出したファイルは同じライブラリのパーサーでしか読み戻していません。7 つ目のライブラリは回路図（安定した ID で結ばれたシンボル、部品、ネット）を持ち、基板と照合します（コンポーネントのない部品、参照記号・値・フットプリント・ネットの違い。コード `PHOENIX-LINK-<NNN>`）。

* **計画中：** EH3D-PHOENIX-PCB は、回路図入力、PCB レイアウト、検証ツール、回路シミュレーション、スイッチング電源と RF 向けの専用モジュールを備えた EDA 環境となることを目指しています。デスクトップ向けのウィンドウ付き Rust プログラム（エンジン クレートのワークスペース、コマンドライン インターフェース、アプリケーション）として計画されており、単独で動作し、計算タスクにはサーバーを利用できます。これらはまだ何も存在しません。

## 🧩 EH3D での役割

EH3D-PHOENIX-PCB は EH3D Fabrication Suite の PHOENIX ファミリーの中心となるプロジェクトで、ファミリーの他のリポジトリはこれを土台にしています。何をするか、まだ何をしないか、要件、他のリポジトリとの関係は `docs/` フォルダーに書かれています。

## 🏛️ アーキテクチャ

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-PHOENIX-PCB のアーキテクチャ：誰が使い、何を持ち、何に依存するか" width="100%">
</p>

このリポジトリの各部分の関係：誰が使い、何を持ち、何に依存するか。図は `docs/architecture.json` から描かれ、コードと照合されます。

## 🚀 クイックスタート

チェックには Python 3.10 以降、コードには Rust ツールチェーンが必要です。

```bash
cargo build --release
cargo test
cargo run -p eh3d-phoenix-cli -- inventory <folder>
python -I tools/ci_validate.py
python -I tools/make_architecture.py --check
python -I tools/make_structure.py --check --readmes
python -I -m unittest discover -s tools -p "test_*.py"
```

## 🧪 テスト

コードには 220 件のテスト（`cargo test`）、基盤には 3 件のテスト（`python -I -m unittest discover -s tools -p "test_*.py"`）があり、すべてコンピューター上で通っています。CI は、リポジトリ チェック、画像のチェック、基盤のテストに加え、Rust コードについては書式チェック、警告をエラーとする clippy、そしてテストを Linux と Windows で実行します。

## 🏗️ ビルドとリリース

バージョンは `VERSION` ファイルと `eh3d.project.json` にあり、各バージョンには [CHANGELOG.md](CHANGELOG.md) の項目があります。`cargo build --release` でライブラリと `phoenix` プログラムをビルドし、リリースは GitHub で公開されます。

## 📂 リポジトリの構成

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-PHOENIX-PCB のリポジトリ構成" width="100%">
</p>

<details>
<summary>ツリーのテキスト版</summary>

```text
EH3D-PHOENIX-PCB/
├── docs/
│   ├── architecture.json structure.json         2 つの図のデータ
│   ├── SCOPE.md REQUIREMENTS.md                 範囲と要件
│   ├── DEPENDENCIES.md THREAT_MODEL.md          他のリポジトリとの関係と脅威モデル
│   ├── DECISIONS.md STATUS.md                   適用される決定事項と正直な状況
│   ├── NOT_YET.md RULES.md                      後回しにしたアイデアとコードの記述規則
│   ├── ROADMAP.md                               基盤からオールインワン スイートまでのフェーズ マップ
│   ├── ERROR_CODES.md                           診断コード PHOENIX-<領域>-<NNN>
│   ├── GATE_F1.md                               第 1 段階のゲートが確認すること、しないこと
│   └── adr/                                     承認された決定の記録
├── AGENTS.md                                    リポジトリを変更する人のための規則
├── deny.toml                                    依存関係のライセンスと勧告の方針
├── crates/                                      棚卸し、モデル、プロジェクト、基板、回路図、EAGLE、ハブ、書き出し、CLI
├── config/                                      設定ファイル（調整する値はすべてここ）
├── Cargo.toml Cargo.lock                        Rust ワークスペースと固定された依存関係
├── tools/                                       リポジトリ チェック、そのテスト、画像のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とダッシュボードのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── VERSION                                      リポジトリのバージョン
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* 初期段階のコード：あるのはコーパスの棚卸しとモデルの土台だけで、棚卸しは自身のテストが作ったフォルダーでしか動かしておらず、実際の設計ファイルでは一度も使っていません。
* 回路図、他の設計ファイルの読み書き、エディター、ルールはなく、プロジェクトファイルと基板モデルは生成したデータでしか試しておらず、実際の設計では一度も使っていません。
* ユーザーインターフェースとジオメトリ ライブラリは未決定のままです（`docs/STATUS.md` を参照）。

**次のステップ**

* このプロジェクトが依存する契約が承認されるのを待ち（`docs/DEPENDENCIES.md` を参照）、その後モデルと最初の読み取り機能をテスト付きで書き、状態の行を実際の結果に置き換えます。

## 🌍 言語

README とテキストは、英語、スペイン語、フランス語、イタリア語、ドイツ語、中国語、日本語、ヨーロッパポルトガル語で提供されています。

## 🔗 関連プロジェクト

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS)：スイートの地図、アーキテクチャ、規約、共有チェック
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON)：このリポジトリの隣でビルドします。このプロジェクトが使う共有プロジェクトコンテナ（`eh3d-container`）と共通の診断（`eh3d-diagnostic`）
* PHOENIX の兄弟リポジトリ：[EH3D-VALIDATION-LAB](https://github.com/JuanenRac/EH3D-VALIDATION-LAB)（コーパスと正解ファイルのオラクル）と [EH3D-PHOENIX-CAD](https://github.com/JuanenRac/EH3D-PHOENIX-CAD)（機構側）。どちらも初期段階のコードです
* 計画中の関係（未構築）（EH3D-CORE-LIBS, EH3D-SERVER, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE）：[docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) を参照

## 📚 ドキュメント

* [スイートの規約](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [変更履歴](CHANGELOG.md)
* [範囲、要件、依存関係、脅威モデル、決定事項](docs/SCOPE.md)

## 🤝 コントリビュートとコミュニティ

* プル リクエストの前に `python -I tools/ci_validate.py` と基盤のテストを実行してください。すべて通る必要があります。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照してください。
* README は 8 言語あります。文章を変更するときは 8 つのファイルすべてを変更し、2 つの図は `docs/architecture.json` と `docs/structure.json` から `tools/make_architecture.py` と `tools/make_structure.py` で描画します。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問、アイデア、報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
