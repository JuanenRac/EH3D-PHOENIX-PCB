<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# 🛠️ EH3D-PHOENIX-PCB

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB：计划作为具备原理图输入、PCB 布局、验证工具、电路仿真以及开关电源和射频专用模块的 EDA 环境。目前只写了语料库清点工具。

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-23-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**诚实性核对 - 今天真正能运行的内容：** 一个 Rust 工作区，只有一条命令 `phoenix inventory`，用来清点私有文件夹中的设计文件，而不显示任何名称或数值：按扩展名、类型和声明版本统计的文件数，以及内容的一个指纹。它有 20 个自己的测试，在计算机上通过。它只在自己的测试生成的文件夹上运行过，从未用于真实的设计文件。没有设计模型，没有设计文件的读取器或写入器，没有编辑器，也没有规则：EDA 的全部功能仍在计划中。

---

> **安全（safety）与信息安全现状：** 本仓库刚刚起步，不是安全系统（safety）。这里写的任何内容都不能取代机床及其控制器的限位、联锁和急停。

## 📋 速览

| | |
|---|---|
| 版本 | `0.0.1` |
| 成熟度 | 脚手架 |
| 测试 | 23 |
| 技术栈 | Rust, Python 3.10+ |
| 真实硬件 | 尚未在硬件上运行 |

**跳转到：** [概述](#-概述) · [架构](#-架构) · [快速开始](#-快速开始) · [测试](#-测试) · [仓库结构](#-仓库结构) · [相关项目](#-相关项目)

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 仓库检查 | 🟡 | 仓库检查、图片检查以及基础部分的 3 个测试都在 CI 中运行并通过。 |
| 测试 | 🟡 | Rust 代码的 20 个测试和基础部分的 3 个测试在计算机上通过。它们只使用测试自己生成的文件夹；没有任何一个在真实设计文件上运行过。 |
| 真实硬件 | 🟠 | 没有在任何机床或电路板上运行过：这里没有任何内容在硬件上试过。 |
| 计划的功能 | 🟠 | 已编写：语料库的脱敏清点。未开始：设计模型、读取器和写入器、规则、查看器和编辑器、仿真以及其余所有计划内容。 |
| 需求 | 🟠 | 30 项需求中有 1 项（脱敏模式）处于原型阶段，29 项仅为计划，没有一项已验证。 |

## 🎯 概述

* **已有的内容：** 一个工作区，其中的库以脱敏模式清点私有的设计文件语料库（只有计数、类型、声明版本和一个指纹，绝不含名称或数值），以及一个命令行 `phoenix inventory`。它只查看文件的前几个字节，不读取设计内容。

* **计划中：** EH3D-PHOENIX-PCB 的目标是成为具备原理图输入、PCB 布局、验证工具、电路仿真以及开关电源和射频专用模块的 EDA 环境。计划做成桌面端带窗口的 Rust 程序（由引擎 crate 工作区、命令行界面和应用组成），可独立运行，并可使用服务器执行计算任务。这些目前都还不存在。

## 🧩 在 EH3D 中的角色

EH3D-PHOENIX-PCB 是 EH3D Fabrication Suite 的 PHOENIX 系列的主项目；该系列的其他仓库都建立在它之上。它将做什么、暂时不做什么、需求以及与其他仓库的关系都写在 `docs/` 文件夹中。

## 🏛️ 架构

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-PHOENIX-PCB 的架构：谁在使用、包含什么、依赖什么" width="100%">
</p>

本仓库各部分如何协作：谁在使用它、它包含什么、它依赖什么。图片由 `docs/architecture.json` 绘制，并与代码核对。

## 🚀 快速开始

检查需要 Python 3.10 或更高版本，代码需要 Rust 工具链。

```bash
cargo build --release
cargo test
cargo run -p eh3d-phoenix-cli -- inventory <folder>
python -I tools/ci_validate.py
python -I tools/make_architecture.py --check
python -I tools/make_structure.py --check --readmes
python -I -m unittest discover -s tools -p "test_*.py"
```

## 🧪 测试

代码有 20 个测试（`cargo test`），基础部分有 3 个（`python -I -m unittest discover -s tools -p "test_*.py"`）；全部在计算机上通过。CI 运行仓库检查、图片检查、基础部分的测试，并对 Rust 代码在 Linux 和 Windows 上运行格式检查、把警告当作错误的 clippy 以及测试。

## 🏗️ 构建与发布

版本号位于 `VERSION` 文件和 `eh3d.project.json`，每个版本在 [CHANGELOG.md](CHANGELOG.md) 中都有条目。`cargo build --release` 构建库和 `phoenix` 程序；版本发布在 GitHub 上。

## 📂 仓库结构

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-PHOENIX-PCB 的仓库结构" width="100%">
</p>

<details>
<summary>目录树的文本版</summary>

```text
EH3D-PHOENIX-PCB/
├── docs/
│   ├── architecture.json structure.json         两张图的数据
│   ├── SCOPE.md REQUIREMENTS.md                 范围和需求
│   ├── DEPENDENCIES.md THREAT_MODEL.md          与其他仓库的关系和威胁模型
│   ├── DECISIONS.md STATUS.md                   适用的决策和如实的状态
│   ├── NOT_YET.md RULES.md                      留待以后的想法和编写代码的规则
│   └── ROADMAP.md                               从基础到一体化套件的阶段图
├── crates/                                      语料库清点工具和命令行
├── config/                                      设置文件（所有调整值）
├── Cargo.toml Cargo.lock                        Rust 工作区及其锁定的依赖
├── tools/                                       仓库检查、其测试以及图片工具
├── images/                                      横幅图以及架构和仓库结构图片
├── .github/                                     CI 与仪表板工作流、Issue 和 PR 模板
├── eh3d.project.json                            清单：版本、角色、成熟度和构建命令
├── VERSION                                      仓库的版本
├── CHANGELOG.md                                 每个版本的变更
├── SECURITY.md SUPPORT.md                       如何报告漏洞及在哪里提问
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   参与贡献、行为准则和许可证
```

</details>

## 🗺️ 路线图与已知限制

**目前已知的限制**

* 早期代码：只有语料库清点工具，并且只在它自己的测试生成的文件夹上运行过，从未用于真实的设计文件。
* 没有设计模型，没有设计文件的读取器或写入器，没有编辑器，也没有规则；除脱敏模式外，没有任何需求已经开始。
* 用户界面、几何库和坐标范围仍是未决事项（见 `docs/STATUS.md`）。

**下一步**

* 待本项目所依赖的契约被接受后（见 `docs/DEPENDENCIES.md`），再编写模型和首批读取器及其测试，并用真实结果替换状态行。

## 🌍 语言

README 和文本提供英语、西班牙语、法语、意大利语、德语、汉语、日语和欧洲葡萄牙语版本。

## 🔗 相关项目

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS)：套件地图、架构、约定和共享检查
* 计划中的关系，尚未构建（EH3D-COMMON, EH3D-CORE-LIBS, EH3D-SERVER, EH3D-PHOENIX-CAD, EH3D-VALIDATION-LAB, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE）：见 [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 📚 文档

* [套件的约定](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [变更日志](CHANGELOG.md)
* [范围、需求、依赖、威胁模型和决策](docs/SCOPE.md)

## 🤝 参与贡献与社区

* 提交 pull request 之前运行 `python -I tools/ci_validate.py` 和基础部分的测试：必须全部通过。见 [CONTRIBUTING.md](CONTRIBUTING.md) 和[行为准则](CODE_OF_CONDUCT.md)。
* README 有八种语言：修改文字时要同步修改全部八个文件，两张图片由 `tools/make_architecture.py` 和 `tools/make_structure.py` 根据 `docs/architecture.json` 和 `docs/structure.json` 绘制。
* 请私下报告漏洞：[SECURITY.md](SECURITY.md)。问题、想法和报告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
