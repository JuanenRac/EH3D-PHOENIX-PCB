// SPDX-License-Identifier: GPL-3.0-or-later
//! The export of a board as a Specctra DSN file, the design file that external autorouters read.
//!
//! The text is a function of the board, the rules and the settings only: the same input always
//! gives the same bytes. Coordinates are integer nanometres (`(unit um)` with
//! `(resolution um 1000)`), so nothing is rounded on the way out. What is written:
//!
//! * the copper layers in the order of the stack, the board outline and the general width and
//!   clearance of the rules;
//! * one image per component, with its pads already in their place on the board (the side and the
//!   rotation of the component are applied here, so the file places every component on the front
//!   with no rotation), and one padstack for each distinct pad shape and set of layers;
//! * the nets with their pads, and one class per net class of the rules (and one for the nets
//!   that have none) with its width, clearance and via;
//! * the tracks, the vias and the copper zones that already exist, as fixed obstacles of their
//!   nets (a zone as a plane of its net).
//!
//! A cutout of a zone is written as a keep-out area on the layer of the zone: Specctra has no hole
//! in a plane, so the autorouter also keeps tracks and vias out of it, and the file says so in
//! its notes. A cutout of fewer than three points is left out and reported.
//!
//! Not written yet: tracks with arc pieces and vias that do not join the first and the last copper
//! layer; a board that has them is refused with a code.

use std::collections::{BTreeMap, BTreeSet};

use eh3d_phoenix_board::connectivity::pad_position;
use eh3d_phoenix_board::{Board, Id, PadRef, PadShape, Segment, Side};
use eh3d_phoenix_model::Point;

use crate::codes::{RouteError, DSN_EXPORT_FAILED};
use crate::live::RuleContext;
use crate::router::{copper_layers, track_width, via_size};
use crate::settings::RouteSettings;

/// A DSN file and what is in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dsn {
    /// The text of the file.
    pub text: String,
    /// The copper layers.
    pub layers: usize,
    /// The components (images).
    pub components: usize,
    /// The pads of all components.
    pub pads: usize,
    /// The nets that have at least one pad.
    pub nets: usize,
    /// The padstacks of pads and vias.
    pub padstacks: usize,
    /// The existing tracks written as fixed wires.
    pub tracks: usize,
    /// The existing vias written as fixed vias.
    pub vias: usize,
    /// The cutouts of zones, written as keep-out areas of the layer of their zone.
    pub zone_cutouts: usize,
    /// What the file cannot say or says in a simpler form, in English.
    pub notes: Vec<String>,
}

fn fail(what: impl Into<String>) -> RouteError {
    RouteError::new(DSN_EXPORT_FAILED, what)
}

/// A name that the file can hold: not empty, with no quote and no control character.
fn check_name(kind: &str, name: &str) -> Result<(), RouteError> {
    if name.is_empty() || name.chars().any(|c| c == '"' || c.is_control()) {
        return Err(fail(format!(
            "the {kind} {name:?} cannot be written: it is empty or has a quote or a control character"
        )));
    }
    Ok(())
}

/// A pin number or a reference designator, which also may not hold a space or a bracket (the
/// pin reference `R1-2` is not quoted).
fn check_pin_word(kind: &str, name: &str) -> Result<(), RouteError> {
    check_name(kind, name)?;
    if name
        .chars()
        .any(|c| c.is_whitespace() || c == '(' || c == ')')
    {
        return Err(fail(format!(
            "the {kind} {name:?} cannot be written: it has a space or a bracket"
        )));
    }
    Ok(())
}

/// The word as the file writes it: bare when it is made of letters, digits and `_ . + : /`,
/// otherwise in double quotes.
fn word(text: &str) -> String {
    let bare = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '+' | ':' | '/'));
    if bare {
        text.to_owned()
    } else {
        format!("\"{text}\"")
    }
}

/// Half of a length, rounded up to the next nanometre.
fn half_up(v: i64) -> i64 {
    (v + 1) / 2
}

/// Micro-degrees as decimal degrees, with the trailing zeros of the fraction left out.
pub(crate) fn degrees(udeg: i64) -> String {
    let whole = udeg / 1_000_000;
    let frac = udeg % 1_000_000;
    if frac == 0 {
        whole.to_string()
    } else {
        let text = format!("{whole}.{:06}", frac.abs());
        text.trim_end_matches('0').to_owned()
    }
}

/// The name of the padstack of a through via of this size.
pub(crate) fn via_name(diameter: i64, drill: i64) -> String {
    format!("Via_{diameter}_{drill}_nm")
}

fn push(out: &mut String, depth: usize, text: &str) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push_str(text);
    out.push('\n');
}

struct Stack {
    ids: Vec<Id>,
    names: Vec<String>,
}

impl Stack {
    fn index(&self, layer: Id) -> Option<usize> {
        self.ids.iter().position(|l| *l == layer)
    }
}

fn read_stack(board: &Board) -> Result<Stack, RouteError> {
    let ids = copper_layers(board);
    if ids.is_empty() {
        return Err(fail("the board has no copper layer"));
    }
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for id in &ids {
        let layer = board.layers.get(id).ok_or_else(|| {
            fail(format!(
                "the copper stack names the layer {id}, which is not there"
            ))
        })?;
        check_name("layer name", &layer.name)?;
        if !seen.insert(layer.name.clone()) {
            return Err(fail(format!(
                "two copper layers are named {:?}",
                layer.name
            )));
        }
        names.push(layer.name.clone());
    }
    Ok(Stack { ids, names })
}

/// The shape and layers of a padstack, with the text that sorts and names it.
struct PadstackSpec {
    shape: PadShape,
    layers: Vec<usize>,
}

fn padstack_key(spec: &PadstackSpec) -> String {
    let shape = match spec.shape {
        PadShape::Circle { diameter } => format!("c{}", diameter.get()),
        PadShape::Rect { width, height } => format!("r{}x{}", width.get(), height.get()),
        PadShape::Oblong { width, height } => format!("o{}x{}", width.get(), height.get()),
    };
    let layers: Vec<String> = spec.layers.iter().map(usize::to_string).collect();
    format!("{shape}|{}", layers.join(","))
}

fn shape_lines(shape: PadShape, layer: &str) -> String {
    let l = word(layer);
    match shape {
        PadShape::Circle { diameter } => format!("(shape (circle {l} {} 0 0))", diameter.get()),
        PadShape::Rect { width, height } => {
            let (hx, hy) = (half_up(width.get()), half_up(height.get()));
            format!("(shape (rect {l} {} {} {hx} {hy}))", -hx, -hy)
        }
        PadShape::Oblong { width, height } => {
            let (w, h) = (width.get(), height.get());
            if w == h {
                format!("(shape (circle {l} {w} 0 0))")
            } else if w > h {
                let e = half_up(w - h);
                format!("(shape (path {l} {h} {} 0 {e} 0))", -e)
            } else {
                let e = half_up(h - w);
                format!("(shape (path {l} {w} 0 {} 0 {e}))", -e)
            }
        }
    }
}

/// What the file says about one component.
struct Image {
    refdes: String,
    at: Point,
    pins: Vec<Pin>,
}

/// A pad of an image: its number, the key of its padstack, its place relative to the origin of
/// the component and the turn of its shape in micro-degrees when it has one.
struct Pin {
    number: String,
    key: String,
    x: i64,
    y: i64,
    turn: Option<i64>,
}

fn net_name(board: &Board, id: Id) -> Result<&str, RouteError> {
    board
        .nets
        .get(&id)
        .map(|n| n.name.as_str())
        .ok_or_else(|| fail(format!("the net {id} is not on the board")))
}

fn rel(p: Point, origin: Point) -> (i64, i64) {
    (
        p.x_nm.get() - origin.x_nm.get(),
        p.y_nm.get() - origin.y_nm.get(),
    )
}

fn polygon(points: &[Point]) -> String {
    let mut out = String::new();
    for p in points.iter().chain(points.first()) {
        out.push_str(&format!(" {} {}", p.x_nm.get(), p.y_nm.get()));
    }
    out
}

/// Writes the board as a Specctra DSN file.
pub fn export(
    board: &Board,
    ctx: &RuleContext,
    settings: &RouteSettings,
) -> Result<Dsn, RouteError> {
    let stack = read_stack(board)?;
    if board.outline.len() < 3 {
        return Err(fail("the board has no outline"));
    }
    check_name(
        "board name",
        if board.name.is_empty() {
            "board"
        } else {
            &board.name
        },
    )?;
    for net in board.nets.values() {
        check_name("net name", &net.name)?;
    }
    let mut net_names: BTreeSet<&str> = BTreeSet::new();
    for net in board.nets.values() {
        if !net_names.insert(net.name.as_str()) {
            return Err(fail(format!("two nets are named {:?}", net.name)));
        }
    }

    // The components, by reference designator.
    let mut order: Vec<(&str, Id)> = board
        .components
        .iter()
        .map(|(id, c)| (c.refdes.as_str(), *id))
        .collect();
    order.sort();
    let mut images: Vec<Image> = Vec::new();
    let mut padstacks: BTreeMap<String, PadstackSpec> = BTreeMap::new();
    let mut pad_count = 0;
    for (refdes, cid) in &order {
        check_pin_word("reference designator", refdes)?;
        if refdes.contains('-') {
            return Err(fail(format!(
                "the reference designator {refdes:?} has a hyphen, which the pin references use"
            )));
        }
        let component = &board.components[cid];
        let footprint = board.footprints.get(&component.footprint).ok_or_else(|| {
            fail(format!(
                "the component {refdes} has no footprint on the board"
            ))
        })?;
        let mut numbers: BTreeSet<&str> = BTreeSet::new();
        let mut image = Image {
            refdes: (*refdes).to_owned(),
            at: component.at,
            pins: Vec::new(),
        };
        for (pid, pad) in &footprint.pads {
            check_pin_word("pad number", &pad.number)?;
            if !numbers.insert(pad.number.as_str()) {
                return Err(fail(format!(
                    "the component {refdes} has two pads numbered {}",
                    pad.number
                )));
            }
            let pad_ref = PadRef {
                component: *cid,
                pad: *pid,
            };
            let at = pad_position(board, pad_ref)
                .ok_or_else(|| fail(format!("the pad {refdes}.{} has no place", pad.number)))?;
            let mut layers: Vec<usize> = if pad.drill.is_some() {
                (0..stack.ids.len()).collect()
            } else {
                pad.layers.iter().filter_map(|l| stack.index(*l)).collect()
            };
            layers.sort_unstable();
            layers.dedup();
            if layers.is_empty() {
                return Err(fail(format!(
                    "the pad {refdes}.{} has no copper layer of the stack",
                    pad.number
                )));
            }
            let spec = PadstackSpec {
                shape: pad.shape,
                layers,
            };
            let key = padstack_key(&spec);
            padstacks.entry(key.clone()).or_insert(spec);
            let turn = if component.side == Side::Bottom {
                component.rotation.get() - pad.rotation.get()
            } else {
                component.rotation.get() + pad.rotation.get()
            };
            let turn = turn.rem_euclid(360_000_000);
            let (x, y) = rel(at, component.at);
            image.pins.push(Pin {
                number: pad.number.clone(),
                key,
                x,
                y,
                turn: (turn != 0 && !matches!(pad.shape, PadShape::Circle { .. })).then_some(turn),
            });
            pad_count += 1;
        }
        images.push(image);
    }
    let pad_names: BTreeMap<String, String> = padstacks
        .keys()
        .enumerate()
        .map(|(i, k)| (k.clone(), format!("Pad_{}", i + 1)))
        .collect();

    // The nets with pads, and the pins of each.
    let mut pins_of: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for entry in &board.pad_nets {
        let component = board
            .components
            .get(&entry.component)
            .ok_or_else(|| fail(format!("a pad of the net {} has no component", entry.net)))?;
        let footprint = board.footprints.get(&component.footprint).ok_or_else(|| {
            fail(format!(
                "the component {} has no footprint",
                component.refdes
            ))
        })?;
        let pad = footprint.pads.get(&entry.pad).ok_or_else(|| {
            fail(format!(
                "a pad of {} is not in its footprint",
                component.refdes
            ))
        })?;
        pins_of
            .entry(net_name(board, entry.net)?)
            .or_default()
            .push(format!("{}-{}", component.refdes, pad.number));
    }
    for pins in pins_of.values_mut() {
        pins.sort();
    }

    // Widths, clearances and vias, per net.
    let clearance_of = |name: &str| -> i64 {
        let class_clearance = ctx
            .classes
            .class_of(name)
            .map_or(0, |c| c.min_clearance.get());
        ctx.rules
            .min_clearance
            .get()
            .max(class_clearance)
            .max(ctx.rules.connect_tolerance.get().saturating_add(1))
            .saturating_add(settings.clearance_margin_nm)
    };
    let mut vias: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut add_via = |drill: i64, diameter: i64| {
        vias.insert(via_name(diameter, drill), (drill, diameter));
    };
    let (d0, p0) = via_size(ctx, settings, "");
    add_via(d0, p0);
    for name in &net_names {
        let (d, p) = via_size(ctx, settings, name);
        add_via(d, p);
    }
    for via in board.vias.values() {
        add_via(via.drill.get(), via.diameter.get());
    }

    let mut out = String::new();
    push(
        &mut out,
        0,
        &format!(
            "(pcb {}",
            quoted(if board.name.is_empty() {
                "board"
            } else {
                &board.name
            })
        ),
    );
    push(&mut out, 1, "(parser");
    push(&mut out, 2, "(string_quote \")");
    push(&mut out, 2, "(space_in_quoted_tokens on)");
    push(&mut out, 2, "(host_cad \"EH3D-PHOENIX-PCB\")");
    push(&mut out, 1, ")");
    push(&mut out, 1, "(resolution um 1000)");
    push(&mut out, 1, "(unit um)");

    push(&mut out, 1, "(structure");
    for (i, name) in stack.names.iter().enumerate() {
        push(&mut out, 2, &format!("(layer {}", word(name)));
        push(&mut out, 3, "(type signal)");
        push(&mut out, 3, &format!("(property (index {i}))"));
        push(&mut out, 2, ")");
    }
    push(
        &mut out,
        2,
        &format!(
            "(boundary (path pcb 0{}))",
            polygon(&board.outline_polygon(&eh3d_phoenix_board::ArcSettings::current()))
        ),
    );
    // an opening inside the board is a keep-out on every layer
    for (i, cutout) in board
        .outline_cutouts
        .iter()
        .filter(|c| c.len() >= 3)
        .enumerate()
    {
        push(
            &mut out,
            2,
            &format!(
                "(keepout \"cutout{}\" (polygon pcb 0{}))",
                i + 1,
                polygon(cutout)
            ),
        );
    }
    for name in vias.keys() {
        push(&mut out, 2, &format!("(via {})", word(name)));
    }
    push(
        &mut out,
        2,
        &format!(
            "(rule (width {}) (clearance {}))",
            track_width(ctx, settings, ""),
            clearance_of("")
        ),
    );
    let mut zone_cutouts = 0usize;
    let mut left_out = 0usize;
    for zone in board.zones.values() {
        let layer = stack
            .index(zone.layer)
            .ok_or_else(|| fail("a zone is on a layer that is not copper"))?;
        if zone.outline.len() < 3 {
            return Err(fail("a zone has fewer than three points"));
        }
        push(
            &mut out,
            2,
            &format!(
                "(plane {} (polygon {} 0{}))",
                word(net_name(board, zone.net)?),
                word(&stack.names[layer]),
                polygon(&zone.outline)
            ),
        );
        for cutout in &zone.cutouts {
            if cutout.len() < 3 {
                left_out += 1;
                continue;
            }
            zone_cutouts += 1;
            push(
                &mut out,
                2,
                &format!(
                    "(keepout \"zone_cutout{}\" (polygon {} 0{}))",
                    zone_cutouts,
                    word(&stack.names[layer]),
                    polygon(cutout)
                ),
            );
        }
    }
    push(&mut out, 1, ")");

    push(&mut out, 1, "(placement");
    for image in &images {
        push(&mut out, 2, &format!("(component {}", word(&image.refdes)));
        push(
            &mut out,
            3,
            &format!(
                "(place {} {} {} front 0)",
                word(&image.refdes),
                image.at.x_nm.get(),
                image.at.y_nm.get()
            ),
        );
        push(&mut out, 2, ")");
    }
    push(&mut out, 1, ")");

    push(&mut out, 1, "(library");
    for image in &images {
        push(&mut out, 2, &format!("(image {}", word(&image.refdes)));
        for pin in &image.pins {
            let name = &pad_names[&pin.key];
            let rotate = pin
                .turn
                .map_or(String::new(), |t| format!(" (rotate {})", degrees(t)));
            push(
                &mut out,
                3,
                &format!(
                    "(pin {name}{rotate} {} {} {})",
                    word(&pin.number),
                    pin.x,
                    pin.y
                ),
            );
        }
        push(&mut out, 2, ")");
    }
    for (key, spec) in &padstacks {
        push(&mut out, 2, &format!("(padstack {}", pad_names[key]));
        for layer in &spec.layers {
            push(&mut out, 3, &shape_lines(spec.shape, &stack.names[*layer]));
        }
        push(&mut out, 3, "(attach off)");
        push(&mut out, 2, ")");
    }
    for (name, (_, diameter)) in &vias {
        push(&mut out, 2, &format!("(padstack {}", word(name)));
        for layer in &stack.names {
            push(
                &mut out,
                3,
                &format!("(shape (circle {} {diameter} 0 0))", word(layer)),
            );
        }
        push(&mut out, 3, "(attach off)");
        push(&mut out, 2, ")");
    }
    push(&mut out, 1, ")");

    push(&mut out, 1, "(network");
    let mut nets_written = 0;
    for (name, pins) in &pins_of {
        push(&mut out, 2, &format!("(net {}", word(name)));
        push(&mut out, 3, &format!("(pins {})", pins.join(" ")));
        push(&mut out, 2, ")");
        nets_written += 1;
    }
    // One class per net class and one for the nets that belong to none.
    let mut default_name = String::from("eh3d_default");
    while ctx.classes.classes().iter().any(|c| c.name == default_name) {
        default_name.push('_');
    }
    let mut classes: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for name in pins_of.keys() {
        let class = ctx
            .classes
            .class_of(name)
            .map_or_else(|| default_name.clone(), |c| c.name.clone());
        classes.entry(class).or_default().push(name);
    }
    for (class, members) in &classes {
        check_name("net class name", class)?;
        let first = members[0];
        let (drill, diameter) = via_size(ctx, settings, first);
        let listed: Vec<String> = members.iter().map(|m| word(m)).collect();
        push(
            &mut out,
            2,
            &format!("(class {} {}", word(class), listed.join(" ")),
        );
        push(
            &mut out,
            3,
            &format!("(circuit (use_via {}))", word(&via_name(diameter, drill))),
        );
        push(
            &mut out,
            3,
            &format!(
                "(rule (width {}) (clearance {}))",
                track_width(ctx, settings, first),
                clearance_of(first)
            ),
        );
        push(&mut out, 2, ")");
    }
    push(&mut out, 1, ")");

    push(&mut out, 1, "(wiring");
    for track in board.tracks.values() {
        let layer = stack
            .index(track.layer)
            .ok_or_else(|| fail("a track is on a layer that is not copper"))?;
        if track.path.is_empty() {
            return Err(fail("a track has no pieces"));
        }
        let mut vertices = String::new();
        for (i, segment) in track.path.iter().enumerate() {
            match segment {
                Segment::Line { start, end } => {
                    if i == 0 {
                        vertices.push_str(&format!(" {} {}", start.x_nm.get(), start.y_nm.get()));
                    }
                    vertices.push_str(&format!(" {} {}", end.x_nm.get(), end.y_nm.get()));
                }
                Segment::Arc { .. } => {
                    return Err(fail("a track has an arc piece, which is not written yet"))
                }
            }
        }
        push(
            &mut out,
            2,
            &format!(
                "(wire (path {} {}{vertices}) (net {}) (type fix))",
                word(&stack.names[layer]),
                track.width.get(),
                word(net_name(board, track.net)?)
            ),
        );
    }
    for via in board.vias.values() {
        let (from, to) = (stack.index(via.from_layer), stack.index(via.to_layer));
        let ends = (0, stack.ids.len() - 1);
        let through = matches!((from, to), (Some(a), Some(b)) if (a, b) == ends || (b, a) == ends);
        if !through {
            return Err(fail(
                "a via does not join the first and the last copper layer, which is not written yet",
            ));
        }
        push(
            &mut out,
            2,
            &format!(
                "(via {} {} {} (net {}) (type fix))",
                word(&via_name(via.diameter.get(), via.drill.get())),
                via.at.x_nm.get(),
                via.at.y_nm.get(),
                word(net_name(board, via.net)?)
            ),
        );
    }
    push(&mut out, 1, ")");
    push(&mut out, 0, ")");

    let mut notes = Vec::new();
    if zone_cutouts > 0 {
        notes.push(format!(
            "{zone_cutouts} cutout(s) of zones written as keep-out areas of their layer: Specctra has no hole in a plane, so tracks and vias are kept out of them too"
        ));
    }
    if left_out > 0 {
        notes.push(format!(
            "{left_out} cutout(s) of zones with fewer than three points left out"
        ));
    }
    Ok(Dsn {
        text: out,
        layers: stack.ids.len(),
        components: images.len(),
        pads: pad_count,
        nets: nets_written,
        padstacks: padstacks.len() + vias.len(),
        tracks: board.tracks.len(),
        vias: board.vias.len(),
        zone_cutouts,
        notes,
    })
}

fn quoted(text: &str) -> String {
    format!("\"{text}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees_drop_the_zeros_of_the_fraction() {
        assert_eq!(degrees(0), "0");
        assert_eq!(degrees(90_000_000), "90");
        assert_eq!(degrees(45_500_000), "45.5");
        assert_eq!(degrees(1), "0.000001");
        assert_eq!(degrees(359_999_999), "359.999999");
    }

    #[test]
    fn a_word_is_quoted_only_when_it_needs_to_be() {
        assert_eq!(word("GND"), "GND");
        assert_eq!(word("+3V3"), "+3V3");
        assert_eq!(word("Net-(R1-Pad1)"), "\"Net-(R1-Pad1)\"");
        assert_eq!(word("two words"), "\"two words\"");
        assert_eq!(word(""), "\"\"");
    }

    #[test]
    fn names_with_a_quote_or_a_control_character_are_refused() {
        assert!(check_name("net name", "A\"B").is_err());
        assert!(check_name("net name", "A\nB").is_err());
        assert!(check_name("net name", "").is_err());
        assert!(check_name("net name", "A B").is_ok());
        assert!(check_pin_word("pad number", "A B").is_err());
        assert!(check_pin_word("pad number", "(1)").is_err());
        assert!(check_pin_word("pad number", "A3").is_ok());
    }

    #[test]
    fn the_via_name_carries_its_size() {
        assert_eq!(via_name(600_000, 300_000), "Via_600000_300000_nm");
    }

    #[test]
    fn half_of_an_odd_length_goes_up() {
        assert_eq!(half_up(5), 3);
        assert_eq!(half_up(6), 3);
    }
}
