<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-PCB
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB：计划作为具备原理图输入、PCB 布局、验证工具、电路仿真以及开关电源和射频专用模块的 EDA 环境。目前只写了语料库清点工具。

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-431-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**诚实性核对 - 今天真正能运行的内容：** 一个 Rust 工作区，其命令 `phoenix inventory` 用来清点私有文件夹中的设计文件，而不显示任何名称或数值：按扩展名、类型和声明版本统计的文件数，以及内容的一个指纹。它有 3123 个自己的测试，在计算机上通过。它只在自己的测试生成的文件夹上运行过，从未用于真实的设计文件。它还有设计模型的基础部分（带范围检查的纳米和微度整数值、对象标识和规范 JSON）、项目文件 `.phoenixproj`（清单、领域、迁移）以及带可撤销操作的电路板模型（层、网络、元件、走线、过孔、覆铜区），都只在生成的数据上试过。电路板库还能判断哪些铜相互接触，并依据规则文件检查尺寸和间距；第五个库把 EAGLE XML 的库、电路板和原理图读成摘要，并把 XML 树原样写回。所有这些都只在生成或手工制作的文件上试过。另有原理图模型及其与电路板的一致性检查。没有把外部设计导入模型的功能，没有编辑器，规则也只覆盖计划检查的一部分：EDA 的大部分功能仍在计划中。电路板模型还包含丝印文字和线条（带内置笔画字体），规则检查和 Gerber 导出会用到；原理图可以画在图纸上，带有已放置的元件和连线，并与网络表及电路板比对；电路板可以写成 EAGLE 的 XML 文件并读回，但只用本代码写出的文件检查过，从未在 EAGLE 中打开；EAGLE 二进制文件与其 XML 配对的约定已写好，但没有二进制文件的读取器。 叠层与传输线计算器（`phoenix stackup`：按已发表公式计算微带线、带状线、共面波导和耦合线对的阻抗、有效介电常数、损耗、延迟和线宽）处于原型阶段，从未与场求解器或实测结果对比。使用 EH3D-COMMON 的材料数据库（`phoenix materials`、`phoenix stackup --material ID`）时，层的 Dk 和 Df 取自数据手册公布的数据点，从不外推；这些数值均未与电路板实测结果对比。一个连通性与规则库（`eh3d-phoenix-rules`）从规则文件或 EAGLE 电路板的设计规则中读取限值，按固定顺序检查电路板（线宽、过孔、钻孔、焊环、间距、板边、钻孔间距、阻焊、铜区、丝印），找出短路和未布线的网络，并把电路板的网络与原理图的网络比对；命令 `phoenix check` 和 `phoenix connectivity` 会打印报告，已确认和无法判定的结果分开列出；它只在手工制作的电路板上试过，从未与参考工具比对。 另有设计规则检查（`phoenix check --advanced`：焊环、到板边的距离、酸液陷阱、按电压的间距、铜岛、断头走线、重复过孔、不属于任何网络的焊盘和超出板框的元件，限值和级别在 `config/drc_advanced.cfg` 中）同样是原型，只在手工制作的电路板上试过，从未与制造商的规则或其他工具比对；间距表中的数值未对照标准核实。 制造导出（`phoenix export`）为每一层生成 Gerber X2 文件，生成镀孔和非镀孔的 Excellon 钻孔文件、贴装坐标文件和物料清单，配置取自 JSON 文件，并带有 SHA-256 清单，以及回读文件并与电路板比对的检查；它是原型，文件只由本仓库的解析器读过，从未在独立的查看器中或由制造商打开。在此之上，`phoenix pnp`、`phoenix stencil` 和 `phoenix fabpack` 写出贴装坐标文件、锡膏钢网的开孔和带清单的确定性 zip 包；它们是原型，从未装入机器、做成钢网或送交制造商。射频工具集（`eh3d-phoenix-rf`，命令 `phoenix rf`）可读写 Touchstone 文件，转换 S、Z、Y 和 ABCD 参数，计算驻波比、损耗、群延迟和稳定性，为复阻抗设计 L 型网络、单枝节和四分之一波长变换段，给出史密斯圆图的坐标，并用理想元件设计巴特沃斯、切比雪夫和贝塞尔梯形滤波器；它是原型，只用教科书数值验证，从未与网络分析仪、场求解器或其他射频程序对比。KiCad 读取器（`eh3d-phoenix-kicad`，命令 `phoenix kicad-read`）把 KiCad 6 至 9 的符号库、封装、带各页的原理图、电路板和项目文件读入与 EAGLE 文件相同的模型，并给出导入报告和保真度级别；它是原型，只用为测试手写的文件试过，从未与 KiCad 对比，也从未在公开的设计集合上运行。 库清理器（`eh3d-phoenix-sanitizer`，命令 `phoenix sanitize`）读取一个旧 EAGLE 库的文件夹，报告重复或近乎重复的封装、符号和器件集，附带规范几何哈希、相对阈值的得分、HTML 与 JSON 报告以及主库计划；它只做分析，从不改写库，也从不应用该计划，并且它是原型，只用其自身测试生成的库试过，从未在真实的库上运行。电气规则检查（`eh3d-phoenix-erc`，命令 `phoenix erc`）评判原理图：不在任何网络上的引脚、只有一个引脚的网络、设置文件中的矩阵不允许同时出现的引脚类型、没有驱动的输入、没有电源的供电输入、相似的网络名、重复的位号、空的值、没有封装的元件、悬空的导线、无人驱动的网络、附近没有去耦电容的芯片电源引脚、可能是拼写错误的网络名和不接触任何东西的标签，已确认和无法判定的结果分开列出；同步（`eh3d-phoenix-sync`，命令 `phoenix sync`）按元件的身份比较原理图与电路板，并把向电路板传递或反向标注回原理图的更改作为软件包执行，先试运行，可撤销和重做；反向标注还可作为计划给出，每项更改带有状态（可应用、冲突或忽略）和确认码（`phoenix annotate`），两份文档也可比较（`phoenix sync --diff`）；第二种格式的规则文件还包含网络类，为走线和过孔设定各自的限值。这一切都是原型，只在手工制作的以及测试生成的原理图和电路板上试过，从未与参考工具比较；原理图编辑器（命令 `phoenix schedit` 和窗口的编辑模式）同样是原型，只在没有屏幕的情况下试过：可撤销的操作、跟随图纸的网表、每次更改后的规则检查，以及向电路板的传送。电路仿真器（`eh3d-phoenix-sim`，命令 `phoenix sim`）可读写 SPICE 风格的网表，把原理图转换为网表，并用自研的改进节点分析求解线性电路、含二极管的电路（含串联电阻、结电容和击穿）、双极型和 MOS 晶体管、电压控制开关以及运算放大器宏模型（直流工作点和单个信号源的直流扫描、带群延迟的交流扫描、噪声分析，以及步长会在信号源转折点停下的瞬态分析），还带有与射频工具梯形滤波器的一致性检查，并可读取网表中的 `.op`、`.dc`、`.tran` 和 `.ac` 行以及同一文件夹内文件的 `.include`；它是原型，仅用教科书数值和闭式解测试，从未与 SPICE 或测量对比，其器件模型都很简单。 它还可以对数值列表重复运行、从结果中取测量值、以固定种子对公差做蒙特卡罗，并由交流响应的拟合得到噪声、极点和零点；这同样是原型，只用闭式解的算例检验过，从未与 SPICE 对比。 编辑与布线逻辑库（`eh3d-phoenix-route`，命令 `phoenix route` 和 `phoenix ratsnest`）列出尚未布线的连接，并在同一网络的两个元件之间以 0、45 和 90 度的线段布线，可在障碍物前停止或绕过障碍物，线宽取自网络类，过孔尺寸由规则决定；它的编辑是可撤销的操作，会报告对规则检查新增了什么。它是原型，仅在手工制作和由测试生成的电路板上试过，没有圆弧和铜区，用它进行编辑的窗口（`phoenix view`）从未在屏幕上看过。 通过外部自动布线器布线是经由文件完成的（`phoenix autoroute`）：电路板被写成 Specctra DSN 文件，Specctra 会话被读取为一个计划，其走线用规则检查，并且要么全部应用、要么全不应用；启动程序的适配器默认关闭，没有设置文件中的开关和命令行选项就不会启动任何程序。它是原型，只用手工制作的会话试过，从未配合真实的自动布线器使用。 脚本解释器（`eh3d-phoenix-script`，命令 `phoenix script run`、`phoenix script report` 和 `phoenix cam-import`）读取以前工作流命令语言的脚本文件，把 `WIRE`、`VIA`、`MOVE`、`CHANGE`、`DELETE`、`DRC` 以及其他有对应实现的命令作为电路板的可撤销操作执行，要么全部成功要么全部不生效，撤销可逐字节还原原件，并把每个脚本的每条命令列为等价、已调整或待处理并给出原因，还能把输出作业读入导出配置；它依据该语言的公开说明构建，是原型，仅用为测试编写的脚本试过，从未与原工具对比，也从未在作者自己的脚本上运行。 射频结构生成器（`eh3d-phoenix-rfgen`，命令 `phoenix rfgen`）根据叠层和频率，用已发表的公式绘制微带线、切角弯和圆弧弯、宽度过渡、枝节、匹配网络、阶梯阻抗低通滤波器和耦合线带通滤波器、威尔金森功分器、分支线耦合器、贴片天线和偶极天线，以及带过孔围栏的 SMA 转接，并把它们作为电路板的可撤销操作放到板上，要么全部成功要么全部不生效，同时给出尺寸、所用公式和结果规则检查的报告；它是原型，仅用教科书数值和由测试生成的电路板试过，从未与电磁仿真器或测量对比。KiCad 写入器（命令 `phoenix kicad-export`）把电路板、原理图、封装和符号写成 KiCad 8 文件，并附上模型无法表达内容的报告；它是原型，写出的文件从未在 KiCad 中打开过。差分对（命令 `phoenix pairs`，以及 `phoenix boardedit` 的操作 `route_pair`、`tune_pair` 和 `fix_pair_gap`）把一对走线并排画出，用蛇形线使长度相等，修正间距，并检查间距、对称性、未耦合段、长度、过孔、宽度，以及在有叠层时检查阻抗；它们是原型，仅用手工制作和由测试生成的电路板试过，窗口中的差分对工具从未在屏幕上见过。推挤布线（`phoenix boardedit` 的操作 `route_shove`）会在文件设定的深度和走线数量内，把新走线所挤压的其他网络的走线平行推开，要么全部完成要么不做，若会留下规则错误则被拒绝；走线中的圆弧（`route_arc`、`round_corners`）以及 45 度线段上的蛇形线也随之而来；它们是原型，仅用手工制作和由测试生成的电路板试过，窗口中的推挤工具从未在屏幕上见过。 打印（`phoenix print`、`eh3d-phoenix-print`）把原理图的各张图纸、电路板的装配图和各层以及拼板写成 SVG 或手写的 PDF；它是原型，其 PDF 只被自身 crate 的检查器读过，从未在真正的 PDF 阅读器中打开过。

---

> **安全（safety）与信息安全现状：** 本仓库刚刚起步，不是安全系统（safety）。这里写的任何内容都不能取代机床及其控制器的限位、联锁和急停。

## 📋 速览

| | |
|---|---|
| 版本 | `0.0.4` |
| 成熟度 | 脚手架 |
| 测试 | 431 |
| 技术栈 | Rust, Python 3.10+ |
| 真实硬件 | 尚未在硬件上运行 |

**跳转到：** [概述](#-概述) · [架构](#-架构) · [快速开始](#-快速开始) · [测试](#-测试) · [仓库结构](#-仓库结构) · [相关项目](#-相关项目)

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 仓库检查 | 🟡 | 仓库检查、图片检查以及基础部分的 3 个测试都在 CI 中运行并通过。 |
| 测试 | 🟡 | Rust 代码的 3123 个测试和基础部分的 3 个测试在计算机上通过。它们只使用测试自己生成的文件夹；没有任何一个在真实设计文件上运行过。 |
| 真实硬件 | 🟠 | 没有在任何机床或电路板上运行过：这里没有任何内容在硬件上试过。 |
| 计划的功能 | 🟠 | 已编写：语料库的脱敏清点、模型的基础部分（单位、标识、规范 JSON）、项目文件 `.phoenixproj`、带撤销和重做的电路板模型及其连通性、首批规则检查，以及 EAGLE XML 的摘要读取器。未开始：把 KiCad 以外的格式导入模型、其他格式的写入器、大部分规则、编辑器和正式的查看器、仿真器与 SPICE 的对比以及其余所有计划内容。电路板模型还包含丝印文字和线条（带内置笔画字体），规则检查和 Gerber 导出会用到；原理图可以画在图纸上，带有已放置的元件和连线，并与网络表及电路板比对；电路板可以写成 EAGLE 的 XML 文件并读回，但只用本代码写出的文件检查过，从未在 EAGLE 中打开；EAGLE 二进制文件与其 XML 配对的约定已写好，但没有二进制文件的读取器。 叠层与传输线计算器（`phoenix stackup`：按已发表公式计算微带线、带状线、共面波导和耦合线对的阻抗、有效介电常数、损耗、延迟和线宽）处于原型阶段，从未与场求解器或实测结果对比。使用 EH3D-COMMON 的材料数据库（`phoenix materials`、`phoenix stackup --material ID`）时，层的 Dk 和 Df 取自数据手册公布的数据点，从不外推；这些数值均未与电路板实测结果对比。射频工具集（`phoenix rf`：Touchstone 文件、二端口参数、匹配和理想梯形滤波器）是原型，只用教科书数值验证。KiCad 读取器（`phoenix kicad-read`：符号、封装、带各页的原理图、电路板和项目文件）是原型，只用手写文件试过。 库清理器（`phoenix sanitize`：EAGLE 库文件夹的清点、几何哈希、带阈值的近似匹配、重复分组、HTML 与 JSON 报告以及从不应用的主库计划）是原型，只用测试生成的库试过。电气规则检查（`phoenix erc`）以及原理图与电路板的同步（`phoenix sync`：向电路板传递与反向标注，以可撤销的软件包形式进行）为原型，仅在手工制作的原理图和电路板上试过。自研的电路仿真器（`phoenix sim`：SPICE 风格网表、原理图网表，以及线性电路和含二极管电路的直流、交流与瞬态分析）是原型，仅用教科书数值检查，从未与 SPICE 对比。 编辑与布线逻辑（`phoenix route`、`phoenix ratsnest`：未布线的连接、0、45 和 90 度线段的布线器，以及带实时规则检查的可撤销编辑）为原型，仅在手工制作和由测试生成的电路板上试过。 脚本解释器（`phoenix script run`、`phoenix script report`、`phoenix cam-import`：把以前工作流命令语言的脚本作为电路板的可撤销操作执行，要么全部成功要么全部不生效，并按命令和按脚本给出兼容性报告，同时把输出作业读入导出配置）是原型，依据该语言的公开说明构建，仅用为测试编写的脚本试过。 射频结构生成器（`phoenix rfgen`：微带线、弯、枝节、匹配网络、滤波器、威尔金森功分器、分支线耦合器、贴片天线和偶极天线，以及带过孔围栏的 SMA 转接，根据叠层和频率作为电路板的可撤销操作绘制，并给出公式与规则检查的报告）是原型，仅用教科书数值检查，从未与电磁仿真器或测量对比。 一组基准用例（`phoenix oracle`）用已发布的数值及其出处和容差检查这些计算；这不是与 EAGLE、KiCad、场求解器或测量的对比，上面的任何状态都不会因此改变。 自有语料库（`phoenix corpus`）收录手写的小型电路板、原理图、库和制造文件，数值由手工算出，包含必须被拒绝的文件、格式之间的往返转换以及对生成电路板的性质检查，属于原型；其中没有任何厂商或用户的文件，也不是与 EAGLE、KiCad 或 CAM 程序的对比。 |
| 窗口 | 🟠 | 电路板与原理图的查看器兼编辑器（`phoenix view`，egui，位于 `gui` 特性之后）已编写，其逻辑已测试：电路板可在撤销、重做和每次编辑的规则检查结果支持下编辑，原理图可检查、与电路板同步并编辑（放置、移动、旋转、镜像、删除、复制和粘贴、导线、标签、电源端口和总线标签，每次更改后进行规则检查），且绝不会覆盖已读取的文件；但窗口从未在屏幕上看过：屏幕上的绘制、鼠标操作和流畅度均未验证。同一窗口还有库模式，可编辑 EAGLE 库的符号、封装和器件（`phoenix libedit`、`phoenix footprint`、`docs/LIBRARY_EDIT.md`），带撤销、库检查和封装生成器；它同样从未在屏幕上看到过。 电路板编辑模式（`phoenix boardedit`，`docs/BOARD_EDIT.md`）可放置元件、绘制 45 度走线和过孔、粘贴网络并用蛇形线调整长度，每个操作都由设计规则检查；它从未在屏幕上看过，也从未用鼠标使用过。 用于文件夹中 Gerber 和 Excellon 文件的窗口（`docs/VIEWER.md`：每个文件一层、不透明度、测量以及与电路板的比较）和装配后电路板的三维窗口（环绕相机、单击选择元件、标记碰撞和高度）都是原型；二者都从未在屏幕上看过，也从未用鼠标使用过。 程序 `phoenix-gui` 以开始画面打开它（打开文件、新建库、最近的文件、拖放文件），在 Windows 上不带控制台窗口。 窗口的通用部分（搜索、测量、图层预设、历史列表、主题、缩放与可更改的按键、帮助和记住的偏好设置）同样是原型，从未在屏幕上见过。 |
| 需求 | 🟠 | 30 项需求中有 19 项（脱敏模式、模型基础部分、可撤销操作、项目文件、诊断代码、解析式射频计算器、连通性与规则检查以及带软件包的制造导出、电路板与原理图查看器兼编辑器KiCad 读取器和写入器、库清理器、原理图检查与同步电路仿真器、布线逻辑以及脚本解释器）处于原型阶段，11 项仅为计划，没有一项已验证。 |

## 🎯 概述

* **已有的内容：** 一个工作区，其中的库以脱敏模式清点私有的设计文件语料库（只有计数、类型、声明版本和一个指纹，绝不含名称或数值），以及一个命令行 `phoenix inventory`。它只查看文件的前几个字节，不读取设计内容。第二个库保存设计模型的基础部分：带范围检查的长度和角度整数、对象标识（UUID v7）和规范 JSON。第三个库在 EH3D-COMMON 的共享容器上读写 `.phoenixproj` 项目：带领域、链接和迁移的清单，无法理解的领域原样保留，以及 `must_understand` 规则。第四个库保存电路板模型：层、网络、封装、元件、由直线和圆弧组成的走线、过孔和覆铜区，用稳定标识相连，带可撤销操作和撤销/重做历史。每种失败都有稳定的代码 `PHOENIX-<领域>-<NNN>`。电路板库还会计算每个网络的连通岛，并依据以毫米为单位的规则文件检查线宽、钻孔、焊环和间距，把已确认和无法判定的结果分开列出。第五个库把 EAGLE XML 的库、电路板和原理图读成摘要，把 XML 树原样写回，并在用 SHA-256 核对过的语料库副本上测量读取率，不改动原件；命令 `phoenix eagle-rate` 可运行它。第六个库通过 EH3D-CORE-LIBS 和 EH3D-COMMON 写出电路板的 Gerber X2、Excellon 和 `.eh3dproj` 1.0.0 文件，同一块板得到相同的字节；这些文件只用同一个库的解析器读回过。第七个库保存原理图（用稳定标识相连的符号、元件和网络），并与电路板核对：没有对应元件的零件，以及位号、值、封装和网络的差异，使用代码 `PHOENIX-LINK-<NNN>`。另有一个库 `eh3d-phoenix-rules`，从铜箔（用几何和容差连接的焊盘、走线、过孔和铜区）推出网络，与原理图的网络比对，并按固定顺序对照从规则文件或 EAGLE 电路板设计规则中读取的限值运行设计规则，已确认和无法判定的结果分在两个列表中；命令 `phoenix check` 和 `phoenix connectivity` 以八种语言的文本或 JSON 打印其报告。

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

**下载并打开窗口。** 发布版的压缩包（Windows 64 位和 32 位、Linux 和 macOS）包含两个程序：命令行 `phoenix`，以及查看器兼编辑器的窗口 `phoenix-gui`。双击 `phoenix-gui`（Windows 上为 `phoenix-gui.exe`）会打开开始画面，可以在其中打开电路板、原理图或库，选择最近的文件，或把文件拖放到窗口上；`phoenix-gui <文件>` 会直接打开文件。在 Windows 上它没有控制台窗口，因此打不开时，原因会写入用户设置文件夹中的 `last_error.txt`（Windows 上为 `%APPDATA%\EH3D-PHOENIX-PCB`，其他系统为 `~/.config/eh3d-phoenix-pcb`）；最近文件列表（`recent.cfg`）也保存在那里，保留的数量由 `config/viewer.cfg` 设定。单独运行 `phoenix` 只输出文字，不会打开窗口。该窗口是尚未用鼠标试过的原型（见上面的状态）。构建这两个程序：`cargo build --release -p eh3d-phoenix-cli --features gui`。

## 🧪 测试

代码有 3123 个测试（`cargo test`），基础部分有 3 个（`python -I -m unittest discover -s tools -p "test_*.py"`）；全部在计算机上通过。CI 运行仓库检查、图片检查、基础部分的测试，并对 Rust 代码在 Linux 和 Windows 上运行格式检查、把警告当作错误的 clippy 以及测试。

## 🏗️ 构建与发布

版本号位于 `VERSION` 文件和 `eh3d.project.json`，每个版本在 [CHANGELOG.md](CHANGELOG.md) 中都有条目。`cargo build --release` 构建库和 `phoenix` 程序；版本发布在 GitHub 上。 发布版的压缩包包含 `phoenix`，旁边是窗口程序 `phoenix-gui`；`cargo build --release -p eh3d-phoenix-cli --features gui` 会同时构建两者。

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
│   ├── ROADMAP.md                               从基础到一体化套件的阶段图
│   ├── ERROR_CODES.md                           诊断代码 PHOENIX-<领域>-<NNN>
│   ├── STACKUP.md                               叠层与传输线计算、公式
│   ├── RF.md                                    射频工具：Touchstone、二端口、匹配、滤波器
│   ├── RFGEN.md                                 电路板上的射频结构：公式、来源与限制
│   ├── KICAD.md                                 KiCad 读取器：每种格式读取的内容
│   ├── KICAD_IMPORT.md                          读取完整的 KiCad 项目：命令与报告
│   ├── KICAD_EXPORT.md                          写出 KiCad 文件：写出的内容、报告与命令
│   ├── VIEWER.md                                查看器兼编辑器原型及其基准测试
│   ├── SHORTCUTS.md                             窗口的键盘快捷键及其修改方法
│   ├── SANITIZER.md                             库清理器：重复项、哈希、报告与计划
│   ├── SCRIPT.md                                脚本解释器：命令、报告与输出作业
│   ├── ERC.md                                   电气规则检查与同步
│   ├── ANNOTATE.md                              反向标注回原理图与比较
│   ├── SCHEMATIC_EDIT.md                        原理图编辑器：操作、网络、窗口
│   ├── HIERARCHY.md BUSES.md                    图纸符号、重复图纸和总线
│   ├── LIBRARY_EDIT.md                          库编辑器：编辑、检查、生成器
│   ├── IPC_LIBRARY.md                           自有封装库：系列、密度、检查
│   ├── LIBINDEX.md                              库的批量导入：质量报告、索引
│   ├── BOARD_EDIT.md                            电路板编辑器：操作、规则检查、窗口
│   ├── SHOVE.md                                 走线推挤与圆弧：操作、设置、窗口
│   ├── BOARD_ZONES.md                           电路板编辑器中的铜区域：填充、优先级、粘贴
│   ├── SIM.md                                   电路仿真：网表、分析、原理图导出
│   ├── SIM_NONLINEAR.md                         仿真器：非线性器件、直流扫描、收敛
│   ├── SIM_ADVANCED.md                          仿真器：扫描、测量、蒙特卡罗、极点
│   ├── NETLIST.md                               原理图的网表：模型表、报告
│   ├── ROUTE.md                                 编辑与布线：飞线、布线器、可撤销编辑
│   ├── DRC_ADVANCED.md                          高级设计规则检查及其设置
│   ├── AUTOROUTE.md                             外部自动布线器布线：DSN、会话、适配器
│   ├── ZONES.md                                 铜区填充：裁剪、热焊盘、孤岛、检查
│   ├── LENGTHS.md                               长度、等长分组与差分对
│   ├── DIFFPAIRS.md                             差分对：定义、操作、检查与命令
│   ├── GERBER_IMPORT.md                         用于审阅的 Gerber 与 Excellon 导入，以及往返检查
│   ├── GERBER_X2.md                             导出中的 Gerber X2 属性及其检查
│   ├── PANEL.md                                 拼板：电路板、边框、连接桥、V 形槽、基准点、工具孔、各层
│   ├── PRINT.md                                 打印：带标题栏的 SVG 与 PDF 页面
│   ├── BOM.md                                   带成本的物料清单、CSV、JSON、HTML
│   ├── PARTS.md                                 本地元件库：搜索、检查、与物料清单的关联
│   ├── PNP.md STENCIL.md FABPACK.md             贴片坐标、钢网和制造包
│   ├── ASSEMBLY3D.md                            3D 装配：STEP 模型、碰撞、OBJ、STEP
│   ├── DXF.md                                   DXF：板框与板、拼板导出
│   ├── ENCLOSURE.md                             电路板与外壳核对，STEP 实体
│   ├── SPATIAL.md                               检查的空间索引与 perf 报告
│   ├── ORACLE.md                                基准用例以及与 EAGLE、KiCad 对比的计划
│   ├── CORPUS.md                                自有语料库：手写样例、往返转换、性质
│   ├── GATE_F1.md                               第一阶段关口检查什么、不检查什么
│   └── adr/                                     决策记录
├── AGENTS.md                                    修改仓库的人需遵守的规则
├── deny.toml                                    依赖项的许可证与公告策略
├── crates/                                      清点、模型、项目、电路板、原理图、EAGLE、KiCad、规则、ERC、同步、枢纽、导出、Gerber 导入、拼板、叠层、射频、仿真器、布线器、铜区、长度、物料清单、3D 装配、清理器、脚本、查看器、命令行
├── config/                                      设置文件（所有调整值）
├── Cargo.toml Cargo.lock                        Rust 工作区及其锁定的依赖
├── tools/                                       仓库检查、其测试以及图片工具
├── tests/                                       基准用例以及手写样例的自有语料库
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

* 早期代码：只有语料库清点工具和模型的基础部分；清点工具只在它自己的测试生成的文件夹上运行过，从未用于真实的设计文件。
* 没有原理图，没有其他设计文件的读取器或写入器，没有编辑器，也没有规则；项目文件和电路板模型只在生成的数据上试过，从未用于真实的设计。
* 用户界面的最终技术栈仍未决定（临时选择记录在 `docs/adr/`），几何库也是如此（见 `docs/STATUS.md`）。

**下一步**

* 待本项目所依赖的契约被接受后（见 `docs/DEPENDENCIES.md`），再编写模型和首批读取器及其测试，并用真实结果替换状态行。

## 🌍 语言

README 和文本提供英语、西班牙语、法语、意大利语、德语、汉语、日语和欧洲葡萄牙语版本。

## 🔗 相关项目

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS)：套件地图、架构、约定和共享检查
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON)：与本仓库并排构建；本项目使用的共享项目容器（`eh3d-container`）和通用诊断（`eh3d-diagnostic`）
* PHOENIX 的兄弟仓库：[EH3D-VALIDATION-LAB](https://github.com/JuanenRac/EH3D-VALIDATION-LAB)（语料库和标准文件预言机）和 [EH3D-PHOENIX-CAD](https://github.com/JuanenRac/EH3D-PHOENIX-CAD)（机械方面），均为早期代码
* 计划中的关系，尚未构建（EH3D-CORE-LIBS, EH3D-SERVER, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE）：见 [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

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
