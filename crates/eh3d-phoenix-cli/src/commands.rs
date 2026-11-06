// SPDX-License-Identifier: GPL-3.0-or-later
//! The table of the commands of `phoenix`: the one list that the dispatch of `run`, the listing of
//! `phoenix --help` and `phoenix help` all read. Each row has the name, the topic of the help, the
//! document that describes it, a summary in the eight languages (en, es, de, fr, it, ja, zh, pt, the
//! order of `Language::ALL`) and the function that carries it out.

use std::io::Write;

/// The function that carries out a command: the arguments after its name, the output and the error
/// stream. It returns the exit code.
pub type Handler = fn(&[&str], &mut dyn Write, &mut dyn Write) -> i32;

/// The topics of `phoenix help`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    /// The topic `schematic`.
    Schematic,
    /// The topic `board`.
    Board,
    /// The topic `libraries`.
    Libraries,
    /// The topic `manufacturing`.
    Manufacturing,
    /// The topic `simulation`.
    Simulation,
    /// The topic `rf`.
    Rf,
    /// The topic `signal`.
    Signal,
    /// The topic `formats`.
    Formats,
    /// The topic `safety`.
    Safety,
    /// The topic `errors`.
    Errors,
    /// The topic `scripts`.
    Scripts,
}

/// One command of the table.
pub struct Command {
    /// The word that follows `phoenix`.
    pub name: &'static str,
    /// The topic of the help that lists it.
    pub topic: Topic,
    /// The document that describes it, from the root of the repository.
    pub doc: &'static str,
    /// A one-line summary in en, es, de, fr, it, ja, zh and pt.
    pub summary: [&'static str; 8],
    /// The function that carries it out, or none for the words that `run` handles itself.
    pub run: Option<Handler>,
}

/// The summary of the command `help`, which is not in the table because `run` answers it.
pub const HELP_SUMMARY: [&str; 8] = [
    "help on a topic or on a command, in the language of --lang",
    "ayuda de un tema o de un comando, en el idioma de --lang",
    "Hilfe zu einem Thema oder Befehl in der Sprache von --lang",
    "aide sur un thème ou une commande, dans la langue de --lang",
    "guida su un tema o un comando, nella lingua di --lang",
    "トピックまたはコマンドのヘルプ（--lang の言語）",
    "关于某个主题或命令的帮助，语言由 --lang 决定",
    "ajuda sobre um tema ou um comando, no idioma de --lang",
];

/// Every command, in the order of `phoenix --help`.
pub static COMMANDS: &[Command] = &[
    Command {
        name: "inventory",
        topic: Topic::Formats,
        doc: "docs/CORPUS.md",
        summary: ["count the design files of a private folder without showing any name or value", "cuenta los ficheros de diseño de una carpeta privada sin mostrar nombres ni valores", "zählt die Entwurfsdateien eines privaten Ordners, ohne Namen oder Werte zu zeigen", "compte les fichiers de conception d'un dossier privé sans montrer ni nom ni valeur", "conta i file di progetto di una cartella privata senza mostrare nomi o valori", "非公開フォルダの設計ファイルを、名前や値を表示せずに数える", "统计私有文件夹中的设计文件，不显示任何名称或数值", "conta os ficheiros de projeto de uma pasta privada sem mostrar nomes nem valores"],
        run: None,
    },
    Command {
        name: "eagle-rate",
        topic: Topic::Formats,
        doc: "docs/CORPUS.md",
        summary: ["copy a folder of EAGLE files and measure how many the reader reads", "copia una carpeta de ficheros EAGLE y mide cuántos lee el lector", "kopiert einen Ordner mit EAGLE-Dateien und misst, wie viele der Leser liest", "copie un dossier de fichiers EAGLE et mesure combien le lecteur en lit", "copia una cartella di file EAGLE e misura quanti ne legge il lettore", "EAGLE ファイルのフォルダをコピーし、リーダが読める数を測る", "复制一个 EAGLE 文件夹并统计读取器能读取多少个", "copia uma pasta de ficheiros EAGLE e mede quantos o leitor lê"],
        run: None,
    },
    Command {
        name: "stackup",
        topic: Topic::Signal,
        doc: "docs/STACKUP.md",
        summary: ["impedance, delay and losses of the lines of a stack-up", "impedancia, retardo y pérdidas de las líneas de un apilado", "Impedanz, Laufzeit und Verluste der Leitungen eines Lagenaufbaus", "impédance, retard et pertes des lignes d'un empilage", "impedenza, ritardo e perdite delle linee di uno stack-up", "層構成の線路のインピーダンス・遅延・損失", "叠层中传输线的阻抗、延迟和损耗", "impedância, atraso e perdas das linhas de um empilhamento"],
        run: Some(crate::stackup_command),
    },
    Command {
        name: "materials",
        topic: Topic::Libraries,
        doc: "docs/STACKUP.md",
        summary: ["browse the board materials database (Dk, Df, glass transition)", "consulta la base de materiales de placa (Dk, Df, transición vítrea)", "durchsucht die Leiterplatten-Materialdatenbank (Dk, Df, Glasübergang)", "consulte la base des matériaux de carte (Dk, Df, transition vitreuse)", "consulta la base dei materiali delle schede (Dk, Df, transizione vetrosa)", "基板材料データベース（Dk、Df、ガラス転移）を参照する", "查询板材数据库（Dk、Df、玻璃化转变）", "consulta a base de materiais de placa (Dk, Df, transição vítrea)"],
        run: Some(crate::materials::materials_command),
    },
    Command {
        name: "check",
        topic: Topic::Board,
        doc: "docs/RULES.md",
        summary: ["design rules of a board: widths, clearances, drills, rings, edges, silkscreen", "reglas de diseño de una placa: anchos, separaciones, taladros, anillos, bordes, serigrafía", "Entwurfsregeln einer Platine: Breiten, Abstände, Bohrungen, Ringe, Ränder, Bestückungsdruck", "règles de conception d'une carte : largeurs, dégagements, perçages, anneaux, bords, sérigraphie", "regole di progetto di una scheda: larghezze, distanze, fori, anelli, bordi, serigrafia", "基板の設計ルール：幅、間隔、穴、リング、端、シルク", "电路板的设计规则：线宽、间距、钻孔、焊环、板边、丝印", "regras de projeto de uma placa: larguras, afastamentos, furos, anéis, bordos, serigrafia"],
        run: Some(crate::check::check),
    },
    Command {
        name: "connectivity",
        topic: Topic::Board,
        doc: "docs/RULES.md",
        summary: ["shorts and unrouted nets, and the board against the schematic", "cortocircuitos y redes sin rutar, y la placa frente al esquema", "Kurzschlüsse und nicht verlegte Netze sowie die Platine gegen den Schaltplan", "courts-circuits et réseaux non routés, et la carte face au schéma", "cortocircuiti e reti non instradate, e la scheda rispetto allo schema", "ショートと未配線のネット、および基板と回路図の突き合わせ", "短路和未布线的网络，以及电路板与原理图的比对", "curto-circuitos e redes por ligar, e a placa face ao esquema"],
        run: Some(crate::check::connectivity),
    },
    Command {
        name: "erc",
        topic: Topic::Schematic,
        doc: "docs/ERC.md",
        summary: ["electrical rule check of a schematic", "comprobación de reglas eléctricas de un esquema", "elektrische Regelprüfung eines Schaltplans", "vérification des règles électriques d'un schéma", "controllo delle regole elettriche di uno schema", "回路図の電気ルールチェック", "原理图的电气规则检查", "verificação das regras elétricas de um esquema"],
        run: Some(crate::design::erc),
    },
    Command {
        name: "sync",
        topic: Topic::Schematic,
        doc: "docs/SCHEMATIC_EDIT.md",
        summary: ["transfer the schematic to the board and back, as a package that can be undone", "pasa el esquema a la placa y viceversa, como un paquete reversible", "überträgt den Schaltplan auf die Platine und zurück, als umkehrbares Paket", "transfère le schéma vers la carte et inversement, en paquet réversible", "trasferisce lo schema alla scheda e viceversa, come pacchetto reversibile", "回路図から基板へ、またその逆へ、元に戻せるパッケージとして反映する", "把原理图同步到电路板，或反向同步，作为可撤销的整体", "passa o esquema para a placa e vice-versa, como um pacote reversível"],
        run: Some(crate::design::sync),
    },
    Command {
        name: "annotate",
        topic: Topic::Schematic,
        doc: "docs/ANNOTATE.md",
        summary: ["annotation back into the schematic as a plan read before it is applied", "anotación de vuelta al esquema como un plan que se lee antes de aplicarlo", "Rückannotation in den Schaltplan als Plan, der vor dem Anwenden gelesen wird", "annotation retour dans le schéma, en plan lu avant d'être appliqué", "annotazione di ritorno nello schema come piano da leggere prima di applicarlo", "回路図への逆アノテーションを、適用前に確認する計画として出す", "把反向标注作为先审阅后应用的计划写回原理图", "anotação de volta ao esquema como um plano lido antes de ser aplicado"],
        run: Some(crate::annotate::annotate),
    },
    Command {
        name: "export",
        topic: Topic::Manufacturing,
        doc: "docs/EXPORT.md",
        summary: ["Gerber X2, drill, placement and bill of materials files with a checksum manifest", "ficheros Gerber X2, de taladros, de posición y lista de materiales con manifiesto de sumas", "Gerber-X2-, Bohr-, Bestückungs- und Stücklistendateien mit Prüfsummen-Manifest", "fichiers Gerber X2, de perçage, de placement et nomenclature avec manifeste de sommes", "file Gerber X2, di foratura, di posizione e distinta base con manifesto di checksum", "Gerber X2、ドリル、実装位置、部品表のファイルとチェックサム一覧を作る", "生成 Gerber X2、钻孔、贴装坐标和物料清单文件及校验清单", "ficheiros Gerber X2, de furação, de posição e lista de materiais com manifesto de somas"],
        run: Some(crate::export::export),
    },
    Command {
        name: "rf",
        topic: Topic::Rf,
        doc: "docs/RF.md",
        summary: ["Touchstone files, impedance matching and ladder filters", "ficheros Touchstone, adaptación de impedancias y filtros en escalera", "Touchstone-Dateien, Impedanzanpassung und Abzweigfilter", "fichiers Touchstone, adaptation d'impédance et filtres en échelle", "file Touchstone, adattamento d'impedenza e filtri a scala", "Touchstone ファイル、インピーダンス整合、ラダーフィルタ", "Touchstone 文件、阻抗匹配和梯形滤波器", "ficheiros Touchstone, adaptação de impedâncias e filtros em escada"],
        run: Some(crate::rf::rf),
    },
    Command {
        name: "rfgen",
        topic: Topic::Rf,
        doc: "docs/RFGEN.md",
        summary: ["draw microstrip RF structures from the stack-up and the frequency", "dibuja estructuras RF de microstrip a partir del apilado y la frecuencia", "zeichnet Mikrostreifen-HF-Strukturen aus Lagenaufbau und Frequenz", "dessine des structures RF microruban à partir de l'empilage et de la fréquence", "disegna strutture RF microstrip dallo stack-up e dalla frequenza", "層構成と周波数からマイクロストリップ RF 構造を描く", "根据叠层和频率绘制微带射频结构", "desenha estruturas RF em microstrip a partir do empilhamento e da frequência"],
        run: Some(crate::rfgen::rfgen_command),
    },
    Command {
        name: "kicad-read",
        topic: Topic::Formats,
        doc: "docs/KICAD.md",
        summary: ["read a KiCad 6 to 9 file and summarise what was kept and left out", "lee un fichero de KiCad 6 a 9 y resume lo que se conservó y lo que no", "liest eine KiCad-6-bis-9-Datei und fasst zusammen, was erhalten blieb und was nicht", "lit un fichier KiCad 6 à 9 et résume ce qui a été gardé ou laissé de côté", "legge un file KiCad da 6 a 9 e riassume ciò che è rimasto e ciò che no", "KiCad 6〜9 のファイルを読み、保持した内容と除外した内容を要約する", "读取 KiCad 6 至 9 文件，并汇总保留和舍弃的内容", "lê um ficheiro KiCad 6 a 9 e resume o que ficou e o que ficou de fora"],
        run: Some(crate::kicad::kicad_read),
    },
    Command {
        name: "kicad-import",
        topic: Topic::Formats,
        doc: "docs/KICAD_IMPORT.md",
        summary: ["read a whole KiCad project into the design model with one report", "lee un proyecto de KiCad completo en el modelo de diseño con un informe", "liest ein ganzes KiCad-Projekt mit einem Bericht in das Entwurfsmodell", "lit un projet KiCad entier dans le modèle de conception avec un rapport", "legge un intero progetto KiCad nel modello di progetto con un rapporto", "KiCad プロジェクト全体を 1 つのレポート付きで設計モデルに読み込む", "把整个 KiCad 工程读入设计模型并给出一份报告", "lê um projeto KiCad completo para o modelo de projeto com um relatório"],
        run: Some(crate::kicad_import::kicad_import),
    },
    Command {
        name: "kicad-export",
        topic: Topic::Formats,
        doc: "docs/KICAD_EXPORT.md",
        summary: ["write a board, a schematic or a library as KiCad 8 files", "escribe una placa, un esquema o una biblioteca como ficheros de KiCad 8", "schreibt eine Platine, einen Schaltplan oder eine Bibliothek als KiCad-8-Dateien", "écrit une carte, un schéma ou une bibliothèque en fichiers KiCad 8", "scrive una scheda, uno schema o una libreria come file KiCad 8", "基板・回路図・ライブラリを KiCad 8 のファイルとして書き出す", "把电路板、原理图或库写成 KiCad 8 文件", "escreve uma placa, um esquema ou uma biblioteca como ficheiros KiCad 8"],
        run: Some(crate::kicad_export::kicad_export),
    },
    Command {
        name: "sanitize",
        topic: Topic::Libraries,
        doc: "docs/SANITIZER.md",
        summary: ["find duplicated footprints, symbols and devices in a folder of EAGLE libraries", "encuentra huellas, símbolos y dispositivos duplicados en una carpeta de bibliotecas EAGLE", "findet doppelte Footprints, Symbole und Bauteile in einem Ordner mit EAGLE-Bibliotheken", "trouve les empreintes, symboles et composants en double dans un dossier de bibliothèques EAGLE", "trova impronte, simboli e dispositivi duplicati in una cartella di librerie EAGLE", "EAGLE ライブラリのフォルダで重複したフットプリント・シンボル・デバイスを探す", "在 EAGLE 库文件夹中查找重复的封装、符号和器件", "encontra footprints, símbolos e dispositivos duplicados numa pasta de bibliotecas EAGLE"],
        run: Some(crate::sanitize::sanitize_command),
    },
    Command {
        name: "script",
        topic: Topic::Scripts,
        doc: "docs/SCRIPT.md",
        summary: ["run, check and list scripts on a board, a schematic or a library", "ejecuta, comprueba y lista scripts sobre una placa, un esquema o una biblioteca", "führt Skripte auf Platine, Schaltplan oder Bibliothek aus, prüft und listet sie", "exécute, vérifie et liste des scripts sur une carte, un schéma ou une bibliothèque", "esegue, controlla ed elenca script su una scheda, uno schema o una libreria", "基板・回路図・ライブラリに対するスクリプトの実行・検査・一覧", "对电路板、原理图或库运行、检查和列出脚本", "executa, verifica e lista scripts sobre uma placa, um esquema ou uma biblioteca"],
        run: Some(crate::script::script_command),
    },
    Command {
        name: "cam-import",
        topic: Topic::Formats,
        doc: "docs/SCRIPT.md",
        summary: ["read an output job (.cam) into an export profile", "lee un trabajo de salida (.cam) como perfil de exportación", "liest einen Ausgabejob (.cam) in ein Exportprofil", "lit un travail de sortie (.cam) dans un profil d'export", "legge un job di uscita (.cam) in un profilo di esportazione", "出力ジョブ（.cam）をエクスポートプロファイルに読み込む", "把输出作业（.cam）读成导出配置", "lê um trabalho de saída (.cam) para um perfil de exportação"],
        run: Some(crate::script::cam_import_command),
    },
    Command {
        name: "route",
        topic: Topic::Board,
        doc: "docs/ROUTE.md",
        summary: ["route between two elements of a net at 0, 45 and 90 degrees", "rutea entre dos elementos de una red a 0, 45 y 90 grados", "verlegt zwischen zwei Elementen eines Netzes in 0, 45 und 90 Grad", "route entre deux éléments d'un réseau à 0, 45 et 90 degrés", "instrada tra due elementi di una rete a 0, 45 e 90 gradi", "ネットの 2 点間を 0・45・90 度で配線する", "在网络的两个元素之间以 0、45、90 度布线", "encaminha entre dois elementos de uma rede a 0, 45 e 90 graus"],
        run: Some(crate::route::route_command),
    },
    Command {
        name: "ratsnest",
        topic: Topic::Board,
        doc: "docs/ROUTE.md",
        summary: ["list the connections still to be routed", "lista las conexiones que faltan por rutar", "listet die noch zu verlegenden Verbindungen auf", "liste les connexions restant à router", "elenca i collegamenti ancora da instradare", "未配線の接続を一覧にする", "列出尚未布线的连接", "lista as ligações que faltam encaminhar"],
        run: Some(crate::route::ratsnest_command),
    },
    Command {
        name: "autoroute",
        topic: Topic::Board,
        doc: "docs/AUTOROUTE.md",
        summary: ["routing through an external autorouter by Specctra DSN and SES files", "ruteo con un autorouter externo mediante ficheros Specctra DSN y SES", "Verlegen mit einem externen Autorouter über Specctra-DSN- und SES-Dateien", "routage avec un autorouteur externe par fichiers Specctra DSN et SES", "instradamento con un autorouter esterno tramite file Specctra DSN e SES", "Specctra の DSN／SES ファイルによる外部オートルータでの配線", "通过 Specctra DSN 和 SES 文件使用外部自动布线器", "encaminhamento com um autorouter externo por ficheiros Specctra DSN e SES"],
        run: Some(crate::autoroute::autoroute_command),
    },
    Command {
        name: "gerber-import",
        topic: Topic::Formats,
        doc: "docs/GERBER_IMPORT.md",
        summary: ["read Gerber and Excellon files back into a board", "lee ficheros Gerber y Excellon y los vuelve a una placa", "liest Gerber- und Excellon-Dateien zurück in eine Platine", "relit des fichiers Gerber et Excellon dans une carte", "rilegge file Gerber ed Excellon in una scheda", "Gerber と Excellon のファイルを基板として読み戻す", "把 Gerber 和 Excellon 文件读回电路板", "lê ficheiros Gerber e Excellon de volta para uma placa"],
        run: Some(crate::gerber_import::gerber_import_command),
    },
    Command {
        name: "gerber-check",
        topic: Topic::Manufacturing,
        doc: "docs/GERBER_IMPORT.md",
        summary: ["check the structure of Gerber and Excellon files", "comprueba la estructura de ficheros Gerber y Excellon", "prüft den Aufbau von Gerber- und Excellon-Dateien", "vérifie la structure de fichiers Gerber et Excellon", "controlla la struttura di file Gerber ed Excellon", "Gerber と Excellon ファイルの構造を検査する", "检查 Gerber 和 Excellon 文件的结构", "verifica a estrutura de ficheiros Gerber e Excellon"],
        run: Some(crate::gerber_check::gerber_check_command),
    },
    Command {
        name: "panel",
        topic: Topic::Manufacturing,
        doc: "docs/PANEL.md",
        summary: ["lay boards out in a panel with gaps, tabs, V-score, fiducials and tooling holes", "dispone placas en un panel con huecos, puentes, corte en V, fiduciales y taladros de herramienta", "ordnet Platinen im Nutzen mit Spalten, Stegen, Ritzen, Passmarken und Werkzeuglöchern an", "dispose des cartes en panneau avec espaces, ponts, rainure en V, mires et trous d'outillage", "dispone schede in un pannello con spazi, ponticelli, taglio a V, fiducial e fori di riferimento", "隙間・タブ・Vカット・フィデューシャル・工具穴付きで基板をパネルに面付けする", "把电路板拼成带间隙、连接桥、V 形槽、定位点和工具孔的拼板", "dispõe placas num painel com folgas, pontes, corte em V, marcas fiduciais e furos de ferramenta"],
        run: Some(crate::panel::panel_command),
    },
    Command {
        name: "print",
        topic: Topic::Manufacturing,
        doc: "docs/PRINT.md",
        summary: ["print schematics, assembly plans and layers as SVG or PDF", "imprime esquemas, planos de montaje y capas como SVG o PDF", "druckt Schaltpläne, Bestückungspläne und Lagen als SVG oder PDF", "imprime schémas, plans d'assemblage et couches en SVG ou PDF", "stampa schemi, piani di montaggio e livelli come SVG o PDF", "回路図・実装図・各層を SVG または PDF で出力する", "把原理图、装配图和各层打印为 SVG 或 PDF", "imprime esquemas, planos de montagem e camadas em SVG ou PDF"],
        run: Some(crate::print::print_command),
    },
    Command {
        name: "dxf-import",
        topic: Topic::Formats,
        doc: "docs/DXF.md",
        summary: ["make the outline of a board from the contours of a DXF drawing", "crea el contorno de una placa a partir de los contornos de un dibujo DXF", "erstellt den Platinenumriss aus den Konturen einer DXF-Zeichnung", "crée le contour d'une carte à partir des contours d'un dessin DXF", "crea il contorno di una scheda dai contorni di un disegno DXF", "DXF 図面の輪郭から基板外形を作る", "根据 DXF 图纸的轮廓生成板框", "cria o contorno de uma placa a partir dos contornos de um desenho DXF"],
        run: Some(crate::dxf_import::dxf_import_command),
    },
    Command {
        name: "dxf-export",
        topic: Topic::Formats,
        doc: "docs/DXF.md",
        summary: ["write a board or a panel as a DXF drawing", "escribe una placa o un panel como dibujo DXF", "schreibt eine Platine oder einen Nutzen als DXF-Zeichnung", "écrit une carte ou un panneau en dessin DXF", "scrive una scheda o un pannello come disegno DXF", "基板またはパネルを DXF 図面として書き出す", "把电路板或拼板写成 DXF 图纸", "escreve uma placa ou um painel como desenho DXF"],
        run: Some(crate::dxf_export::dxf_export_command),
    },
    Command {
        name: "enclosure",
        topic: Topic::Manufacturing,
        doc: "docs/ENCLOSURE.md",
        summary: ["check a board against the box of its enclosure", "comprueba una placa frente a la caja de su carcasa", "prüft eine Platine gegen den Raum ihres Gehäuses", "vérifie une carte par rapport à la boîte de son boîtier", "controlla una scheda rispetto al volume del suo contenitore", "基板を筐体の空間と照合する", "对照外壳的空间检查电路板", "verifica uma placa face à caixa do seu invólucro"],
        run: Some(crate::enclosure::enclosure_command),
    },
    Command {
        name: "perf",
        topic: Topic::Board,
        doc: "docs/SPATIAL.md",
        summary: ["complexity benchmarks of the spatial indexes on large boards", "pruebas de complejidad de los índices espaciales en placas grandes", "Komplexitätstests der räumlichen Indizes auf großen Platinen", "tests de complexité des index spatiaux sur de grandes cartes", "prove di complessità degli indici spaziali su schede grandi", "大きな基板での空間インデックスの計算量テスト", "大型电路板上空间索引的复杂度基准测试", "testes de complexidade dos índices espaciais em placas grandes"],
        run: Some(crate::perf::perf_command),
    },
    Command {
        name: "sim",
        topic: Topic::Simulation,
        doc: "docs/SIM.md",
        summary: ["simulate a circuit: operating point, DC, AC, transient, steps, measures, Monte Carlo", "simula un circuito: punto de trabajo, CC, CA, transitorio, barridos, medidas, Monte Carlo", "simuliert eine Schaltung: Arbeitspunkt, DC, AC, Transient, Läufe, Messungen, Monte Carlo", "simule un circuit : point de repos, continu, alternatif, transitoire, pas, mesures, Monte Carlo", "simula un circuito: punto di lavoro, DC, AC, transitorio, passi, misure, Monte Carlo", "回路のシミュレーション：動作点、DC、AC、過渡、ステップ、測定、モンテカルロ", "电路仿真：工作点、直流、交流、瞬态、步进、测量、蒙特卡洛", "simula um circuito: ponto de funcionamento, CC, CA, transitório, passos, medidas, Monte Carlo"],
        run: Some(crate::sim::sim_command),
    },
    Command {
        name: "netlist",
        topic: Topic::Simulation,
        doc: "docs/NETLIST.md",
        summary: ["make the simulation netlist of a schematic from a table of models", "crea la netlist de simulación de un esquema a partir de una tabla de modelos", "erstellt die Simulations-Netzliste eines Schaltplans aus einer Modelltabelle", "crée la netlist de simulation d'un schéma à partir d'une table de modèles", "crea la netlist di simulazione di uno schema da una tabella di modelli", "モデル表から回路図のシミュレーション用ネットリストを作る", "根据模型表生成原理图的仿真网表", "cria a netlist de simulação de um esquema a partir de uma tabela de modelos"],
        run: Some(crate::netlist::netlist_command),
    },
    Command {
        name: "hier",
        topic: Topic::Schematic,
        doc: "docs/HIERARCHY.md",
        summary: ["hierarchy of sheets, instances and buses of a schematic", "jerarquía de hojas, instancias y buses de un esquema", "Hierarchie der Blätter, Instanzen und Busse eines Schaltplans", "hiérarchie des feuilles, instances et bus d'un schéma", "gerarchia di fogli, istanze e bus di uno schema", "回路図のシート階層、インスタンス、バス", "原理图的图页层次、实例和总线", "hierarquia de folhas, instâncias e barramentos de um esquema"],
        run: Some(crate::hier::hier_command),
    },
    Command {
        name: "schedit",
        topic: Topic::Schematic,
        doc: "docs/SCHEMATIC_EDIT.md",
        summary: ["edit a schematic with reversible operations and the rule check after each change", "edita un esquema con operaciones reversibles y la comprobación tras cada cambio", "bearbeitet einen Schaltplan mit umkehrbaren Operationen und Regelprüfung nach jeder Änderung", "édite un schéma avec des opérations réversibles et la vérification après chaque changement", "modifica uno schema con operazioni reversibili e controllo dopo ogni modifica", "元に戻せる操作で回路図を編集し、変更ごとにルールを検査する", "用可撤销的操作编辑原理图，每次修改后做规则检查", "edita um esquema com operações reversíveis e a verificação após cada alteração"],
        run: Some(crate::schedit::schedit_command),
    },
    Command {
        name: "boardedit",
        topic: Topic::Board,
        doc: "docs/BOARD_EDIT.md",
        summary: ["edit a board with reversible operations: move, route, pairs, push and shove, zones", "edita una placa con operaciones reversibles: mover, rutear, pares, empujar, zonas", "bearbeitet eine Platine mit umkehrbaren Operationen: Verschieben, Verlegen, Paare, Schieben, Flächen", "édite une carte avec des opérations réversibles : déplacer, router, paires, pousser, zones", "modifica una scheda con operazioni reversibili: sposta, instrada, coppie, spingi, zone", "元に戻せる操作で基板を編集する：移動、配線、ペア、押し退け、ゾーン", "用可撤销的操作编辑电路板：移动、布线、差分对、推挤、铜区", "edita uma placa com operações reversíveis: mover, encaminhar, pares, empurrar, zonas"],
        run: Some(crate::boardedit::boardedit_command),
    },
    Command {
        name: "pairs",
        topic: Topic::Signal,
        doc: "docs/DIFFPAIRS.md",
        summary: ["check the differential pairs of a board: gap, symmetry, lengths, vias, impedance", "comprueba los pares diferenciales de una placa: separación, simetría, longitudes, vías, impedancia", "prüft die Differenzialpaare einer Platine: Abstand, Symmetrie, Längen, Vias, Impedanz", "vérifie les paires différentielles d'une carte : écart, symétrie, longueurs, vias, impédance", "controlla le coppie differenziali di una scheda: distanza, simmetria, lunghezze, via, impedenza", "基板の差動ペアを検査する：間隔、対称性、長さ、ビア、インピーダンス", "检查电路板的差分对：间距、对称性、长度、过孔、阻抗", "verifica os pares diferenciais de uma placa: separação, simetria, comprimentos, vias, impedância"],
        run: Some(crate::pairs::pairs_command),
    },
    Command {
        name: "libedit",
        topic: Topic::Libraries,
        doc: "docs/LIBRARY_EDIT.md",
        summary: ["edit an EAGLE library with a file of operations and check it", "edita una biblioteca EAGLE con un fichero de operaciones y la comprueba", "bearbeitet eine EAGLE-Bibliothek mit einer Operationsdatei und prüft sie", "édite une bibliothèque EAGLE avec un fichier d'opérations et la vérifie", "modifica una libreria EAGLE con un file di operazioni e la controlla", "操作ファイルで EAGLE ライブラリを編集し、検査する", "用操作文件编辑 EAGLE 库并检查", "edita uma biblioteca EAGLE com um ficheiro de operações e verifica-a"],
        run: Some(crate::libedit::libedit_command),
    },
    Command {
        name: "ipc",
        topic: Topic::Libraries,
        doc: "docs/IPC_LIBRARY.md",
        summary: ["own library of footprints by the method of IPC-7351B, with three density levels", "biblioteca propia de huellas por el método de IPC-7351B, con tres niveles de densidad", "eigene Footprint-Bibliothek nach IPC-7351B mit drei Dichtestufen", "bibliothèque propre d'empreintes selon IPC-7351B, avec trois niveaux de densité", "libreria propria di impronte con il metodo IPC-7351B, con tre livelli di densità", "IPC-7351B の方法による独自フットプリントライブラリ（3 段階の密度）", "按 IPC-7351B 方法建立的自有封装库，含三个密度等级", "biblioteca própria de footprints pelo método IPC-7351B, com três níveis de densidade"],
        run: Some(crate::ipclib::ipc_command),
    },
    Command {
        name: "libindex",
        topic: Topic::Libraries,
        doc: "docs/LIBINDEX.md",
        summary: ["index many EAGLE and KiCad libraries and judge each footprint and symbol", "indexa muchas bibliotecas EAGLE y KiCad y juzga cada huella y símbolo", "indiziert viele EAGLE- und KiCad-Bibliotheken und beurteilt jeden Footprint und jedes Symbol", "indexe de nombreuses bibliothèques EAGLE et KiCad et juge chaque empreinte et symbole", "indicizza molte librerie EAGLE e KiCad e giudica ogni impronta e simbolo", "多数の EAGLE／KiCad ライブラリを索引化し、各フットプリントとシンボルを評価する", "为大量 EAGLE 和 KiCad 库建立索引，并评估每个封装和符号", "indexa muitas bibliotecas EAGLE e KiCad e avalia cada footprint e símbolo"],
        run: Some(crate::ipclib::libindex_command),
    },
    Command {
        name: "footprint",
        topic: Topic::Libraries,
        doc: "docs/LIBRARY_EDIT.md",
        summary: ["make a footprint from parameters (chip, SOIC, TSSOP, QFN, QFP, DIP, header)", "crea una huella a partir de parámetros (chip, SOIC, TSSOP, QFN, QFP, DIP, regleta)", "erstellt einen Footprint aus Parametern (Chip, SOIC, TSSOP, QFN, QFP, DIP, Stiftleiste)", "crée une empreinte à partir de paramètres (puce, SOIC, TSSOP, QFN, QFP, DIP, barrette)", "crea un'impronta da parametri (chip, SOIC, TSSOP, QFN, QFP, DIP, strip)", "パラメータからフットプリントを作る（チップ、SOIC、TSSOP、QFN、QFP、DIP、ヘッダ）", "根据参数生成封装（贴片、SOIC、TSSOP、QFN、QFP、DIP、排针）", "cria um footprint a partir de parâmetros (chip, SOIC, TSSOP, QFN, QFP, DIP, barra de pinos)"],
        run: Some(crate::libedit::footprint_command),
    },
    Command {
        name: "zones",
        topic: Topic::Board,
        doc: "docs/ZONES.md",
        summary: ["fill copper zones with clearances, thermal relief and islands", "rellena zonas de cobre con separaciones, alivio térmico e islas", "füllt Kupferflächen mit Abständen, Wärmefallen und Inseln", "remplit des zones de cuivre avec dégagements, thermiques et îlots", "riempie zone di rame con distanze, relief termici e isole", "クリアランス、サーマルリリーフ、島付きで銅箔ゾーンを塗りつぶす", "填充铜区，含间距、热焊盘和孤岛处理", "preenche zonas de cobre com afastamentos, alívio térmico e ilhas"],
        run: Some(crate::zones::zones_command),
    },
    Command {
        name: "lengths",
        topic: Topic::Signal,
        doc: "docs/LENGTHS.md",
        summary: ["lengths and delays of nets and pairs, and matching groups", "longitudes y retardos de redes y pares, y grupos de igualación", "Längen und Laufzeiten von Netzen und Paaren sowie Abgleichgruppen", "longueurs et retards des réseaux et paires, et groupes d'égalisation", "lunghezze e ritardi di reti e coppie, e gruppi di allineamento", "ネットとペアの長さ・遅延、および長さ合わせグループ", "网络和差分对的长度与延迟，以及等长组", "comprimentos e atrasos de redes e pares, e grupos de igualação"],
        run: Some(crate::lengths::lengths_command),
    },
    Command {
        name: "si",
        topic: Topic::Signal,
        doc: "docs/SI.md",
        summary: ["signal integrity estimates: crosstalk, reflections, terminations, eye, return path", "estimaciones de integridad de señal: diafonía, reflexiones, terminaciones, ojo, retorno", "Signalintegritäts-Schätzungen: Übersprechen, Reflexionen, Abschlüsse, Auge, Rückweg", "estimations d'intégrité du signal : diaphonie, réflexions, terminaisons, œil, retour", "stime di integrità del segnale: diafonia, riflessioni, terminazioni, occhio, ritorno", "シグナルインテグリティの見積り：クロストーク、反射、終端、アイ、リターンパス", "信号完整性估算：串扰、反射、端接、眼图、回流路径", "estimativas de integridade de sinal: diafonia, reflexões, terminações, olho, retorno"],
        run: Some(crate::si::si_command),
    },
    Command {
        name: "eagle-check",
        topic: Topic::Formats,
        doc: "docs/EAGLE_DEEP.md",
        summary: ["what the deep reading of an EAGLE file keeps and what the model would lose", "qué conserva la lectura profunda de un fichero EAGLE y qué perdería el modelo", "was die tiefe Lesung einer EAGLE-Datei behält und was das Modell verlöre", "ce que la lecture approfondie d'un fichier EAGLE garde et ce que le modèle perdrait", "cosa conserva la lettura approfondita di un file EAGLE e cosa perderebbe il modello", "EAGLE ファイルの詳細読み込みで保持される内容と、モデルで失われる内容", "EAGLE 文件的深度读取保留什么，模型会丢失什么", "o que a leitura profunda de um ficheiro EAGLE mantém e o que o modelo perderia"],
        run: Some(crate::eagle_deep::eagle_check_command),
    },
    Command {
        name: "eagle-convert",
        topic: Topic::Formats,
        doc: "docs/EAGLE_DEEP.md",
        summary: ["convert design rules (.dru) and output jobs (.cam) to and from the rule and export files", "convierte reglas (.dru) y trabajos de salida (.cam) a ficheros de reglas y de exportación y viceversa", "wandelt Entwurfsregeln (.dru) und Ausgabejobs (.cam) in Regel- und Exportdateien um und zurück", "convertit règles (.dru) et travaux de sortie (.cam) en fichiers de règles et d'export, et inversement", "converte regole (.dru) e job di uscita (.cam) in file di regole e di esportazione, e viceversa", "デザインルール（.dru）と出力ジョブ（.cam）を、ルール／エクスポートファイルと相互に変換する", "在设计规则（.dru）、输出作业（.cam）与规则、导出文件之间互相转换", "converte regras (.dru) e trabalhos de saída (.cam) para ficheiros de regras e de exportação, e vice-versa"],
        run: Some(crate::eagle_deep::eagle_convert_command),
    },
    Command {
        name: "bom",
        topic: Topic::Manufacturing,
        doc: "docs/BOM.md",
        summary: ["bill of materials with prices by quantity break and the cost per board", "lista de materiales con precios por tramo de cantidad y el coste por placa", "Stückliste mit Staffelpreisen und den Kosten je Platine", "nomenclature avec prix par palier de quantité et coût par carte", "distinta base con prezzi per scaglione di quantità e costo per scheda", "数量段ごとの価格付き部品表と基板 1 枚あたりのコスト", "带数量阶梯价格的物料清单及单板成本", "lista de materiais com preços por escalão de quantidade e o custo por placa"],
        run: Some(crate::bom::bom_command),
    },
    Command {
        name: "parts",
        topic: Topic::Libraries,
        doc: "docs/PARTS.md",
        summary: ["local parts base: check, import and export part data", "base local de piezas: comprueba, importa y exporta datos de piezas", "lokale Bauteilbasis: prüft, importiert und exportiert Bauteildaten", "base locale de pièces : vérifie, importe et exporte les données de pièces", "base locale di componenti: controlla, importa ed esporta i dati dei componenti", "ローカルの部品データベース：データの検査・取り込み・書き出し", "本地元件库：检查、导入和导出元件数据", "base local de peças: verifica, importa e exporta dados de peças"],
        run: Some(crate::parts::parts_command),
    },
    Command {
        name: "pnp",
        topic: Topic::Manufacturing,
        doc: "docs/PNP.md",
        summary: ["pick and place files of each face, with rotations and parts not fitted", "ficheros de posicionamiento de cada cara, con rotaciones y piezas no montadas", "Bestückungsdateien je Seite mit Drehungen und unbestückten Teilen", "fichiers de placement de chaque face, avec rotations et pièces non montées", "file di posizionamento di ogni lato, con rotazioni e componenti non montati", "各面の実装位置ファイル（回転・未実装部品の指定付き）", "每个面的贴装坐标文件，含旋转和不贴装元件", "ficheiros de posicionamento de cada face, com rotações e peças não montadas"],
        run: Some(crate::pnp::pnp_command),
    },
    Command {
        name: "stencil",
        topic: Topic::Manufacturing,
        doc: "docs/STENCIL.md",
        summary: ["solder paste stencil openings and their report", "aberturas de la plantilla de pasta de soldar y su informe", "Öffnungen der Lotpastenschablone und ihr Bericht", "ouvertures du pochoir de pâte à braser et leur rapport", "aperture dello stencil per pasta saldante e il relativo rapporto", "はんだペーストのステンシル開口とそのレポート", "锡膏钢网的开口及其报告", "aberturas do estêncil de pasta de soldar e o seu relatório"],
        run: Some(crate::stencil::stencil_command),
    },
    Command {
        name: "fabpack",
        topic: Topic::Manufacturing,
        doc: "docs/FABPACK.md",
        summary: ["the fabrication package of a board in one deterministic zip with a manifest", "el paquete de fabricación de una placa en un zip determinista con manifiesto", "das Fertigungspaket einer Platine als deterministisches Zip mit Manifest", "le paquet de fabrication d'une carte en un zip déterministe avec manifeste", "il pacchetto di fabbricazione di una scheda in uno zip deterministico con manifesto", "基板の製造パッケージを、マニフェスト付きの決定的な zip 1 つにまとめる", "把电路板的制造包打成带清单的确定性 zip", "o pacote de fabrico de uma placa num zip determinista com manifesto"],
        run: Some(crate::fabpack::fabpack_command),
    },
    Command {
        name: "assembly3d",
        topic: Topic::Manufacturing,
        doc: "docs/ASSEMBLY3D.md",
        summary: ["3D assembly of the board with component models: collisions and heights", "montaje 3D de la placa con modelos de componentes: colisiones y alturas", "3D-Bestückung der Platine mit Bauteilmodellen: Kollisionen und Höhen", "assemblage 3D de la carte avec des modèles de composants : collisions et hauteurs", "assemblaggio 3D della scheda con modelli dei componenti: collisioni e altezze", "部品モデルによる基板の 3D アセンブリ：干渉と高さ", "带元件模型的电路板 3D 装配：碰撞与高度", "montagem 3D da placa com modelos de componentes: colisões e alturas"],
        run: Some(crate::assembly3d::assembly3d_command),
    },
    Command {
        name: "oracle",
        topic: Topic::Errors,
        doc: "docs/ORACLE.md",
        summary: ["run the reference cases that check the calculations against published values", "ejecuta los casos de referencia que comprueban los cálculos con valores publicados", "führt die Referenzfälle aus, die die Rechnungen an veröffentlichten Werten prüfen", "exécute les cas de référence qui vérifient les calculs avec des valeurs publiées", "esegue i casi di riferimento che verificano i calcoli con valori pubblicati", "公表値で計算を検証する基準ケースを実行する", "运行用公开数值校验计算的参考用例", "executa os casos de referência que verificam os cálculos com valores publicados"],
        run: Some(crate::oracle::oracle_command),
    },
    Command {
        name: "corpus",
        topic: Topic::Errors,
        doc: "docs/CORPUS.md",
        summary: ["run the own corpus of small hand-written pieces", "ejecuta el corpus propio de piezas pequeñas escritas a mano", "führt das eigene Korpus kleiner handgeschriebener Stücke aus", "exécute le corpus propre de petites pièces écrites à la main", "esegue il corpus proprio di piccoli pezzi scritti a mano", "手書きの小さなファイルからなる独自コーパスを実行する", "运行由手写小文件组成的自有语料库", "executa o corpus próprio de pequenas peças escritas à mão"],
        run: Some(crate::corpus::corpus_command),
    },
    Command {
        name: "view",
        topic: Topic::Board,
        doc: "docs/VIEWER.md",
        summary: ["open a board, a schematic or a panel in the window", "abre una placa, un esquema o un panel en la ventana", "öffnet eine Platine, einen Schaltplan oder einen Nutzen im Fenster", "ouvre une carte, un schéma ou un panneau dans la fenêtre", "apre una scheda, uno schema o un pannello nella finestra", "基板・回路図・パネルをウィンドウで開く", "在窗口中打开电路板、原理图或拼板", "abre uma placa, um esquema ou um painel na janela"],
        run: Some(crate::view::view),
    },
];

/// The command with this name.
pub fn command(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}
