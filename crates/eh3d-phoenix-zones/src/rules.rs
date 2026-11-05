// SPDX-License-Identifier: GPL-3.0-or-later
//! The values of the design rules that a fill keeps, taken from the rule file, and the optional
//! file with the priority and the limits of single zones.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{Board, Id};
use eh3d_phoenix_rules::{NetClasses, RuleValues};
use serde::Deserialize;

use crate::codes::{ZoneError, ZONE_FILE_INVALID};
use crate::settings::Connection;

/// The clearances a fill keeps, in nanometres.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZoneRules {
    /// The least gap between copper of different nets.
    pub min_clearance_nm: i64,
    /// The least gap between copper and the edge of the board.
    pub min_edge_clearance_nm: i64,
    /// The least gap of the class of a net, by net.
    pub class_clearance_nm: BTreeMap<Id, i64>,
}

impl ZoneRules {
    /// The clearances of a rule file for the nets of a board: the general ones and the one of the
    /// class each net belongs to.
    pub fn from_rules(board: &Board, values: &RuleValues, classes: &NetClasses) -> ZoneRules {
        let mut class_clearance_nm = BTreeMap::new();
        for (id, net) in &board.nets {
            if let Some(class) = classes.class_of(&net.name) {
                if class.min_clearance.get() > 0 {
                    class_clearance_nm.insert(*id, class.min_clearance.get());
                }
            }
        }
        ZoneRules {
            min_clearance_nm: values.min_clearance.get(),
            min_edge_clearance_nm: values.min_edge_clearance.get(),
            class_clearance_nm,
        }
    }

    /// The gap a zone of net `zone_net`, with a clearance of its own, keeps from copper of
    /// another net (or of no net): the largest of the zone's own, the general rule and the
    /// classes of both nets.
    pub fn clearance(&self, zone_clearance: i64, zone_net: Id, other: Option<Id>) -> i64 {
        let mut gap = zone_clearance.max(self.min_clearance_nm);
        if let Some(v) = self.class_clearance_nm.get(&zone_net) {
            gap = gap.max(*v);
        }
        if let Some(v) = other.and_then(|n| self.class_clearance_nm.get(&n)) {
            gap = gap.max(*v);
        }
        gap
    }
}

/// The name of the format of the file of zone rules.
pub const ZONES_FORMAT: &str = "phoenix-zones/1";

/// What the file says about one zone.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneEntry {
    /// The zone.
    pub zone: Id,
    /// The priority; a zone of a higher one is filled first and the others keep away from it.
    #[serde(default)]
    pub priority: i32,
    /// A clearance of its own, in nanometres.
    #[serde(default)]
    pub clearance_nm: Option<i64>,
    /// A narrowest island of its own, in nanometres.
    #[serde(default)]
    pub min_width_nm: Option<i64>,
    /// A smallest island area of its own, in square nanometres.
    #[serde(default)]
    pub min_island_area_nm2: Option<i64>,
    /// How its pads are joined.
    #[serde(default)]
    pub connection: Option<Connection>,
}

/// The file with the priority and the limits of single zones.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneFile {
    /// Must be [`ZONES_FORMAT`].
    pub format: String,
    /// The zones it speaks of.
    pub zones: Vec<ZoneEntry>,
}

impl ZoneFile {
    /// Reads and checks the text of a file of zone rules.
    pub fn from_json(text: &str) -> Result<ZoneFile, ZoneError> {
        let file: ZoneFile = serde_json::from_str(text)
            .map_err(|e| ZoneError::new(ZONE_FILE_INVALID, e.to_string()))?;
        if file.format != ZONES_FORMAT {
            return Err(ZoneError::new(
                ZONE_FILE_INVALID,
                format!("the format must be {ZONES_FORMAT}"),
            ));
        }
        let mut seen: Vec<Id> = Vec::new();
        for entry in &file.zones {
            if seen.contains(&entry.zone) {
                return Err(ZoneError::new(
                    ZONE_FILE_INVALID,
                    format!("the zone {} is given twice", entry.zone),
                ));
            }
            seen.push(entry.zone);
            for (name, value) in [
                ("clearance_nm", entry.clearance_nm),
                ("min_width_nm", entry.min_width_nm),
                ("min_island_area_nm2", entry.min_island_area_nm2),
            ] {
                if value.is_some_and(|v| v < 0) {
                    return Err(ZoneError::new(
                        ZONE_FILE_INVALID,
                        format!("{name} must not be negative"),
                    ));
                }
            }
        }
        Ok(file)
    }

    /// The entry of a zone, when the file has one.
    pub fn entry(&self, zone: Id) -> Option<&ZoneEntry> {
        self.zones.iter().find(|e| e.zone == zone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID_A: &str = "01890a5d-ac96-7ab2-80e2-4536629c134d";

    #[test]
    fn a_file_of_zone_rules_is_read_and_checked() {
        let text = format!(
            "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{ID_A}\",\"priority\":2,\"connection\":\"solid\"}}]}}"
        );
        let file = ZoneFile::from_json(&text).unwrap();
        assert_eq!(file.zones.len(), 1);
        assert_eq!(file.zones[0].priority, 2);
        assert_eq!(file.zones[0].connection, Some(Connection::Solid));
        assert_eq!(file.zones[0].clearance_nm, None);
        let id = file.zones[0].zone;
        assert!(file.entry(id).is_some());
    }

    #[test]
    fn a_wrong_format_an_unknown_field_a_repeat_or_a_negative_value_is_refused() {
        let wrong = "{\"format\":\"other/1\",\"zones\":[]}";
        assert!(ZoneFile::from_json(wrong).is_err());
        let unknown = format!(
            "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{ID_A}\",\"colour\":1}}]}}"
        );
        assert_eq!(
            ZoneFile::from_json(&unknown).unwrap_err().code,
            ZONE_FILE_INVALID
        );
        let twice = format!(
            "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{ID_A}\"}},{{\"zone\":\"{ID_A}\"}}]}}"
        );
        assert!(ZoneFile::from_json(&twice)
            .unwrap_err()
            .detail
            .contains("twice"));
        let negative = format!(
            "{{\"format\":\"phoenix-zones/1\",\"zones\":[{{\"zone\":\"{ID_A}\",\"clearance_nm\":-1}}]}}"
        );
        assert!(ZoneFile::from_json(&negative).is_err());
        assert!(ZoneFile::from_json("not json").is_err());
    }

    #[test]
    fn the_gap_is_the_largest_of_the_zone_the_rule_and_the_classes() {
        let a: Id = ID_A.parse().unwrap();
        let b: Id = "01890a5d-ac96-7ab2-80e2-4536629c134e".parse().unwrap();
        let mut rules = ZoneRules {
            min_clearance_nm: 200_000,
            min_edge_clearance_nm: 300_000,
            class_clearance_nm: BTreeMap::new(),
        };
        assert_eq!(rules.clearance(100_000, a, Some(b)), 200_000);
        assert_eq!(rules.clearance(250_000, a, None), 250_000);
        rules.class_clearance_nm.insert(b, 400_000);
        assert_eq!(rules.clearance(100_000, a, Some(b)), 400_000);
        assert_eq!(rules.clearance(100_000, a, None), 200_000);
    }
}
