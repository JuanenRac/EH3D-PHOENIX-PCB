# 把检查结果写成表格和网页，并按制造商的能力档案检查电路板

运行设计规则，把结果保存为 CSV 表格和按类别分组的 HTML 网页，然后用制造商的能力档案检查一块小的 EAGLE 电路板。示例中的档案是虚构的：它只展示文件的形式，并不是任何制造商的能力。

请在仓库根目录运行命令；参见[如何运行指南](README.md)。

1. 运行规则文件中的规则并写出两份报告。每个选项都写入新文件：已存在的文件不会被替换，若有一个输出被拒绝则什么也不写（退出码 2）。电路板的走线比文件要求的细，所以有一条已确认的发现，退出码为 1。

```console
$ phoenix check --rules examples/tutorials/rules_strict.json --csv build/check.csv --html build/check.html examples/tutorials/kicad/demo.kicad_pcb
```

2. 用另一种语言写出同样的报告。CSV 的标题列和网页的标题遵循 `--lang`；列、类别和严重程度的键保持英文，脚本因此可以读取任何语言的报告。

```console
$ phoenix check --lang es --rules examples/tutorials/rules_strict.json --csv build/check_es.csv --html build/check_es.html examples/tutorials/kicad/demo.kicad_pcb
```

3. 原理图与电路板的比较也接受这两个选项。

```console
$ phoenix connectivity --csv build/connectivity.csv --html build/connectivity.html examples/tutorials/tut.sch examples/tutorials/tut.brd
```

4. 用档案 `examples/guides/dfm_small_shop.cfg` 检查 EAGLE 电路板。使用 `--dfm` 时，档案的限值取代电路板自身的规则，所以不要再给 `--rules`。电路板为 30 mm × 20 mm，而档案在短边上只允许 10 mm：得到代码为 `PHOENIX-DFM-003` 的错误，退出码为 1。铜层多于一层的电路板还会得到 `PHOENIX-DFM-001`，信息 `PHOENIX-DFM-006` 提示该档案未经验证。

```console
$ phoenix check --dfm examples/guides/dfm_small_shop.cfg --csv build/dfm.csv --html build/dfm.html examples/tutorials/tut.brd
```

5. 再看仓库里的示例档案 `config/dfm/example.cfg`。电路板在它的所有限值之内，所以“能力”类别中唯一的发现就是该档案未经验证的提示信息。

```console
$ phoenix check --dfm config/dfm/example.cfg --html build/dfm_example.html examples/tutorials/tut.brd
```

## 如何阅读结果

* CSV 的表头是 `list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail`，每条发现一行，已证明的在前（`list` 为 `confirmed` 或 `inconclusive`）。电子表格会当作公式执行的文本单元格，前面会加一个引号。
* HTML 网页不含脚本和外部文件，并对来自电路板的所有文本做了转义，可以放心在浏览器中打开。页面开头是每个类别的已证明和未定论发现的数量，之后是每个有发现的类别各一张表；档案的发现属于类别 `capability`。
* 档案是由 `键 = 值` 行组成的文本文件，单位为纳米；缺少的行或值为 0 的行表示不启用。复制 `config/dfm/example.cfg`，填入你在制造商文档中读到的数值，在 `source` 中写明出处，然后才把 `verified` 设为 `true`。没有任何东西会把该文件与制造商核对。

## 另见

* [docs/DRC_ADVANCED.md](../../../../docs/DRC_ADVANCED.md)
* [docs/DFM.md](../../../../docs/DFM.md)
* [docs/RULES.md](../../../../docs/RULES.md)
