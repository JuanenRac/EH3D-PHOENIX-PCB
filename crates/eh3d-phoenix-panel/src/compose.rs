// SPDX-License-Identifier: GPL-3.0-or-later
//! The board of the panel: N copies of the boards of a plan, each moved to its place, turned and
//! mirrored as the plan says, with new ids for everything, and the marks of the panel (fiducials,
//! tooling holes and mouse bite holes) as parts of their own. The export of the layers
//! (`eh3d-phoenix-export`) then writes this board like any other, so the panel gets the copper,
//! the solder mask, the paste, the legend and the drill files of the boards without a second
//! writer of those files.
//!
//! A mirrored copy is the board seen from its other side: its positions are mirrored about the
//! vertical axis of its box, its layers change sides (the top copper layer becomes the bottom one,
//! the second layer the one before the last, a top legend layer the bottom one), its components
//! change sides and its text is written back to front.

use std::collections::BTreeMap;

use eh3d_phoenix_board::{
    Board, Component, Footprint, Id, Layer, LayerKind, Net, Pad, PadNet, PadShape, Segment, Side,
    SilkItem, Track, Via, Zone,
};
use eh3d_phoenix_model::{Nm, ObjectId, Point, Udeg};

use crate::codes::{PanelError, LAYERS_DIFFER, PANEL_TOO_LARGE};
use crate::layout::{local_map, too_large, BoardShape, PanelPlan, Pt};
use crate::settings::PanelSettings;

const FULL_TURN_UDEG: i64 = 360_000_000;

/// Makes ids for the parts of the panel: the same plan always gives the same ids.
struct Ids {
    next: u64,
}

impl Ids {
    fn make(&mut self) -> Result<Id, PanelError> {
        self.next += 1;
        ObjectId::from_parts(1_900_000_000_000, (self.next & 0xfff) as u16, self.next)
            .map_err(|e| PanelError::new(PANEL_TOO_LARGE, format!("no id for a part: {e:?}")))
    }
}

/// Where the parts of one copy go.
struct Xf {
    source: Pt,
    width: i64,
    height: i64,
    rotation_deg: u32,
    mirror: bool,
    dest: Pt,
}

impl Xf {
    fn point(&self, p: Point) -> Result<Point, PanelError> {
        let q = local_map(
            p.x_nm.get() - self.source.x,
            p.y_nm.get() - self.source.y,
            self.width,
            self.height,
            self.rotation_deg,
            self.mirror,
        );
        Point::from_nm(self.dest.x + q.x, self.dest.y + q.y)
            .map_err(|_| too_large("a point leaves the coordinate range"))
    }

    /// The rotation of a component or of a text.
    fn angle(&self, angle: Udeg) -> Result<Udeg, PanelError> {
        let turn = i64::from(self.rotation_deg) * 1_000_000;
        let value = if self.mirror {
            turn - angle.get()
        } else {
            angle.get() + turn
        };
        Udeg::new(value.rem_euclid(FULL_TURN_UDEG))
            .map_err(|_| too_large("an angle leaves the range"))
    }

    /// The rotation of a pad inside its footprint.
    fn pad_angle(&self, angle: Udeg) -> Result<Udeg, PanelError> {
        if !self.mirror {
            return Ok(angle);
        }
        Udeg::new((-angle.get()).rem_euclid(FULL_TURN_UDEG))
            .map_err(|_| too_large("an angle leaves the range"))
    }

    fn path(&self, path: &[Segment]) -> Result<Vec<Segment>, PanelError> {
        let mut out = Vec::with_capacity(path.len());
        for piece in path {
            out.push(match piece {
                Segment::Line { start, end } => Segment::Line {
                    start: self.point(*start)?,
                    end: self.point(*end)?,
                },
                Segment::Arc { start, end, sweep } => Segment::Arc {
                    start: self.point(*start)?,
                    end: self.point(*end)?,
                    sweep: if self.mirror {
                        Udeg::new(-sweep.get())
                            .map_err(|_| too_large("a sweep leaves the range"))?
                    } else {
                        *sweep
                    },
                },
            });
        }
        Ok(out)
    }
}

fn nm(value: i64) -> Result<Nm, PanelError> {
    Nm::new(value).map_err(|_| too_large("a length leaves the range"))
}

fn point(x: i64, y: i64) -> Result<Point, PanelError> {
    Point::from_nm(x, y).map_err(|_| too_large("a point leaves the coordinate range"))
}

/// For each layer of a board (but the outline), the layer of the first board that is the same:
/// same kind, name and side.
fn layer_map(base: &Board, other: &Board, number: usize) -> Result<BTreeMap<Id, Id>, PanelError> {
    let mut map = BTreeMap::new();
    for (id, layer) in &other.layers {
        if layer.kind == LayerKind::Outline {
            continue;
        }
        let found = base
            .layers
            .iter()
            .find(|(_, b)| b.kind == layer.kind && b.name == layer.name && b.side == layer.side);
        match found {
            Some((base_id, _)) => {
                map.insert(*id, *base_id);
            }
            None => {
                return Err(PanelError::new(
                    LAYERS_DIFFER,
                    format!(
                        "board {number} has a layer {:?} that the first board does not have",
                        layer.name
                    ),
                ))
            }
        }
    }
    Ok(map)
}

/// The layer that a layer becomes on the other side of the board.
fn mirror_layers(panel: &Board) -> BTreeMap<Id, Id> {
    let mut map = BTreeMap::new();
    let n = panel.copper_stack.len();
    for (i, id) in panel.copper_stack.iter().enumerate() {
        map.insert(*id, panel.copper_stack[n - 1 - i]);
    }
    for (id, layer) in &panel.layers {
        if map.contains_key(id) {
            continue;
        }
        let other = layer.side.and_then(|side| {
            panel
                .layers
                .iter()
                .find(|(_, o)| o.kind == layer.kind && o.side.is_some() && o.side != Some(side))
                .map(|(other_id, _)| *other_id)
        });
        map.insert(*id, other.unwrap_or(*id));
    }
    map
}

/// The layers of the first board that the copies use, with the maps of the others.
struct Layers {
    maps: Vec<BTreeMap<Id, Id>>,
    mirror: BTreeMap<Id, Id>,
}

impl Layers {
    fn of(&self, board: usize, mirrored: bool, layer: Id) -> Option<Id> {
        let id = self.maps.get(board)?.get(&layer).copied()?;
        if mirrored {
            self.mirror.get(&id).copied()
        } else {
            Some(id)
        }
    }

    fn need(&self, board: usize, mirrored: bool, layer: Id) -> Result<Id, PanelError> {
        self.of(board, mirrored, layer).ok_or_else(|| {
            PanelError::new(
                LAYERS_DIFFER,
                format!(
                    "a part of board {} is on a layer that has no twin",
                    board + 1
                ),
            )
        })
    }
}

/// The top or bottom copper layer and the solder mask layer of a side.
fn side_layers(panel: &Board, side: Side) -> (Option<Id>, Option<Id>) {
    let by_side = |kind: LayerKind| {
        panel
            .layers
            .iter()
            .find(|(_, l)| l.kind == kind && l.side == Some(side))
            .map(|(id, _)| *id)
    };
    let copper = match (side, panel.copper_stack.len()) {
        (_, 0) => by_side(LayerKind::Copper),
        (Side::Top, _) => panel.copper_stack.first().copied(),
        (Side::Bottom, 1) => None,
        (Side::Bottom, _) => panel.copper_stack.last().copied(),
    };
    (copper, by_side(LayerKind::SolderMask))
}

fn plain_pad(
    ids: &mut Ids,
    shape_nm: i64,
    drill: Option<i64>,
    layers: Vec<Id>,
) -> Result<(Id, Pad), PanelError> {
    let pad = Pad {
        number: "1".to_owned(),
        shape: PadShape::Circle {
            diameter: nm(shape_nm)?,
        },
        at: point(0, 0)?,
        rotation: Udeg::ZERO,
        drill: drill.map(nm).transpose()?,
        layers,
        roundness: 0,
        plated: true,
    };
    Ok((ids.make()?, pad))
}

/// A part of the panel that is not a part of a board: a fiducial, a tooling hole or a mouse bite
/// hole.
struct Hardware<'a> {
    out: &'a mut Board,
    ids: &'a mut Ids,
    footprints: BTreeMap<(u8, i64), Id>,
}

impl Hardware<'_> {
    fn place(&mut self, refdes: String, footprint: Id, at: Pt) -> Result<(), PanelError> {
        let id = self.ids.make()?;
        self.out.components.insert(
            id,
            Component {
                refdes,
                value: String::new(),
                symbol: None,
                footprint,
                at: point(at.x, at.y)?,
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        Ok(())
    }

    /// The footprint of a hole of a diameter: one pad with a drill and on no layer, so it is a
    /// hole that is not plated and draws nothing.
    fn hole(&mut self, diameter: i64) -> Result<Id, PanelError> {
        if let Some(id) = self.footprints.get(&(0, diameter)) {
            return Ok(*id);
        }
        let (pad_id, pad) = plain_pad(self.ids, diameter, Some(diameter), Vec::new())?;
        let id = self.ids.make()?;
        self.out.footprints.insert(
            id,
            Footprint {
                name: "PANEL_HOLE".to_owned(),
                pads: BTreeMap::from([(pad_id, pad)]),
            },
        );
        self.footprints.insert((0, diameter), id);
        Ok(id)
    }

    /// The footprint of a fiducial on a side: a copper circle and a solder mask opening around it.
    /// The opening is drawn larger than its pad by the expansion of the export, so the pad is
    /// smaller by as much.
    fn fiducial(
        &mut self,
        side: Side,
        copper_nm: i64,
        mask_nm: i64,
        expansion_nm: i64,
    ) -> Result<Option<Id>, PanelError> {
        let key = (if side == Side::Top { 1 } else { 2 }, copper_nm);
        if let Some(id) = self.footprints.get(&key) {
            return Ok(Some(*id));
        }
        let (copper, mask) = side_layers(self.out, side);
        let Some(copper) = copper else {
            return Ok(None);
        };
        let mut pads = BTreeMap::new();
        let (pad_id, pad) = plain_pad(self.ids, copper_nm, None, vec![copper])?;
        pads.insert(pad_id, pad);
        if let Some(mask) = mask {
            let opening = mask_nm
                .saturating_sub(expansion_nm.saturating_mul(2))
                .max(1);
            let (pad_id, mut pad) = plain_pad(self.ids, opening, None, vec![mask])?;
            pad.number = "2".to_owned();
            pads.insert(pad_id, pad);
        }
        let id = self.ids.make()?;
        self.out.footprints.insert(
            id,
            Footprint {
                name: "PANEL_FIDUCIAL".to_owned(),
                pads,
            },
        );
        self.footprints.insert(key, id);
        Ok(Some(id))
    }
}

/// The net of a copy that stands for a net of the board; a part that is on a net the board does not
/// have gets a net of the copy that has no name.
fn net_for(net_ids: &mut BTreeMap<Id, Id>, ids: &mut Ids, net: Id) -> Result<Id, PanelError> {
    if let Some(found) = net_ids.get(&net) {
        return Ok(*found);
    }
    let made = ids.make()?;
    net_ids.insert(net, made);
    Ok(made)
}

fn copy_suffix(settings: &PanelSettings, number: usize) -> String {
    if settings.copy_suffix {
        format!("{}{}", settings.copy_suffix_separator, number)
    } else {
        String::new()
    }
}

fn with_suffix(name: &str, suffix: &str) -> String {
    if name.is_empty() {
        String::new()
    } else {
        format!("{name}{suffix}")
    }
}

/// Makes the board of the panel from the boards of the plan. With `hardware` the fiducials, the
/// tooling holes and the mouse bite holes are parts of the board too; without it the board holds
/// only the copies (for the placement file and the bill of materials of the panel).
///
/// The first board gives the layers: every other board needs a layer of the same kind, name and
/// side for each of its own layers, and the same number of copper layers.
pub fn compose(
    boards: &[Board],
    plan: &PanelPlan,
    settings: &PanelSettings,
    mask_expansion_nm: i64,
    hardware: bool,
) -> Result<Board, PanelError> {
    let Some(base) = boards.first() else {
        return Err(PanelError::new(LAYERS_DIFFER, "there is no board"));
    };
    let mut shapes: Vec<BoardShape> = Vec::new();
    let mut maps: Vec<BTreeMap<Id, Id>> = Vec::new();
    for (index, board) in boards.iter().enumerate() {
        if board.copper_stack.len() != base.copper_stack.len() {
            return Err(PanelError::new(
                LAYERS_DIFFER,
                format!(
                    "board {} has {} copper layers in its stack and the first one {}",
                    index + 1,
                    board.copper_stack.len(),
                    base.copper_stack.len()
                ),
            ));
        }
        shapes.push(BoardShape::from_board(board)?.0);
        maps.push(layer_map(base, board, index + 1)?);
    }
    let mut out = Board {
        name: if base.name.trim().is_empty() {
            "panel".to_owned()
        } else {
            format!("{}_panel", base.name)
        },
        outline: vec![
            point(0, 0)?,
            point(plan.width, 0)?,
            point(plan.width, plan.height)?,
            point(0, plan.height)?,
        ],
        ..Board::default()
    };
    for (id, layer) in &base.layers {
        if layer.kind != LayerKind::Outline {
            out.layers.insert(
                *id,
                Layer {
                    name: layer.name.clone(),
                    kind: layer.kind,
                    side: layer.side,
                },
            );
        }
    }
    out.copper_stack = base.copper_stack.clone();
    let layers = Layers {
        maps,
        mirror: mirror_layers(&out),
    };
    let mut ids = Ids { next: 0 };
    let mut footprint_ids: BTreeMap<(usize, Id, bool), Id> = BTreeMap::new();
    let mut pad_nets: Vec<PadNet> = Vec::new();
    for (index, copy) in plan.copies.iter().enumerate() {
        let source = boards.get(copy.board).ok_or_else(|| {
            PanelError::new(
                LAYERS_DIFFER,
                format!(
                    "the plan names board {} of {}",
                    copy.board + 1,
                    boards.len()
                ),
            )
        })?;
        let shape = &shapes[copy.board];
        let xf = Xf {
            source: shape.origin,
            width: shape.width,
            height: shape.height,
            rotation_deg: copy.rotation_deg,
            mirror: copy.mirror,
            dest: copy.origin,
        };
        let suffix = copy_suffix(settings, index + 1);
        let mut net_ids: BTreeMap<Id, Id> = BTreeMap::new();
        for (net_id, net) in &source.nets {
            let id = ids.make()?;
            net_ids.insert(*net_id, id);
            out.nets.insert(
                id,
                Net {
                    name: with_suffix(&net.name, &suffix),
                },
            );
        }
        let mut component_ids: BTreeMap<Id, Id> = BTreeMap::new();
        for (component_id, component) in &source.components {
            let Some(source_footprint) = source.footprints.get(&component.footprint) else {
                continue;
            };
            let key = (copy.board, component.footprint, copy.mirror);
            let footprint = match footprint_ids.get(&key) {
                Some(id) => *id,
                None => {
                    let mut pads = BTreeMap::new();
                    for (pad_id, pad) in &source_footprint.pads {
                        let mut on = Vec::new();
                        for layer in &pad.layers {
                            if let Some(found) = layers.of(copy.board, copy.mirror, *layer) {
                                on.push(found);
                            }
                        }
                        pads.insert(
                            *pad_id,
                            Pad {
                                number: pad.number.clone(),
                                shape: pad.shape,
                                at: pad.at,
                                rotation: xf.pad_angle(pad.rotation)?,
                                drill: pad.drill,
                                layers: on,
                                roundness: 0,
                                plated: true,
                            },
                        );
                    }
                    let id = ids.make()?;
                    out.footprints.insert(
                        id,
                        Footprint {
                            name: source_footprint.name.clone(),
                            pads,
                        },
                    );
                    footprint_ids.insert(key, id);
                    id
                }
            };
            let id = ids.make()?;
            component_ids.insert(*component_id, id);
            out.components.insert(
                id,
                Component {
                    refdes: with_suffix(&component.refdes, &suffix),
                    value: component.value.clone(),
                    symbol: component.symbol,
                    footprint,
                    at: xf.point(component.at)?,
                    rotation: xf.angle(component.rotation)?,
                    side: match (component.side, copy.mirror) {
                        (Side::Top, true) => Side::Bottom,
                        (Side::Bottom, true) => Side::Top,
                        (side, false) => side,
                    },
                },
            );
        }
        for pad_net in &source.pad_nets {
            if let (Some(component), Some(net)) = (
                component_ids.get(&pad_net.component),
                net_ids.get(&pad_net.net),
            ) {
                pad_nets.push(PadNet {
                    component: *component,
                    pad: pad_net.pad,
                    net: *net,
                });
            }
        }
        for track in source.tracks.values() {
            let net = net_for(&mut net_ids, &mut ids, track.net)?;
            let id = ids.make()?;
            out.tracks.insert(
                id,
                Track {
                    net,
                    layer: layers.need(copy.board, copy.mirror, track.layer)?,
                    width: track.width,
                    path: xf.path(&track.path)?,
                },
            );
        }
        for via in source.vias.values() {
            let net = net_for(&mut net_ids, &mut ids, via.net)?;
            let id = ids.make()?;
            out.vias.insert(
                id,
                Via {
                    net,
                    at: xf.point(via.at)?,
                    drill: via.drill,
                    diameter: via.diameter,
                    from_layer: layers.need(copy.board, copy.mirror, via.from_layer)?,
                    to_layer: layers.need(copy.board, copy.mirror, via.to_layer)?,
                },
            );
        }
        for zone in source.zones.values() {
            let net = net_for(&mut net_ids, &mut ids, zone.net)?;
            let mut outline = Vec::with_capacity(zone.outline.len());
            for p in &zone.outline {
                outline.push(xf.point(*p)?);
            }
            let id = ids.make()?;
            out.zones.insert(
                id,
                Zone {
                    net,
                    layer: layers.need(copy.board, copy.mirror, zone.layer)?,
                    outline,
                    clearance: zone.clearance,
                    cutouts: Vec::new(),
                },
            );
        }
        for item in source.silkscreen.values() {
            let moved = match item {
                SilkItem::Text {
                    layer,
                    at,
                    text,
                    height,
                    stroke,
                    rotation,
                    mirrored,
                } => SilkItem::Text {
                    layer: layers.need(copy.board, copy.mirror, *layer)?,
                    at: xf.point(*at)?,
                    text: text.clone(),
                    height: *height,
                    stroke: *stroke,
                    rotation: xf.angle(*rotation)?,
                    mirrored: *mirrored != copy.mirror,
                },
                SilkItem::Stroke { layer, width, path } => SilkItem::Stroke {
                    layer: layers.need(copy.board, copy.mirror, *layer)?,
                    width: *width,
                    path: xf.path(path)?,
                },
            };
            let id = ids.make()?;
            out.silkscreen.insert(id, moved);
        }
    }
    pad_nets.sort_by_key(PadNet::pad_ref);
    out.pad_nets = pad_nets;

    if hardware {
        let mut parts = Hardware {
            out: &mut out,
            ids: &mut ids,
            footprints: BTreeMap::new(),
        };
        let mut tooling = 0usize;
        let mut bites = 0usize;
        for hole in &plan.holes {
            let (prefix, counter) = match hole.kind {
                crate::layout::HoleKind::Tooling => ("TOOL", &mut tooling),
                crate::layout::HoleKind::MouseBite => ("MB", &mut bites),
                _ => continue,
            };
            *counter += 1;
            let footprint = parts.hole(hole.diameter)?;
            parts.place(format!("{prefix}{counter}"), footprint, hole.at)?;
        }
        let sides: &[(Side, &str)] = if settings.fiducials_both_sides {
            &[(Side::Top, ""), (Side::Bottom, "B")]
        } else {
            &[(Side::Top, "")]
        };
        let marks = plan
            .fiducials
            .iter()
            .map(|m| ("FID", m))
            .chain(plan.local_fiducials.iter().map(|m| ("FIDL", m)));
        let mut counts = (0usize, 0usize);
        for (prefix, mark) in marks {
            let counter = if prefix == "FID" {
                &mut counts.0
            } else {
                &mut counts.1
            };
            *counter += 1;
            let number = *counter;
            for (side, tail) in sides {
                let Some(footprint) = parts.fiducial(
                    *side,
                    mark.copper_diameter,
                    mark.mask_diameter,
                    mask_expansion_nm,
                )?
                else {
                    continue;
                };
                parts.place(format!("{prefix}{number}{tail}"), footprint, mark.at)?;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::plan;

    const MM: i64 = 1_000_000;

    fn id(n: u64) -> Id {
        ObjectId::from_parts(1_700_000_000_000, (n & 0xfff) as u16, n).unwrap()
    }

    fn pt(x: i64, y: i64) -> Point {
        Point::from_nm(x * MM, y * MM).unwrap()
    }

    /// A board of 30 by 20 mm with two copper layers, a mask and a legend on each side, one
    /// resistor with two pads, a through-hole pad, a track, a via, a zone and a text.
    fn sample(name: &str) -> Board {
        let (top, bottom, mask_top, mask_bottom, silk_top, silk_bottom, outline_layer) =
            (id(1), id(2), id(3), id(4), id(5), id(6), id(7));
        let mut b = Board {
            name: name.to_owned(),
            outline: vec![pt(0, 0), pt(30, 0), pt(30, 20), pt(0, 20)],
            ..Board::default()
        };
        for (layer, text, kind, side) in [
            (top, "Top", LayerKind::Copper, Some(Side::Top)),
            (bottom, "Bottom", LayerKind::Copper, Some(Side::Bottom)),
            (mask_top, "Mask top", LayerKind::SolderMask, Some(Side::Top)),
            (
                mask_bottom,
                "Mask bottom",
                LayerKind::SolderMask,
                Some(Side::Bottom),
            ),
            (
                silk_top,
                "Legend top",
                LayerKind::Silkscreen,
                Some(Side::Top),
            ),
            (
                silk_bottom,
                "Legend bottom",
                LayerKind::Silkscreen,
                Some(Side::Bottom),
            ),
            (outline_layer, "Outline", LayerKind::Outline, None),
        ] {
            b.layers.insert(
                layer,
                Layer {
                    name: text.to_owned(),
                    kind,
                    side,
                },
            );
        }
        b.copper_stack = vec![top, bottom];
        let net = id(10);
        b.nets.insert(
            net,
            Net {
                name: "SIG".to_owned(),
            },
        );
        let (smd, thru) = (id(20), id(21));
        let pad = |number: &str, x: i64, drill: Option<i64>, layers: Vec<Id>| Pad {
            number: number.to_owned(),
            shape: PadShape::Rect {
                width: Nm::new(1_200_000).unwrap(),
                height: Nm::new(1_000_000).unwrap(),
            },
            at: Point::from_nm(x, 0).unwrap(),
            rotation: Udeg::ZERO,
            drill: drill.map(|d| Nm::new(d).unwrap()),
            layers,
            roundness: 0,
            plated: true,
        };
        b.footprints.insert(
            smd,
            Footprint {
                name: "R0603".to_owned(),
                pads: BTreeMap::from([
                    (id(30), pad("1", -800_000, None, vec![top, mask_top])),
                    (id(31), pad("2", 800_000, None, vec![top, mask_top])),
                ]),
            },
        );
        b.footprints.insert(
            thru,
            Footprint {
                name: "HEADER".to_owned(),
                pads: BTreeMap::from([(
                    id(32),
                    pad(
                        "1",
                        0,
                        Some(900_000),
                        vec![top, bottom, mask_top, mask_bottom],
                    ),
                )]),
            },
        );
        b.components.insert(
            id(40),
            Component {
                refdes: "R1".to_owned(),
                value: "10k".to_owned(),
                symbol: None,
                footprint: smd,
                at: pt(10, 5),
                rotation: Udeg::ZERO,
                side: Side::Top,
            },
        );
        b.components.insert(
            id(41),
            Component {
                refdes: "J1".to_owned(),
                value: String::new(),
                symbol: None,
                footprint: thru,
                at: pt(25, 15),
                rotation: Udeg::new(90_000_000).unwrap(),
                side: Side::Top,
            },
        );
        b.pad_nets = vec![
            PadNet {
                component: id(40),
                pad: id(30),
                net,
            },
            PadNet {
                component: id(41),
                pad: id(32),
                net,
            },
        ];
        b.pad_nets.sort_by_key(PadNet::pad_ref);
        b.tracks.insert(
            id(50),
            Track {
                net,
                layer: top,
                width: Nm::new(250_000).unwrap(),
                path: vec![Segment::Line {
                    start: pt(10, 5),
                    end: pt(25, 15),
                }],
            },
        );
        b.vias.insert(
            id(51),
            Via {
                net,
                at: pt(5, 5),
                drill: Nm::new(300_000).unwrap(),
                diameter: Nm::new(600_000).unwrap(),
                from_layer: top,
                to_layer: bottom,
            },
        );
        b.zones.insert(
            id(52),
            Zone {
                net,
                layer: bottom,
                outline: vec![pt(1, 1), pt(9, 1), pt(9, 9), pt(1, 9)],
                clearance: Nm::new(200_000).unwrap(),
                cutouts: Vec::new(),
            },
        );
        b.silkscreen.insert(
            id(53),
            SilkItem::Text {
                layer: silk_top,
                at: pt(2, 17),
                text: "R1".to_owned(),
                height: Nm::new(1_000_000).unwrap(),
                stroke: Nm::new(150_000).unwrap(),
                rotation: Udeg::ZERO,
                mirrored: false,
            },
        );
        b
    }

    fn panel_of(board: &Board, text: &str) -> (PanelSettings, PanelPlan) {
        let settings = PanelSettings::parse(text).unwrap();
        let (shape, _) = BoardShape::from_board(board).unwrap();
        let plan = plan(&shape, &settings).unwrap();
        (settings, plan)
    }

    #[test]
    fn a_matrix_holds_n_copies_of_every_part_with_new_ids() {
        let board = sample("one");
        let (settings, plan) = panel_of(&board, "columns = 3\nrows = 2\n");
        let panel = compose(
            std::slice::from_ref(&board),
            &plan,
            &settings,
            50_000,
            false,
        )
        .unwrap();
        assert_eq!(panel.name, "one_panel");
        assert_eq!(panel.components.len(), 6 * 2);
        assert_eq!(panel.tracks.len(), 6);
        assert_eq!(panel.vias.len(), 6);
        assert_eq!(panel.zones.len(), 6);
        assert_eq!(panel.silkscreen.len(), 6);
        assert_eq!(panel.nets.len(), 6);
        assert_eq!(panel.pad_nets.len(), 12);
        // no outline layer, the same other layers
        assert_eq!(panel.layers.len(), board.layers.len() - 1);
        assert_eq!(panel.copper_stack, board.copper_stack);
        // the pad nets are sorted, so the net of a pad can be found
        assert!(panel
            .pad_nets
            .windows(2)
            .all(|w| w[0].pad_ref() < w[1].pad_ref()));
        // a copy is the board moved: the first copy of the via is at the origin of the first box
        let origin = plan.copies[0].origin;
        assert!(panel
            .vias
            .values()
            .any(|v| v.at.x_nm.get() == origin.x + 5 * MM && v.at.y_nm.get() == origin.y + 5 * MM));
        // the same plan gives the same panel
        let again = compose(&[board], &plan, &settings, 50_000, false).unwrap();
        assert_eq!(panel, again);
    }

    #[test]
    fn the_copies_can_carry_their_number_on_the_names() {
        let board = sample("one");
        let (settings, plan) = panel_of(&board, "columns = 2\nrows = 1\ncopy_suffix = true\n");
        let panel = compose(std::slice::from_ref(&board), &plan, &settings, 0, false).unwrap();
        let mut refdes: Vec<&str> = panel
            .components
            .values()
            .map(|c| c.refdes.as_str())
            .collect();
        refdes.sort_unstable();
        assert_eq!(refdes, ["J1_1", "J1_2", "R1_1", "R1_2"]);
        let mut nets: Vec<&str> = panel.nets.values().map(|n| n.name.as_str()).collect();
        nets.sort_unstable();
        assert_eq!(nets, ["SIG_1", "SIG_2"]);
        let plain = PanelSettings::parse("columns = 2\nrows = 1\n").unwrap();
        let panel = compose(&[board], &plan, &plain, 0, false).unwrap();
        assert!(
            panel
                .components
                .values()
                .filter(|c| c.refdes == "R1")
                .count()
                == 2
        );
    }

    #[test]
    fn a_turned_copy_moves_its_pads_where_the_turned_board_has_them() {
        let board = sample("one");
        let (settings, plan) = panel_of(&board, "columns = 1\nrows = 1\ncopy_rotation_deg = 90\n");
        let panel = compose(std::slice::from_ref(&board), &plan, &settings, 0, false).unwrap();
        // the resistor R1 at (10, 5) of a box of 30 by 20, turned 90 degrees: (20 - 5, 10) = (15, 10)
        let origin = plan.copies[0].origin;
        let r1 = panel
            .components
            .values()
            .find(|c| c.refdes == "R1")
            .unwrap();
        assert_eq!(
            (r1.at.x_nm.get(), r1.at.y_nm.get()),
            (origin.x + 15 * MM, origin.y + 10 * MM)
        );
        assert_eq!(r1.rotation.get(), 90_000_000);
        let j1 = panel
            .components
            .values()
            .find(|c| c.refdes == "J1")
            .unwrap();
        assert_eq!(j1.rotation.get(), 180_000_000);
        assert_eq!(plan.copies[0].width, 20 * MM);
    }

    #[test]
    fn a_mirrored_copy_is_the_board_seen_from_the_other_side() {
        let board = sample("one");
        let (settings, plan) = panel_of(&board, "columns = 1\nrows = 1\ncopy_mirror = true\n");
        let panel = compose(std::slice::from_ref(&board), &plan, &settings, 0, false).unwrap();
        let origin = plan.copies[0].origin;
        // the via at (5, 5) in a box 30 wide goes to (25, 5) and its layers change sides
        let via = panel.vias.values().next().unwrap();
        assert_eq!(via.at.x_nm.get(), origin.x + 25 * MM);
        assert_eq!(via.from_layer, id(2));
        assert_eq!(via.to_layer, id(1));
        // the track was on the top layer and is now on the bottom layer
        assert_eq!(panel.tracks.values().next().unwrap().layer, id(2));
        // the text is on the bottom legend, written back to front
        match panel.silkscreen.values().next().unwrap() {
            SilkItem::Text {
                layer, mirrored, ..
            } => {
                assert_eq!(*layer, id(6));
                assert!(*mirrored);
            }
            other => panic!("{other:?}"),
        }
        // the components are on the other side and the pads of their footprints on the other layers
        assert!(panel.components.values().all(|c| c.side == Side::Bottom));
        let smd = panel
            .footprints
            .values()
            .find(|f| f.name == "R0603")
            .unwrap();
        assert!(smd.pads.values().all(|p| p.layers == [id(2), id(4)]));
        // the zone was on the bottom layer and is now on the top one
        assert_eq!(panel.zones.values().next().unwrap().layer, id(1));
    }

    #[test]
    fn the_marks_are_parts_of_the_panel_only_when_asked_for() {
        let board = sample("one");
        let (settings, plan) =
            panel_of(&board, "columns = 2\nrows = 1\nlocal_fiducial_count = 2\n");
        let bare = compose(
            std::slice::from_ref(&board),
            &plan,
            &settings,
            50_000,
            false,
        )
        .unwrap();
        let full = compose(&[board], &plan, &settings, 50_000, true).unwrap();
        // 3 global fiducials, 4 tooling holes, 4 local fiducials on 2 boards, and the mouse bites
        let bites = plan
            .holes
            .iter()
            .filter(|h| h.kind == crate::layout::HoleKind::MouseBite)
            .count();
        assert_eq!(
            full.components.len(),
            bare.components.len() + 3 + 4 + 4 + bites
        );
        assert!(full.footprints.values().any(|f| f.name == "PANEL_FIDUCIAL"));
        let fiducial = full
            .footprints
            .values()
            .find(|f| f.name == "PANEL_FIDUCIAL")
            .unwrap();
        // the opening is 2 mm; the export adds 0.05 mm on each side, so the pad is 1.9 mm
        assert!(fiducial.pads.values().any(
            |p| matches!(p.shape, PadShape::Circle { diameter } if diameter.get() == 1_900_000)
        ));
        let hole = full
            .footprints
            .values()
            .find(|f| f.name == "PANEL_HOLE")
            .unwrap();
        assert!(hole
            .pads
            .values()
            .all(|p| p.layers.is_empty() && p.drill.is_some()));
    }

    #[test]
    fn boards_with_other_layers_are_refused() {
        let first = sample("one");
        let mut second = sample("two");
        let layer = second.layers.get_mut(&id(3)).unwrap();
        layer.name = "Other".to_owned();
        let settings = PanelSettings::parse("frame = none\n").unwrap();
        let shapes = [
            BoardShape::from_board(&first).unwrap().0,
            BoardShape::from_board(&second).unwrap().0,
        ];
        let items = crate::mixed::items_from_settings(2, &settings);
        let plan = crate::mixed::plan_items(&shapes, &items, &settings).unwrap();
        let e = compose(&[first.clone(), second], &plan, &settings, 0, false).unwrap_err();
        assert_eq!(e.code, LAYERS_DIFFER);
        let mut third = sample("three");
        third.copper_stack.pop();
        let e = compose(&[first, third], &plan, &settings, 0, false).unwrap_err();
        assert_eq!(e.code, LAYERS_DIFFER);
    }
}
