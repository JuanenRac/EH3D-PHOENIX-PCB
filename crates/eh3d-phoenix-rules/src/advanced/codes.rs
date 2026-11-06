// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-DRC-<NNN>` of the advanced checks, with their titles in the eight
//! languages of the suite. The English title is the one in [`REGISTRY`], which
//! `docs/ERROR_CODES.md` repeats; the codes themselves are never translated.

use crate::codes::Lang;

/// An annular ring (via pad or drilled pad against its drill) thinner than annular_ring_min_nm.
pub const RING: &str = "PHOENIX-DRC-001";
/// A track or a via closer to the outline than copper_edge_min_nm.
pub const COPPER_EDGE: &str = "PHOENIX-DRC-002";
/// A pad closer to the outline than pad_edge_min_nm.
pub const PAD_EDGE: &str = "PHOENIX-DRC-003";
/// A via or a drilled pad whose drill is below the minimum of the class of its net (or the default).
pub const DRILL_CLASS: &str = "PHOENIX-DRC-004";
/// Silkscreen closer to the drill of a through hole than silk_hole_min_nm.
pub const SILK_HOLE: &str = "PHOENIX-DRC-005";
/// A track narrower than neck_min_width_nm whose end joins wider copper of the same net.
pub const NECK: &str = "PHOENIX-DRC-006";
/// Two pieces of tracks of one net that meet at an angle below acid_trap_min_angle_deg.
pub const ACID_TRAP: &str = "PHOENIX-DRC-007";
/// Copper of two nets with a voltage between them closer than the table of IPC-2221 asks (figures not verified against the standard).
pub const ISOLATION: &str = "PHOENIX-DRC-008";
/// Tracks, vias or zones of a net that are joined together but to no pad.
pub const ISLAND: &str = "PHOENIX-DRC-009";
/// A track with one end that touches nothing of its net while the other end is joined.
pub const STUB: &str = "PHOENIX-DRC-010";
/// Two vias that share a layer and are at most via_duplicate_max_distance_nm apart.
pub const VIA_DUP: &str = "PHOENIX-DRC-011";
/// A pad of a placed component that is assigned to no net.
pub const PAD_NO_NET: &str = "PHOENIX-DRC-012";
/// A component with a pad whose centre is outside the outline of the board.
pub const OUTSIDE: &str = "PHOENIX-DRC-013";
/// An item the advanced checks cannot judge: a shape too coarse to tell, numbers too large, a board without an outline (information, never a failure).
pub const NOT_JUDGED: &str = "PHOENIX-DRC-014";
/// A settings file that is too large, has a key it does not know or written twice, or a value out of range.
pub const CONFIG_INVALID: &str = "PHOENIX-DRC-015";

/// Every code with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (RING, "Annular ring thinner than the advanced minimum"),
    (COPPER_EDGE, "Copper too close to the board edge"),
    (PAD_EDGE, "Pad too close to the board edge"),
    (
        DRILL_CLASS,
        "Drill smaller than the minimum of its net class",
    ),
    (SILK_HOLE, "Silkscreen too close to a drill hole"),
    (NECK, "Neck: narrow track joined to wider copper"),
    (
        ACID_TRAP,
        "Acid trap: acute angle between tracks of one net",
    ),
    (
        ISOLATION,
        "Spacing below the minimum for the voltage between nets",
    ),
    (ISLAND, "Copper island joined to no pad"),
    (STUB, "Dead-end track (stub)"),
    (VIA_DUP, "Duplicate or stacked vias"),
    (PAD_NO_NET, "Pad on no net"),
    (OUTSIDE, "Part outside the board outline"),
    (NOT_JUDGED, "Advanced check could not be judged"),
    (CONFIG_INVALID, "Advanced check settings are not valid"),
];

const TITLES: &[(&str, [&str; 8])] = &[
    (
        RING,
        [
            "Annular ring thinner than the advanced minimum",
            "Anillo anular más fino que el mínimo avanzado",
            "Restring dünner als das erweiterte Minimum",
            "Anneau de cuivre plus fin que le minimum avancé",
            "Anello anulare più sottile del minimo avanzato",
            "アニュラリングが詳細チェックの最小値より薄い",
            "环形焊环比高级检查的最小值更薄",
            "Anel anular mais fino do que o mínimo avançado",
        ],
    ),
    (
        COPPER_EDGE,
        [
            "Copper too close to the board edge",
            "Cobre demasiado cerca del borde de la placa",
            "Kupfer zu nah am Platinenrand",
            "Cuivre trop près du bord de la carte",
            "Rame troppo vicino al bordo della scheda",
            "銅箔が基板の端に近すぎる",
            "铜箔离板边太近",
            "Cobre demasiado perto do limite da placa",
        ],
    ),
    (
        PAD_EDGE,
        [
            "Pad too close to the board edge",
            "Pad demasiado cerca del borde de la placa",
            "Pad zu nah am Platinenrand",
            "Pastille trop près du bord de la carte",
            "Pad troppo vicino al bordo della scheda",
            "パッドが基板の端に近すぎる",
            "焊盘离板边太近",
            "Pad demasiado perto do limite da placa",
        ],
    ),
    (
        DRILL_CLASS,
        [
            "Drill smaller than the minimum of its net class",
            "Taladro menor que el mínimo de su clase de red",
            "Bohrung kleiner als das Minimum ihrer Netzklasse",
            "Perçage plus petit que le minimum de sa classe de réseau",
            "Foro più piccolo del minimo della sua classe di rete",
            "ドリル径がネットクラスの最小値より小さい",
            "钻孔小于其网络类别的最小值",
            "Furo mais pequeno do que o mínimo da sua classe de rede",
        ],
    ),
    (
        SILK_HOLE,
        [
            "Silkscreen too close to a drill hole",
            "Serigrafía demasiado cerca de un taladro",
            "Bestückungsdruck zu nah an einer Bohrung",
            "Sérigraphie trop près d'un perçage",
            "Serigrafia troppo vicina a un foro",
            "シルクが穴に近すぎる",
            "丝印离钻孔太近",
            "Serigrafia demasiado perto de um furo",
        ],
    ),
    (
        NECK,
        [
            "Neck: narrow track joined to wider copper",
            "Cuello: pista estrecha unida a cobre más ancho",
            "Engstelle: schmale Leiterbahn an breiterem Kupfer",
            "Goulot : piste étroite reliée à du cuivre plus large",
            "Strozzatura: pista stretta unita a rame più largo",
            "ネック：細い配線が太い銅箔につながっている",
            "瓶颈：窄走线连接到更宽的铜箔",
            "Estrangulamento: pista estreita ligada a cobre mais largo",
        ],
    ),
    (
        ACID_TRAP,
        [
            "Acid trap: acute angle between tracks of one net",
            "Trampa de ácido: ángulo agudo entre pistas de una red",
            "Säurefalle: spitzer Winkel zwischen Leiterbahnen eines Netzes",
            "Piège à acide : angle aigu entre pistes d'un réseau",
            "Trappola d'acido: angolo acuto tra piste di una rete",
            "アシッドトラップ：同一ネットの配線間の鋭角",
            "酸液陷阱：同一网络走线间的锐角",
            "Armadilha de ácido: ângulo agudo entre pistas de uma rede",
        ],
    ),
    (
        ISOLATION,
        [
            "Spacing below the minimum for the voltage between nets",
            "Separación inferior al mínimo para la tensión entre redes",
            "Abstand unter dem Minimum für die Spannung zwischen Netzen",
            "Distance inférieure au minimum pour la tension entre réseaux",
            "Distanza inferiore al minimo per la tensione tra reti",
            "ネット間の電圧に対する間隔が最小値未満",
            "间距小于网络间电压要求的最小值",
            "Distância inferior ao mínimo para a tensão entre redes",
        ],
    ),
    (
        ISLAND,
        [
            "Copper island joined to no pad",
            "Isla de cobre sin ningún pad",
            "Kupferinsel ohne Pad",
            "Îlot de cuivre sans aucune pastille",
            "Isola di rame senza alcun pad",
            "パッドにつながらない銅箔の島",
            "未连接任何焊盘的铜岛",
            "Ilha de cobre sem nenhum pad",
        ],
    ),
    (
        STUB,
        [
            "Dead-end track (stub)",
            "Pista muerta (sin pad ni vía en un extremo)",
            "Blinde Leiterbahn (Stummel)",
            "Piste borgne (tronçon sans issue)",
            "Pista cieca (moncone)",
            "行き止まりの配線（スタブ）",
            "断头走线（残桩）",
            "Pista sem saída (cotoco)",
        ],
    ),
    (
        VIA_DUP,
        [
            "Duplicate or stacked vias",
            "Vías duplicadas o apiladas",
            "Doppelte oder gestapelte Durchkontaktierungen",
            "Vias dupliqués ou empilés",
            "Via duplicate o impilate",
            "重複または積み重なったビア",
            "重复或堆叠的过孔",
            "Vias duplicadas ou empilhadas",
        ],
    ),
    (
        PAD_NO_NET,
        [
            "Pad on no net",
            "Pad sin ninguna red",
            "Pad ohne Netz",
            "Pastille sans réseau",
            "Pad senza alcuna rete",
            "どのネットにも属さないパッド",
            "不属于任何网络的焊盘",
            "Pad sem nenhuma rede",
        ],
    ),
    (
        OUTSIDE,
        [
            "Part outside the board outline",
            "Pieza fuera del contorno de la placa",
            "Bauteil außerhalb des Platinenumrisses",
            "Pièce hors du contour de la carte",
            "Componente fuori dal contorno della scheda",
            "部品が基板外形の外にある",
            "元件超出板框",
            "Peça fora do contorno da placa",
        ],
    ),
    (
        NOT_JUDGED,
        [
            "Advanced check could not be judged",
            "Comprobación avanzada sin poder juzgar",
            "Erweiterte Prüfung nicht entscheidbar",
            "Contrôle avancé non concluant",
            "Controllo avanzato non conclusivo",
            "詳細チェックを判定できない",
            "高级检查无法判定",
            "Verificação avançada sem conclusão",
        ],
    ),
    (
        CONFIG_INVALID,
        [
            "Advanced check settings are not valid",
            "Ajustes de las comprobaciones avanzadas no válidos",
            "Einstellungen der erweiterten Prüfungen ungültig",
            "Réglages des contrôles avancés non valides",
            "Impostazioni dei controlli avanzati non valide",
            "詳細チェックの設定が正しくない",
            "高级检查的设置无效",
            "Definições das verificações avançadas inválidas",
        ],
    ),
];

/// The title of a code in a language; `None` for a code that is not one of these.
pub fn title(code: &str, lang: Lang) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[lang.column()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_title_in_all_eight_languages() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        for (code, english) in REGISTRY {
            for lang in Lang::ALL {
                let text = title(code, lang).unwrap_or_default();
                assert!(!text.is_empty(), "{code} has no title in {}", lang.tag());
            }
            assert_eq!(title(code, Lang::En), Some(*english), "{code}");
            assert_ne!(title(code, Lang::Es), Some(*english), "{code}");
            assert_ne!(title(code, Lang::Pt), Some(*english), "{code}");
            assert_ne!(title(code, Lang::Ja), Some(*english), "{code}");
        }
    }

    #[test]
    fn the_documented_table_has_every_code() {
        let doc = include_str!("../../../../docs/ERROR_CODES.md");
        for (code, title) in REGISTRY {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }
}
