// SPDX-License-Identifier: GPL-3.0-or-later
//! The diagnostic codes of the text netlist, `PHOENIX-NETS-<NNN>`, with their titles in the eight
//! languages of the suite. The codes are stable: one never changes its meaning and is never
//! reused.

/// The netlist file or the board could not be read, or the file is larger than its limit.
pub const INPUT_UNREADABLE: &str = "PHOENIX-NETS-001";
/// A line of the netlist file is not a net line.
pub const LINE_INVALID: &str = "PHOENIX-NETS-002";
/// A net is written twice in the netlist file.
pub const NET_TWICE: &str = "PHOENIX-NETS-003";
/// A pin is not written as `REF.PAD`.
pub const PIN_INVALID: &str = "PHOENIX-NETS-004";
/// A pin is on two nets of the netlist file.
pub const PIN_ON_TWO_NETS: &str = "PHOENIX-NETS-005";
/// A limit of `config/nets.cfg` is exceeded.
pub const LIMIT_EXCEEDED: &str = "PHOENIX-NETS-006";
/// A name of the board cannot be written in the netlist: the pin is left out or the net renamed.
pub const NAME_UNWRITABLE: &str = "PHOENIX-NETS-007";
/// An option or an argument of the command is not usable.
pub const OPTION_INVALID: &str = "PHOENIX-NETS-008";
/// The output file could not be written.
pub const OUTPUT_FAILED: &str = "PHOENIX-NETS-009";
/// The settings file is not valid.
pub const SETTINGS_INVALID: &str = "PHOENIX-NETS-010";
/// A net of the netlist file is not on the board.
pub const NET_ONLY_IN_FILE: &str = "PHOENIX-NETS-011";
/// A net of the board is not in the netlist file.
pub const NET_ONLY_ON_BOARD: &str = "PHOENIX-NETS-012";
/// A pin is on another net on the board.
pub const PIN_ON_OTHER_NET: &str = "PHOENIX-NETS-013";
/// A pin of the netlist file is on no net of the board.
pub const PIN_NOT_ON_BOARD: &str = "PHOENIX-NETS-014";
/// A pin that is on a net of the board is on no net of the netlist file.
pub const PIN_NOT_IN_FILE: &str = "PHOENIX-NETS-015";
/// A net has another name on the board but the same pins.
pub const NET_RENAMED: &str = "PHOENIX-NETS-016";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        INPUT_UNREADABLE,
        [
            "Netlist input cannot be read",
            "No se puede leer la entrada de la lista de redes",
            "Die Eingabe der Netzliste kann nicht gelesen werden",
            "L'entrée de la liste de réseaux ne peut pas être lue",
            "L'input della lista di reti non può essere letto",
            "ネットリストの入力を読み込めません",
            "无法读取网表输入",
            "Não é possível ler a entrada da lista de redes",
        ],
    ),
    (
        LINE_INVALID,
        [
            "Line of the netlist is not a net line",
            "La línea de la lista de redes no es una línea de red",
            "Zeile der Netzliste ist keine Netzzeile",
            "La ligne de la liste de réseaux n'est pas une ligne de réseau",
            "La riga della lista di reti non è una riga di rete",
            "ネットリストの行がネット行ではありません",
            "网表的这一行不是网络行",
            "A linha da lista de redes não é uma linha de rede",
        ],
    ),
    (
        NET_TWICE,
        [
            "Net is written twice in the netlist",
            "La red está escrita dos veces en la lista de redes",
            "Das Netz steht zweimal in der Netzliste",
            "Le réseau est écrit deux fois dans la liste de réseaux",
            "La rete è scritta due volte nella lista di reti",
            "ネットがネットリストに2回書かれています",
            "该网络在网表中出现了两次",
            "A rede está escrita duas vezes na lista de redes",
        ],
    ),
    (
        PIN_INVALID,
        [
            "Pin is not written as part and pad",
            "El pin no está escrito como pieza y pad",
            "Der Pin ist nicht als Bauteil und Pad geschrieben",
            "La broche n'est pas écrite comme pièce et pastille",
            "Il pin non è scritto come componente e pad",
            "ピンが部品とパッドの形式で書かれていません",
            "引脚未按元件和焊盘的形式书写",
            "O pino não está escrito como peça e pastilha",
        ],
    ),
    (
        PIN_ON_TWO_NETS,
        [
            "Pin is on two nets of the netlist",
            "El pin está en dos redes de la lista de redes",
            "Der Pin liegt in zwei Netzen der Netzliste",
            "La broche est sur deux réseaux de la liste de réseaux",
            "Il pin è su due reti della lista di reti",
            "ピンがネットリストの2つのネットに属しています",
            "该引脚属于网表中的两个网络",
            "O pino está em duas redes da lista de redes",
        ],
    ),
    (
        LIMIT_EXCEEDED,
        [
            "A limit of the netlist settings is exceeded",
            "Se supera un límite de los ajustes de la lista de redes",
            "Ein Grenzwert der Netzlisten-Einstellungen wird überschritten",
            "Une limite des réglages de la liste de réseaux est dépassée",
            "Un limite delle impostazioni della lista di reti è superato",
            "ネットリスト設定の上限を超えています",
            "超过了网表设置的某个限制",
            "Um limite das definições da lista de redes é excedido",
        ],
    ),
    (
        NAME_UNWRITABLE,
        [
            "A name of the board cannot be written in the netlist",
            "Un nombre de la placa no se puede escribir en la lista de redes",
            "Ein Name der Platine kann nicht in die Netzliste geschrieben werden",
            "Un nom de la carte ne peut pas être écrit dans la liste de réseaux",
            "Un nome della scheda non può essere scritto nella lista di reti",
            "基板の名前をネットリストに書き出せません",
            "电路板上的某个名称无法写入网表",
            "Um nome da placa não pode ser escrito na lista de redes",
        ],
    ),
    (
        OPTION_INVALID,
        [
            "Option or argument of the netlist command is not usable",
            "Una opción o un argumento del comando de lista de redes no es válido",
            "Eine Option oder ein Argument des Netzlisten-Befehls ist nicht verwendbar",
            "Une option ou un argument de la commande de liste de réseaux est inutilisable",
            "Un'opzione o un argomento del comando della lista di reti non è utilizzabile",
            "ネットリストコマンドのオプションまたは引数が使えません",
            "网表命令的选项或参数不可用",
            "Uma opção ou um argumento do comando da lista de redes não é utilizável",
        ],
    ),
    (
        OUTPUT_FAILED,
        [
            "Netlist output file could not be written",
            "No se pudo escribir el fichero de salida de la lista de redes",
            "Die Ausgabedatei der Netzliste konnte nicht geschrieben werden",
            "Le fichier de sortie de la liste de réseaux n'a pas pu être écrit",
            "Il file di uscita della lista di reti non ha potuto essere scritto",
            "ネットリストの出力ファイルを書き込めませんでした",
            "无法写入网表输出文件",
            "O ficheiro de saída da lista de redes não pôde ser escrito",
        ],
    ),
    (
        SETTINGS_INVALID,
        [
            "Netlist settings are not valid",
            "Los ajustes de la lista de redes no son válidos",
            "Die Einstellungen der Netzliste sind ungültig",
            "Les réglages de la liste de réseaux ne sont pas valides",
            "Le impostazioni della lista di reti non sono valide",
            "ネットリストの設定が無効です",
            "网表设置无效",
            "As definições da lista de redes não são válidas",
        ],
    ),
    (
        NET_ONLY_IN_FILE,
        [
            "Net of the netlist is not on the board",
            "La red de la lista de redes no está en la placa",
            "Das Netz der Netzliste ist nicht auf der Platine",
            "Le réseau de la liste de réseaux n'est pas sur la carte",
            "La rete della lista di reti non è sulla scheda",
            "ネットリストのネットが基板にありません",
            "网表中的网络不在电路板上",
            "A rede da lista de redes não está na placa",
        ],
    ),
    (
        NET_ONLY_ON_BOARD,
        [
            "Net of the board is not in the netlist",
            "La red de la placa no está en la lista de redes",
            "Das Netz der Platine steht nicht in der Netzliste",
            "Le réseau de la carte n'est pas dans la liste de réseaux",
            "La rete della scheda non è nella lista di reti",
            "基板のネットがネットリストにありません",
            "电路板上的网络不在网表中",
            "A rede da placa não está na lista de redes",
        ],
    ),
    (
        PIN_ON_OTHER_NET,
        [
            "Pin is on another net on the board",
            "El pin está en otra red en la placa",
            "Der Pin liegt auf der Platine in einem anderen Netz",
            "La broche est sur un autre réseau sur la carte",
            "Il pin è su un'altra rete sulla scheda",
            "ピンは基板上で別のネットにあります",
            "该引脚在电路板上属于另一个网络",
            "O pino está noutra rede na placa",
        ],
    ),
    (
        PIN_NOT_ON_BOARD,
        [
            "Pin of the netlist is on no net of the board",
            "El pin de la lista de redes no está en ninguna red de la placa",
            "Der Pin der Netzliste liegt in keinem Netz der Platine",
            "La broche de la liste de réseaux n'est sur aucun réseau de la carte",
            "Il pin della lista di reti non è su nessuna rete della scheda",
            "ネットリストのピンは基板のどのネットにもありません",
            "网表中的引脚不在电路板的任何网络上",
            "O pino da lista de redes não está em nenhuma rede da placa",
        ],
    ),
    (
        PIN_NOT_IN_FILE,
        [
            "Pin of a net of the board is on no net of the netlist",
            "El pin de una red de la placa no está en ninguna red de la lista de redes",
            "Der Pin eines Netzes der Platine liegt in keinem Netz der Netzliste",
            "La broche d'un réseau de la carte n'est sur aucun réseau de la liste de réseaux",
            "Il pin di una rete della scheda non è su nessuna rete della lista di reti",
            "基板のネットのピンがネットリストのどのネットにもありません",
            "电路板网络上的引脚不在网表的任何网络上",
            "O pino de uma rede da placa não está em nenhuma rede da lista de redes",
        ],
    ),
    (
        NET_RENAMED,
        [
            "Net has another name on the board and the same pins",
            "La red tiene otro nombre en la placa y los mismos pines",
            "Das Netz hat auf der Platine einen anderen Namen und dieselben Pins",
            "Le réseau a un autre nom sur la carte et les mêmes broches",
            "La rete ha un altro nome sulla scheda e gli stessi pin",
            "ネットは基板上で名前が異なり、ピンは同じです",
            "该网络在电路板上名称不同但引脚相同",
            "A rede tem outro nome na placa e os mesmos pinos",
        ],
    ),
];

/// The title of a code in the language at `column` of `Lang::ALL` (0 is English); empty for a code
/// that is not in the table or a column that does not exist.
pub fn title(code: &str, column: usize) -> &'static str {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .and_then(|(_, row)| row.get(column).copied())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_numbered_in_order_and_has_a_title_in_each_language() {
        for (i, (code, row)) in TITLES.iter().enumerate() {
            assert_eq!(*code, format!("PHOENIX-NETS-{:03}", i + 1));
            for (column, text) in row.iter().enumerate() {
                assert!(!text.is_empty(), "{code} column {column}");
            }
            for text in &row[1..] {
                assert_ne!(*text, row[0], "{code}");
            }
        }
        assert_eq!(title("PHOENIX-NETS-999", 0), "");
        assert_eq!(title(INPUT_UNREADABLE, 9), "");
        assert_eq!(
            title(NET_RENAMED, 7),
            "A rede tem outro nome na placa e os mesmos pinos"
        );
    }

    #[test]
    fn the_documented_table_has_every_code_of_this_module() {
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
