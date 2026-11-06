// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-PAIR-<NNN>` of the differential pairs, with their titles in the eight
//! languages of the suite. The errors of the editor keep their own codes (`PHOENIX-BEDIT-<NNN>`);
//! these are the codes of the checks of a pair and of the pair operations that cannot be done.

use eh3d_phoenix_rules::Lang;

/// A file of pairs or the settings of the pairs that is not valid.
pub const PAIRS_INVALID: &str = "PHOENIX-PAIR-001";
/// A net of a pair that the board does not have.
pub const NET_MISSING: &str = "PHOENIX-PAIR-002";
/// A gap between the two tracks of a pair that is out of tolerance.
pub const GAP_OUT: &str = "PHOENIX-PAIR-003";
/// A pair whose two nets are coupled on different lengths.
pub const ASYMMETRIC: &str = "PHOENIX-PAIR-004";
/// A pair that runs uncoupled for longer than allowed.
pub const UNCOUPLED_LONG: &str = "PHOENIX-PAIR-005";
/// Two nets of a pair whose lengths differ by more than allowed.
pub const LENGTH_MISMATCH: &str = "PHOENIX-PAIR-006";
/// Vias of a pair that have no partner on the other net.
pub const VIAS_UNPAIRED: &str = "PHOENIX-PAIR-007";
/// Tracks of a pair that are not of the width asked for.
pub const WIDTH_WRONG: &str = "PHOENIX-PAIR-008";
/// A differential impedance that is off the target.
pub const IMPEDANCE_OFF: &str = "PHOENIX-PAIR-009";
/// A pair with no stretch that runs coupled.
pub const NOT_COUPLED: &str = "PHOENIX-PAIR-010";
/// An operation on a pair that cannot be done.
pub const PAIR_OPERATION_REFUSED: &str = "PHOENIX-PAIR-011";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        PAIRS_INVALID,
        [
            "Pair definitions are not valid",
            "Las definiciones de pares no son válidas",
            "Die Paardefinitionen sind ungültig",
            "Les définitions de paires ne sont pas valides",
            "Le definizioni delle coppie non sono valide",
            "ペアの定義が無効です",
            "差分对定义无效",
            "As definições de pares não são válidas",
        ],
    ),
    (
        NET_MISSING,
        [
            "Net of a pair is not on the board",
            "Una red del par no está en la placa",
            "Ein Netz des Paares ist nicht auf der Platine",
            "Un réseau de la paire n'est pas sur la carte",
            "Una rete della coppia non è sulla scheda",
            "ペアのネットが基板にありません",
            "差分对的网络不在电路板上",
            "Uma rede do par não está na placa",
        ],
    ),
    (
        GAP_OUT,
        [
            "Separation of the pair is out of tolerance",
            "La separación del par está fuera de tolerancia",
            "Der Abstand des Paares liegt außerhalb der Toleranz",
            "L'écart de la paire est hors tolérance",
            "La distanza della coppia è fuori tolleranza",
            "ペアの間隔が許容差を外れています",
            "差分对的间距超出公差",
            "A separação do par está fora da tolerância",
        ],
    ),
    (
        ASYMMETRIC,
        [
            "Pair is not symmetric",
            "El par no es simétrico",
            "Das Paar ist nicht symmetrisch",
            "La paire n'est pas symétrique",
            "La coppia non è simmetrica",
            "ペアが対称ではありません",
            "差分对不对称",
            "O par não é simétrico",
        ],
    ),
    (
        UNCOUPLED_LONG,
        [
            "Pair runs uncoupled for too long",
            "El par va sin acoplar demasiado tramo",
            "Das Paar läuft zu lange ungekoppelt",
            "La paire court trop longtemps sans couplage",
            "La coppia corre troppo a lungo senza accoppiamento",
            "ペアが結合していない区間が長すぎます",
            "差分对未耦合的长度过长",
            "O par segue desacoplado durante demasiado tempo",
        ],
    ),
    (
        LENGTH_MISMATCH,
        [
            "Lengths of the pair differ too much",
            "Las longitudes del par difieren demasiado",
            "Die Längen des Paares weichen zu stark ab",
            "Les longueurs de la paire diffèrent trop",
            "Le lunghezze della coppia differiscono troppo",
            "ペアの長さの差が大きすぎます",
            "差分对的长度相差过大",
            "Os comprimentos do par diferem demasiado",
        ],
    ),
    (
        VIAS_UNPAIRED,
        [
            "Vias of the pair are not paired",
            "Las vías del par no están emparejadas",
            "Die Vias des Paares sind nicht gepaart",
            "Les vias de la paire ne sont pas appariés",
            "I via della coppia non sono appaiati",
            "ペアのビアが対になっていません",
            "差分对的过孔未成对",
            "As vias do par não estão emparelhadas",
        ],
    ),
    (
        WIDTH_WRONG,
        [
            "Tracks of the pair have the wrong width",
            "Las pistas del par no tienen el ancho pedido",
            "Die Leiterbahnen des Paares haben nicht die verlangte Breite",
            "Les pistes de la paire n'ont pas la largeur demandée",
            "Le piste della coppia non hanno la larghezza richiesta",
            "ペアの配線幅が指定と異なります",
            "差分对的走线宽度不符",
            "As pistas do par não têm a largura pedida",
        ],
    ),
    (
        IMPEDANCE_OFF,
        [
            "Differential impedance is off the target",
            "La impedancia diferencial se aparta del objetivo",
            "Die differenzielle Impedanz weicht vom Ziel ab",
            "L'impédance différentielle s'écarte de la cible",
            "L'impedenza differenziale si discosta dall'obiettivo",
            "差動インピーダンスが目標から外れています",
            "差分阻抗偏离目标",
            "A impedância diferencial afasta-se do objetivo",
        ],
    ),
    (
        NOT_COUPLED,
        [
            "Pair has no coupled stretch",
            "El par no tiene ningún tramo acoplado",
            "Das Paar hat keinen gekoppelten Abschnitt",
            "La paire n'a aucun tronçon couplé",
            "La coppia non ha alcun tratto accoppiato",
            "ペアに結合した区間がありません",
            "差分对没有耦合段",
            "O par não tem nenhum troço acoplado",
        ],
    ),
    (
        PAIR_OPERATION_REFUSED,
        [
            "Operation on a pair cannot be done",
            "No se puede hacer la operación sobre el par",
            "Die Operation am Paar kann nicht ausgeführt werden",
            "L'opération sur la paire ne peut pas être faite",
            "L'operazione sulla coppia non può essere eseguita",
            "ペアに対する操作を実行できません",
            "无法对差分对执行该操作",
            "Não é possível fazer a operação sobre o par",
        ],
    ),
];

/// The codes of this module with their titles in English.
pub fn registry() -> Vec<(&'static str, &'static str)> {
    TITLES.iter().map(|(code, row)| (*code, row[0])).collect()
}

/// The title of a code in a language; empty for a code that is not in the table.
pub fn title(code: &str, lang: Lang) -> &'static str {
    let column = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("", |(_, row)| row[column])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_table_has_every_code_of_the_pairs() {
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, title) in registry() {
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(title), "{code}: {row}");
        }
    }

    #[test]
    fn every_code_has_a_title_in_every_language() {
        assert_eq!(TITLES.len(), 11);
        for (code, row) in TITLES {
            assert!(code.starts_with("PHOENIX-PAIR-"));
            assert_eq!(row.len(), 8);
            for lang in Lang::ALL {
                assert!(!title(code, lang).is_empty(), "{code}");
            }
        }
        assert_eq!(title("PHOENIX-PAIR-999", Lang::En), "");
    }

    #[test]
    fn the_portuguese_titles_are_those_of_portugal() {
        assert!(title(NOT_COUPLED, Lang::Pt).contains("troço"));
        assert!(title(VIAS_UNPAIRED, Lang::Pt).contains("emparelhadas"));
    }
}
