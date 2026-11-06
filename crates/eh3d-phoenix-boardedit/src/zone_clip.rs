// SPDX-License-Identifier: GPL-3.0-or-later
//! Copy and paste of a net or of a group of tracks, vias and zones, inside a board and between
//! boards. A clipboard holds the copper by the names of its nets and layers (the ids of one board
//! mean nothing in another) and its points relative to a reference point, the lower left corner of
//! its box. Pasting puts the reference point at a place, with a turn by quarter turns and a mirror
//! about the vertical axis, so every coordinate stays an exact integer. The ids of the copies come
//! from the supply of the editor, so the same board and the same paste give the same bytes, and a
//! paste is refused when the design rules find a new error.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::{Board, Id, LayerKind, Operation, Segment, Track, Via, Zone};
use eh3d_phoenix_model::{Nm, Point, Udeg};
use serde::{Deserialize, Serialize};

use crate::codes::BoardEditError;
use crate::ids::IdSupply;
use crate::zone::{error, ZoneData, ZoneEnv, ZonePlan, ZoneProps};
use crate::zone_codes::{CLIPBOARD_INVALID, PASTE_REFUSED};

/// The name of the format of a clipboard.
pub const CLIP_FORMAT: &str = "phoenix-clipboard/1";

/// A copied track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipTrack {
    /// The name of its net.
    pub net: String,
    /// The name of its layer.
    pub layer: String,
    /// The copper width in nanometres.
    pub width_nm: i64,
    /// The pieces, in the coordinates of the board it was copied from.
    pub path: Vec<Segment>,
}

/// A copied via.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipVia {
    /// The name of its net.
    pub net: String,
    /// Where it was.
    pub at: Point,
    /// The drill in nanometres.
    pub drill_nm: i64,
    /// The diameter of the pad in nanometres.
    pub diameter_nm: i64,
    /// The name of its first layer.
    pub from_layer: String,
    /// The name of its last layer.
    pub to_layer: String,
}

/// A copied zone with its properties (not its fill, which is made again).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipZone {
    /// The name of its net.
    pub net: String,
    /// The name of its layer.
    pub layer: String,
    /// The outline.
    pub outline: Vec<Point>,
    /// The clearance in nanometres.
    pub clearance_nm: i64,
    /// The properties, with the cutouts.
    pub props: ZoneProps,
}

/// A group of copper that was copied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clipboard {
    /// Must be [`CLIP_FORMAT`].
    pub format: String,
    /// The reference point: the lower left corner of the box of the group.
    pub origin: Point,
    /// The tracks.
    #[serde(default)]
    pub tracks: Vec<ClipTrack>,
    /// The vias.
    #[serde(default)]
    pub vias: Vec<ClipVia>,
    /// The zones.
    #[serde(default)]
    pub zones: Vec<ClipZone>,
}

fn net_name(board: &Board, id: Id) -> Result<String, BoardEditError> {
    match board.nets.get(&id) {
        Some(n) if !n.name.is_empty() => Ok(n.name.clone()),
        Some(_) => Err(error(
            CLIPBOARD_INVALID,
            format!("the net {id} has no name; a clipboard finds nets by name"),
        )),
        None => Err(error(CLIPBOARD_INVALID, format!("no net {id}"))),
    }
}

fn layer_name(board: &Board, id: Id) -> Result<String, BoardEditError> {
    board
        .layers
        .get(&id)
        .map(|l| l.name.clone())
        .ok_or_else(|| error(CLIPBOARD_INVALID, format!("no layer {id}")))
}

fn points_of_segment(s: &Segment) -> [Point; 2] {
    [s.start(), s.end()]
}

impl Clipboard {
    /// Copies every track, via and zone of a net.
    pub fn copy_net(board: &Board, data: &ZoneData, net: Id) -> Result<Clipboard, BoardEditError> {
        let tracks: Vec<Id> = board
            .tracks
            .iter()
            .filter(|(_, t)| t.net == net)
            .map(|(id, _)| *id)
            .collect();
        let vias: Vec<Id> = board
            .vias
            .iter()
            .filter(|(_, v)| v.net == net)
            .map(|(id, _)| *id)
            .collect();
        let zones: Vec<Id> = board
            .zones
            .iter()
            .filter(|(_, z)| z.net == net)
            .map(|(id, _)| *id)
            .collect();
        Clipboard::copy_items(board, data, &tracks, &vias, &zones)
    }

    /// Copies the tracks, vias and zones with these ids (in the order of their ids).
    pub fn copy_items(
        board: &Board,
        data: &ZoneData,
        tracks: &[Id],
        vias: &[Id],
        zones: &[Id],
    ) -> Result<Clipboard, BoardEditError> {
        let sorted = |ids: &[Id]| -> Vec<Id> {
            let set: BTreeSet<Id> = ids.iter().copied().collect();
            set.into_iter().collect()
        };
        let mut all: Vec<Point> = Vec::new();
        let mut out = Clipboard {
            format: CLIP_FORMAT.to_owned(),
            origin: Point::from_nm(0, 0).map_err(|e| error(CLIPBOARD_INVALID, e.to_string()))?,
            tracks: Vec::new(),
            vias: Vec::new(),
            zones: Vec::new(),
        };
        for id in sorted(tracks) {
            let t = board
                .tracks
                .get(&id)
                .ok_or_else(|| error(CLIPBOARD_INVALID, format!("no track {id}")))?;
            for s in &t.path {
                all.extend(points_of_segment(s));
            }
            out.tracks.push(ClipTrack {
                net: net_name(board, t.net)?,
                layer: layer_name(board, t.layer)?,
                width_nm: t.width.get(),
                path: t.path.clone(),
            });
        }
        for id in sorted(vias) {
            let v = board
                .vias
                .get(&id)
                .ok_or_else(|| error(CLIPBOARD_INVALID, format!("no via {id}")))?;
            all.push(v.at);
            out.vias.push(ClipVia {
                net: net_name(board, v.net)?,
                at: v.at,
                drill_nm: v.drill.get(),
                diameter_nm: v.diameter.get(),
                from_layer: layer_name(board, v.from_layer)?,
                to_layer: layer_name(board, v.to_layer)?,
            });
        }
        for id in sorted(zones) {
            let z = board
                .zones
                .get(&id)
                .ok_or_else(|| error(CLIPBOARD_INVALID, format!("no zone {id}")))?;
            all.extend(z.outline.iter().copied());
            out.zones.push(ClipZone {
                net: net_name(board, z.net)?,
                layer: layer_name(board, z.layer)?,
                outline: z.outline.clone(),
                clearance_nm: z.clearance.get(),
                props: data.props_of(id),
            });
        }
        let Some(min_x) = all.iter().map(|p| p.x_nm.get()).min() else {
            return Err(error(CLIPBOARD_INVALID, "there is nothing to copy"));
        };
        let min_y = all.iter().map(|p| p.y_nm.get()).min().unwrap_or(0);
        out.origin =
            Point::from_nm(min_x, min_y).map_err(|e| error(CLIPBOARD_INVALID, e.to_string()))?;
        Ok(out)
    }

    /// True when nothing was copied.
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty() && self.vias.is_empty() && self.zones.is_empty()
    }

    /// The clipboard as the text of a JSON file.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Reads a clipboard from JSON text. Unknown fields and another format are refused.
    pub fn from_json(text: &str) -> Result<Clipboard, BoardEditError> {
        let clip: Clipboard = serde_json::from_str(text).map_err(|e| {
            error(
                CLIPBOARD_INVALID,
                format!("the clipboard cannot be read: {e}"),
            )
        })?;
        if clip.format != CLIP_FORMAT {
            return Err(error(
                CLIPBOARD_INVALID,
                format!("the format must be {CLIP_FORMAT}"),
            ));
        }
        Ok(clip)
    }
}

/// What a paste asks for.
pub(crate) struct PasteRequest<'a> {
    pub clipboard: Option<&'a Clipboard>,
    pub from_net: Option<Id>,
    pub at: Point,
    pub quarter_turns: u8,
    pub mirror: bool,
    pub net_map: &'a BTreeMap<String, String>,
}

/// Where a point of the clipboard goes: relative to the reference point, mirrored about the
/// vertical axis, turned by quarter turns counter-clockwise, then placed at `at`.
fn transform(
    p: Point,
    origin: Point,
    at: Point,
    quarter_turns: u8,
    mirror: bool,
) -> Result<Point, BoardEditError> {
    let mut x = p.x_nm.get() - origin.x_nm.get();
    let mut y = p.y_nm.get() - origin.y_nm.get();
    if mirror {
        x = -x;
    }
    for _ in 0..quarter_turns {
        (x, y) = (-y, x);
    }
    Point::from_nm(at.x_nm.get() + x, at.y_nm.get() + y).map_err(|e| {
        error(
            PASTE_REFUSED,
            format!("a point leaves the design range: {e}"),
        )
    })
}

fn transform_segment(
    s: &Segment,
    origin: Point,
    at: Point,
    quarter_turns: u8,
    mirror: bool,
) -> Result<Segment, BoardEditError> {
    let tp = |p: Point| transform(p, origin, at, quarter_turns, mirror);
    match s {
        Segment::Line { start, end } => Ok(Segment::Line {
            start: tp(*start)?,
            end: tp(*end)?,
        }),
        Segment::Arc { start, end, sweep } => {
            let sweep = if mirror {
                Udeg::new(-sweep.get()).map_err(|e| error(PASTE_REFUSED, e.to_string()))?
            } else {
                *sweep
            };
            Ok(Segment::Arc {
                start: tp(*start)?,
                end: tp(*end)?,
                sweep,
            })
        }
    }
}

fn nm(value: i64) -> Result<Nm, BoardEditError> {
    Nm::new(value).map_err(|e| error(CLIPBOARD_INVALID, e.to_string()))
}

/// The id of the net a copy gets: the net of that name (after the map), or a new one.
struct Names<'a> {
    board: &'a Board,
    net_map: &'a BTreeMap<String, String>,
    nets: BTreeMap<String, Id>,
    new_nets: Vec<(Id, String)>,
}

impl Names<'_> {
    fn net(&mut self, name: &str, ids: &mut IdSupply) -> Id {
        let target = self.net_map.get(name).map_or(name, String::as_str);
        if let Some(id) = self.nets.get(target) {
            return *id;
        }
        let found = self
            .board
            .nets
            .iter()
            .find(|(_, n)| n.name == target)
            .map(|(id, _)| *id);
        let id = match found {
            Some(id) => id,
            None => {
                let id = ids.next_for(self.board);
                self.new_nets.push((id, target.to_owned()));
                id
            }
        };
        self.nets.insert(target.to_owned(), id);
        id
    }

    fn layer(&self, name: &str) -> Result<Id, BoardEditError> {
        self.board
            .layers
            .iter()
            .find(|(_, l)| l.name == name && l.kind == LayerKind::Copper)
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                error(
                    PASTE_REFUSED,
                    format!("the board has no copper layer called {name}"),
                )
            })
    }
}

/// Plans a paste.
pub(crate) fn plan_paste(
    board: &Board,
    env: &ZoneEnv,
    ids: &mut IdSupply,
    req: &PasteRequest,
) -> Result<ZonePlan, BoardEditError> {
    if req.quarter_turns > 3 {
        return Err(error(
            PASTE_REFUSED,
            "quarter_turns must be 0, 1, 2 or 3 (a turn by another angle would round the points)",
        ));
    }
    let clip: Clipboard = match (req.clipboard, req.from_net) {
        (Some(c), None) => {
            if c.format != CLIP_FORMAT {
                return Err(error(
                    CLIPBOARD_INVALID,
                    format!("the format must be {CLIP_FORMAT}"),
                ));
            }
            c.clone()
        }
        (None, Some(net)) => Clipboard::copy_net(board, env.data, net)?,
        _ => {
            return Err(error(
                PASTE_REFUSED,
                "give either a clipboard or the net to copy, not both and not neither",
            ))
        }
    };
    if clip.is_empty() {
        return Err(error(PASTE_REFUSED, "the clipboard is empty"));
    }
    let (origin, at, q, m) = (clip.origin, req.at, req.quarter_turns, req.mirror);
    let mut names = Names {
        board,
        net_map: req.net_map,
        nets: BTreeMap::new(),
        new_nets: Vec::new(),
    };
    let mut made: Vec<Operation> = Vec::new();
    let mut created: Vec<Id> = Vec::new();
    let mut data = env.data.clone();
    for t in &clip.tracks {
        let net = names.net(&t.net, ids);
        let layer = names.layer(&t.layer)?;
        let mut path: Vec<Segment> = Vec::with_capacity(t.path.len());
        for s in &t.path {
            path.push(transform_segment(s, origin, at, q, m)?);
        }
        let id = ids.next_for(board);
        created.push(id);
        made.push(Operation::AddTrack {
            id,
            track: Track {
                net,
                layer,
                width: nm(t.width_nm)?,
                path,
            },
        });
    }
    for v in &clip.vias {
        let net = names.net(&v.net, ids);
        let from_layer = names.layer(&v.from_layer)?;
        let to_layer = names.layer(&v.to_layer)?;
        let id = ids.next_for(board);
        created.push(id);
        made.push(Operation::AddVia {
            id,
            via: Via {
                net,
                at: transform(v.at, origin, at, q, m)?,
                drill: nm(v.drill_nm)?,
                diameter: nm(v.diameter_nm)?,
                from_layer,
                to_layer,
            },
        });
    }
    for z in &clip.zones {
        let net = names.net(&z.net, ids);
        let layer = names.layer(&z.layer)?;
        let mut outline: Vec<Point> = Vec::with_capacity(z.outline.len());
        for p in &z.outline {
            outline.push(transform(*p, origin, at, q, m)?);
        }
        let mut props = z.props.clone();
        for hole in &mut props.cutouts {
            for p in hole.iter_mut() {
                *p = transform(*p, origin, at, q, m)?;
            }
        }
        let id = ids.next_for(board);
        created.push(id);
        data.props.insert(id, props);
        made.push(Operation::AddZone {
            id,
            zone: Zone {
                net,
                layer,
                outline,
                clearance: nm(z.clearance_nm)?,
            },
        });
    }
    let mut ops: Vec<Operation> = Vec::with_capacity(made.len() + names.new_nets.len());
    for (id, name) in &names.new_nets {
        ops.push(Operation::AddNet {
            id: *id,
            name: name.clone(),
        });
    }
    ops.extend(made);
    let notes = vec![format!(
        "pasted {} tracks, {} vias and {} zones ({} new nets); the zones have no fill yet",
        clip.tracks.len(),
        clip.vias.len(),
        clip.zones.len(),
        names.new_nets.len()
    )];
    Ok(ZonePlan {
        board_op: Operation::Batch { ops },
        data,
        created,
        notes,
        strict: true,
    })
}
