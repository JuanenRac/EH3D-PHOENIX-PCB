// SPDX-License-Identifier: GPL-3.0-or-later
//! The texts of the panel window in the eight languages of the suite: English, Spanish, German,
//! French, Italian, Japanese, Chinese and European Portuguese.

use eh3d_phoenix_rules::Lang;

use crate::codes::column;

/// A text of the panel window and of its button on the start screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelText {
    /// The title of the window.
    Title,
    /// The button of the start screen that lays the typed board out as a panel.
    Open,
    /// The line of the start screen that says what the button does.
    Hint,
    /// The button that closes the window.
    Close,
    /// The label of the box with the board files.
    BoardFiles,
    /// The label of the box with the settings file.
    SettingsFile,
    /// The button that lays the panel out again.
    LayOut,
    /// The label of the number of columns.
    Columns,
    /// The label of the number of rows.
    Rows,
    /// The box that separates the boards with V-score lines instead of mouse bites.
    VScore,
    /// The word for the size of the panel.
    Size,
    /// The word for the boards.
    Boards,
    /// The label of the part of the area that the boards take.
    Use,
    /// The word for the tabs.
    Tabs,
    /// The title of the notes of the layout.
    Notes,
    /// The name of the layer of the frame.
    LayerFrame,
    /// The name of the layer of the routed outline.
    LayerOutline,
    /// The name of the layer of the V-score lines.
    LayerVScore,
    /// The name of the layer of the mouse bite holes.
    LayerMouseBites,
    /// The name of the layer of the fiducials.
    LayerFiducials,
    /// The name of the layer of the tooling holes.
    LayerTooling,
    /// The name of the layer of the holes of the boards.
    LayerHoles,
    /// The line shown while no panel is laid out.
    Nothing,
    /// The note that the window only draws the panel.
    ReadOnly,
    /// The word for a board that is turned.
    Turned,
    /// The word for a board that is mirrored.
    Mirrored,
    /// The word for one board.
    Board,
}

impl PanelText {
    /// Every text, for the tests that check the table.
    pub const ALL: [PanelText; 27] = [
        PanelText::Title,
        PanelText::Open,
        PanelText::Hint,
        PanelText::Close,
        PanelText::BoardFiles,
        PanelText::SettingsFile,
        PanelText::LayOut,
        PanelText::Columns,
        PanelText::Rows,
        PanelText::VScore,
        PanelText::Size,
        PanelText::Boards,
        PanelText::Use,
        PanelText::Tabs,
        PanelText::Notes,
        PanelText::LayerFrame,
        PanelText::LayerOutline,
        PanelText::LayerVScore,
        PanelText::LayerMouseBites,
        PanelText::LayerFiducials,
        PanelText::LayerTooling,
        PanelText::LayerHoles,
        PanelText::Nothing,
        PanelText::ReadOnly,
        PanelText::Turned,
        PanelText::Mirrored,
        PanelText::Board,
    ];

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.row()[column(lang)]
    }

    fn row(self) -> [&'static str; 8] {
        match self {
            PanelText::Title => [
                "Panel of boards",
                "Panel de placas",
                "Nutzen aus Platinen",
                "Panneau de cartes",
                "Pannello di schede",
                "基板のパネル",
                "电路板拼板",
                "Painel de placas",
            ],
            PanelText::Open => [
                "Panel", "Panel", "Nutzen", "Panneau", "Pannello", "パネル", "拼板", "Painel",
            ],
            PanelText::Hint => [
                "The board file typed above is laid out as a panel of boards.",
                "El archivo de placa escrito arriba se compone como panel de placas.",
                "Die oben eingegebene Platinendatei wird als Nutzen angelegt.",
                "Le fichier de carte saisi ci-dessus est composé en panneau de cartes.",
                "Il file della scheda digitato sopra viene composto come pannello di schede.",
                "上に入力した基板ファイルをパネルとして構成します。",
                "将上面输入的电路板文件生成拼板。",
                "O ficheiro de placa escrito acima é composto como painel de placas.",
            ],
            PanelText::Close => [
                "Close", "Cerrar", "Schließen", "Fermer", "Chiudi", "閉じる", "关闭", "Fechar",
            ],
            PanelText::BoardFiles => [
                "Board files (separated by commas)",
                "Archivos de placa (separados por comas)",
                "Platinendateien (durch Kommas getrennt)",
                "Fichiers de cartes (séparés par des virgules)",
                "File delle schede (separati da virgole)",
                "基板ファイル（カンマ区切り）",
                "电路板文件（以逗号分隔）",
                "Ficheiros de placa (separados por vírgulas)",
            ],
            PanelText::SettingsFile => [
                "Settings file (config/panel.cfg)",
                "Archivo de ajustes (config/panel.cfg)",
                "Einstellungsdatei (config/panel.cfg)",
                "Fichier de réglages (config/panel.cfg)",
                "File delle impostazioni (config/panel.cfg)",
                "設定ファイル (config/panel.cfg)",
                "设置文件 (config/panel.cfg)",
                "Ficheiro de definições (config/panel.cfg)",
            ],
            PanelText::LayOut => [
                "Lay out",
                "Componer",
                "Anlegen",
                "Composer",
                "Componi",
                "構成",
                "生成",
                "Compor",
            ],
            PanelText::Columns => [
                "Columns",
                "Columnas",
                "Spalten",
                "Colonnes",
                "Colonne",
                "列",
                "列数",
                "Colunas",
            ],
            PanelText::Rows => [
                "Rows", "Filas", "Zeilen", "Lignes", "Righe", "行", "行数", "Linhas",
            ],
            PanelText::VScore => [
                "V-score instead of mouse bites",
                "V-score en lugar de pestañas con agujeros",
                "V-Ritzen statt Sollbruchstellen",
                "Rainure en V au lieu de ponts perforés",
                "V-score invece di linguette forate",
                "ミシン目の代わりにVカット",
                "用V形槽代替邮票孔",
                "V-score em vez de patilhas com furos",
            ],
            PanelText::Size => [
                "Size",
                "Tamaño",
                "Größe",
                "Taille",
                "Dimensioni",
                "サイズ",
                "尺寸",
                "Tamanho",
            ],
            PanelText::Boards => [
                "Boards",
                "Placas",
                "Platinen",
                "Cartes",
                "Schede",
                "基板",
                "电路板",
                "Placas",
            ],
            PanelText::Use => [
                "Area used by the boards",
                "Área que ocupan las placas",
                "Von den Platinen belegte Fläche",
                "Surface occupée par les cartes",
                "Area occupata dalle schede",
                "基板が占める面積",
                "电路板占用面积",
                "Área ocupada pelas placas",
            ],
            PanelText::Tabs => [
                "Tabs",
                "Pestañas",
                "Stege",
                "Ponts",
                "Linguette",
                "タブ",
                "连接桥",
                "Patilhas",
            ],
            PanelText::Notes => [
                "Notes", "Notas", "Hinweise", "Remarques", "Note", "注記", "备注", "Notas",
            ],
            PanelText::LayerFrame => [
                "Frame",
                "Marco",
                "Rahmen",
                "Cadre",
                "Cornice",
                "フレーム",
                "边框",
                "Moldura",
            ],
            PanelText::LayerOutline => [
                "Routed outline",
                "Contorno fresado",
                "Gefräste Kontur",
                "Contour fraisé",
                "Contorno fresato",
                "ルーター外形",
                "铣削外形",
                "Contorno fresado",
            ],
            PanelText::LayerVScore => [
                "V-score lines",
                "Líneas de V-score",
                "V-Ritzlinien",
                "Lignes de rainure en V",
                "Linee di V-score",
                "Vカット線",
                "V形槽线",
                "Linhas de V-score",
            ],
            PanelText::LayerMouseBites => [
                "Mouse bite holes",
                "Agujeros de rotura",
                "Sollbruchbohrungen",
                "Trous de rupture",
                "Fori di rottura",
                "ミシン目の穴",
                "邮票孔",
                "Furos de rutura",
            ],
            PanelText::LayerFiducials => [
                "Fiducials",
                "Fiduciales",
                "Passmarken",
                "Repères",
                "Fiducial",
                "フィデューシャル",
                "基准点",
                "Fiduciais",
            ],
            PanelText::LayerTooling => [
                "Tooling holes",
                "Taladros de herramienta",
                "Werkzeuglöcher",
                "Trous d'outillage",
                "Fori di riferimento",
                "ツーリング穴",
                "工艺孔",
                "Furos de ferramenta",
            ],
            PanelText::LayerHoles => [
                "Board holes",
                "Taladros de las placas",
                "Bohrungen der Platinen",
                "Perçages des cartes",
                "Fori delle schede",
                "基板の穴",
                "电路板孔",
                "Furos das placas",
            ],
            PanelText::Nothing => [
                "Type the board files and press Lay out.",
                "Escriba los archivos de placa y pulse Componer.",
                "Platinendateien eingeben und Anlegen drücken.",
                "Saisissez les fichiers de cartes puis appuyez sur Composer.",
                "Digita i file delle schede e premi Componi.",
                "基板ファイルを入力して「構成」を押してください。",
                "请输入电路板文件，然后点击“生成”。",
                "Escreva os ficheiros de placa e prima Compor.",
            ],
            PanelText::ReadOnly => [
                "The panel is only drawn here; the command phoenix panel writes the files.",
                "El panel solo se dibuja aquí; el comando phoenix panel escribe los archivos.",
                "Der Nutzen wird hier nur gezeichnet; der Befehl phoenix panel schreibt die Dateien.",
                "Le panneau est seulement dessiné ici ; la commande phoenix panel écrit les fichiers.",
                "Il pannello è solo disegnato qui; il comando phoenix panel scrive i file.",
                "ここではパネルを描画するだけです。ファイルは phoenix panel コマンドで書き出します。",
                "此处仅绘制拼板；phoenix panel 命令负责写出文件。",
                "O painel só é desenhado aqui; o comando phoenix panel escreve os ficheiros.",
            ],
            PanelText::Turned => [
                "turned", "girada", "gedreht", "tournée", "ruotata", "回転", "旋转", "rodada",
            ],
            PanelText::Mirrored => [
                "mirrored",
                "reflejada",
                "gespiegelt",
                "en miroir",
                "specchiata",
                "ミラー",
                "镜像",
                "espelhada",
            ],
            PanelText::Board => [
                "Board", "Placa", "Platine", "Carte", "Scheda", "基板", "电路板", "Placa",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for text in PanelText::ALL {
            for lang in Lang::ALL {
                assert!(!text.tr(lang).is_empty(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn the_japanese_and_chinese_texts_are_in_their_scripts() {
        for text in PanelText::ALL {
            for lang in [Lang::Ja, Lang::Zh] {
                assert!(!text.tr(lang).is_ascii(), "{text:?} {lang:?}");
            }
        }
    }

    #[test]
    fn portuguese_is_the_portuguese_of_portugal() {
        assert!(PanelText::BoardFiles.tr(Lang::Pt).contains("Ficheiros"));
        assert!(PanelText::SettingsFile.tr(Lang::Pt).contains("definições"));
        for text in PanelText::ALL {
            let pt = text.tr(Lang::Pt);
            for brazilian in ["arquivo", "Arquivo", "tela", "salvar", "Configurações"] {
                assert!(!pt.contains(brazilian), "{text:?}: {pt}");
            }
        }
    }

    #[test]
    fn the_table_lists_each_text_once() {
        for (i, a) in PanelText::ALL.iter().enumerate() {
            for b in &PanelText::ALL[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn the_texts_that_name_things_differ_from_the_english_ones_in_the_other_languages() {
        for text in [PanelText::ReadOnly, PanelText::Nothing, PanelText::Hint] {
            for lang in Lang::ALL.into_iter().skip(1) {
                assert_ne!(text.tr(lang), text.tr(Lang::En), "{text:?} {lang:?}");
            }
        }
    }
}
