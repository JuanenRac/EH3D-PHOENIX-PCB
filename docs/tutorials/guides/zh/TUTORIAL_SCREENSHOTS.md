# 用 phoenix 的截图模式为窗口截图

`phoenix --shot FILE.png` 会打开窗口，等待几帧，把窗口画面保存为 PNG 文件，然后自行关闭。README 中的图片就是这样做出来的。在此模式下窗口既不读也不写偏好设置和最近文件列表，所以运行后你的设置保持原样。`phoenix` 是窗口程序，在发布压缩包中与 `phoenix` 放在一起；它用 `gui` 特性构建。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 给起始画面截图，指定尺寸、主题和等待的帧数。尺寸是以像素为单位的 `宽x高`，每边 320 到 8192；主题为 `dark`、`light` 或 `contrast`。

```console
phoenix --shot build/start.png --size 1280x720 --theme dark --wait 6
```

2. 给电路板窗口截图：`--open` 指定文件，显示的窗口默认是 `document`。电路板窗口需要 `--open`，起始画面则不能带它。

```console
phoenix --shot build/board.png --open examples/tutorials/led_board.brd --theme light --wait 12
```

3. 用 `--window` 给其他窗口截图：`3d`、`panel`、`gerber`、`search`、`measures`、`layers`、`history`、`appearance`、`shortcuts` 或 `help`（`3d`、`panel` 和 `gerber` 需要 `--open`）。`--lang` 设定窗口语言。

```console
phoenix --shot build/help_pt.png --window help --lang pt --size 1280x720
```

4. 用仓库里的工具为一个版本拍全套图片。`--dry-run` 只打印命令行而不运行任何东西；不加它时，工具会为每个窗口启动一次程序，检查每个 PNG（不能为空或单色，且小于 400 KB），并写出文件 `phoenix-pcb-NN-窗口名.png`。

```console
py -3.14 -I tools/make_shots.py --exe PATH/phoenix --out build/screens --dry-run
```

## 如何阅读结果

* 图片是窗口绘制的内容，以物理像素计，所以在启用缩放的屏幕上可能比 `--size` 更大。退出码：图片写出为 0，没有写出为 1，参数有误为 2。已存在的文件除非给出 `--replace`，否则不会被覆盖；不加它时，程序会在打开窗口之前就停止。
* 该工具每次运行都在临时文件夹中使用自己的设置文件夹，所以你的设置和最近文件不会被读写。截图本身还没有在真实屏幕上运行过，在你亲眼看到之前，不能断言任何图片看起来是正确的。

## 另见

* [docs/VIEWER.md](../../../../docs/VIEWER.md)
* [tools/make_shots.py](../../../../tools/make_shots.py)
