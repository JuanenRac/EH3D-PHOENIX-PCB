// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the library of footprints, `PHOENIX-IPC-<NNN>`, and of the bulk import
//! of libraries, `PHOENIX-LIBIDX-<NNN>`, with their titles in the eight languages of the suite,
//! and the error type that carries one.
//!
//! `PHOENIX-IPC-001` to `010` are the findings of the checker of the library, `020` and above are
//! errors of the generator and of the commands, and `040` to `042` are the differences found by
//! the comparison with a footprint of the owner. `PHOENIX-LIBIDX-001` to `010` are the findings
//! of the quality report of an imported item and `020` and above are errors of the import.

use eh3d_phoenix_rules::Lang;

/// A family file is not valid.
pub const FAMILY_INVALID: &str = "PHOENIX-IPC-001";
/// Two footprints of the library have the same name.
pub const NAME_DUPLICATE: &str = "PHOENIX-IPC-002";
/// Two pads of a footprint overlap.
pub const PADS_OVERLAP: &str = "PHOENIX-IPC-003";
/// The distance between pads is not the pitch of the family.
pub const PITCH_WRONG: &str = "PHOENIX-IPC-004";
/// The origin is not at the centre of the pads.
pub const ORIGIN_OFF: &str = "PHOENIX-IPC-005";
/// The solder mask or the paste openings of a pad are not coherent.
pub const MASK_PASTE: &str = "PHOENIX-IPC-006";
/// The silkscreen is too close to a pad.
pub const SILK_ON_PAD: &str = "PHOENIX-IPC-007";
/// The courtyard does not follow the density level.
pub const COURTYARD: &str = "PHOENIX-IPC-008";
/// The name does not follow the naming of IPC-7351B.
pub const NAME_NOT_IPC: &str = "PHOENIX-IPC-009";
/// A pad has no size, or its hole is larger than the pad.
pub const PAD_SIZE: &str = "PHOENIX-IPC-010";

/// A family does not exist.
pub const FAMILY_UNKNOWN: &str = "PHOENIX-IPC-020";
/// A variant does not exist.
pub const VARIANT_UNKNOWN: &str = "PHOENIX-IPC-021";
/// A parameter of the generator is not valid.
pub const PARAM_INVALID: &str = "PHOENIX-IPC-022";
/// A density level is not valid.
pub const DENSITY_INVALID: &str = "PHOENIX-IPC-023";
/// The dimensions give lands that cannot be made.
pub const LAND_IMPOSSIBLE: &str = "PHOENIX-IPC-024";
/// An option of the command is not valid.
pub const OPTION_INVALID: &str = "PHOENIX-IPC-025";
/// A file cannot be read.
pub const FILE_UNUSABLE: &str = "PHOENIX-IPC-026";
/// A file cannot be written, or exists.
pub const WRITE_FAILED: &str = "PHOENIX-IPC-027";
/// A settings file is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-IPC-028";
/// A pad of the footprint of the owner differs from the generated one.
pub const COMPARE_DIFFERS: &str = "PHOENIX-IPC-040";
/// A pad of the generated footprint is missing from the footprint of the owner.
pub const COMPARE_MISSING: &str = "PHOENIX-IPC-041";
/// A pad of the footprint of the owner has no counterpart in the generated one.
pub const COMPARE_EXTRA: &str = "PHOENIX-IPC-042";

/// Two pads of an imported footprint have the same name.
pub const IDX_PAD_DUPLICATE: &str = "PHOENIX-LIBIDX-001";
/// A pad of an imported footprint has no copper.
pub const IDX_PAD_NO_COPPER: &str = "PHOENIX-LIBIDX-002";
/// The hole of a pad is larger than the pad.
pub const IDX_DRILL_TOO_BIG: &str = "PHOENIX-LIBIDX-003";
/// The origin of an imported footprint is far from its pads.
pub const IDX_NO_ORIGIN: &str = "PHOENIX-LIBIDX-004";
/// An imported item has an empty name.
pub const IDX_NAME_EMPTY: &str = "PHOENIX-LIBIDX-005";
/// A footprint has no pads or a symbol has no pins.
pub const IDX_NO_CONTENT: &str = "PHOENIX-LIBIDX-006";
/// Two pins of an imported symbol have the same name.
pub const IDX_PIN_DUPLICATE: &str = "PHOENIX-LIBIDX-007";
/// The same name is used more than once.
pub const IDX_DUP_NAME: &str = "PHOENIX-LIBIDX-008";
/// The same geometry is found under more than one name.
pub const IDX_DUP_GEOMETRY: &str = "PHOENIX-LIBIDX-009";
/// A pin of an imported symbol has no name.
pub const IDX_PIN_NO_NAME: &str = "PHOENIX-LIBIDX-010";
/// A file cannot be read or is not a library.
pub const IDX_FILE_UNREADABLE: &str = "PHOENIX-LIBIDX-020";
/// A file is larger than the limit and was skipped.
pub const IDX_FILE_TOO_BIG: &str = "PHOENIX-LIBIDX-021";
/// The folder has more files than the limit.
pub const IDX_TOO_MANY_FILES: &str = "PHOENIX-LIBIDX-022";
/// A folder is deeper than the limit.
pub const IDX_TOO_DEEP: &str = "PHOENIX-LIBIDX-023";
/// A link, or a path that leaves the folder, was skipped.
pub const IDX_LINK_SKIPPED: &str = "PHOENIX-LIBIDX-024";
/// There are more items than the limit.
pub const IDX_TOO_MANY_ITEMS: &str = "PHOENIX-LIBIDX-025";
/// An option of the command is not valid.
pub const IDX_OPTION_INVALID: &str = "PHOENIX-LIBIDX-026";
/// The index cannot be written, or the file exists.
pub const IDX_WRITE_FAILED: &str = "PHOENIX-LIBIDX-027";
/// A settings file is not valid.
pub const IDX_SETTINGS_INVALID: &str = "PHOENIX-LIBIDX-028";
/// The folder cannot be read.
pub const IDX_FOLDER_UNUSABLE: &str = "PHOENIX-LIBIDX-029";

/// The titles in the eight languages of the suite, in the order of [`Lang::ALL`].
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        FAMILY_INVALID,
        [
            "Family file is not valid",
            "El fichero de familia no es válido",
            "Die Familiendatei ist ungültig",
            "Le fichier de famille n'est pas valide",
            "Il file della famiglia non è valido",
            "ファミリファイルが正しくありません",
            "系列文件无效",
            "O ficheiro da família não é válido",
        ],
    ),
    (
        NAME_DUPLICATE,
        [
            "Two footprints have the same name",
            "Dos huellas tienen el mismo nombre",
            "Zwei Footprints haben denselben Namen",
            "Deux empreintes ont le même nom",
            "Due footprint hanno lo stesso nome",
            "2つのフットプリントが同じ名前です",
            "两个封装名称相同",
            "Duas pegadas têm o mesmo nome",
        ],
    ),
    (
        PADS_OVERLAP,
        [
            "Pads overlap",
            "Pads que se solapan",
            "Pads überlappen sich",
            "Pastilles qui se chevauchent",
            "Pad che si sovrappongono",
            "パッドが重なっています",
            "焊盘重叠",
            "Pads que se sobrepõem",
        ],
    ),
    (
        PITCH_WRONG,
        [
            "Distance between pads is not the pitch of the family",
            "La distancia entre pads no es el paso de la familia",
            "Der Abstand der Pads ist nicht das Rastermaß der Familie",
            "La distance entre pastilles n'est pas le pas de la famille",
            "La distanza tra i pad non è il passo della famiglia",
            "パッド間の距離がファミリのピッチと一致しません",
            "焊盘间距不是该系列的引脚间距",
            "A distância entre pads não é o passo da família",
        ],
    ),
    (
        ORIGIN_OFF,
        [
            "Origin is not at the centre of the pads",
            "El origen no está en el centro de los pads",
            "Der Ursprung liegt nicht in der Mitte der Pads",
            "L'origine n'est pas au centre des pastilles",
            "L'origine non è al centro dei pad",
            "原点がパッドの中心にありません",
            "原点不在焊盘中心",
            "A origem não está no centro dos pads",
        ],
    ),
    (
        MASK_PASTE,
        [
            "Solder mask or paste openings are not coherent",
            "Las aberturas de máscara o de pasta no son coherentes",
            "Lötstopp- oder Pastenöffnungen sind nicht stimmig",
            "Les ouvertures de masque ou de pâte ne sont pas cohérentes",
            "Le aperture di maschera o di pasta non sono coerenti",
            "ソルダーマスクまたはペーストの開口が一貫していません",
            "阻焊或锡膏开口不一致",
            "As aberturas de máscara ou de pasta não são coerentes",
        ],
    ),
    (
        SILK_ON_PAD,
        [
            "Silkscreen is too close to a pad",
            "La serigrafía está demasiado cerca de un pad",
            "Bestückungsdruck zu nah an einem Pad",
            "La sérigraphie est trop proche d'une pastille",
            "La serigrafia è troppo vicina a un pad",
            "シルクがパッドに近すぎます",
            "丝印离焊盘太近",
            "A serigrafia está demasiado perto de um pad",
        ],
    ),
    (
        COURTYARD,
        [
            "Courtyard does not follow the density level",
            "El área de cortesía no sigue el nivel de densidad",
            "Der Platzbedarf folgt nicht der Dichtestufe",
            "La zone d'encombrement ne suit pas le niveau de densité",
            "L'area di ingombro non segue il livello di densità",
            "コートヤードが密度レベルに従っていません",
            "占位区不符合密度等级",
            "A área de ocupação não segue o nível de densidade",
        ],
    ),
    (
        NAME_NOT_IPC,
        [
            "Name does not follow the naming of IPC-7351B",
            "El nombre no sigue la nomenclatura de IPC-7351B",
            "Der Name folgt nicht der Benennung nach IPC-7351B",
            "Le nom ne suit pas la nomenclature d'IPC-7351B",
            "Il nome non segue la nomenclatura di IPC-7351B",
            "名前がIPC-7351Bの命名規則に従っていません",
            "名称不符合IPC-7351B的命名规则",
            "O nome não segue a nomenclatura da IPC-7351B",
        ],
    ),
    (
        PAD_SIZE,
        [
            "Pad without a size, or a hole larger than the pad",
            "Pad sin tamaño, o taladro mayor que el pad",
            "Pad ohne Größe oder Bohrung größer als das Pad",
            "Pastille sans taille, ou perçage plus grand que la pastille",
            "Pad senza dimensione, o foro più grande del pad",
            "サイズのないパッド、またはパッドより大きい穴",
            "焊盘没有尺寸，或孔大于焊盘",
            "Pad sem tamanho, ou furo maior do que o pad",
        ],
    ),
    (
        FAMILY_UNKNOWN,
        [
            "Family does not exist",
            "La familia no existe",
            "Die Familie existiert nicht",
            "La famille n'existe pas",
            "La famiglia non esiste",
            "ファミリが存在しません",
            "该系列不存在",
            "A família não existe",
        ],
    ),
    (
        VARIANT_UNKNOWN,
        [
            "Variant does not exist",
            "La variante no existe",
            "Die Variante existiert nicht",
            "La variante n'existe pas",
            "La variante non esiste",
            "バリアントが存在しません",
            "该变体不存在",
            "A variante não existe",
        ],
    ),
    (
        PARAM_INVALID,
        [
            "Parameter of the generator is not valid",
            "Un parámetro del generador no es válido",
            "Ein Parameter des Generators ist ungültig",
            "Un paramètre du générateur n'est pas valide",
            "Un parametro del generatore non è valido",
            "ジェネレータのパラメータが正しくありません",
            "生成器参数无效",
            "Um parâmetro do gerador não é válido",
        ],
    ),
    (
        DENSITY_INVALID,
        [
            "Density level is not A, B or C",
            "El nivel de densidad no es A, B ni C",
            "Die Dichtestufe ist nicht A, B oder C",
            "Le niveau de densité n'est pas A, B ou C",
            "Il livello di densità non è A, B o C",
            "密度レベルがA、B、Cのいずれでもありません",
            "密度等级不是A、B或C",
            "O nível de densidade não é A, B nem C",
        ],
    ),
    (
        LAND_IMPOSSIBLE,
        [
            "The dimensions give lands that cannot be made",
            "Las dimensiones dan pads que no se pueden hacer",
            "Die Maße ergeben Pads, die sich nicht herstellen lassen",
            "Les dimensions donnent des pastilles irréalisables",
            "Le dimensioni danno pad che non si possono fare",
            "寸法から作れないランドになります",
            "这些尺寸得到的焊盘无法制作",
            "As dimensões dão pads que não se podem fazer",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Command option is not valid",
            "Una opción del comando no es válida",
            "Eine Option des Befehls ist ungültig",
            "Une option de la commande n'est pas valide",
            "Un'opzione del comando non è valida",
            "コマンドのオプションが正しくありません",
            "命令选项无效",
            "Uma opção do comando não é válida",
        ],
    ),
    (
        FILE_UNUSABLE,
        [
            "File cannot be read",
            "No se puede leer el fichero",
            "Die Datei kann nicht gelesen werden",
            "Le fichier ne peut pas être lu",
            "Il file non può essere letto",
            "ファイルを読み込めません",
            "无法读取文件",
            "Não é possível ler o ficheiro",
        ],
    ),
    (
        WRITE_FAILED,
        [
            "File cannot be written, or it exists",
            "No se puede escribir el fichero, o ya existe",
            "Die Datei kann nicht geschrieben werden oder existiert bereits",
            "Le fichier ne peut pas être écrit ou existe déjà",
            "Il file non può essere scritto o esiste già",
            "ファイルを書き込めないか、既に存在します",
            "无法写入文件，或文件已存在",
            "Não é possível escrever o ficheiro, ou já existe",
        ],
    ),
    (
        SETTINGS_INVALID,
        [
            "Settings file is not valid",
            "El fichero de ajustes no es válido",
            "Die Einstellungsdatei ist ungültig",
            "Le fichier de réglages n'est pas valide",
            "Il file delle impostazioni non è valido",
            "設定ファイルが正しくありません",
            "设置文件无效",
            "O ficheiro de definições não é válido",
        ],
    ),
    (
        COMPARE_DIFFERS,
        [
            "Pad differs from the generated one",
            "El pad difiere del generado",
            "Das Pad weicht vom erzeugten ab",
            "La pastille diffère de celle générée",
            "Il pad differisce da quello generato",
            "パッドが生成されたものと異なります",
            "焊盘与生成的焊盘不同",
            "O pad difere do gerado",
        ],
    ),
    (
        COMPARE_MISSING,
        [
            "Pad of the generated footprint is missing",
            "Falta un pad de la huella generada",
            "Ein Pad des erzeugten Footprints fehlt",
            "Il manque une pastille de l'empreinte générée",
            "Manca un pad del footprint generato",
            "生成されたフットプリントのパッドがありません",
            "缺少生成封装中的焊盘",
            "Falta um pad da pegada gerada",
        ],
    ),
    (
        COMPARE_EXTRA,
        [
            "Pad has no counterpart in the generated footprint",
            "El pad no tiene pareja en la huella generada",
            "Das Pad hat keine Entsprechung im erzeugten Footprint",
            "La pastille n'a pas d'équivalent dans l'empreinte générée",
            "Il pad non ha corrispondenza nel footprint generato",
            "生成されたフットプリントに対応するパッドがありません",
            "生成的封装中没有对应的焊盘",
            "O pad não tem correspondente na pegada gerada",
        ],
    ),
    (
        IDX_PAD_DUPLICATE,
        [
            "Pads with the same name",
            "Pads con el mismo nombre",
            "Pads mit gleichem Namen",
            "Pastilles avec le même nom",
            "Pad con lo stesso nome",
            "同じ名前のパッド",
            "名称相同的焊盘",
            "Pads com o mesmo nome",
        ],
    ),
    (
        IDX_PAD_NO_COPPER,
        [
            "Pad without copper",
            "Pad sin cobre",
            "Pad ohne Kupfer",
            "Pastille sans cuivre",
            "Pad senza rame",
            "銅箔のないパッド",
            "没有铜的焊盘",
            "Pad sem cobre",
        ],
    ),
    (
        IDX_DRILL_TOO_BIG,
        [
            "Hole larger than the pad",
            "Taladro mayor que el pad",
            "Bohrung größer als das Pad",
            "Perçage plus grand que la pastille",
            "Foro più grande del pad",
            "パッドより大きい穴",
            "孔大于焊盘",
            "Furo maior do que o pad",
        ],
    ),
    (
        IDX_NO_ORIGIN,
        [
            "Origin far from the pads",
            "Origen lejos de los pads",
            "Ursprung weit von den Pads entfernt",
            "Origine loin des pastilles",
            "Origine lontana dai pad",
            "原点がパッドから遠すぎます",
            "原点远离焊盘",
            "Origem longe dos pads",
        ],
    ),
    (
        IDX_NAME_EMPTY,
        [
            "Name is empty",
            "El nombre está vacío",
            "Der Name ist leer",
            "Le nom est vide",
            "Il nome è vuoto",
            "名前が空です",
            "名称为空",
            "O nome está vazio",
        ],
    ),
    (
        IDX_NO_CONTENT,
        [
            "Footprint without pads or symbol without pins",
            "Huella sin pads o símbolo sin pines",
            "Footprint ohne Pads oder Symbol ohne Pins",
            "Empreinte sans pastilles ou symbole sans broches",
            "Footprint senza pad o simbolo senza pin",
            "パッドのないフットプリントまたはピンのないシンボル",
            "没有焊盘的封装或没有引脚的符号",
            "Pegada sem pads ou símbolo sem pinos",
        ],
    ),
    (
        IDX_PIN_DUPLICATE,
        [
            "Pins of a symbol with the same name",
            "Pines de un símbolo con el mismo nombre",
            "Pins eines Symbols mit gleichem Namen",
            "Broches d'un symbole avec le même nom",
            "Pin di un simbolo con lo stesso nome",
            "同じ名前を持つシンボルのピン",
            "符号中名称相同的引脚",
            "Pinos de um símbolo com o mesmo nome",
        ],
    ),
    (
        IDX_DUP_NAME,
        [
            "Same name used more than once",
            "El mismo nombre se usa más de una vez",
            "Derselbe Name wird mehrfach verwendet",
            "Le même nom est utilisé plusieurs fois",
            "Lo stesso nome è usato più volte",
            "同じ名前が複数回使われています",
            "同一名称被多次使用",
            "O mesmo nome é usado mais de uma vez",
        ],
    ),
    (
        IDX_DUP_GEOMETRY,
        [
            "Same geometry under more than one name",
            "La misma geometría con más de un nombre",
            "Dieselbe Geometrie unter mehreren Namen",
            "La même géométrie sous plusieurs noms",
            "Stessa geometria con più nomi",
            "同じ形状が複数の名前で存在します",
            "同一几何形状有多个名称",
            "A mesma geometria com mais de um nome",
        ],
    ),
    (
        IDX_PIN_NO_NAME,
        [
            "Pin of a symbol without a name",
            "Pin de un símbolo sin nombre",
            "Pin eines Symbols ohne Namen",
            "Broche d'un symbole sans nom",
            "Pin di un simbolo senza nome",
            "名前のないシンボルのピン",
            "没有名称的符号引脚",
            "Pino de um símbolo sem nome",
        ],
    ),
    (
        IDX_FILE_UNREADABLE,
        [
            "File cannot be read or is not a library",
            "No se puede leer el fichero o no es una biblioteca",
            "Die Datei kann nicht gelesen werden oder ist keine Bibliothek",
            "Le fichier ne peut pas être lu ou n'est pas une bibliothèque",
            "Il file non può essere letto o non è una libreria",
            "ファイルを読み込めないか、ライブラリではありません",
            "无法读取文件，或不是库文件",
            "Não é possível ler o ficheiro, ou não é uma biblioteca",
        ],
    ),
    (
        IDX_FILE_TOO_BIG,
        [
            "File is larger than the limit and was skipped",
            "El fichero supera el límite y se omitió",
            "Die Datei überschreitet das Limit und wurde übersprungen",
            "Le fichier dépasse la limite et a été ignoré",
            "Il file supera il limite ed è stato saltato",
            "ファイルが上限を超えているためスキップしました",
            "文件超过上限，已跳过",
            "O ficheiro excede o limite e foi ignorado",
        ],
    ),
    (
        IDX_TOO_MANY_FILES,
        [
            "Folder has more files than the limit",
            "La carpeta tiene más ficheros que el límite",
            "Der Ordner enthält mehr Dateien als das Limit",
            "Le dossier contient plus de fichiers que la limite",
            "La cartella ha più file del limite",
            "フォルダのファイル数が上限を超えています",
            "文件夹中的文件数超过上限",
            "A pasta tem mais ficheiros do que o limite",
        ],
    ),
    (
        IDX_TOO_DEEP,
        [
            "Folder is deeper than the limit",
            "La carpeta es más profunda que el límite",
            "Der Ordner ist tiefer verschachtelt als das Limit",
            "Le dossier est plus profond que la limite",
            "La cartella è più profonda del limite",
            "フォルダの階層が上限を超えています",
            "文件夹层级超过上限",
            "A pasta é mais profunda do que o limite",
        ],
    ),
    (
        IDX_LINK_SKIPPED,
        [
            "Link or path outside the folder was skipped",
            "Se omitió un enlace o una ruta fuera de la carpeta",
            "Ein Link oder ein Pfad außerhalb des Ordners wurde übersprungen",
            "Un lien ou un chemin hors du dossier a été ignoré",
            "Un collegamento o un percorso fuori dalla cartella è stato saltato",
            "リンクまたはフォルダ外のパスをスキップしました",
            "已跳过链接或文件夹之外的路径",
            "Foi ignorada uma ligação ou um caminho fora da pasta",
        ],
    ),
    (
        IDX_TOO_MANY_ITEMS,
        [
            "More items than the limit",
            "Hay más elementos que el límite",
            "Mehr Elemente als das Limit",
            "Plus d'éléments que la limite",
            "Più elementi del limite",
            "要素数が上限を超えています",
            "元素数量超过上限",
            "Há mais elementos do que o limite",
        ],
    ),
    (
        IDX_OPTION_INVALID,
        [
            "Command option is not valid",
            "Una opción del comando no es válida",
            "Eine Option des Befehls ist ungültig",
            "Une option de la commande n'est pas valide",
            "Un'opzione del comando non è valida",
            "コマンドのオプションが正しくありません",
            "命令选项无效",
            "Uma opção do comando não é válida",
        ],
    ),
    (
        IDX_WRITE_FAILED,
        [
            "Index cannot be written, or the file exists",
            "No se puede escribir el índice, o el fichero ya existe",
            "Der Index kann nicht geschrieben werden oder die Datei existiert bereits",
            "L'index ne peut pas être écrit ou le fichier existe déjà",
            "L'indice non può essere scritto o il file esiste già",
            "インデックスを書き込めないか、ファイルが既に存在します",
            "无法写入索引，或文件已存在",
            "Não é possível escrever o índice, ou o ficheiro já existe",
        ],
    ),
    (
        IDX_SETTINGS_INVALID,
        [
            "Settings file of the import is not valid",
            "El fichero de ajustes de la importación no es válido",
            "Die Einstellungsdatei des Imports ist ungültig",
            "Le fichier de réglages de l'import n'est pas valide",
            "Il file delle impostazioni dell'importazione non è valido",
            "インポートの設定ファイルが正しくありません",
            "导入的设置文件无效",
            "O ficheiro de definições da importação não é válido",
        ],
    ),
    (
        IDX_FOLDER_UNUSABLE,
        [
            "Folder cannot be read",
            "No se puede leer la carpeta",
            "Der Ordner kann nicht gelesen werden",
            "Le dossier ne peut pas être lu",
            "La cartella non può essere letta",
            "フォルダを読み込めません",
            "无法读取文件夹",
            "Não é possível ler a pasta",
        ],
    ),
];

/// The English title of a code, or an empty text for a code that is not in the table.
pub fn english(code: &str) -> &'static str {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[0])
}

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

/// An error of this crate: a registered code and what is specific to the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcError {
    /// The code, `PHOENIX-IPC-<NNN>` or `PHOENIX-LIBIDX-<NNN>`.
    pub code: &'static str,
    /// What is specific to the case: a name, a line or the text of a reader.
    pub detail: String,
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for IpcError {}

impl IpcError {
    /// An error with a code and its detail.
    pub fn new(code: &'static str, detail: impl Into<String>) -> IpcError {
        IpcError {
            code,
            detail: detail.into(),
        }
    }

    /// The message in a language: the code, the title and the detail.
    pub fn message(&self, lang: Lang) -> String {
        let text = title(self.code, lang);
        if self.detail.is_empty() {
            format!("{}: {text}", self.code)
        } else {
            format!("{}: {text}: {}", self.code, self.detail)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_each_language() {
        for (code, row) in TITLES {
            for (i, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {i}");
            }
            assert!(
                code.starts_with("PHOENIX-IPC-") || code.starts_with("PHOENIX-LIBIDX-"),
                "{code}"
            );
        }
    }

    #[test]
    fn codes_are_unique_and_in_order() {
        for pair in TITLES.windows(2) {
            assert!(pair[0].0 < pair[1].0, "{} {}", pair[0].0, pair[1].0);
        }
    }

    #[test]
    fn the_japanese_and_chinese_titles_are_in_their_scripts() {
        for (code, row) in TITLES {
            assert!(!row[5].is_ascii(), "{code} ja");
            assert!(!row[6].is_ascii(), "{code} zh");
        }
    }

    #[test]
    fn the_portuguese_titles_are_those_of_portugal() {
        for (code, row) in TITLES {
            let text = row[7];
            assert!(!text.contains("arquivo"), "{code}");
            assert!(!text.contains("tela"), "{code}");
            assert!(!text.contains("Salvar"), "{code}");
        }
        assert!(title(FILE_UNUSABLE, Lang::Pt).contains("ficheiro"));
    }

    #[test]
    fn the_message_names_the_code_the_title_and_the_detail() {
        let e = IpcError::new(FAMILY_UNKNOWN, "chop");
        assert_eq!(
            e.message(Lang::En),
            "PHOENIX-IPC-020: Family does not exist: chop"
        );
        assert_eq!(e.to_string(), "PHOENIX-IPC-020: chop");
        let bare = IpcError::new(FAMILY_UNKNOWN, "");
        assert_eq!(
            bare.message(Lang::Es),
            "PHOENIX-IPC-020: La familia no existe"
        );
        assert_eq!(english("PHOENIX-IPC-999"), "");
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_crate() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, row) in TITLES {
            let line = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(line.contains(row[0]), "{code}: {line}");
        }
    }
}
