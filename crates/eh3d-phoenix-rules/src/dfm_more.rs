// SPDX-License-Identifier: GPL-3.0-or-later
//! Further lines of a capability profile of a board maker, beyond the ones of [`crate::dfm`]:
//! the number of different drill sizes, the aspect ratio of the holes, vias placed in pads, the
//! size of the silkscreen, an odd number of copper layers, the proportions of the board, blind
//! and buried vias and the smallest surface-mount pad.
//!
//! Every line is off when it is missing, 0 or `false`, so a profile written before these lines
//! existed gives the same findings as it did. The findings are errors and take part in the exit
//! code of `phoenix check --dfm`; the figures that decide them are in the profile file and
//! nowhere else.

use std::collections::BTreeSet;

use eh3d_phoenix_board::{Board, Id, PadShape, SilkItem, Via};
use eh3d_phoenix_model::Point;

use crate::codes::Lang;
use crate::dfm::{sides, DfmProfile};
use crate::geometry::{pad_min_side, point_segment};
use crate::items::{placed_pads, PlacedPad};
use crate::report::{Finding, Severity};

/// More different drill sizes than the profile accepts.
pub const DRILL_SIZES: &str = "PHOENIX-DFM-007";
/// A hole whose depth is too large for its drill.
pub const HOLE_ASPECT: &str = "PHOENIX-DFM-008";
/// A via placed inside a pad.
pub const VIA_IN_PAD: &str = "PHOENIX-DFM-009";
/// A silkscreen text smaller than the profile accepts.
pub const SILK_TEXT_SMALL: &str = "PHOENIX-DFM-010";
/// A silkscreen line thinner than the profile accepts.
pub const SILK_LINE_THIN: &str = "PHOENIX-DFM-011";
/// An odd number of copper layers.
pub const ODD_LAYERS: &str = "PHOENIX-DFM-012";
/// A board longer for its width than the profile accepts.
pub const BOARD_PROPORTION: &str = "PHOENIX-DFM-013";
/// A blind or buried via.
pub const BLIND_VIA: &str = "PHOENIX-DFM-014";
/// A surface-mount pad smaller than the profile accepts.
pub const SMD_PAD_SMALL: &str = "PHOENIX-DFM-015";
/// The annular ring of a drilled pad thinner than the profile accepts.
pub const PAD_RING_THIN: &str = "PHOENIX-DFM-016";
/// A hole closer to the edge of the board than the profile accepts.
pub const HOLE_EDGE_CLOSE: &str = "PHOENIX-DFM-017";
/// A cutout of the outline narrower than the profile accepts.
pub const CUTOUT_NARROW: &str = "PHOENIX-DFM-018";
/// A hole that is not plated, smaller than the profile accepts.
pub const UNPLATED_DRILL_SMALL: &str = "PHOENIX-DFM-019";
/// Two surface-mount parts closer than the table of the profile asks.
pub const PARTS_TOO_CLOSE: &str = "PHOENIX-DFM-020";

/// Every code of this module with its English title, in order.
pub const REGISTRY: &[(&str, &str)] = &[
    (
        DRILL_SIZES,
        "More different drill sizes than the profile accepts",
    ),
    (
        HOLE_ASPECT,
        "Hole aspect ratio above the limit of the profile",
    ),
    (
        VIA_IN_PAD,
        "Via inside a pad, which the profile does not allow",
    ),
    (
        SILK_TEXT_SMALL,
        "Silkscreen text smaller than the profile accepts",
    ),
    (
        SILK_LINE_THIN,
        "Silkscreen line thinner than the profile accepts",
    ),
    (
        ODD_LAYERS,
        "Odd number of copper layers, which the profile does not build",
    ),
    (
        BOARD_PROPORTION,
        "Board too long for its width, beyond the proportion the profile accepts",
    ),
    (
        BLIND_VIA,
        "Blind or buried via, which the profile does not build",
    ),
    (
        SMD_PAD_SMALL,
        "Surface-mount pad smaller than the profile accepts",
    ),
    (
        PAD_RING_THIN,
        "Annular ring of a drilled pad thinner than the profile accepts",
    ),
    (
        HOLE_EDGE_CLOSE,
        "Hole closer to the board edge than the profile accepts",
    ),
    (CUTOUT_NARROW, "Cutout narrower than the profile accepts"),
    (
        UNPLATED_DRILL_SMALL,
        "Unplated hole smaller than the profile accepts",
    ),
    (
        PARTS_TOO_CLOSE,
        "Surface-mount parts closer than the profile asks",
    ),
];

const TITLES: &[(&str, [&str; 8])] = &[
    (
        DRILL_SIZES,
        [
            "More different drill sizes than the profile accepts",
            "Más tamaños de taladro distintos de los que acepta el perfil",
            "Mehr verschiedene Bohrdurchmesser, als das Profil zulässt",
            "Plus de diamètres de perçage différents que le profil n'en accepte",
            "Più diametri di foratura diversi di quanti ne accetti il profilo",
            "異なるドリル径の種類がプロファイルの許容を超えています",
            "不同钻孔尺寸的数量超过配置文件允许的数量",
            "Mais tamanhos de broca diferentes do que o perfil aceita",
        ],
    ),
    (
        HOLE_ASPECT,
        [
            "Hole aspect ratio above the limit of the profile",
            "Relación de aspecto de un agujero por encima del límite del perfil",
            "Aspektverhältnis einer Bohrung über dem Grenzwert des Profils",
            "Rapport d'aspect d'un perçage supérieur à la limite du profil",
            "Rapporto di forma di un foro superiore al limite del profilo",
            "穴のアスペクト比がプロファイルの上限を超えています",
            "孔的纵横比超过配置文件的限值",
            "Relação de aspeto de um furo acima do limite do perfil",
        ],
    ),
    (
        VIA_IN_PAD,
        [
            "Via inside a pad, which the profile does not allow",
            "Vía dentro de un pad, que el perfil no permite",
            "Durchkontaktierung in einem Pad, die das Profil nicht zulässt",
            "Via dans une pastille, que le profil n'autorise pas",
            "Via dentro un pad, che il profilo non consente",
            "プロファイルが許可していないパッド内のビア",
            "配置文件不允许的焊盘内过孔",
            "Via dentro de um pad, que o perfil não permite",
        ],
    ),
    (
        SILK_TEXT_SMALL,
        [
            "Silkscreen text smaller than the profile accepts",
            "Texto de serigrafía más pequeño de lo que acepta el perfil",
            "Bestückungsdrucktext kleiner, als das Profil zulässt",
            "Texte de sérigraphie plus petit que ce que le profil accepte",
            "Testo di serigrafia più piccolo di quanto accetti il profilo",
            "シルク文字がプロファイルの許容より小さい",
            "丝印文字小于配置文件允许的尺寸",
            "Texto de serigrafia mais pequeno do que o perfil aceita",
        ],
    ),
    (
        SILK_LINE_THIN,
        [
            "Silkscreen line thinner than the profile accepts",
            "Línea de serigrafía más fina de lo que acepta el perfil",
            "Bestückungsdruck-Linie dünner, als das Profil zulässt",
            "Trait de sérigraphie plus fin que ce que le profil accepte",
            "Tratto di serigrafia più sottile di quanto accetti il profilo",
            "シルク線がプロファイルの許容より細い",
            "丝印线条细于配置文件允许的宽度",
            "Traço de serigrafia mais fino do que o perfil aceita",
        ],
    ),
    (
        ODD_LAYERS,
        [
            "Odd number of copper layers, which the profile does not build",
            "Número impar de capas de cobre, que el perfil no fabrica",
            "Ungerade Anzahl von Kupferlagen, die das Profil nicht fertigt",
            "Nombre impair de couches de cuivre, que le profil ne fabrique pas",
            "Numero dispari di strati di rame, che il profilo non produce",
            "プロファイルが製造しない奇数の銅層数",
            "配置文件不制造的奇数铜层",
            "Número ímpar de camadas de cobre, que o perfil não fabrica",
        ],
    ),
    (
        BOARD_PROPORTION,
        [
            "Board too long for its width, beyond the proportion the profile accepts",
            "Placa demasiado alargada para la proporción que acepta el perfil",
            "Platine zu langgestreckt für das vom Profil zugelassene Seitenverhältnis",
            "Carte trop allongée pour la proportion que le profil accepte",
            "Scheda troppo allungata rispetto alla proporzione accettata dal profilo",
            "基板がプロファイルの許容する縦横比より細長い",
            "电路板过于细长，超出配置文件允许的比例",
            "Placa demasiado alongada para a proporção que o perfil aceita",
        ],
    ),
    (
        BLIND_VIA,
        [
            "Blind or buried via, which the profile does not build",
            "Vía ciega o enterrada, que el perfil no fabrica",
            "Sackloch- oder vergrabene Durchkontaktierung, die das Profil nicht fertigt",
            "Via borgne ou enterré, que le profil ne fabrique pas",
            "Via cieca o interrata, che il profilo non produce",
            "プロファイルが製造しないブラインドまたはベリードビア",
            "配置文件不制造的盲孔或埋孔",
            "Via cega ou enterrada, que o perfil não fabrica",
        ],
    ),
    (
        SMD_PAD_SMALL,
        [
            "Surface-mount pad smaller than the profile accepts",
            "Pad de montaje superficial más pequeño de lo que acepta el perfil",
            "SMD-Pad kleiner, als das Profil zulässt",
            "Pastille CMS plus petite que ce que le profil accepte",
            "Pad SMD più piccolo di quanto accetti il profilo",
            "表面実装パッドがプロファイルの許容より小さい",
            "表贴焊盘小于配置文件允许的尺寸",
            "Pad de montagem em superfície mais pequeno do que o perfil aceita",
        ],
    ),
    (
        PAD_RING_THIN,
        [
            "Annular ring of a drilled pad thinner than the profile accepts",
            "Anillo anular de un pad taladrado más fino de lo que acepta el perfil",
            "Restring eines gebohrten Pads dünner, als das Profil zulässt",
            "Anneau d'une pastille percée plus fin que ce que le profil accepte",
            "Anello di un pad forato più sottile di quanto accetti il profilo",
            "穴あきパッドのアニュラリングがプロファイルの許容より細い",
            "钻孔焊盘的环宽小于配置文件允许的宽度",
            "Anel de um pad furado mais fino do que o perfil aceita",
        ],
    ),
    (
        HOLE_EDGE_CLOSE,
        [
            "Hole closer to the board edge than the profile accepts",
            "Agujero más cerca del borde de la placa de lo que acepta el perfil",
            "Bohrung näher am Platinenrand, als das Profil zulässt",
            "Perçage plus proche du bord de la carte que ce que le profil accepte",
            "Foro più vicino al bordo della scheda di quanto accetti il profilo",
            "穴が基板端にプロファイルの許容より近い",
            "孔距板边小于配置文件允许的距离",
            "Furo mais perto do limite da placa do que o perfil aceita",
        ],
    ),
    (
        CUTOUT_NARROW,
        [
            "Cutout narrower than the profile accepts",
            "Recorte más estrecho de lo que acepta el perfil",
            "Ausschnitt schmaler, als das Profil zulässt",
            "Découpe plus étroite que ce que le profil accepte",
            "Ritaglio più stretto di quanto accetti il profilo",
            "切り欠きがプロファイルの許容より狭い",
            "开槽窄于配置文件允许的宽度",
            "Recorte mais estreito do que o perfil aceita",
        ],
    ),
    (
        UNPLATED_DRILL_SMALL,
        [
            "Unplated hole smaller than the profile accepts",
            "Agujero no metalizado más pequeño de lo que acepta el perfil",
            "Nicht durchkontaktiertes Loch kleiner, als das Profil zulässt",
            "Trou non métallisé plus petit que ce que le profil accepte",
            "Foro non metallizzato più piccolo di quanto accetti il profilo",
            "メッキなし穴がプロファイルの許容より小さい",
            "非金属化孔小于配置文件允许的尺寸",
            "Furo não metalizado mais pequeno do que o perfil aceita",
        ],
    ),
    (
        PARTS_TOO_CLOSE,
        [
            "Surface-mount parts closer than the profile asks",
            "Piezas de montaje superficial más juntas de lo que pide el perfil",
            "SMD-Bauteile näher beieinander, als das Profil verlangt",
            "Composants CMS plus proches que ne le demande le profil",
            "Componenti a montaggio superficiale più vicini di quanto chieda il profilo",
            "表面実装部品の間隔がプロファイルの要求より狭い",
            "表面贴装元件的间距小于配置文件的要求",
            "Peças de montagem à superfície mais juntas do que o perfil pede",
        ],
    ),
];

/// The title of a code of this module in a language; `None` for any other code.
pub fn title(code: &str, lang: Lang) -> Option<&'static str> {
    TITLES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, texts)| texts[lang.column()])
}

/// The drill of every hole of the board: the vias and the drilled pads of the placed components.
fn drills(board: &Board) -> Vec<i64> {
    let mut out: Vec<i64> = board.vias.values().map(|via| via.drill.get()).collect();
    for component in board.components.values() {
        let Some(footprint) = board.footprints.get(&component.footprint) else {
            continue;
        };
        out.extend(
            footprint
                .pads
                .values()
                .filter_map(|pad| pad.drill)
                .map(|drill| drill.get()),
        );
    }
    out
}

/// The findings of the lines of this module; `name` is the name of the profile for the words.
pub(crate) fn findings(board: &Board, profile: &DfmProfile, name: &str, out: &mut Vec<Finding>) {
    drill_sizes(board, profile, name, out);
    smd_pads(board, profile, out);
    pad_rings(board, profile, out);
    hole_edges(board, profile, out);
    cutouts(board, profile, out);
    unplated_drills(board, profile, out);
    blind_vias(board, profile, out);
    board_proportion(board, profile, out);
    even_layers(board, profile, out);
    silk_size(board, profile, out);
    via_in_pad(board, profile, out);
    hole_aspect(board, profile, out);
    crate::smd_board::findings(board, profile, out);
}

fn drill_sizes(board: &Board, profile: &DfmProfile, name: &str, out: &mut Vec<Finding>) {
    if profile.max_drill_sizes == 0 {
        return;
    }
    let sizes: BTreeSet<i64> = drills(board).into_iter().collect();
    if sizes.len() as u64 > u64::from(profile.max_drill_sizes) {
        out.push(
            Finding::new(
                46,
                "dfm_drill_sizes",
                DRILL_SIZES,
                Severity::Error,
                "holes".to_owned(),
            )
            .why(&format!(
                "{} different drill sizes, the profile {name} accepts {}",
                sizes.len(),
                profile.max_drill_sizes
            )),
        );
    }
}

/// The smallest drill that the board thickness of the profile allows at the aspect ratio limit,
/// rounded up to a whole nanometre: `thickness / drill <= max_x10 / 10`.
fn least_drill_nm(thickness_nm: i64, max_aspect_x10: u32) -> i64 {
    let thickness = u128::try_from(thickness_nm).unwrap_or(0) * 10;
    let max = u128::from(max_aspect_x10).max(1);
    i64::try_from(thickness.div_ceil(max)).unwrap_or(i64::MAX)
}

fn hole_aspect(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    if profile.board_thickness_nm <= 0 || profile.max_hole_aspect_x10 == 0 {
        return;
    }
    let Some(smallest) = drills(board).into_iter().filter(|d| *d > 0).min() else {
        return;
    };
    let needed = least_drill_nm(profile.board_thickness_nm, profile.max_hole_aspect_x10);
    if smallest < needed {
        let tenths = i128::from(profile.board_thickness_nm) * 10 / i128::from(smallest);
        out.push(
            Finding::new(
                47,
                "dfm_hole_aspect",
                HOLE_ASPECT,
                Severity::Error,
                "holes".to_owned(),
            )
            .values(smallest, needed)
            .why(&format!(
                "aspect ratio {}.{}:1 for the smallest drill, the limit is {}.{}:1",
                tenths / 10,
                tenths % 10,
                profile.max_hole_aspect_x10 / 10,
                profile.max_hole_aspect_x10 % 10
            )),
        );
    }
}

/// The copper layers a via joins: those of the stack from its first layer to its last, or just
/// the two it names when they are not both in the stack.
fn via_layers(board: &Board, via: &Via) -> Vec<Id> {
    let stack = &board.copper_stack;
    let first = stack.iter().position(|l| *l == via.from_layer);
    let last = stack.iter().position(|l| *l == via.to_layer);
    match (first, last) {
        (Some(a), Some(b)) => stack[a.min(b)..=a.max(b)].to_vec(),
        _ => vec![via.from_layer, via.to_layer],
    }
}

/// Whether a point is inside the copper of a pad, with the real shape of the pad: the circle, the
/// rectangle with its rounded corners or the stadium of an oblong pad, turned by the angle of the
/// pad on the board. At a right angle the test is exact in whole nanometres; at any other angle the
/// point is turned into the frame of the pad and rounded to the nanometre.
fn point_in_pad(pad: &PlacedPad, p: Point) -> bool {
    let dx = i128::from(p.x_nm.get()) - i128::from(pad.at.x_nm.get());
    let dy = i128::from(p.y_nm.get()) - i128::from(pad.at.y_nm.get());
    let (u, v) = match pad.angle_udeg {
        0 | 180_000_000 => (dx.abs(), dy.abs()),
        90_000_000 | 270_000_000 => (dy.abs(), dx.abs()),
        angle => {
            let rad = (angle as f64) / 1_000_000.0 * std::f64::consts::PI / 180.0;
            let (s, c) = rad.sin_cos();
            let (fx, fy) = (dx as f64, dy as f64);
            (
                ((fx * c + fy * s).round() as i128).abs(),
                ((fy * c - fx * s).round() as i128).abs(),
            )
        }
    };
    // every length is multiplied by 2000 so that the half sides and the radius of the corners are
    // whole numbers
    let (big_u, big_v) = (2000 * u, 2000 * v);
    match pad.shape {
        PadShape::Circle { diameter } => {
            let d = i128::from(diameter.get());
            4 * (u * u + v * v) <= d * d
        }
        PadShape::Rect { width, height } => {
            let (w, h) = (i128::from(width.get()), i128::from(height.get()));
            let (half_u, half_v) = (1000 * w, 1000 * h);
            let round = i128::from(pad.roundness.min(1000)) * w.min(h);
            big_u <= half_u
                && big_v <= half_v
                && (big_u <= half_u - round
                    || big_v <= half_v - round
                    || (big_u - (half_u - round)).pow(2) + (big_v - (half_v - round)).pow(2)
                        <= round * round)
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = (i128::from(width.get()), i128::from(height.get()));
            // the stadium: the distance to the middle line of the long side is at most the radius
            if w >= h {
                let along = (big_u - 1000 * (w - h)).max(0);
                along * along + big_v * big_v <= (1000 * h).pow(2)
            } else {
                let along = (big_v - 1000 * (h - w)).max(0);
                along * along + big_u * big_u <= (1000 * w).pow(2)
            }
        }
    }
}

/// A via whose centre is inside a pad of a part, with the real shape and angle of the pad, on a
/// layer both reach. Only the centre of the via counts, so a via that overlaps the edge of a pad
/// with its centre outside is not reported.
fn via_in_pad(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    if !profile.forbid_via_in_pad {
        return;
    }
    let pads = placed_pads(board);
    for via in board.vias.values() {
        let layers = via_layers(board, via);
        for pad in &pads {
            if !point_in_pad(pad, via.at) {
                continue;
            }
            if !pad.layers.iter().any(|l| layers.contains(l)) {
                continue;
            }
            out.push(
                Finding::new(
                    48,
                    "dfm_via_in_pad",
                    VIA_IN_PAD,
                    Severity::Error,
                    pad.label.clone(),
                )
                .at(via.at.x_nm.get(), via.at.y_nm.get())
                .why("a via is placed inside this pad"),
            );
        }
    }
}

/// The height of the letters and the width of the printed lines of the silkscreen against the
/// least that the profile accepts.
fn silk_size(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    let (least_height, least_width) = (profile.min_silk_text_height_nm, profile.min_silk_stroke_nm);
    if least_height <= 0 && least_width <= 0 {
        return;
    }
    for (sid, item) in &board.silkscreen {
        let label = format!("silk {sid}");
        let (height, width, at) = match item {
            SilkItem::Text {
                at, height, stroke, ..
            } => (Some(height.get()), stroke.get(), Some(*at)),
            SilkItem::Stroke { width, .. } => (None, width.get(), None),
        };
        if let Some(h) = height.filter(|h| least_height > 0 && *h < least_height) {
            let mut finding = Finding::new(
                49,
                "dfm_silk_text_small",
                SILK_TEXT_SMALL,
                Severity::Error,
                label.clone(),
            )
            .values(h, least_height);
            if let Some(p) = at {
                finding = finding.at(p.x_nm.get(), p.y_nm.get());
            }
            out.push(finding);
        }
        if least_width > 0 && width < least_width {
            let mut finding = Finding::new(
                50,
                "dfm_silk_line_thin",
                SILK_LINE_THIN,
                Severity::Error,
                label,
            )
            .values(width, least_width);
            if let Some(p) = at {
                finding = finding.at(p.x_nm.get(), p.y_nm.get());
            }
            out.push(finding);
        }
    }
}

/// A stack of three or more copper layers with an odd count, which a maker that presses cores in
/// pairs does not build. A board of one layer is not judged: it has no stack to balance.
fn even_layers(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    let layers = board.copper_stack.len();
    if !profile.require_even_layers || layers < 3 || layers & 1 == 0 {
        return;
    }
    out.push(
        Finding::new(
            51,
            "dfm_odd_layers",
            ODD_LAYERS,
            Severity::Error,
            "board".to_owned(),
        )
        .why(&format!(
            "{layers} copper layers, the profile accepts an even number"
        )),
    );
}

/// The long side of the box of the outline against the short side: a board that is much longer
/// than it is wide bends in the press and the reflow oven.
fn board_proportion(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    if profile.max_board_aspect_x10 == 0 {
        return;
    }
    let Some((short, long)) = sides(board) else {
        return;
    };
    if short <= 0 {
        return;
    }
    let max = i128::from(profile.max_board_aspect_x10);
    if i128::from(long) * 10 > i128::from(short) * max {
        let allowed = i128::from(short) * max / 10;
        let tenths = i128::from(long) * 10 / i128::from(short);
        out.push(
            Finding::new(
                52,
                "dfm_board_proportion",
                BOARD_PROPORTION,
                Severity::Error,
                "board".to_owned(),
            )
            .values(long, i64::try_from(allowed).unwrap_or(i64::MAX))
            .why(&format!(
                "the long side is {}.{} times the short side, the limit is {}.{}",
                tenths / 10,
                tenths % 10,
                max / 10,
                max % 10
            )),
        );
    }
}

/// The vias that do not join the first and the last copper layer of the stack.
fn blind_vias(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    if !profile.forbid_blind_vias {
        return;
    }
    let (Some(top), Some(bottom)) = (board.copper_stack.first(), board.copper_stack.last()) else {
        return;
    };
    if top == bottom {
        return;
    }
    for (vid, via) in &board.vias {
        let through = (via.from_layer == *top && via.to_layer == *bottom)
            || (via.from_layer == *bottom && via.to_layer == *top);
        if !through {
            out.push(
                Finding::new(
                    53,
                    "dfm_blind_via",
                    BLIND_VIA,
                    Severity::Error,
                    format!("via {vid}"),
                )
                .at(via.at.x_nm.get(), via.at.y_nm.get())
                .why("the via does not join the first and the last copper layer"),
            );
        }
    }
}

/// The smaller side of the pads without a drill against the least that the profile accepts.
fn smd_pads(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    let least = profile.min_smd_pad_nm;
    if least <= 0 {
        return;
    }
    for pad in placed_pads(board).iter().filter(|p| p.drill.is_none()) {
        let side = i64::try_from(pad_min_side(pad.shape)).unwrap_or(i64::MAX);
        if side < least {
            out.push(
                Finding::new(
                    54,
                    "dfm_smd_pad_small",
                    SMD_PAD_SMALL,
                    Severity::Error,
                    pad.label.clone(),
                )
                .at(pad.at.x_nm.get(), pad.at.y_nm.get())
                .values(side, least),
            );
        }
    }
}

/// The annular ring of the plated drilled pads against the least that the profile accepts for the
/// pads of parts (half of the smaller side less the drill, as the rule check measures it).
fn pad_rings(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    let least = profile.min_pad_ring_nm;
    if least <= 0 {
        return;
    }
    for pad in placed_pads(board).iter().filter(|p| p.plated) {
        let Some(drill) = pad.drill else {
            continue;
        };
        let ring2 = pad_min_side(pad.shape) - i128::from(drill.get());
        if ring2 < 2 * i128::from(least) {
            let ring = i64::try_from(ring2.div_euclid(2).max(0)).unwrap_or(0);
            out.push(
                Finding::new(
                    55,
                    "dfm_pad_ring_thin",
                    PAD_RING_THIN,
                    Severity::Error,
                    pad.label.clone(),
                )
                .at(pad.at.x_nm.get(), pad.at.y_nm.get())
                .values(ring, least),
            );
        }
    }
}

/// The distance from the wall of every hole to the straight pieces of the outline and of its
/// cutouts. The arcs of an outline are not looked at, and a distance that does not fit the exact
/// test is not reported.
fn hole_edges(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    let least = profile.min_hole_edge_nm;
    if least <= 0 {
        return;
    }
    let rings: Vec<&Vec<Point>> = std::iter::once(&board.outline)
        .chain(board.outline_cutouts.iter())
        .filter(|ring| ring.len() >= 3)
        .collect();
    if rings.is_empty() {
        return;
    }
    let mut holes: Vec<(String, Point, i64)> = board
        .vias
        .iter()
        .map(|(vid, via)| (format!("via {vid}"), via.at, via.drill.get()))
        .collect();
    for pad in placed_pads(board) {
        if let Some(drill) = pad.drill {
            holes.push((pad.label.clone(), pad.at, drill.get()));
        }
    }
    let doubled_least = 2 * i128::from(least);
    for (label, at, drill) in holes {
        let mut broken = false;
        let mut nearest = f64::MAX;
        for ring in &rings {
            for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                let Some(d) = point_segment(at, *a, *b) else {
                    continue;
                };
                nearest = nearest.min(d.nm_f64());
                if d.doubled_below(doubled_least + i128::from(drill)) == Some(true) {
                    broken = true;
                }
            }
        }
        if broken {
            let gap = (nearest - drill as f64 / 2.0).round().max(0.0) as i64;
            out.push(
                Finding::new(
                    56,
                    "dfm_hole_edge_close",
                    HOLE_EDGE_CLOSE,
                    Severity::Error,
                    label,
                )
                .at(at.x_nm.get(), at.y_nm.get())
                .values(gap, least),
            );
        }
    }
}

/// The smaller side of the box of each cutout of the outline against the least that the profile
/// accepts (the milling bit of the maker cannot cut a slot or a window narrower than it is). The
/// box is never narrower than the cutout, so a cutout that is reported is certainly too narrow.
fn cutouts(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    let least = profile.min_cutout_nm;
    if least <= 0 {
        return;
    }
    for (index, ring) in board.outline_cutouts.iter().enumerate() {
        if ring.len() < 3 {
            continue;
        }
        let xs = ring.iter().map(|p| p.x_nm.get());
        let ys = ring.iter().map(|p| p.y_nm.get());
        let (Some(x0), Some(x1), Some(y0), Some(y1)) =
            (xs.clone().min(), xs.max(), ys.clone().min(), ys.max())
        else {
            continue;
        };
        let narrow = (x1 - x0).min(y1 - y0);
        if narrow < least {
            out.push(
                Finding::new(
                    57,
                    "dfm_cutout_narrow",
                    CUTOUT_NARROW,
                    Severity::Error,
                    format!("cutout {}", index + 1),
                )
                .at(x0 + (x1 - x0) / 2, y0 + (y1 - y0) / 2)
                .values(narrow, least),
            );
        }
    }
}

/// The drill of the holes that are not plated (mounting holes) against the least that the profile
/// accepts for them.
fn unplated_drills(board: &Board, profile: &DfmProfile, out: &mut Vec<Finding>) {
    let least = profile.min_unplated_drill_nm;
    if least <= 0 {
        return;
    }
    for pad in placed_pads(board).iter().filter(|p| !p.plated) {
        let Some(drill) = pad.drill else {
            continue;
        };
        if drill.get() < least {
            out.push(
                Finding::new(
                    58,
                    "dfm_unplated_drill_small",
                    UNPLATED_DRILL_SMALL,
                    Severity::Error,
                    pad.label.clone(),
                )
                .at(pad.at.x_nm.get(), pad.at.y_nm.get())
                .values(drill.get(), least),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dfm::dfm_findings;

    fn codes(board: &Board, text: &str) -> Vec<&'static str> {
        let profile = DfmProfile::parse(&format!("verified = true\n{text}")).unwrap();
        dfm_findings(board, &profile)
            .confirmed
            .iter()
            .map(|f| f.code)
            .collect()
    }

    #[test]
    fn the_different_drill_sizes_are_counted_once_each() {
        // the vias drill 0.3 mm and 0.8 mm and the two pads 1.0 mm: three sizes
        let board = crate::dfm::tests::board();
        assert_eq!(drills(&board).len(), 4);
        assert_eq!(codes(&board, "max_drill_sizes = 3\n"), Vec::<&str>::new());
        assert_eq!(codes(&board, "max_drill_sizes = 2\n"), [DRILL_SIZES]);
        assert_eq!(codes(&board, "max_drill_sizes = 0\n"), Vec::<&str>::new());
        let profile = DfmProfile::parse("verified = true\nmax_drill_sizes = 1\n").unwrap();
        let report = dfm_findings(&board, &profile);
        assert!(report.confirmed[0]
            .detail
            .starts_with("3 different drill sizes"));
        assert!(!report.passes());
    }

    #[test]
    fn a_board_without_holes_has_no_drill_sizes() {
        assert_eq!(
            codes(&Board::default(), "max_drill_sizes = 1\n"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn the_value_of_the_line_must_be_a_small_whole_number() {
        for text in [
            "max_drill_sizes = -1\n",
            "max_drill_sizes = many\n",
            "max_drill_sizes = 10001\n",
        ] {
            assert!(DfmProfile::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn a_hole_deeper_than_the_aspect_limit_is_reported_with_the_drill_that_would_pass() {
        // the smallest drill is 0.3 mm and the board is 1.6 mm thick: 5.3 to 1
        let board = crate::dfm::tests::board();
        let text = "board_thickness_nm = 1600000\nmax_hole_aspect_x10 = 50\n";
        assert_eq!(codes(&board, text), [HOLE_ASPECT]);
        let profile = DfmProfile::parse(&format!("verified = true\n{text}")).unwrap();
        let finding = dfm_findings(&board, &profile).confirmed.remove(0);
        // 16 mm / 5 = 0.32 mm is the least drill that passes
        assert_eq!(
            (finding.measured_nm, finding.limit_nm),
            (Some(300_000), Some(320_000))
        );
        assert!(finding.detail.starts_with("aspect ratio 5.3:1"));
        assert!(finding.detail.ends_with("is 5.0:1"));
        // 6 to 1 accepts it, and the line is off when either figure is 0
        let wide = "board_thickness_nm = 1600000\nmax_hole_aspect_x10 = 60\n";
        assert_eq!(codes(&board, wide), Vec::<&str>::new());
        for off in [
            "board_thickness_nm = 0\nmax_hole_aspect_x10 = 10\n",
            "board_thickness_nm = 1600000\nmax_hole_aspect_x10 = 0\n",
            "board_thickness_nm = 1600000\n",
        ] {
            assert_eq!(codes(&board, off), Vec::<&str>::new(), "{off}");
        }
    }

    #[test]
    fn the_aspect_limit_is_exact_at_the_edge_and_never_overflows() {
        // exactly 0.32 mm at 5.0 to 1 passes, one nanometre less does not
        assert_eq!(least_drill_nm(1_600_000, 50), 320_000);
        assert_eq!(least_drill_nm(1_600_001, 50), 320_001);
        assert_eq!(least_drill_nm(1_000_000_000_000, 1), 10_000_000_000_000);
        assert_eq!(least_drill_nm(1, 0), 10);
        assert!(DfmProfile::parse("board_thickness_nm = -1\n").is_err());
        assert!(DfmProfile::parse("max_hole_aspect_x10 = 1001\n").is_err());
        assert!(DfmProfile::parse("max_hole_aspect_x10 = 1000\n").is_ok());
    }

    fn with_via(at: (i64, i64), layer: u32) -> Board {
        use crate::dfm::tests::{id, pt};
        let mut board = crate::dfm::tests::board();
        let mut via = board.vias.values().next().unwrap().clone();
        via.at = pt(at.0, at.1);
        via.from_layer = id(layer);
        via.to_layer = id(layer);
        board.vias.insert(id(60), via);
        board
    }

    #[test]
    fn a_via_inside_a_pad_is_reported_where_it_is() {
        // the first pad is a circle of 1.8 mm at (20 mm, 12 mm) on the layers 1 and 2
        let inside = with_via((20_400_000, 12_000_000), 2);
        let profile = DfmProfile::parse("verified = true\nforbid_via_in_pad = true\n").unwrap();
        let report = dfm_findings(&inside, &profile);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        let finding = &report.confirmed[0];
        assert_eq!(finding.code, VIA_IN_PAD);
        assert_eq!(finding.location, "J1 pad 1");
        assert_eq!(finding.at_nm, Some([20_400_000, 12_000_000]));
        assert!(!report.passes());
        // the line is off unless it is switched on
        assert_eq!(
            codes(&inside, "forbid_via_in_pad = false\n"),
            Vec::<&str>::new()
        );
        // the vias of the plain board are far from the pads
        let plain = crate::dfm::tests::board();
        assert_eq!(
            codes(&plain, "forbid_via_in_pad = true\n"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn a_via_just_outside_the_pad_or_on_another_layer_is_not_in_it() {
        // the radius of the pad is 0.9 mm: 0.9 mm away is inside (the edge counts), 0.91 is not
        assert_eq!(
            codes(
                &with_via((20_900_000, 12_000_000), 1),
                "forbid_via_in_pad = true\n"
            ),
            [VIA_IN_PAD]
        );
        assert_eq!(
            codes(
                &with_via((20_910_000, 12_000_000), 1),
                "forbid_via_in_pad = true\n"
            ),
            Vec::<&str>::new()
        );
        // a via on a layer that the pad does not reach
        assert_eq!(
            codes(
                &with_via((20_000_000, 12_000_000), 3),
                "forbid_via_in_pad = true\n"
            ),
            Vec::<&str>::new()
        );
        assert!(DfmProfile::parse("forbid_via_in_pad = maybe\n").is_err());
    }

    /// The board of the tests with one pad at (20 mm, 12 mm), of the given shape, turned and on the
    /// given side, on the layers 1 and 2.
    fn with_shaped_pad(
        shape: PadShape,
        roundness: u16,
        pad_rotation: i64,
        part_rotation: i64,
        side: eh3d_phoenix_board::Side,
    ) -> Board {
        use crate::dfm::tests::id;
        use eh3d_phoenix_model::Udeg;
        let mut board = crate::dfm::tests::board();
        let footprint = board.footprints.get_mut(&id(40)).unwrap();
        footprint.pads.remove(&id(42));
        let pad = footprint.pads.get_mut(&id(41)).unwrap();
        pad.shape = shape;
        pad.roundness = roundness;
        pad.rotation = Udeg::new(pad_rotation).unwrap();
        let part = board.components.get_mut(&id(50)).unwrap();
        part.rotation = Udeg::new(part_rotation).unwrap();
        part.side = side;
        board
    }

    /// Whether a via on the layer 1 at the given place (in micrometres from the origin) is found
    /// inside a pad.
    fn via_found(board: &Board, x_um: i64, y_um: i64) -> bool {
        use crate::dfm::tests::{id, pt};
        let mut board = board.clone();
        let mut via = board.vias.values().next().unwrap().clone();
        via.at = pt(x_um * 1000, y_um * 1000);
        via.from_layer = id(1);
        via.to_layer = id(1);
        board.vias.insert(id(60), via);
        codes(&board, "forbid_via_in_pad = true\n").contains(&VIA_IN_PAD)
    }

    fn rect(w: i64, h: i64) -> PadShape {
        use eh3d_phoenix_model::Nm;
        PadShape::Rect {
            width: Nm::new(w).unwrap(),
            height: Nm::new(h).unwrap(),
        }
    }

    fn oblong(w: i64, h: i64) -> PadShape {
        use eh3d_phoenix_model::Nm;
        PadShape::Oblong {
            width: Nm::new(w).unwrap(),
            height: Nm::new(h).unwrap(),
        }
    }

    #[test]
    fn a_via_in_the_corner_of_a_rectangular_pad_is_found_and_one_outside_it_is_not() {
        use eh3d_phoenix_board::Side;
        // a pad of 3 mm by 1 mm centred at (20 mm, 12 mm): the circle inside it has a radius of
        // 0.5 mm, but the pad reaches 1.5 mm along x and 0.5 mm along y
        let flat = with_shaped_pad(rect(3_000_000, 1_000_000), 0, 0, 0, Side::Top);
        assert!(via_found(&flat, 21_400, 12_000));
        assert!(via_found(&flat, 21_400, 12_400));
        assert!(via_found(&flat, 18_600, 11_600));
        // the edge counts, one micrometre beyond does not
        assert!(via_found(&flat, 21_500, 12_500));
        assert!(!via_found(&flat, 21_501, 12_000));
        assert!(!via_found(&flat, 20_000, 12_501));
        assert!(!via_found(&flat, 21_500, 12_501));
        // the same pad turned a quarter: it is long along y
        let upright = with_shaped_pad(rect(3_000_000, 1_000_000), 0, 90_000_000, 0, Side::Top);
        assert!(via_found(&upright, 20_000, 13_400));
        assert!(!via_found(&upright, 21_400, 12_000));
        // the turn of the part adds to the turn of the pad: 90 + 90 is flat again
        let both = with_shaped_pad(
            rect(3_000_000, 1_000_000),
            0,
            90_000_000,
            90_000_000,
            Side::Top,
        );
        assert!(via_found(&both, 21_400, 12_000));
        assert!(!via_found(&both, 20_000, 13_400));
        // on the bottom side the turn of the pad is mirrored: 90 - 90 is flat, and a part turned
        // a quarter with a pad that is not turned is upright
        let bottom = with_shaped_pad(
            rect(3_000_000, 1_000_000),
            0,
            90_000_000,
            90_000_000,
            Side::Bottom,
        );
        assert!(via_found(&bottom, 21_400, 12_000));
        let bottom_upright =
            with_shaped_pad(rect(3_000_000, 1_000_000), 0, 0, 90_000_000, Side::Bottom);
        assert!(via_found(&bottom_upright, 20_000, 13_400));
        assert!(!via_found(&bottom_upright, 21_400, 12_000));
        // a full turn and a turn of 180 degrees change nothing
        let half = with_shaped_pad(rect(3_000_000, 1_000_000), 0, 180_000_000, 0, Side::Top);
        assert!(via_found(&half, 21_400, 12_400));
        assert!(!via_found(&half, 20_000, 12_600));
    }

    #[test]
    fn a_pad_turned_by_any_angle_is_judged_in_its_own_frame() {
        use eh3d_phoenix_board::Side;
        // 3 mm by 1 mm turned 45 degrees (by the pad, then by the part): the long axis points
        // up and to the right, so (20.98995, 12.98995) is 1.4 mm along it
        for (pad, part) in [(45_000_000, 0), (0, 45_000_000)] {
            let board = with_shaped_pad(rect(3_000_000, 1_000_000), 0, pad, part, Side::Top);
            assert!(via_found(&board, 20_990, 12_990), "{pad} {part}");
            assert!(via_found(&board, 19_010, 11_010), "{pad} {part}");
            // the same distance along the other diagonal is across the pad, 1.4 mm from its axis
            assert!(!via_found(&board, 20_990, 11_010), "{pad} {part}");
            // 0.8 mm to the right of the centre is 0.57 mm across the axis: outside
            assert!(!via_found(&board, 20_800, 12_000), "{pad} {part}");
        }
    }

    #[test]
    fn an_oblong_pad_and_a_rounded_corner_do_not_reach_the_corner_of_their_box() {
        use eh3d_phoenix_board::Side;
        // an oblong of 3 mm by 1 mm is a bar with a half circle at each end (radius 0.5 mm, centres
        // at 1.0 mm from the middle)
        let bar = with_shaped_pad(oblong(3_000_000, 1_000_000), 0, 0, 0, Side::Top);
        assert!(via_found(&bar, 21_400, 12_000));
        assert!(via_found(&bar, 21_300, 12_300));
        assert!(via_found(&bar, 20_500, 12_500));
        assert!(!via_found(&bar, 21_400, 12_400));
        assert!(!via_found(&bar, 21_501, 12_000));
        // one that is taller than it is wide
        let tall = with_shaped_pad(oblong(1_000_000, 3_000_000), 0, 0, 0, Side::Top);
        assert!(via_found(&tall, 20_000, 13_400));
        assert!(!via_found(&tall, 20_400, 13_400));
        assert!(!via_found(&tall, 20_501, 12_000));
        // a rectangle with square corners reaches the corner of its box, a fully rounded one does
        // not, and one half rounded (radius 0.25 mm) reaches it only up to its curve
        let square = with_shaped_pad(rect(3_000_000, 1_000_000), 0, 0, 0, Side::Top);
        assert!(via_found(&square, 21_450, 12_450));
        let round = with_shaped_pad(rect(3_000_000, 1_000_000), 1000, 0, 0, Side::Top);
        assert!(!via_found(&round, 21_400, 12_400));
        assert!(via_found(&round, 21_300, 12_300));
        let half = with_shaped_pad(rect(3_000_000, 1_000_000), 500, 0, 0, Side::Top);
        assert!(via_found(&half, 21_400, 12_400));
        assert!(!via_found(&half, 21_450, 12_450));
        assert!(via_found(&half, 21_500, 12_250));
    }

    fn with_silkscreen() -> Board {
        use crate::dfm::tests::{id, pt};
        use eh3d_phoenix_model::{Nm, Udeg};
        let mut board = crate::dfm::tests::board();
        board.silkscreen.insert(
            id(70),
            SilkItem::Text {
                layer: id(71),
                at: pt(3_000_000, 4_000_000),
                text: "R1".into(),
                height: Nm::new(800_000).unwrap(),
                stroke: Nm::new(100_000).unwrap(),
                rotation: Udeg::new(0).unwrap(),
                mirrored: false,
            },
        );
        board.silkscreen.insert(
            id(72),
            SilkItem::Stroke {
                layer: id(71),
                width: Nm::new(80_000).unwrap(),
                path: Vec::new(),
            },
        );
        board
    }

    #[test]
    fn small_letters_and_thin_lines_of_the_silkscreen_are_reported_with_their_figures() {
        let board = with_silkscreen();
        let profile = DfmProfile::parse(
            "verified = true\nmin_silk_text_height_nm = 1000000\nmin_silk_stroke_nm = 150000\n",
        )
        .unwrap();
        let report = dfm_findings(&board, &profile);
        let mut found: Vec<_> = report
            .confirmed
            .iter()
            .map(|f| (f.code, f.measured_nm, f.limit_nm))
            .collect();
        found.sort();
        // the text is 0.8 mm high (limit 1.0 mm) and 0.1 mm wide (limit 0.15 mm); the stroke is
        // 0.08 mm wide
        assert_eq!(
            found,
            [
                (SILK_TEXT_SMALL, Some(800_000), Some(1_000_000)),
                (SILK_LINE_THIN, Some(80_000), Some(150_000)),
                (SILK_LINE_THIN, Some(100_000), Some(150_000)),
            ]
        );
        let text = report.confirmed.iter().find(|f| f.code == SILK_TEXT_SMALL);
        assert_eq!(text.unwrap().at_nm, Some([3_000_000, 4_000_000]));
        let strokes = report.confirmed.iter().filter(|f| f.at_nm.is_none());
        assert_eq!(strokes.count(), 1);
        assert!(!report.passes());
    }

    #[test]
    fn silkscreen_that_meets_the_limits_or_lines_that_are_off_give_nothing() {
        let board = with_silkscreen();
        for text in [
            "min_silk_text_height_nm = 800000\nmin_silk_stroke_nm = 80000\n",
            "min_silk_text_height_nm = 0\nmin_silk_stroke_nm = 0\n",
            "",
        ] {
            assert_eq!(codes(&board, text), Vec::<&str>::new(), "{text}");
        }
        // only the height is limited: the stroke is not judged
        assert_eq!(
            codes(&board, "min_silk_text_height_nm = 900000\n"),
            [SILK_TEXT_SMALL]
        );
        // a board without silkscreen has nothing to judge
        assert_eq!(
            codes(&Board::default(), "min_silk_text_height_nm = 900000\n"),
            Vec::<&str>::new()
        );
    }

    fn with_layers(count: u32) -> Board {
        use crate::dfm::tests::id;
        let mut board = crate::dfm::tests::board();
        board.copper_stack = (1..=count).map(id).collect();
        board
    }

    #[test]
    fn an_odd_stack_of_three_or_more_layers_is_refused_when_the_line_is_on() {
        let on = "require_even_layers = true\n";
        for (count, expected) in [
            (1, Vec::<&str>::new()),
            (2, Vec::new()),
            (3, vec![ODD_LAYERS]),
            (4, Vec::new()),
            (5, vec![ODD_LAYERS]),
        ] {
            assert_eq!(codes(&with_layers(count), on), expected, "{count} layers");
        }
        // off by default and when it says false
        assert_eq!(codes(&with_layers(3), ""), Vec::<&str>::new());
        assert_eq!(
            codes(&with_layers(3), "require_even_layers = false\n"),
            Vec::<&str>::new()
        );
        let profile = DfmProfile::parse(&format!("verified = true\n{on}")).unwrap();
        let report = dfm_findings(&with_layers(3), &profile);
        assert!(report.confirmed[0].detail.starts_with("3 copper layers"));
        assert!(!report.passes());
        assert!(DfmProfile::parse("require_even_layers = 1\n").is_err());
    }

    #[test]
    fn a_board_longer_than_the_proportion_is_reported_with_the_longest_side_allowed() {
        // 40 mm by 25 mm is 1.6 to 1
        let board = crate::dfm::tests::board();
        let tight = "max_board_aspect_x10 = 15\n";
        assert_eq!(codes(&board, tight), [BOARD_PROPORTION]);
        let profile = DfmProfile::parse(&format!("verified = true\n{tight}")).unwrap();
        let finding = dfm_findings(&board, &profile).confirmed.remove(0);
        // 25 mm times 1.5 is 37.5 mm
        assert_eq!(
            (finding.measured_nm, finding.limit_nm),
            (Some(40_000_000), Some(37_500_000))
        );
        assert!(finding.detail.starts_with("the long side is 1.6 times"));
        assert!(finding.detail.ends_with("the limit is 1.5"));
        // exactly 1.6 passes; 0 is off
        assert_eq!(
            codes(&board, "max_board_aspect_x10 = 16\n"),
            Vec::<&str>::new()
        );
        assert_eq!(
            codes(&board, "max_board_aspect_x10 = 0\n"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn a_board_without_an_outline_or_with_no_width_is_not_judged_for_its_proportion() {
        assert_eq!(
            codes(&Board::default(), "max_board_aspect_x10 = 10\n"),
            Vec::<&str>::new()
        );
        let mut flat = crate::dfm::tests::board();
        flat.outline = vec![
            crate::dfm::tests::pt(0, 0),
            crate::dfm::tests::pt(40_000_000, 0),
        ];
        assert_eq!(
            codes(&flat, "max_board_aspect_x10 = 10\n"),
            Vec::<&str>::new()
        );
        assert!(DfmProfile::parse("max_board_aspect_x10 = 10001\n").is_err());
    }

    #[test]
    fn only_the_vias_that_do_not_cross_the_whole_stack_are_blind_or_buried() {
        use crate::dfm::tests::id;
        // four layers: the via 30 goes from 1 to 4, the via 31 from 1 to 2
        let mut board = with_layers(4);
        board.vias.get_mut(&id(30)).unwrap().to_layer = id(4);
        let on = "forbid_blind_vias = true\n";
        let profile = DfmProfile::parse(&format!("verified = true\n{on}")).unwrap();
        let report = dfm_findings(&board, &profile);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        assert_eq!(report.confirmed[0].code, BLIND_VIA);
        assert!(report.confirmed[0].location.starts_with("via "));
        assert_eq!(report.confirmed[0].at_nm, Some([9_000_000, 5_000_000]));
        // a via written from the last layer to the first is a through via too
        let via = board.vias.get_mut(&id(31)).unwrap();
        via.from_layer = id(4);
        via.to_layer = id(1);
        assert_eq!(codes(&board, on), Vec::<&str>::new());
        // off by default
        assert_eq!(codes(&with_layers(4), ""), Vec::<&str>::new());
        // the two-layer board of the tests has only through vias
        assert_eq!(codes(&with_layers(2), on), Vec::<&str>::new());
    }

    #[test]
    fn a_board_without_a_stack_has_no_blind_vias() {
        let mut board = with_layers(4);
        board.copper_stack.clear();
        assert_eq!(
            codes(&board, "forbid_blind_vias = true\n"),
            Vec::<&str>::new()
        );
        board.copper_stack = vec![crate::dfm::tests::id(1)];
        assert_eq!(
            codes(&board, "forbid_blind_vias = true\n"),
            Vec::<&str>::new()
        );
        assert!(DfmProfile::parse("forbid_blind_vias = yes\n").is_err());
    }

    fn with_smd_pads() -> Board {
        use crate::dfm::tests::{id, pt};
        use eh3d_phoenix_board::{Pad, PadShape};
        use eh3d_phoenix_model::{Nm, Udeg};
        let mut board = crate::dfm::tests::board();
        let footprint = board.footprints.get_mut(&id(40)).unwrap();
        for (n, width, height, number) in [(43, 150_000, 500_000, "3"), (44, 600_000, 900_000, "4")]
        {
            footprint.pads.insert(
                id(n),
                Pad {
                    number: number.into(),
                    shape: PadShape::Rect {
                        width: Nm::new(width).unwrap(),
                        height: Nm::new(height).unwrap(),
                    },
                    at: pt(0, 3_000_000),
                    rotation: Udeg::new(0).unwrap(),
                    drill: None,
                    layers: vec![id(1)],
                    roundness: 0,
                    plated: false,
                },
            );
        }
        board
    }

    #[test]
    fn a_surface_mount_pad_below_the_least_side_is_reported_with_its_size() {
        let board = with_smd_pads();
        let profile = DfmProfile::parse("verified = true\nmin_smd_pad_nm = 200000\n").unwrap();
        let report = dfm_findings(&board, &profile);
        // only the pad of 0.15 mm by 0.5 mm; the drilled pads are not surface-mount pads
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        let finding = &report.confirmed[0];
        assert_eq!(finding.code, SMD_PAD_SMALL);
        assert_eq!(finding.location, "J1 pad 3");
        assert_eq!(
            (finding.measured_nm, finding.limit_nm),
            (Some(150_000), Some(200_000))
        );
        assert_eq!(finding.at_nm, Some([20_000_000, 15_000_000]));
        // a limit equal to the side passes; 0 is off
        assert_eq!(
            codes(&board, "min_smd_pad_nm = 150000\n"),
            Vec::<&str>::new()
        );
        assert_eq!(codes(&board, "min_smd_pad_nm = 0\n"), Vec::<&str>::new());
        assert_eq!(codes(&board, "min_smd_pad_nm = 700000\n").len(), 2);
    }

    #[test]
    fn a_board_without_surface_mount_pads_gives_nothing_for_that_line() {
        let board = crate::dfm::tests::board();
        assert_eq!(
            codes(&board, "min_smd_pad_nm = 5000000\n"),
            Vec::<&str>::new()
        );
        assert!(DfmProfile::parse("min_smd_pad_nm = -1\n").is_err());
    }

    #[test]
    fn the_ring_of_a_plated_drilled_pad_is_judged_by_its_own_limit() {
        use crate::dfm::tests::id;
        // the pads are circles of 1.8 mm with a drill of 1.0 mm: a ring of 0.4 mm
        let mut board = crate::dfm::tests::board();
        assert_eq!(
            codes(&board, "min_pad_ring_nm = 400000\n"),
            Vec::<&str>::new()
        );
        assert_eq!(codes(&board, "min_pad_ring_nm = 0\n"), Vec::<&str>::new());
        assert_eq!(
            codes(&board, "min_pad_ring_nm = 400100\n"),
            [PAD_RING_THIN, PAD_RING_THIN]
        );
        let profile = DfmProfile::parse("verified = true\nmin_pad_ring_nm = 500000\n").unwrap();
        let report = dfm_findings(&board, &profile);
        for finding in &report.confirmed {
            assert_eq!(finding.location, "J1 pad 1");
            assert_eq!(
                (finding.measured_nm, finding.limit_nm),
                (Some(400_000), Some(500_000))
            );
        }
        let mut places: Vec<_> = report.confirmed.iter().map(|f| f.at_nm).collect();
        places.sort();
        assert_eq!(
            places,
            [
                Some([20_000_000, 12_000_000]),
                Some([22_540_000, 12_000_000])
            ]
        );
        // a hole that is not plated has no ring, and a pad turned into a smaller one is measured
        // by its smaller side
        let footprint = board.footprints.get_mut(&id(40)).unwrap();
        footprint.pads.get_mut(&id(42)).unwrap().plated = false;
        footprint.pads.get_mut(&id(41)).unwrap().shape = oblong(3_000_000, 1_400_000);
        let found = codes(&board, "min_pad_ring_nm = 250000\n");
        assert_eq!(found, [PAD_RING_THIN]);
        let report = dfm_findings(
            &board,
            &DfmProfile::parse("verified = true\nmin_pad_ring_nm = 250000\n").unwrap(),
        );
        // (1.4 mm - 1.0 mm) / 2 = 0.2 mm
        assert_eq!(report.confirmed[0].measured_nm, Some(200_000));
    }

    #[test]
    fn a_hole_too_near_the_edge_of_the_board_or_of_a_cutout_is_reported_with_its_gap() {
        use crate::dfm::tests::pt;
        let mut board = crate::dfm::tests::board();
        // the board is 40 mm by 25 mm; the vias at 5 mm from two edges have drills of 0.3 mm and
        // 0.8 mm, so the walls are 4.85 mm and 4.6 mm from the edge
        assert_eq!(
            codes(&board, "min_hole_edge_nm = 4600000\n"),
            Vec::<&str>::new()
        );
        assert_eq!(codes(&board, "min_hole_edge_nm = 0\n"), Vec::<&str>::new());
        let profile = DfmProfile::parse("verified = true\nmin_hole_edge_nm = 4600001\n").unwrap();
        let report = dfm_findings(&board, &profile);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        let finding = &report.confirmed[0];
        assert_eq!(finding.code, HOLE_EDGE_CLOSE);
        assert!(finding.location.starts_with("via "), "{}", finding.location);
        assert_eq!(
            (finding.measured_nm, finding.limit_nm),
            (Some(4_600_000), Some(4_600_001))
        );
        assert_eq!(finding.at_nm, Some([9_000_000, 5_000_000]));
        // the pads are 12 mm from the nearest edge, so their walls are 11.5 mm away: with 11.6 mm
        // all four holes are too near
        assert_eq!(codes(&board, "min_hole_edge_nm = 11600000\n").len(), 4);
        // a cutout is an edge too: a square from (19 mm, 11 mm) to (21 mm, 13 mm) is 1 mm from the
        // centre of the first pad, so the wall of its drill is 0.5 mm away
        board.outline_cutouts.push(vec![
            pt(19_000_000, 11_000_000),
            pt(21_000_000, 11_000_000),
            pt(21_000_000, 13_000_000),
            pt(19_000_000, 13_000_000),
        ]);
        let profile = DfmProfile::parse("verified = true\nmin_hole_edge_nm = 600000\n").unwrap();
        let report = dfm_findings(&board, &profile);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        assert_eq!(report.confirmed[0].location, "J1 pad 1");
        assert_eq!(report.confirmed[0].measured_nm, Some(500_000));
    }

    #[test]
    fn the_new_lines_are_lengths_and_a_board_without_an_outline_is_not_judged_by_the_edge() {
        for text in [
            "min_pad_ring_nm = -1\n",
            "min_pad_ring_nm = wide\n",
            "min_hole_edge_nm = -1\n",
            "min_hole_edge_nm = 1.5\n",
        ] {
            assert!(DfmProfile::parse(text).is_err(), "{text}");
        }
        let mut board = crate::dfm::tests::board();
        board.outline.clear();
        assert_eq!(
            codes(&board, "min_hole_edge_nm = 90000000\n"),
            Vec::<&str>::new()
        );
        // the profile of JLCPCB asks 0.15 mm of the ring of the pads of parts
        let jlc = DfmProfile::parse(crate::dfm::builtin_profile("jlcpcb").unwrap()).unwrap();
        assert_eq!((jlc.min_pad_ring_nm, jlc.min_hole_edge_nm), (150_000, 0));
    }

    #[test]
    fn a_cutout_narrower_than_the_bit_is_reported_with_its_width_and_its_number() {
        use crate::dfm::tests::pt;
        let mut board = crate::dfm::tests::board();
        let rectangle = |x0: i64, y0: i64, x1: i64, y1: i64| {
            vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)]
        };
        // a slot of 0.6 mm by 3 mm, a window of 5 mm by 5 mm and a ring of two points
        board
            .outline_cutouts
            .push(rectangle(2_000_000, 2_000_000, 2_600_000, 5_000_000));
        board
            .outline_cutouts
            .push(rectangle(10_000_000, 10_000_000, 15_000_000, 15_000_000));
        board
            .outline_cutouts
            .push(vec![pt(1_000_000, 1_000_000), pt(1_100_000, 1_100_000)]);
        assert_eq!(
            codes(&board, "min_cutout_nm = 600000\n"),
            Vec::<&str>::new()
        );
        assert_eq!(codes(&board, "min_cutout_nm = 0\n"), Vec::<&str>::new());
        let profile = DfmProfile::parse("verified = true\nmin_cutout_nm = 600001\n").unwrap();
        let report = dfm_findings(&board, &profile);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        let finding = &report.confirmed[0];
        assert_eq!(finding.code, CUTOUT_NARROW);
        assert_eq!(finding.location, "cutout 1");
        assert_eq!(
            (finding.measured_nm, finding.limit_nm),
            (Some(600_000), Some(600_001))
        );
        assert_eq!(finding.at_nm, Some([2_300_000, 3_500_000]));
        // a limit above the window reports both, the ring of two points never
        let both = codes(&board, "min_cutout_nm = 5000001\n");
        assert_eq!(both, [CUTOUT_NARROW, CUTOUT_NARROW]);
        assert!(DfmProfile::parse("min_cutout_nm = -1\n").is_err());
    }

    #[test]
    fn a_hole_that_is_not_plated_is_judged_by_its_own_least_drill() {
        use crate::dfm::tests::id;
        let mut board = crate::dfm::tests::board();
        // both pads drill 1.0 mm and are plated: nothing to judge
        assert_eq!(
            codes(&board, "min_unplated_drill_nm = 2000000\n"),
            Vec::<&str>::new()
        );
        let footprint = board.footprints.get_mut(&id(40)).unwrap();
        footprint.pads.get_mut(&id(42)).unwrap().plated = false;
        assert_eq!(
            codes(&board, "min_unplated_drill_nm = 1000000\n"),
            Vec::<&str>::new()
        );
        assert_eq!(
            codes(&board, "min_unplated_drill_nm = 0\n"),
            Vec::<&str>::new()
        );
        let profile =
            DfmProfile::parse("verified = true\nmin_unplated_drill_nm = 1000100\n").unwrap();
        let report = dfm_findings(&board, &profile);
        assert_eq!(report.confirmed.len(), 1, "{:?}", report.confirmed);
        let finding = &report.confirmed[0];
        assert_eq!(finding.code, UNPLATED_DRILL_SMALL);
        assert_eq!(finding.at_nm, Some([22_540_000, 12_000_000]));
        assert_eq!(
            (finding.measured_nm, finding.limit_nm),
            (Some(1_000_000), Some(1_000_100))
        );
        assert!(DfmProfile::parse("min_unplated_drill_nm = big\n").is_err());
    }

    #[test]
    fn the_example_profile_names_every_further_line_and_leaves_it_off() {
        let text = include_str!("../../../config/dfm/example.cfg");
        for key in [
            "max_drill_sizes",
            "board_thickness_nm",
            "max_hole_aspect_x10",
            "forbid_via_in_pad",
            "min_silk_text_height_nm",
            "min_silk_stroke_nm",
            "require_even_layers",
            "max_board_aspect_x10",
            "forbid_blind_vias",
            "min_smd_pad_nm",
            "min_pad_ring_nm",
            "min_hole_edge_nm",
            "min_cutout_nm",
            "min_unplated_drill_nm",
        ] {
            assert!(text.contains(&format!("\n{key} = ")), "{key}");
        }
        let profile = DfmProfile::parse(text).unwrap();
        assert_eq!(profile.max_drill_sizes, 0);
        assert_eq!(profile.board_thickness_nm, 0);
        assert_eq!(profile.max_hole_aspect_x10, 0);
        assert!(!profile.forbid_via_in_pad && !profile.forbid_blind_vias);
        assert!(!profile.require_even_layers);
        assert_eq!(profile.min_silk_text_height_nm, 0);
        assert_eq!(profile.min_silk_stroke_nm, 0);
        assert_eq!(profile.max_board_aspect_x10, 0);
        assert_eq!(profile.min_smd_pad_nm, 0);
        assert_eq!((profile.min_pad_ring_nm, profile.min_hole_edge_nm), (0, 0));
        assert_eq!(
            (profile.min_cutout_nm, profile.min_unplated_drill_nm),
            (0, 0)
        );
        // every code of this module is a finding of the check and none is the information
        for (code, _) in REGISTRY {
            assert!(code.starts_with("PHOENIX-DFM-"), "{code}");
        }
    }

    #[test]
    fn every_code_has_a_title_in_all_eight_languages_and_a_row_in_the_table() {
        assert_eq!(REGISTRY.len(), TITLES.len());
        let doc = include_str!("../../../docs/ERROR_CODES.md");
        for (code, english) in REGISTRY {
            for lang in Lang::ALL {
                assert!(!title(code, lang).unwrap_or_default().is_empty(), "{code}");
            }
            assert_eq!(title(code, Lang::En), Some(*english));
            assert_ne!(title(code, Lang::Es), Some(*english), "{code}");
            assert_ne!(title(code, Lang::Pt), Some(*english), "{code}");
            assert_eq!(crate::codes::title(code, Lang::De), title(code, Lang::De));
            let row = doc
                .lines()
                .find(|l| l.starts_with(&format!("| {code} |")))
                .unwrap_or_else(|| panic!("{code} is not documented"));
            assert!(row.contains(english), "{code}: {row}");
        }
    }
}
