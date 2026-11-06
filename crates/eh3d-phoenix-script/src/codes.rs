// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the script interpreter, `PHOENIX-SCRIPT-<NNN>`, with their titles in the
//! eight languages of the suite, and the diagnostic that carries one with the place in the script.
//!
//! Codes 001 to 026 are errors and notes of a run or of a reader; codes 027 to 042 are the causes
//! that the compatibility report gives for a command that is pending or adapted.

use eh3d_phoenix_rules::Lang;

/// The settings file is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-SCRIPT-001";
/// A limit of the settings was exceeded (size, lines, commands, arguments, text, operations, files).
pub const LIMIT_EXCEEDED: &str = "PHOENIX-SCRIPT-002";
/// A text in quotes is not closed.
pub const SYNTAX_STRING: &str = "PHOENIX-SCRIPT-003";
/// A coordinate in parentheses is not valid.
pub const SYNTAX_COORD: &str = "PHOENIX-SCRIPT-004";
/// A command name was expected.
pub const SYNTAX_COMMAND: &str = "PHOENIX-SCRIPT-005";
/// The command is not known.
pub const UNKNOWN_COMMAND: &str = "PHOENIX-SCRIPT-006";
/// An argument of the command is missing or not valid.
pub const ARGUMENT_INVALID: &str = "PHOENIX-SCRIPT-007";
/// A number or a unit is not valid.
pub const NUMBER_INVALID: &str = "PHOENIX-SCRIPT-008";
/// A named object does not exist.
pub const OBJECT_NOT_FOUND: &str = "PHOENIX-SCRIPT-009";
/// A layer does not exist on the board.
pub const LAYER_UNKNOWN: &str = "PHOENIX-SCRIPT-010";
/// The layer cannot hold the object.
pub const LAYER_KIND: &str = "PHOENIX-SCRIPT-011";
/// The board refused an operation.
pub const OPERATION_REFUSED: &str = "PHOENIX-SCRIPT-012";
/// A file path is not allowed.
pub const PATH_REFUSED: &str = "PHOENIX-SCRIPT-013";
/// A file cannot be read or written.
pub const FILE_FAILED: &str = "PHOENIX-SCRIPT-014";
/// Scripts are nested deeper than the limit.
pub const NESTING: &str = "PHOENIX-SCRIPT-015";
/// The command does not apply to the kind of document.
pub const WRONG_TARGET: &str = "PHOENIX-SCRIPT-016";
/// No new identity could be made.
pub const ID_EXHAUSTED: &str = "PHOENIX-SCRIPT-017";
/// A rotation that is not supported.
pub const ROTATION_INVALID: &str = "PHOENIX-SCRIPT-018";
/// A form of a command that is not implemented was met while running.
pub const PENDING_FORM: &str = "PHOENIX-SCRIPT-019";
/// An option of the command line is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-SCRIPT-020";
/// An input file cannot be read.
pub const INPUT_UNREADABLE: &str = "PHOENIX-SCRIPT-021";
/// An output file cannot be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-SCRIPT-022";
/// An output job cannot be read.
pub const CAM_INVALID: &str = "PHOENIX-SCRIPT-023";
/// The device of an output job has no equivalent.
pub const CAM_DEVICE: &str = "PHOENIX-SCRIPT-024";
/// An option of an output job was not translated.
pub const CAM_OPTION: &str = "PHOENIX-SCRIPT-025";
/// Several layers in one file are written as one file per layer.
pub const CAM_MERGED: &str = "PHOENIX-SCRIPT-026";
/// Cause: a known command that has no equivalent yet.
pub const C_KNOWN_PENDING: &str = "PHOENIX-SCRIPT-027";
/// Cause: selecting by pointing at a coordinate is not implemented.
pub const C_PICK: &str = "PHOENIX-SCRIPT-028";
/// Cause: curved wires are not implemented.
pub const C_CURVE: &str = "PHOENIX-SCRIPT-029";
/// Cause: the board has no operation to create a layer.
pub const C_LAYER_CREATE: &str = "PHOENIX-SCRIPT-030";
/// Cause: mirrored placement is not implemented.
pub const C_MIRROR: &str = "PHOENIX-SCRIPT-031";
/// Cause: the option of SET is not implemented.
pub const C_SET_OPTION: &str = "PHOENIX-SCRIPT-032";
/// Difference: the display state is only recorded.
pub const A_VIEW: &str = "PHOENIX-SCRIPT-033";
/// Difference: the work is done by the engine of this tool.
pub const A_ENGINE: &str = "PHOENIX-SCRIPT-034";
/// Difference: drawn as an outline stroke on a silkscreen layer.
pub const A_SILK: &str = "PHOENIX-SCRIPT-035";
/// Difference: text uses the built-in stroke font.
pub const A_FONT: &str = "PHOENIX-SCRIPT-036";
/// Difference: accepted and ignored.
pub const A_IGNORED: &str = "PHOENIX-SCRIPT-037";
/// Difference: only relative paths allowed by the settings.
pub const A_FILES: &str = "PHOENIX-SCRIPT-038";
/// Difference: the included script is not followed by the report.
pub const A_INCLUDE: &str = "PHOENIX-SCRIPT-039";
/// Difference: objects are selected by name.
pub const A_SELECT: &str = "PHOENIX-SCRIPT-040";
/// Difference: the shape of a via is ignored.
pub const A_VIA: &str = "PHOENIX-SCRIPT-041";
/// A limit of the walk of a folder was reached.
pub const REPORT_LIMIT: &str = "PHOENIX-SCRIPT-042";

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (SETTINGS_INVALID, "Script settings are not valid"),
    (LIMIT_EXCEEDED, "A limit of the settings was exceeded"),
    (SYNTAX_STRING, "Text in quotes is not closed"),
    (SYNTAX_COORD, "Coordinate is not valid"),
    (SYNTAX_COMMAND, "A command name was expected"),
    (UNKNOWN_COMMAND, "Unknown command"),
    (ARGUMENT_INVALID, "Command argument is missing or not valid"),
    (NUMBER_INVALID, "Number or unit is not valid"),
    (OBJECT_NOT_FOUND, "Object not found"),
    (LAYER_UNKNOWN, "Layer not found"),
    (LAYER_KIND, "The layer cannot hold this object"),
    (OPERATION_REFUSED, "The board refused the operation"),
    (PATH_REFUSED, "File path is not allowed"),
    (FILE_FAILED, "File cannot be read or written"),
    (NESTING, "Scripts are nested too deeply"),
    (WRONG_TARGET, "The command does not apply to this document"),
    (ID_EXHAUSTED, "No new identity could be made"),
    (ROTATION_INVALID, "Rotation is not supported here"),
    (
        PENDING_FORM,
        "This form of the command is not implemented yet",
    ),
    (OPTION_INVALID, "Command option is not valid"),
    (INPUT_UNREADABLE, "The input cannot be read"),
    (OUTPUT_FAILED, "Output file cannot be written"),
    (CAM_INVALID, "The output job cannot be read"),
    (CAM_DEVICE, "The output device has no equivalent"),
    (CAM_OPTION, "An option of the output job was not translated"),
    (
        CAM_MERGED,
        "Several layers in one file are written as one file per layer",
    ),
    (C_KNOWN_PENDING, "Known command without an equivalent yet"),
    (
        C_PICK,
        "Selecting by pointing at a coordinate is not implemented",
    ),
    (C_CURVE, "Curved wires are not implemented"),
    (
        C_LAYER_CREATE,
        "The board has no operation to create a layer",
    ),
    (C_MIRROR, "Mirrored placement is not implemented"),
    (C_SET_OPTION, "This option of SET is not implemented"),
    (
        A_VIEW,
        "Only recorded: the display is not part of the board",
    ),
    (
        A_ENGINE,
        "Delegated to the engine of this tool, not compared with the original tool",
    ),
    (A_SILK, "Drawn as an outline stroke on a silkscreen layer"),
    (A_FONT, "Text uses the built-in stroke font"),
    (
        A_IGNORED,
        "Accepted and ignored: it only changes how the original tool looks or behaves",
    ),
    (
        A_FILES,
        "Only relative paths allowed by the settings; boards are JSON files",
    ),
    (
        A_INCLUDE,
        "The included script is read at run time and the report does not follow it",
    ),
    (A_SELECT, "Selects objects by name, not by a rectangle"),
    (
        A_VIA,
        "The shape is ignored: vias are round and join the outer copper layers",
    ),
    (REPORT_LIMIT, "A limit of the folder walk was reached"),
];

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SETTINGS_INVALID,
        [
            "Script settings are not valid",
            "La configuración de los scripts no es válida",
            "Die Einstellungen der Skripte sind ungültig",
            "Les réglages des scripts ne sont pas valides",
            "Le impostazioni degli script non sono valide",
            "スクリプトの設定が正しくありません",
            "脚本设置无效",
            "As definições dos scripts não são válidas",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the settings was exceeded",
            "Se superó un límite de la configuración",
            "Eine Grenze der Einstellungen wurde überschritten",
            "Une limite des réglages est dépassée",
            "È stato superato un limite delle impostazioni",
            "設定の上限を超えました",
            "超出了设置中的限制",
            "Foi excedido um limite das definições",
        ],
    ),
    (
        SYNTAX_STRING,
        [
            "Text in quotes is not closed",
            "El texto entre comillas no está cerrado",
            "Der Text in Anführungszeichen ist nicht geschlossen",
            "Le texte entre guillemets n'est pas fermé",
            "Il testo tra apici non è chiuso",
            "引用符で囲んだ文字列が閉じられていません",
            "引号内的文本没有结束",
            "O texto entre aspas não está fechado",
        ],
    ),
    (
        SYNTAX_COORD,
        [
            "Coordinate is not valid",
            "La coordenada no es válida",
            "Die Koordinate ist ungültig",
            "La coordonnée n'est pas valide",
            "La coordinata non è valida",
            "座標が正しくありません",
            "坐标无效",
            "A coordenada não é válida",
        ],
    ),
    (
        SYNTAX_COMMAND,
        [
            "A command name was expected",
            "Se esperaba el nombre de una orden",
            "Ein Befehlsname wurde erwartet",
            "Un nom de commande était attendu",
            "Era previsto il nome di un comando",
            "コマンド名が必要です",
            "此处应为命令名",
            "Era esperado o nome de um comando",
        ],
    ),
    (
        UNKNOWN_COMMAND,
        [
            "Unknown command",
            "Orden desconocida",
            "Unbekannter Befehl",
            "Commande inconnue",
            "Comando sconosciuto",
            "不明なコマンド",
            "未知命令",
            "Comando desconhecido",
        ],
    ),
    (
        ARGUMENT_INVALID,
        [
            "Command argument is missing or not valid",
            "Falta un argumento de la orden o no es válido",
            "Ein Argument des Befehls fehlt oder ist ungültig",
            "Un argument de la commande manque ou n'est pas valide",
            "Un argomento del comando manca o non è valido",
            "コマンドの引数がないか正しくありません",
            "命令参数缺失或无效",
            "Falta um argumento do comando ou não é válido",
        ],
    ),
    (
        NUMBER_INVALID,
        [
            "Number or unit is not valid",
            "El número o la unidad no es válido",
            "Zahl oder Einheit ist ungültig",
            "Le nombre ou l'unité n'est pas valide",
            "Il numero o l'unità non è valido",
            "数値または単位が正しくありません",
            "数字或单位无效",
            "O número ou a unidade não é válido",
        ],
    ),
    (
        OBJECT_NOT_FOUND,
        [
            "Object not found",
            "Objeto no encontrado",
            "Objekt nicht gefunden",
            "Objet introuvable",
            "Oggetto non trovato",
            "オブジェクトが見つかりません",
            "未找到对象",
            "Objeto não encontrado",
        ],
    ),
    (
        LAYER_UNKNOWN,
        [
            "Layer not found",
            "Capa no encontrada",
            "Ebene nicht gefunden",
            "Couche introuvable",
            "Livello non trovato",
            "レイヤーが見つかりません",
            "未找到层",
            "Camada não encontrada",
        ],
    ),
    (
        LAYER_KIND,
        [
            "The layer cannot hold this object",
            "La capa no admite este objeto",
            "Die Ebene kann dieses Objekt nicht aufnehmen",
            "La couche ne peut pas contenir cet objet",
            "Il livello non può contenere questo oggetto",
            "このレイヤーにはこのオブジェクトを置けません",
            "该层不能容纳此对象",
            "A camada não pode conter este objeto",
        ],
    ),
    (
        OPERATION_REFUSED,
        [
            "The board refused the operation",
            "La placa rechazó la operación",
            "Die Platine hat die Operation abgelehnt",
            "La carte a refusé l'opération",
            "La scheda ha rifiutato l'operazione",
            "基板が操作を拒否しました",
            "电路板拒绝了该操作",
            "A placa recusou a operação",
        ],
    ),
    (
        PATH_REFUSED,
        [
            "File path is not allowed",
            "La ruta del archivo no está permitida",
            "Der Dateipfad ist nicht erlaubt",
            "Le chemin du fichier n'est pas autorisé",
            "Il percorso del file non è consentito",
            "ファイルのパスは許可されていません",
            "不允许使用该文件路径",
            "O caminho do ficheiro não é permitido",
        ],
    ),
    (
        FILE_FAILED,
        [
            "File cannot be read or written",
            "No se puede leer o escribir el archivo",
            "Die Datei kann nicht gelesen oder geschrieben werden",
            "Le fichier ne peut pas être lu ou écrit",
            "Il file non può essere letto o scritto",
            "ファイルを読み書きできません",
            "无法读取或写入文件",
            "Não é possível ler ou escrever o ficheiro",
        ],
    ),
    (
        NESTING,
        [
            "Scripts are nested too deeply",
            "Los scripts están anidados con demasiada profundidad",
            "Die Skripte sind zu tief verschachtelt",
            "Les scripts sont imbriqués trop profondément",
            "Gli script sono annidati troppo in profondità",
            "スクリプトの入れ子が深すぎます",
            "脚本嵌套过深",
            "Os scripts estão aninhados com demasiada profundidade",
        ],
    ),
    (
        WRONG_TARGET,
        [
            "The command does not apply to this document",
            "La orden no se aplica a este documento",
            "Der Befehl gilt nicht für dieses Dokument",
            "La commande ne s'applique pas à ce document",
            "Il comando non si applica a questo documento",
            "このコマンドはこのドキュメントには使えません",
            "该命令不适用于此文档",
            "O comando não se aplica a este documento",
        ],
    ),
    (
        ID_EXHAUSTED,
        [
            "No new identity could be made",
            "No se pudo crear una identidad nueva",
            "Es konnte keine neue Identität erzeugt werden",
            "Aucune nouvelle identité n'a pu être créée",
            "Non è stato possibile creare una nuova identità",
            "新しい識別子を作成できませんでした",
            "无法生成新的标识",
            "Não foi possível criar uma nova identidade",
        ],
    ),
    (
        ROTATION_INVALID,
        [
            "Rotation is not supported here",
            "La rotación no es admisible aquí",
            "Die Drehung wird hier nicht unterstützt",
            "La rotation n'est pas prise en charge ici",
            "La rotazione non è supportata qui",
            "ここでは、この回転は使えません",
            "此处不支持该旋转",
            "A rotação não é suportada aqui",
        ],
    ),
    (
        PENDING_FORM,
        [
            "This form of the command is not implemented yet",
            "Esta forma de la orden aún no está implementada",
            "Diese Form des Befehls ist noch nicht umgesetzt",
            "Cette forme de la commande n'est pas encore implémentée",
            "Questa forma del comando non è ancora implementata",
            "このコマンドの形式はまだ実装されていません",
            "该命令的这种形式尚未实现",
            "Esta forma do comando ainda não está implementada",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Command option is not valid",
            "La opción del comando no es válida",
            "Die Option des Befehls ist ungültig",
            "L'option de la commande n'est pas valide",
            "L'opzione del comando non è valida",
            "コマンドのオプションが正しくありません",
            "命令选项无效",
            "A opção do comando não é válida",
        ],
    ),
    (
        INPUT_UNREADABLE,
        [
            "The input cannot be read",
            "No se puede leer la entrada",
            "Die Eingabe kann nicht gelesen werden",
            "L'entrée ne peut pas être lue",
            "L'input non può essere letto",
            "入力を読み込めません",
            "无法读取输入",
            "Não é possível ler a entrada",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "Output file cannot be written",
            "No se puede escribir el archivo de salida",
            "Die Ausgabedatei kann nicht geschrieben werden",
            "Le fichier de sortie ne peut pas être écrit",
            "Il file di uscita non può essere scritto",
            "出力ファイルを書き込めません",
            "无法写入输出文件",
            "Não é possível escrever o ficheiro de saída",
        ],
    ),
    (
        CAM_INVALID,
        [
            "The output job cannot be read",
            "No se puede leer el trabajo de salida",
            "Der Ausgabejob kann nicht gelesen werden",
            "Le travail de sortie ne peut pas être lu",
            "Il job di uscita non può essere letto",
            "出力ジョブを読み込めません",
            "无法读取输出作业",
            "Não é possível ler o trabalho de saída",
        ],
    ),
    (
        CAM_DEVICE,
        [
            "The output device has no equivalent",
            "El dispositivo de salida no tiene equivalente",
            "Das Ausgabegerät hat keine Entsprechung",
            "Le périphérique de sortie n'a pas d'équivalent",
            "Il dispositivo di uscita non ha un equivalente",
            "出力デバイスに相当するものがありません",
            "输出设备没有对应项",
            "O dispositivo de saída não tem equivalente",
        ],
    ),
    (
        CAM_OPTION,
        [
            "An option of the output job was not translated",
            "Una opción del trabajo de salida no se tradujo",
            "Eine Option des Ausgabejobs wurde nicht übersetzt",
            "Une option du travail de sortie n'a pas été traduite",
            "Un'opzione del job di uscita non è stata tradotta",
            "出力ジョブのオプションは変換されませんでした",
            "输出作业的某个选项未被转换",
            "Uma opção do trabalho de saída não foi traduzida",
        ],
    ),
    (
        CAM_MERGED,
        [
            "Several layers in one file are written as one file per layer",
            "Varias capas en un archivo se escriben como un archivo por capa",
            "Mehrere Ebenen in einer Datei werden als je eine Datei pro Ebene geschrieben",
            "Plusieurs couches dans un fichier sont écrites en un fichier par couche",
            "Più livelli in un file vengono scritti come un file per livello",
            "1 つのファイルにまとめた複数のレイヤーはレイヤーごとのファイルとして書き出されます",
            "一个文件中的多个层将按每层一个文件写出",
            "Várias camadas num ficheiro são escritas como um ficheiro por camada",
        ],
    ),
    (
        C_KNOWN_PENDING,
        [
            "Known command without an equivalent yet",
            "Orden conocida sin equivalente todavía",
            "Bekannter Befehl ohne Entsprechung",
            "Commande connue sans équivalent pour l'instant",
            "Comando noto senza ancora un equivalente",
            "対応するものがまだないコマンド",
            "已知命令，尚无对应实现",
            "Comando conhecido sem equivalente por enquanto",
        ],
    ),
    (
        C_PICK,
        [
            "Selecting by pointing at a coordinate is not implemented",
            "La selección señalando una coordenada no está implementada",
            "Die Auswahl durch Zeigen auf eine Koordinate ist nicht umgesetzt",
            "La sélection en pointant une coordonnée n'est pas implémentée",
            "La selezione puntando una coordinata non è implementata",
            "座標を指してオブジェクトを選ぶ方法は実装されていません",
            "通过指向坐标来选择对象的方式尚未实现",
            "A seleção por apontamento a uma coordenada não está implementada",
        ],
    ),
    (
        C_CURVE,
        [
            "Curved wires are not implemented",
            "Los hilos curvos no están implementados",
            "Gebogene Leiterbahnen sind nicht umgesetzt",
            "Les fils courbes ne sont pas implémentés",
            "I fili curvi non sono implementati",
            "曲線の配線は実装されていません",
            "弧形导线尚未实现",
            "Os fios curvos não estão implementados",
        ],
    ),
    (
        C_LAYER_CREATE,
        [
            "The board has no operation to create a layer",
            "La placa no tiene una operación para crear una capa",
            "Die Platine hat keine Operation zum Anlegen einer Ebene",
            "La carte n'a pas d'opération pour créer une couche",
            "La scheda non ha un'operazione per creare un livello",
            "基板にはレイヤーを作成する操作がありません",
            "电路板没有创建层的操作",
            "A placa não tem uma operação para criar uma camada",
        ],
    ),
    (
        C_MIRROR,
        [
            "Mirrored placement is not implemented",
            "La colocación en espejo no está implementada",
            "Gespiegelte Platzierung ist nicht umgesetzt",
            "Le placement en miroir n'est pas implémenté",
            "Il posizionamento speculare non è implementato",
            "ミラー配置は実装されていません",
            "镜像放置尚未实现",
            "A colocação em espelho não está implementada",
        ],
    ),
    (
        C_SET_OPTION,
        [
            "This option of SET is not implemented",
            "Esta opción de SET no está implementada",
            "Diese Option von SET ist nicht umgesetzt",
            "Cette option de SET n'est pas implémentée",
            "Questa opzione di SET non è implementata",
            "SET のこのオプションは実装されていません",
            "SET 的此选项尚未实现",
            "Esta opção de SET não está implementada",
        ],
    ),
    (
        A_VIEW,
        [
            "Only recorded: the display is not part of the board",
            "Solo se registra: la visualización no forma parte de la placa",
            "Nur vermerkt: die Anzeige gehört nicht zur Platine",
            "Simplement enregistré : l'affichage ne fait pas partie de la carte",
            "Solo registrato: la visualizzazione non fa parte della scheda",
            "記録のみ。表示は基板の一部ではありません",
            "仅作记录：显示状态不属于电路板",
            "Apenas registado: a visualização não faz parte da placa",
        ],
    ),
    (
        A_ENGINE,
        [
            "Delegated to the engine of this tool, not compared with the original tool",
            "Delegado en el motor de esta herramienta, sin comparar con la herramienta original",
            "An die Engine dieses Werkzeugs übergeben, nicht mit dem Originalwerkzeug verglichen",
            "Confié au moteur de cet outil, non comparé à l'outil d'origine",
            "Delegato al motore di questo strumento, non confrontato con lo strumento originale",
            "このツールのエンジンに任せており、元のツールとは比較していません",
            "交由本工具的引擎处理，未与原工具比较",
            "Delegado no motor desta ferramenta, não comparado com a ferramenta original",
        ],
    ),
    (
        A_SILK,
        [
            "Drawn as an outline stroke on a silkscreen layer",
            "Se dibuja como un trazo de contorno en una capa de serigrafía",
            "Als Konturlinie auf einer Bestückungsdruck-Ebene gezeichnet",
            "Dessiné comme un tracé de contour sur une couche de sérigraphie",
            "Disegnato come tratto di contorno su un livello di serigrafia",
            "シルク層の輪郭線として描画されます",
            "作为丝印层上的轮廓线绘制",
            "Desenhado como um traço de contorno numa camada de serigrafia",
        ],
    ),
    (
        A_FONT,
        [
            "Text uses the built-in stroke font",
            "El texto usa la fuente de trazos incorporada",
            "Der Text verwendet die eingebaute Strichschrift",
            "Le texte utilise la police à traits intégrée",
            "Il testo usa il carattere a tratti integrato",
            "文字は内蔵のストロークフォントを使います",
            "文本使用内置的笔画字体",
            "O texto usa o tipo de letra de traços incorporado",
        ],
    ),
    (
        A_IGNORED,
        [
            "Accepted and ignored: it only changes how the original tool looks or behaves",
            "Aceptado e ignorado: solo cambia el aspecto o el comportamiento de la herramienta original",
            "Akzeptiert und ignoriert: es ändert nur Aussehen oder Verhalten des Originalwerkzeugs",
            "Accepté et ignoré : cela ne change que l'aspect ou le comportement de l'outil d'origine",
            "Accettato e ignorato: cambia solo l'aspetto o il comportamento dello strumento originale",
            "受け付けますが無視されます。元のツールの見た目や動作を変えるだけです",
            "已接受但忽略：它只改变原工具的外观或行为",
            "Aceite e ignorado: só altera o aspeto ou o comportamento da ferramenta original",
        ],
    ),
    (
        A_FILES,
        [
            "Only relative paths allowed by the settings; boards are JSON files",
            "Solo rutas relativas permitidas por la configuración; las placas son archivos JSON",
            "Nur relative Pfade, die die Einstellungen erlauben; Platinen sind JSON-Dateien",
            "Seulement des chemins relatifs autorisés par les réglages ; les cartes sont des fichiers JSON",
            "Solo percorsi relativi consentiti dalle impostazioni; le schede sono file JSON",
            "設定で許可された相対パスのみ。基板は JSON ファイルです",
            "仅允许设置中许可的相对路径；电路板为 JSON 文件",
            "Só caminhos relativos permitidos pelas definições; as placas são ficheiros JSON",
        ],
    ),
    (
        A_INCLUDE,
        [
            "The included script is read at run time and the report does not follow it",
            "El script incluido se lee al ejecutar y el informe no lo sigue",
            "Das eingebundene Skript wird zur Laufzeit gelesen und der Bericht folgt ihm nicht",
            "Le script inclus est lu à l'exécution et le rapport ne le suit pas",
            "Lo script incluso viene letto durante l'esecuzione e il rapporto non lo segue",
            "読み込まれるスクリプトは実行時に読まれ、レポートでは追跡しません",
            "被包含的脚本在运行时读取，报告不会跟踪它",
            "O script incluído é lido durante a execução e o relatório não o segue",
        ],
    ),
    (
        A_SELECT,
        [
            "Selects objects by name, not by a rectangle",
            "Selecciona objetos por nombre, no por un rectángulo",
            "Wählt Objekte nach Namen aus, nicht durch ein Rechteck",
            "Sélectionne des objets par leur nom, pas par un rectangle",
            "Seleziona gli oggetti per nome, non con un rettangolo",
            "矩形ではなく名前でオブジェクトを選びます",
            "按名称选择对象，而不是按矩形",
            "Seleciona objetos pelo nome, não por um retângulo",
        ],
    ),
    (
        A_VIA,
        [
            "The shape is ignored: vias are round and join the outer copper layers",
            "Se ignora la forma: las vías son redondas y unen las capas exteriores de cobre",
            "Die Form wird ignoriert: Durchkontaktierungen sind rund und verbinden die äußeren Kupferlagen",
            "La forme est ignorée : les vias sont rondes et relient les couches de cuivre extérieures",
            "La forma viene ignorata: i via sono rotondi e collegano i livelli di rame esterni",
            "形状は無視されます。ビアは円形で外層の銅を結びます",
            "忽略形状：过孔为圆形，连接外层铜层",
            "A forma é ignorada: as vias são redondas e ligam as camadas de cobre exteriores",
        ],
    ),
    (
        REPORT_LIMIT,
        [
            "A limit of the folder walk was reached",
            "Se alcanzó un límite del recorrido de la carpeta",
            "Eine Grenze beim Durchlaufen des Ordners wurde erreicht",
            "Une limite du parcours du dossier est atteinte",
            "È stato raggiunto un limite della scansione della cartella",
            "フォルダー走査の上限に達しました",
            "已达到遍历文件夹的限制",
            "Foi atingido um limite da pesquisa da pasta",
        ],
    ),
];

/// The column of a language in the title table.
pub fn column(lang: Lang) -> usize {
    Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0)
}

/// The title of a code in a language, from this table or from the one of the scripts of the second
/// version; empty for a code that is in neither.
pub fn title(code: &str, lang: Lang) -> &'static str {
    match TITLES.iter().find(|(c, _)| *c == code) {
        Some((_, row)) => row[column(lang)],
        None => crate::v2::codes::title(code, lang).unwrap_or(""),
    }
}

/// A diagnostic: a registered code, the place in the script (line and column from 1, or 0 when
/// the diagnostic is not about a place) and what is specific to the case, in English.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    /// The code, `PHOENIX-SCRIPT-<NNN>`.
    pub code: &'static str,
    /// The line of the script, or 0.
    pub line: usize,
    /// The column of the script, or 0.
    pub col: usize,
    /// What is specific to the case.
    pub detail: String,
}

impl Diag {
    /// A diagnostic at a place of a script.
    pub fn new(code: &'static str, line: usize, col: usize, detail: impl Into<String>) -> Diag {
        Diag {
            code,
            line,
            col,
            detail: detail.into(),
        }
    }

    /// A diagnostic that is not about a place.
    pub fn plain(code: &'static str, detail: impl Into<String>) -> Diag {
        Diag::new(code, 0, 0, detail)
    }

    /// The same diagnostic moved to the place of the command that caused it.
    pub fn moved(mut self, line: usize, col: usize) -> Diag {
        if self.line == 0 {
            self.line = line;
            self.col = col;
        }
        self
    }

    /// The message in a language: the code, the title, the place and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let mut text = format!("{}: {}", self.code, title(self.code, lang));
        if self.line > 0 {
            text.push_str(&format!(" [{}:{}]", self.line, self.col));
        }
        if !self.detail.is_empty() {
            text.push_str(": ");
            text.push_str(&self.detail);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_each_language_and_the_english_one_is_the_registry() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        for ((code, english), (tcode, row)) in REGISTRY.iter().zip(TITLES.iter()) {
            assert_eq!(code, tcode);
            assert_eq!(*english, row[0]);
            for (i, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {i}");
            }
            assert!(code.starts_with("PHOENIX-SCRIPT-"));
        }
    }

    #[test]
    fn the_codes_are_numbered_without_gaps_or_repeats() {
        for (i, (code, _)) in REGISTRY.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-SCRIPT-{:03}", i + 1));
        }
    }

    #[test]
    fn no_title_of_another_language_is_a_copy_of_the_english_one() {
        for (code, row) in TITLES.iter() {
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
    }

    #[test]
    fn the_message_names_the_code_the_title_the_place_and_the_detail() {
        let d = Diag::new(UNKNOWN_COMMAND, 3, 5, "FOO");
        assert_eq!(
            d.message(Lang::Pt),
            "PHOENIX-SCRIPT-006: Comando desconhecido [3:5]: FOO"
        );
        let bare = Diag::plain(SETTINGS_INVALID, "");
        assert_eq!(
            bare.message(Lang::En),
            "PHOENIX-SCRIPT-001: Script settings are not valid"
        );
        let moved = Diag::plain(NUMBER_INVALID, "x").moved(7, 2);
        assert_eq!((moved.line, moved.col), (7, 2));
        let kept = Diag::new(NUMBER_INVALID, 1, 1, "x").moved(7, 2);
        assert_eq!((kept.line, kept.col), (1, 1));
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
