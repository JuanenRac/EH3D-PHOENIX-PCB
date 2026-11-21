// SPDX-License-Identifier: GPL-3.0-or-later
//! The named integer values of the language that programs use for the shapes of pads, the units
//! of the grid, the numbers of the standard layers, the directions and the lengths of pins. They
//! are the numbers that the published description of the language gives.

/// The names and their values.
pub const TABLE: &[(&str, i64)] = &[
    ("PAD_SHAPE_SQUARE", 0),
    ("PAD_SHAPE_ROUND", 1),
    ("PAD_SHAPE_OCTAGON", 2),
    ("PAD_SHAPE_LONG", 3),
    ("PAD_SHAPE_OFFSET", 4),
    ("GRID_UNIT_MIC", 0),
    ("GRID_UNIT_MM", 1),
    ("GRID_UNIT_MIL", 2),
    ("GRID_UNIT_INCH", 3),
    ("GRID_STYLE_LINES", 0),
    ("GRID_STYLE_DOTS", 1),
    ("PIN_DIRECTION_NC", 0),
    ("PIN_DIRECTION_IN", 1),
    ("PIN_DIRECTION_OUT", 2),
    ("PIN_DIRECTION_IO", 3),
    ("PIN_DIRECTION_OC", 4),
    ("PIN_DIRECTION_PWR", 5),
    ("PIN_DIRECTION_PAS", 6),
    ("PIN_DIRECTION_HIZ", 7),
    ("PIN_DIRECTION_SUP", 8),
    ("PIN_LENGTH_POINT", 0),
    ("PIN_LENGTH_SHORT", 1),
    ("PIN_LENGTH_MIDDLE", 2),
    ("PIN_LENGTH_LONG", 3),
    ("VIA_SHAPE_SQUARE", 0),
    ("VIA_SHAPE_ROUND", 1),
    ("VIA_SHAPE_OCTAGON", 2),
    ("FONT_VECTOR", 0),
    ("FONT_PROPORTIONAL", 1),
    ("FONT_FIXED", 2),
    ("WIRE_STYLE_CONTINUOUS", 0),
    ("WIRE_STYLE_LONGDASH", 1),
    ("WIRE_STYLE_SHORTDASH", 2),
    ("WIRE_STYLE_DASHDOT", 3),
    ("CAP_FLAT", 0),
    ("CAP_ROUND", 1),
    ("POLYGON_POUR_SOLID", 0),
    ("POLYGON_POUR_HATCH", 1),
    ("POLYGON_POUR_CUTOUT", 2),
    ("PIN_FUNCTION_FLAG_NONE", 0),
    ("PIN_FUNCTION_FLAG_DOT", 1),
    ("PIN_FUNCTION_FLAG_CLK", 2),
    ("PIN_VISIBLE_FLAG_OFF", 0),
    ("PIN_VISIBLE_FLAG_PAD", 1),
    ("PIN_VISIBLE_FLAG_PIN", 2),
    ("LAYER_TOP", 1),
    ("LAYER_BOTTOM", 16),
    ("LAYER_PADS", 17),
    ("LAYER_VIAS", 18),
    ("LAYER_UNROUTED", 19),
    ("LAYER_DIMENSION", 20),
    ("LAYER_TPLACE", 21),
    ("LAYER_BPLACE", 22),
    ("LAYER_TORIGINS", 23),
    ("LAYER_BORIGINS", 24),
    ("LAYER_TNAMES", 25),
    ("LAYER_BNAMES", 26),
    ("LAYER_TVALUES", 27),
    ("LAYER_BVALUES", 28),
    ("LAYER_TSTOP", 29),
    ("LAYER_BSTOP", 30),
    ("LAYER_TCREAM", 31),
    ("LAYER_BCREAM", 32),
    ("LAYER_TFINISH", 33),
    ("LAYER_BFINISH", 34),
    ("LAYER_TGLUE", 35),
    ("LAYER_BGLUE", 36),
    ("LAYER_TTEST", 37),
    ("LAYER_BTEST", 38),
    ("LAYER_TKEEPOUT", 39),
    ("LAYER_BKEEPOUT", 40),
    ("LAYER_TRESTRICT", 41),
    ("LAYER_BRESTRICT", 42),
    ("LAYER_VRESTRICT", 43),
    ("LAYER_DRILLS", 44),
    ("LAYER_HOLES", 45),
    ("LAYER_MILLING", 46),
    ("LAYER_MEASURES", 47),
    ("LAYER_DOCUMENT", 48),
    ("LAYER_REFERENCE", 49),
    ("LAYER_TDOCU", 51),
    ("LAYER_BDOCU", 52),
    ("LAYER_NETS", 91),
    ("LAYER_BUSSES", 92),
    ("LAYER_PINS", 93),
    ("LAYER_SYMBOLS", 94),
    ("LAYER_NAMES", 95),
    ("LAYER_VALUES", 96),
    ("LAYER_INFO", 97),
    ("LAYER_GUIDE", 98),
];

/// The value of a named integer, when the language has it.
pub fn lookup(name: &str) -> Option<i64> {
    TABLE.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_unique_and_the_values_are_the_published_ones() {
        let mut names: Vec<&str> = TABLE.iter().map(|(n, _)| *n).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TABLE.len());
        assert_eq!(lookup("PAD_SHAPE_ROUND"), Some(1));
        assert_eq!(lookup("GRID_UNIT_INCH"), Some(3));
        assert_eq!(lookup("LAYER_BOTTOM"), Some(16));
        assert_eq!(lookup("PIN_DIRECTION_SUP"), Some(8));
        assert_eq!(lookup("POLYGON_POUR_CUTOUT"), Some(2));
        assert_eq!(lookup("FONT_FIXED"), Some(2));
        assert_eq!(lookup("LAYER_NOPE"), None);
    }

    #[test]
    fn the_standard_layers_are_in_the_order_of_their_numbers_and_never_repeat() {
        let layers: Vec<i64> = TABLE
            .iter()
            .filter(|(n, _)| n.starts_with("LAYER_"))
            .map(|(_, v)| *v)
            .collect();
        let mut sorted = layers.clone();
        sorted.sort_unstable();
        assert_eq!(layers, sorted);
        sorted.dedup();
        assert_eq!(sorted.len(), layers.len());
    }
}
