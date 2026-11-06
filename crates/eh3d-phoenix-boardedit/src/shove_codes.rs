// SPDX-License-Identifier: GPL-3.0-or-later
//! The stable codes `PHOENIX-SHOVE-<NNN>` of the push of neighbour tracks and `PHOENIX-ARC-<NNN>`
//! of the arcs in tracks, with their titles in the eight languages of the suite. The errors of the
//! editor keep their own codes (`PHOENIX-BEDIT-<NNN>`); these are the codes of the operations
//! `route_shove`, `route_arc` and `round_corners` that cannot be done.

use eh3d_phoenix_rules::Lang;

/// A push that fixed copper (a pad, a via or the edge of the board) stops.
pub const SHOVE_BLOCKED: &str = "PHOENIX-SHOVE-001";
/// A push that would go deeper than the setting allows.
pub const SHOVE_TOO_DEEP: &str = "PHOENIX-SHOVE-002";
/// A push that would move more tracks than the setting allows.
pub const SHOVE_TOO_MANY: &str = "PHOENIX-SHOVE-003";
/// A track in the way that cannot be pushed.
pub const SHOVE_NOT_PUSHABLE: &str = "PHOENIX-SHOVE-004";
/// A route that crosses a track of another net.
pub const SHOVE_CROSSING: &str = "PHOENIX-SHOVE-005";
/// A push that would leave a rule error.
pub const SHOVE_ADDS_ERROR: &str = "PHOENIX-SHOVE-006";
/// A radius of an arc that is not valid.
pub const ARC_RADIUS: &str = "PHOENIX-ARC-001";
/// A corner that cannot be rounded.
pub const ARC_NO_CORNER: &str = "PHOENIX-ARC-002";
/// An arc that is too close to other copper or to the edge of the board.
pub const ARC_CLEARANCE: &str = "PHOENIX-ARC-003";

/// Code, then the title in English, Spanish, German, French, Italian, Japanese, Chinese and
/// European Portuguese.
pub const TITLES: &[(&str, [&str; 8])] = &[
    (
        SHOVE_BLOCKED,
        [
            "Push is blocked by fixed copper",
            "El empuje está bloqueado por cobre fijo",
            "Das Verdrängen wird von festem Kupfer blockiert",
            "Le poussage est bloqué par du cuivre fixe",
            "La spinta è bloccata da rame fisso",
            "固定された銅箔に阻まれて押しのけできません",
            "被固定铜箔阻挡，无法推挤",
            "O empurrar está bloqueado por cobre fixo",
        ],
    ),
    (
        SHOVE_TOO_DEEP,
        [
            "Push goes deeper than allowed",
            "El empuje llega más hondo de lo permitido",
            "Das Verdrängen reicht tiefer als erlaubt",
            "Le poussage va plus profond que permis",
            "La spinta va più in profondità del consentito",
            "押しのけが許容の深さを超えます",
            "推挤的层数超过允许值",
            "O empurrar vai mais fundo do que o permitido",
        ],
    ),
    (
        SHOVE_TOO_MANY,
        [
            "Push would move too many tracks",
            "El empuje movería demasiadas pistas",
            "Das Verdrängen würde zu viele Leiterbahnen bewegen",
            "Le poussage déplacerait trop de pistes",
            "La spinta sposterebbe troppe piste",
            "押しのけで移動する配線が多すぎます",
            "推挤将移动过多走线",
            "O empurrar moveria demasiadas pistas",
        ],
    ),
    (
        SHOVE_NOT_PUSHABLE,
        [
            "Track in the way cannot be pushed",
            "La pista que estorba no se puede empujar",
            "Die im Weg liegende Leiterbahn kann nicht verdrängt werden",
            "La piste qui gêne ne peut pas être poussée",
            "La pista d'intralcio non può essere spostata",
            "邪魔になる配線は押しのけできません",
            "挡路的走线无法推挤",
            "A pista que está no caminho não pode ser empurrada",
        ],
    ),
    (
        SHOVE_CROSSING,
        [
            "Route crosses a track of another net",
            "La ruta cruza una pista de otra red",
            "Die Leitung kreuzt eine Leiterbahn eines anderen Netzes",
            "Le tracé croise une piste d'un autre réseau",
            "Il percorso incrocia una pista di un'altra rete",
            "経路が別ネットの配線と交差します",
            "走线与其他网络的走线交叉",
            "O trajeto cruza uma pista de outra rede",
        ],
    ),
    (
        SHOVE_ADDS_ERROR,
        [
            "Push would leave a rule error",
            "El empuje dejaría un error de reglas",
            "Das Verdrängen würde einen Regelfehler hinterlassen",
            "Le poussage laisserait une erreur de règles",
            "La spinta lascerebbe un errore di regole",
            "押しのけで規則エラーが残ります",
            "推挤会留下规则错误",
            "O empurrar deixaria um erro de regras",
        ],
    ),
    (
        ARC_RADIUS,
        [
            "Radius of the arc is not valid",
            "El radio del arco no es válido",
            "Der Radius des Bogens ist ungültig",
            "Le rayon de l'arc n'est pas valide",
            "Il raggio dell'arco non è valido",
            "円弧の半径が無効です",
            "圆弧半径无效",
            "O raio do arco não é válido",
        ],
    ),
    (
        ARC_NO_CORNER,
        [
            "Corner cannot be rounded",
            "La esquina no se puede redondear",
            "Die Ecke kann nicht abgerundet werden",
            "Le coin ne peut pas être arrondi",
            "L'angolo non può essere arrotondato",
            "角を丸められません",
            "无法将拐角圆角化",
            "O canto não pode ser arredondado",
        ],
    ),
    (
        ARC_CLEARANCE,
        [
            "Arc is too close to other copper or to the edge",
            "El arco queda demasiado cerca de otro cobre o del borde",
            "Der Bogen liegt zu nah an anderem Kupfer oder am Rand",
            "L'arc est trop près d'un autre cuivre ou du bord",
            "L'arco è troppo vicino ad altro rame o al bordo",
            "円弧が他の銅箔や基板端に近すぎます",
            "圆弧距其他铜箔或板边太近",
            "O arco fica demasiado perto de outro cobre ou da margem",
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
    fn the_documented_table_has_every_code_of_the_push_and_the_arcs() {
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
        assert_eq!(TITLES.len(), 9);
        for (code, row) in TITLES {
            assert!(code.starts_with("PHOENIX-SHOVE-") || code.starts_with("PHOENIX-ARC-"));
            assert_eq!(row.len(), 8);
            for lang in Lang::ALL {
                assert!(!title(code, lang).is_empty(), "{code}");
            }
        }
        assert_eq!(title("PHOENIX-SHOVE-999", Lang::En), "");
    }

    #[test]
    fn the_portuguese_titles_are_those_of_portugal() {
        assert!(title(SHOVE_NOT_PUSHABLE, Lang::Pt).contains("pista"));
        assert!(title(ARC_CLEARANCE, Lang::Pt).contains("margem"));
        for (code, _) in TITLES {
            assert!(!title(code, Lang::Pt).contains("trilha"), "{code}");
        }
    }
}
