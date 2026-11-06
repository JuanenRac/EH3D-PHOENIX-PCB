// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of `phoenix help` in the eight languages (en, es, de, fr, it, ja, zh, pt): one
//! paragraph for each topic and the labels of the listings.

use crate::commands::Topic;

/// The text of one topic of the help.
pub struct TopicText {
    /// The topic.
    pub topic: Topic,
    /// The word that names it on the command line.
    pub key: &'static str,
    /// A document that belongs to the topic and to no command, if there is one.
    pub extra_doc: Option<&'static str>,
    /// The paragraph in en, es, de, fr, it, ja, zh and pt.
    pub text: [&'static str; 8],
}

/// The topics, in the order of the listing.
pub static TOPICS: &[TopicText] = &[
    TopicText {
        topic: Topic::Schematic,
        key: "schematic",
        extra_doc: None,
        text: ["Capture and check a schematic: the electrical rules, sheets and buses, editing with undo, the transfer to the board and the annotation back.", "Captura y comprobación de un esquema: reglas eléctricas, hojas y buses, edición con deshacer, paso a la placa y anotación de vuelta.", "Schaltplan erfassen und prüfen: elektrische Regeln, Blätter und Busse, Bearbeiten mit Rückgängig, Übertragung auf die Platine und Rückannotation.", "Saisir et vérifier un schéma : règles électriques, feuilles et bus, édition avec annulation, transfert vers la carte et annotation retour.", "Acquisire e controllare uno schema: regole elettriche, fogli e bus, modifica con annullamento, trasferimento alla scheda e annotazione di ritorno.", "回路図の入力と検査：電気ルール、シートとバス、取り消し付き編集、基板への反映と逆アノテーション。", "原理图的录入与检查：电气规则、图页与总线、可撤销的编辑、同步到电路板以及反向标注。", "Captura e verificação de um esquema: regras elétricas, folhas e barramentos, edição com desfazer, passagem para a placa e anotação de volta."],
    },
    TopicText {
        topic: Topic::Board,
        key: "board",
        extra_doc: None,
        text: ["Check, edit and route a board: design rules, connectivity, routing, zones, reversible edits and the window.", "Comprobar, editar y rutear una placa: reglas de diseño, conectividad, ruteo, zonas, ediciones reversibles y la ventana.", "Platine prüfen, bearbeiten und verlegen: Entwurfsregeln, Konnektivität, Verlegen, Flächen, umkehrbare Änderungen und das Fenster.", "Vérifier, éditer et router une carte : règles de conception, connectivité, routage, zones, éditions réversibles et la fenêtre.", "Controllare, modificare e instradare una scheda: regole di progetto, connettività, instradamento, zone, modifiche reversibili e la finestra.", "基板の検査・編集・配線：設計ルール、接続性、配線、ゾーン、元に戻せる編集、ウィンドウ。", "电路板的检查、编辑与布线：设计规则、连通性、布线、铜区、可撤销的编辑和窗口。", "Verificar, editar e encaminhar uma placa: regras de projeto, conectividade, encaminhamento, zonas, edições reversíveis e a janela."],
    },
    TopicText {
        topic: Topic::Libraries,
        key: "libraries",
        extra_doc: None,
        text: ["Footprints, symbols and parts: edit EAGLE libraries, generate footprints by IPC-7351B, index and clean libraries, and the parts and materials bases.", "Huellas, símbolos y piezas: editar bibliotecas EAGLE, generar huellas por IPC-7351B, indexar y limpiar bibliotecas, y las bases de piezas y materiales.", "Footprints, Symbole und Bauteile: EAGLE-Bibliotheken bearbeiten, Footprints nach IPC-7351B erzeugen, Bibliotheken indizieren und bereinigen, Bauteil- und Materialdatenbanken.", "Empreintes, symboles et pièces : éditer des bibliothèques EAGLE, générer des empreintes IPC-7351B, indexer et nettoyer les bibliothèques, bases de pièces et de matériaux.", "Impronte, simboli e componenti: modificare librerie EAGLE, generare impronte IPC-7351B, indicizzare e ripulire le librerie, basi di componenti e materiali.", "フットプリント、シンボル、部品：EAGLE ライブラリの編集、IPC-7351B によるフットプリント生成、ライブラリの索引化と整理、部品・材料データベース。", "封装、符号和元件：编辑 EAGLE 库、按 IPC-7351B 生成封装、为库建立索引并清理，以及元件和材料数据库。", "Footprints, símbolos e peças: editar bibliotecas EAGLE, gerar footprints pelo IPC-7351B, indexar e limpar bibliotecas, e as bases de peças e materiais."],
    },
    TopicText {
        topic: Topic::Manufacturing,
        key: "manufacturing",
        extra_doc: None,
        text: ["From board to factory: Gerber and drill files, the fabrication package, placement, stencil, bill of materials, panels, printing and the enclosure.", "De la placa a la fábrica: ficheros Gerber y de taladros, paquete de fabricación, posición, plantilla, lista de materiales, paneles, impresión y carcasa.", "Von der Platine zur Fabrik: Gerber- und Bohrdateien, Fertigungspaket, Bestückung, Schablone, Stückliste, Nutzen, Druck und Gehäuse.", "De la carte à l'usine : fichiers Gerber et de perçage, paquet de fabrication, placement, pochoir, nomenclature, panneaux, impression et boîtier.", "Dalla scheda alla fabbrica: file Gerber e di foratura, pacchetto di fabbricazione, posizionamento, stencil, distinta base, pannelli, stampa e contenitore.", "基板から工場へ：Gerber とドリルのファイル、製造パッケージ、実装位置、ステンシル、部品表、パネル、印刷、筐体。", "从电路板到工厂：Gerber 与钻孔文件、制造包、贴装坐标、钢网、物料清单、拼板、打印和外壳。", "Da placa à fábrica: ficheiros Gerber e de furação, pacote de fabrico, posicionamento, estêncil, lista de materiais, painéis, impressão e invólucro."],
    },
    TopicText {
        topic: Topic::Simulation,
        key: "simulation",
        extra_doc: None,
        text: ["Simulate a circuit with the solver of this program: netlists, DC, AC and transient analyses, steps, measures, Monte Carlo, noise, poles and zeros.", "Simular un circuito con el solucionador de este programa: netlists, análisis CC, CA y transitorio, barridos, medidas, Monte Carlo, ruido, polos y ceros.", "Eine Schaltung mit dem Löser dieses Programms simulieren: Netzlisten, DC-, AC- und Transientenanalyse, Läufe, Messungen, Monte Carlo, Rauschen, Pole und Nullstellen.", "Simuler un circuit avec le solveur de ce programme : netlists, analyses continue, alternative et transitoire, pas, mesures, Monte Carlo, bruit, pôles et zéros.", "Simulare un circuito con il solutore di questo programma: netlist, analisi DC, AC e transitoria, passi, misure, Monte Carlo, rumore, poli e zeri.", "本プログラムのソルバで回路をシミュレーション：ネットリスト、DC・AC・過渡解析、ステップ、測定、モンテカルロ、ノイズ、極と零点。", "用本程序的求解器仿真电路：网表、直流/交流/瞬态分析、步进、测量、蒙特卡洛、噪声、极点与零点。", "Simular um circuito com o solucionador deste programa: netlists, análises CC, CA e transitória, passos, medidas, Monte Carlo, ruído, polos e zeros."],
    },
    TopicText {
        topic: Topic::Rf,
        key: "rf",
        extra_doc: None,
        text: ["RF tools: Touchstone files, impedance matching, ladder filters and the generator of microstrip structures. Prototypes checked with textbook values.", "Herramientas RF: ficheros Touchstone, adaptación de impedancias, filtros en escalera y el generador de estructuras microstrip. Prototipos comprobados con valores de libros de texto.", "HF-Werkzeuge: Touchstone-Dateien, Impedanzanpassung, Abzweigfilter und der Generator für Mikrostreifen-Strukturen. Prototypen, mit Lehrbuchwerten geprüft.", "Outils RF : fichiers Touchstone, adaptation d'impédance, filtres en échelle et générateur de structures microruban. Prototypes vérifiés avec des valeurs de manuels.", "Strumenti RF: file Touchstone, adattamento d'impedenza, filtri a scala e generatore di strutture microstrip. Prototipi verificati con valori da manuale.", "RF ツール：Touchstone ファイル、インピーダンス整合、ラダーフィルタ、マイクロストリップ構造の生成。教科書の値で確認したプロトタイプです。", "射频工具：Touchstone 文件、阻抗匹配、梯形滤波器和微带结构生成器。均为用教科书数值验证过的原型。", "Ferramentas RF: ficheiros Touchstone, adaptação de impedâncias, filtros em escada e o gerador de estruturas microstrip. Protótipos verificados com valores de manuais."],
    },
    TopicText {
        topic: Topic::Signal,
        key: "signal",
        extra_doc: None,
        text: ["Signal integrity: stack-up and line impedance, lengths and delays, differential pairs, crosstalk, reflections and the eye diagram. Quasi-static estimates.", "Integridad de señal: apilado e impedancia de líneas, longitudes y retardos, pares diferenciales, diafonía, reflexiones y diagrama de ojo. Estimaciones cuasi estáticas.", "Signalintegrität: Lagenaufbau und Leitungsimpedanz, Längen und Laufzeiten, Differenzialpaare, Übersprechen, Reflexionen und Augendiagramm. Quasistatische Schätzungen.", "Intégrité du signal : empilage et impédance des lignes, longueurs et retards, paires différentielles, diaphonie, réflexions et diagramme de l'œil. Estimations quasi statiques.", "Integrità del segnale: stack-up e impedenza delle linee, lunghezze e ritardi, coppie differenziali, diafonia, riflessioni e diagramma a occhio. Stime quasi statiche.", "シグナルインテグリティ：層構成と線路インピーダンス、長さと遅延、差動ペア、クロストーク、反射、アイダイアグラム。準静的な見積りです。", "信号完整性：叠层与线路阻抗、长度与延迟、差分对、串扰、反射和眼图。均为准静态估算。", "Integridade de sinal: empilhamento e impedância das linhas, comprimentos e atrasos, pares diferenciais, diafonia, reflexões e diagrama de olho. Estimativas quase estáticas."],
    },
    TopicText {
        topic: Topic::Formats,
        key: "formats",
        extra_doc: None,
        text: ["Other tools' files: read and write EAGLE and KiCad, Gerber and Excellon, DXF, design rule files and output jobs, and what each reading keeps or loses.", "Ficheros de otras herramientas: leer y escribir EAGLE y KiCad, Gerber y Excellon, DXF, reglas de diseño y trabajos de salida, y qué conserva o pierde cada lectura.", "Dateien anderer Werkzeuge: EAGLE und KiCad, Gerber und Excellon, DXF, Entwurfsregeln und Ausgabejobs lesen und schreiben, und was jede Lesung behält oder verliert.", "Fichiers d'autres outils : lire et écrire EAGLE et KiCad, Gerber et Excellon, DXF, règles de conception et travaux de sortie, et ce que chaque lecture garde ou perd.", "File di altri strumenti: leggere e scrivere EAGLE e KiCad, Gerber ed Excellon, DXF, regole di progetto e job di uscita, e cosa conserva o perde ogni lettura.", "他ツールのファイル：EAGLE、KiCad、Gerber、Excellon、DXF、デザインルール、出力ジョブの読み書きと、各読み込みで保たれる内容・失われる内容。", "其他工具的文件：读写 EAGLE 和 KiCad、Gerber 与 Excellon、DXF、设计规则和输出作业，以及每次读取保留或丢失的内容。", "Ficheiros de outras ferramentas: ler e escrever EAGLE e KiCad, Gerber e Excellon, DXF, regras de projeto e trabalhos de saída, e o que cada leitura mantém ou perde."],
    },
    TopicText {
        topic: Topic::Safety,
        key: "safety",
        extra_doc: Some("docs/THREAT_MODEL.md"),
        text: ["Dry run first: a command writes nothing unless --out names a new file or folder; nothing that exists is replaced and an input is never overwritten. No command uses the network or starts another program unless a setting and an option both allow it.", "Simulacro primero: un comando no escribe nada salvo que --out nombre un fichero o una carpeta nuevos; nada de lo que existe se reemplaza y un fichero de entrada nunca se sobrescribe. Ningún comando usa la red ni lanza otro programa salvo que lo permitan a la vez un ajuste y una opción.", "Erst der Probelauf: Ein Befehl schreibt nichts, solange --out keine neue Datei und keinen neuen Ordner nennt; nichts Vorhandenes wird ersetzt, eine Eingabedatei nie überschrieben. Kein Befehl nutzt das Netz oder startet ein Programm, außer eine Einstellung und eine Option erlauben es zugleich.", "D'abord l'essai à blanc : une commande n'écrit rien tant que --out ne nomme pas un nouveau fichier ou dossier ; rien d'existant n'est remplacé et un fichier d'entrée n'est jamais écrasé. Aucune commande n'utilise le réseau ni ne lance un autre programme sans qu'un réglage et une option le permettent ensemble.", "Prima la prova a secco: un comando non scrive nulla finché --out non indica un file o una cartella nuovi; nulla di esistente viene sostituito e un file di ingresso non viene mai sovrascritto. Nessun comando usa la rete né avvia altri programmi se un'impostazione e un'opzione non lo consentono insieme.", "まずドライラン：--out が新しいファイルまたはフォルダを指さない限り、コマンドは何も書き込みません。既存のものは置き換えず、入力ファイルは上書きしません。設定とオプションの両方が許可しない限り、ネットワークの使用や他プログラムの起動は行いません。", "先试运行：除非 --out 指定一个新文件或新文件夹，命令不会写入任何内容；已有文件不会被替换，输入文件绝不会被覆盖。除非设置和选项同时允许，任何命令都不会使用网络或启动其他程序。", "Primeiro o simulacro: um comando não escreve nada a menos que --out indique um ficheiro ou uma pasta novos; nada do que existe é substituído e um ficheiro de entrada nunca é sobrescrito. Nenhum comando usa a rede nem lança outro programa sem que uma definição e uma opção o permitam em conjunto."],
    },
    TopicText {
        topic: Topic::Errors,
        key: "errors",
        extra_doc: Some("docs/ERROR_CODES.md"),
        text: ["Exit codes: 0 done, 1 something was proven wrong or there are changes to make, 2 the arguments or the input are unusable. Findings carry a code such as PHOENIX-SIM-016, listed with a title in eight languages in docs/ERROR_CODES.md; --lang chooses the language. The reference cases and the own corpus check the tool itself.", "Códigos de salida: 0 hecho, 1 se probó un error o hay cambios que hacer, 2 los argumentos o la entrada no se pueden usar. Cada hallazgo lleva un código como PHOENIX-SIM-016, con título en ocho idiomas en docs/ERROR_CODES.md; --lang elige el idioma. Los casos de referencia y el corpus propio comprueban la propia herramienta.", "Exit-Codes: 0 erledigt, 1 etwas wurde als falsch bewiesen oder es gibt Änderungen, 2 die Argumente oder die Eingabe sind unbrauchbar. Befunde tragen einen Code wie PHOENIX-SIM-016, mit Titel in acht Sprachen in docs/ERROR_CODES.md; --lang wählt die Sprache. Die Referenzfälle und das eigene Korpus prüfen das Werkzeug selbst.", "Codes de sortie : 0 fait, 1 une erreur est prouvée ou des changements sont à faire, 2 les arguments ou l'entrée sont inutilisables. Chaque constat porte un code comme PHOENIX-SIM-016, avec un titre en huit langues dans docs/ERROR_CODES.md ; --lang choisit la langue. Les cas de référence et le corpus propre vérifient l'outil lui-même.", "Codici di uscita: 0 fatto, 1 è stato provato un errore o ci sono modifiche da fare, 2 gli argomenti o l'ingresso sono inutilizzabili. Ogni rilievo ha un codice come PHOENIX-SIM-016, con titolo in otto lingue in docs/ERROR_CODES.md; --lang sceglie la lingua. I casi di riferimento e il corpus proprio verificano lo strumento stesso.", "終了コード：0 完了、1 誤りが証明された、または変更が必要、2 引数または入力が使用不能。指摘には PHOENIX-SIM-016 のようなコードが付き、8 言語のタイトルが docs/ERROR_CODES.md にあります。言語は --lang で選びます。基準ケースと独自コーパスがツール自体を検証します。", "退出码：0 完成，1 已证实有错误或有待做的更改，2 参数或输入无法使用。每条结果带有 PHOENIX-SIM-016 之类的代码，八种语言的标题见 docs/ERROR_CODES.md；用 --lang 选择语言。参考用例和自有语料库用来检验工具本身。", "Códigos de saída: 0 feito, 1 foi provado um erro ou há alterações a fazer, 2 os argumentos ou a entrada não são utilizáveis. Cada resultado traz um código como PHOENIX-SIM-016, com título em oito idiomas em docs/ERROR_CODES.md; --lang escolhe o idioma. Os casos de referência e o corpus próprio verificam a própria ferramenta."],
    },
    TopicText {
        topic: Topic::Scripts,
        key: "scripts",
        extra_doc: None,
        text: ["Scripts: the old command language and phoenix-script/2 (variables, bounded loops, queries and edits as one reversible step, in a sandbox with limits). Run them on a board, a schematic or a library.", "Scripts: el lenguaje de comandos antiguo y phoenix-script/2 (variables, bucles acotados, consultas y ediciones como un paso reversible, en un entorno aislado con límites). Se ejecutan sobre una placa, un esquema o una biblioteca.", "Skripte: die alte Befehlssprache und phoenix-script/2 (Variablen, begrenzte Schleifen, Abfragen und Änderungen als ein umkehrbarer Schritt, in einer Sandbox mit Grenzen). Sie laufen auf Platine, Schaltplan oder Bibliothek.", "Scripts : l'ancien langage de commandes et phoenix-script/2 (variables, boucles bornées, requêtes et éditions en une étape réversible, dans un bac à sable limité). Ils s'exécutent sur une carte, un schéma ou une bibliothèque.", "Script: il vecchio linguaggio di comandi e phoenix-script/2 (variabili, cicli limitati, interrogazioni e modifiche come un unico passo reversibile, in una sandbox con limiti). Si eseguono su una scheda, uno schema o una libreria.", "スクリプト：従来のコマンド言語と phoenix-script/2（変数、回数制限付きループ、問い合わせと編集を元に戻せる 1 ステップとして実行、制限付きサンドボックス）。基板・回路図・ライブラリに対して実行します。", "脚本：旧命令语言和 phoenix-script/2（变量、有界循环、查询与编辑作为一个可撤销步骤，在有限制的沙箱中运行）。可对电路板、原理图或库运行。", "Scripts: a linguagem de comandos antiga e o phoenix-script/2 (variáveis, ciclos limitados, consultas e edições como um passo reversível, numa sandbox com limites). Executam-se sobre uma placa, um esquema ou uma biblioteca."],
    },
];

/// The labels of the listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The label `title`.
    Title,
    /// The label `topics`.
    Topics,
    /// The label `commands`.
    Commands,
    /// The label `commands_of`.
    CommandsOf,
    /// The label `doc`.
    Doc,
    /// The label `usage`.
    Usage,
    /// The label `topic`.
    Topic,
    /// The label `tutorials`.
    Tutorials,
    /// The label `lang`.
    Lang,
    /// The label `unknown`.
    Unknown,
    /// The label `try`.
    Try,
}

/// The text of a label in en, es, de, fr, it, ja, zh and pt.
pub fn label(which: Label) -> [&'static str; 8] {
    match which {
        Label::Title => [
            "EH3D-PHOENIX-PCB command line",
            "Línea de comandos de EH3D-PHOENIX-PCB",
            "Befehlszeile von EH3D-PHOENIX-PCB",
            "Ligne de commande d'EH3D-PHOENIX-PCB",
            "Riga di comando di EH3D-PHOENIX-PCB",
            "EH3D-PHOENIX-PCB のコマンドライン",
            "EH3D-PHOENIX-PCB 命令行",
            "Linha de comandos do EH3D-PHOENIX-PCB",
        ],
        Label::Topics => [
            "Help topics (phoenix help <topic>)",
            "Temas de ayuda (phoenix help <tema>)",
            "Hilfethemen (phoenix help <Thema>)",
            "Thèmes d'aide (phoenix help <thème>)",
            "Temi della guida (phoenix help <tema>)",
            "ヘルプのトピック（phoenix help <トピック>）",
            "帮助主题（phoenix help <主题>）",
            "Temas de ajuda (phoenix help <tema>)",
        ],
        Label::Commands => [
            "Commands (phoenix help <command>)",
            "Comandos (phoenix help <comando>)",
            "Befehle (phoenix help <Befehl>)",
            "Commandes (phoenix help <commande>)",
            "Comandi (phoenix help <comando>)",
            "コマンド（phoenix help <コマンド>）",
            "命令（phoenix help <命令>）",
            "Comandos (phoenix help <comando>)",
        ],
        Label::CommandsOf => [
            "Commands",
            "Comandos",
            "Befehle",
            "Commandes",
            "Comandi",
            "コマンド",
            "命令",
            "Comandos",
        ],
        Label::Doc => [
            "Documentation",
            "Documentación",
            "Dokumentation",
            "Documentation",
            "Documentazione",
            "ドキュメント",
            "文档",
            "Documentação",
        ],
        Label::Usage => [
            "Usage",
            "Uso",
            "Aufruf",
            "Utilisation",
            "Uso",
            "使い方",
            "用法",
            "Utilização",
        ],
        Label::Topic => [
            "Topic",
            "Tema",
            "Thema",
            "Thème",
            "Tema",
            "トピック",
            "主题",
            "Tema",
        ],
        Label::Tutorials => [
            "Tutorials: docs/tutorials/",
            "Tutoriales: docs/tutorials/",
            "Anleitungen: docs/tutorials/",
            "Tutoriels : docs/tutorials/",
            "Tutorial: docs/tutorials/",
            "チュートリアル：docs/tutorials/",
            "教程：docs/tutorials/",
            "Tutoriais: docs/tutorials/",
        ],
        Label::Lang => [
            "--lang is one of en, es, de, fr, it, ja, zh, pt",
            "--lang es uno de en, es, de, fr, it, ja, zh, pt",
            "--lang ist eines von en, es, de, fr, it, ja, zh, pt",
            "--lang est l'un de en, es, de, fr, it, ja, zh, pt",
            "--lang è uno tra en, es, de, fr, it, ja, zh, pt",
            "--lang は en, es, de, fr, it, ja, zh, pt のいずれか",
            "--lang 取值为 en、es、de、fr、it、ja、zh、pt 之一",
            "--lang é um de en, es, de, fr, it, ja, zh, pt",
        ],
        Label::Unknown => [
            "no help topic or command with that name",
            "no hay tema ni comando de ayuda con ese nombre",
            "kein Hilfethema und kein Befehl mit diesem Namen",
            "aucun thème ni commande d'aide de ce nom",
            "nessun tema o comando con questo nome",
            "その名前のトピックまたはコマンドはありません",
            "没有这个名称的帮助主题或命令",
            "não há tema nem comando de ajuda com esse nome",
        ],
        Label::Try => [
            "run `phoenix help` for the list",
            "ejecute `phoenix help` para ver la lista",
            "`phoenix help` zeigt die Liste",
            "lancez `phoenix help` pour la liste",
            "eseguire `phoenix help` per l'elenco",
            "`phoenix help` で一覧を表示します",
            "运行 `phoenix help` 查看列表",
            "execute `phoenix help` para ver a lista",
        ],
    }
}
