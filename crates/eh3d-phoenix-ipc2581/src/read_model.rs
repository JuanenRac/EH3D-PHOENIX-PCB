// SPDX-License-Identifier: GPL-3.0-or-later
//! What the reader of IPC-2581 makes of a file: the layers, the stack-up, and for each step the
//! outline, the holes, the parts, the nets and the copper, mask and legend geometry. Every length
//! is an integer number of nanometres; the unit of the file is converted once, exactly, when the
//! value is read (see [`crate::units`]).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::codes::Finding;
use crate::units::Unit;

/// What a layer of the file is for (the attribute `layerFunction`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerFunction {
    /// A signal layer (`SIGNAL`, `CONDUCTOR`, `MIXED`, `CONDFOIL`, `CONDFILM`).
    Signal,
    /// A plane (`PLANE`).
    Plane,
    /// Solder mask (`SOLDERMASK`, `COATINGNONCOND`).
    SolderMask,
    /// Paste mask or solder paste (`PASTEMASK`, `SOLDERPASTE`).
    Paste,
    /// Legend (`SILKSCREEN`, `LEGEND`).
    Silkscreen,
    /// A rigid core (`DIELCORE`, `DIELBASE`).
    DielCore,
    /// A prepreg or a bonding sheet (`DIELPREG`, `DIELADHV`, `DIELBONDPLY`, `DIELCOVERLAY`).
    DielPreg,
    /// Drill data (`DRILL`).
    Drill,
    /// The outline of the board (`BOARD_OUTLINE`, `BOARDOUTLINE`).
    Outline,
    /// A documentation layer (`DOCUMENT`, `ASSEMBLY`, `GRAPHIC`, `COURTYARD`, `FIXTURE`, ...).
    Document,
    /// Any other function, as written.
    Other(String),
}

impl LayerFunction {
    /// The function of a name written in a file.
    pub fn parse(text: &str) -> LayerFunction {
        match text.trim().to_ascii_uppercase().as_str() {
            "SIGNAL" | "CONDUCTOR" | "MIXED" | "CONDFOIL" | "CONDFILM" => LayerFunction::Signal,
            "PLANE" => LayerFunction::Plane,
            "SOLDERMASK" | "COATINGNONCOND" => LayerFunction::SolderMask,
            "PASTEMASK" | "SOLDERPASTE" => LayerFunction::Paste,
            "SILKSCREEN" | "LEGEND" => LayerFunction::Silkscreen,
            "DIELCORE" | "DIELBASE" => LayerFunction::DielCore,
            "DIELPREG" | "DIELADHV" | "DIELBONDPLY" | "DIELCOVERLAY" => LayerFunction::DielPreg,
            "DRILL" => LayerFunction::Drill,
            "BOARD_OUTLINE" | "BOARDOUTLINE" => LayerFunction::Outline,
            "DOCUMENT" | "ASSEMBLY" | "GRAPHIC" | "COURTYARD" | "FIXTURE" | "BOARDFAB" => {
                LayerFunction::Document
            }
            _ => LayerFunction::Other(text.trim().to_owned()),
        }
    }

    /// Whether the layer carries copper.
    pub fn is_copper(&self) -> bool {
        matches!(self, LayerFunction::Signal | LayerFunction::Plane)
    }

    /// Whether the layer is a dielectric of the stack-up.
    pub fn is_dielectric(&self) -> bool {
        matches!(self, LayerFunction::DielCore | LayerFunction::DielPreg)
    }

    /// A short word for the reports.
    pub fn word(&self) -> &str {
        match self {
            LayerFunction::Signal => "signal",
            LayerFunction::Plane => "plane",
            LayerFunction::SolderMask => "solder_mask",
            LayerFunction::Paste => "paste",
            LayerFunction::Silkscreen => "legend",
            LayerFunction::DielCore => "core",
            LayerFunction::DielPreg => "prepreg",
            LayerFunction::Drill => "drill",
            LayerFunction::Outline => "outline",
            LayerFunction::Document => "document",
            LayerFunction::Other(_) => "other",
        }
    }
}

/// The side of a layer (the attribute `side`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The top of the board.
    Top,
    /// The bottom of the board.
    Bottom,
    /// Inside the board (`INTERNAL`, `INNER`).
    Internal,
    /// Both sides (`BOTH`, `ALL`).
    Both,
    /// No side (`NONE`) or a value that is not known.
    None,
}

impl Side {
    /// The side of a name written in a file.
    pub fn parse(text: &str) -> Side {
        match text.trim().to_ascii_uppercase().as_str() {
            "TOP" => Side::Top,
            "BOTTOM" => Side::Bottom,
            "INTERNAL" | "INNER" => Side::Internal,
            "BOTH" | "ALL" => Side::Both,
            _ => Side::None,
        }
    }
}

/// A layer of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcLayer {
    /// The name the other elements refer to.
    pub name: String,
    /// What the layer is for.
    pub function: LayerFunction,
    /// The side.
    pub side: Side,
    /// Whether the polarity is `NEGATIVE` (a plane drawn as what is removed).
    pub negative: bool,
    /// The layers a drill layer goes from and to (`Span`), when given.
    pub span: Option<(String, String)>,
    /// The names of the specifications (`SpecRef`) of the layer.
    pub spec_refs: Vec<String>,
}

/// The properties a `Spec` of the header gives.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IpcSpec {
    /// The name the layers refer to.
    pub name: String,
    /// The material (`General type=MATERIAL`, property `text`).
    pub material: Option<String>,
    /// The dielectric constant (`DIELECTRIC_CONSTANT`).
    pub dk: Option<f64>,
    /// The loss tangent (`LOSS_TANGENT`).
    pub df: Option<f64>,
    /// The resin content in percent (`RESIN_CONTENT`).
    pub resin_percent: Option<f64>,
    /// The conductivity of a conductor in siemens per metre (`CONDUCTIVITY`, from `MHO/CM`).
    pub conductivity_s_per_m: Option<f64>,
    /// An impedance in ohms (`Impedance`).
    pub impedance_ohm: Option<f64>,
}

/// One layer of the stack-up, from the top to the bottom as the file orders them.
#[derive(Debug, Clone, PartialEq)]
pub struct StackLayer {
    /// The name of the layer (or of the group) it refers to.
    pub layer_ref: String,
    /// The thickness in nanometres.
    pub thickness_nm: Option<i64>,
    /// The tolerance above the thickness, in nanometres.
    pub tol_plus_nm: Option<i64>,
    /// The tolerance below the thickness, in nanometres.
    pub tol_minus_nm: Option<i64>,
    /// The position in the group (`sequence`).
    pub sequence: Option<u32>,
    /// The name of the group it is in.
    pub group: String,
    /// The material type written on the layer (`materialType`).
    pub material_type: Option<String>,
    /// The specification, resolved from `SpecRef`.
    pub spec: Option<IpcSpec>,
}

/// A stack-up of the file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IpcStackup {
    /// The name of the stack-up.
    pub name: String,
    /// The thickness of the board in nanometres.
    pub overall_thickness_nm: Option<i64>,
    /// The tolerance above, in nanometres.
    pub tol_plus_nm: Option<i64>,
    /// The tolerance below, in nanometres.
    pub tol_minus_nm: Option<i64>,
    /// Where the thickness is measured (`METAL`, `MASK`, `LAMINATE`, `OTHER`), as written.
    pub where_measured: String,
    /// The layers in the order of the file.
    pub layers: Vec<StackLayer>,
}

/// The plating of a hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plating {
    /// `PLATED`.
    Plated,
    /// `NONPLATED`.
    NonPlated,
    /// `VIA` or `VIA_CAPPED`.
    Via,
    /// Not written or not known.
    Unknown,
}

impl Plating {
    /// The plating of a name written in a file.
    pub fn parse(text: &str) -> Plating {
        match text.trim().to_ascii_uppercase().as_str() {
            "PLATED" => Plating::Plated,
            "NONPLATED" => Plating::NonPlated,
            "VIA" | "VIA_CAPPED" => Plating::Via,
            _ => Plating::Unknown,
        }
    }
}

/// A drilled hole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hole {
    /// The name of the hole.
    pub name: String,
    /// The finished diameter in nanometres.
    pub diameter_nm: i64,
    /// The plating.
    pub plating: Plating,
    /// The tolerance above, in nanometres.
    pub plus_tol_nm: i64,
    /// The tolerance below, in nanometres.
    pub minus_tol_nm: i64,
    /// The place.
    pub x_nm: i64,
    /// The place.
    pub y_nm: i64,
    /// The layer of the file the hole is on (the drill layer); empty for a hole of a padstack.
    pub layer: String,
}

/// A corner of a polygon or a polyline: a straight segment to the point, or an arc round a centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolyPoint {
    /// The place.
    pub x_nm: i64,
    /// The place.
    pub y_nm: i64,
    /// The centre of the arc that ends here, when the step is a curve.
    pub center_nm: Option<(i64, i64)>,
    /// Whether the arc runs clockwise.
    pub clockwise: bool,
}

/// The outline of the board with its cutouts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IpcOutline {
    /// The outer polygon; the first point is the start.
    pub outer: Vec<PolyPoint>,
    /// The cutouts.
    pub cutouts: Vec<Vec<PolyPoint>>,
}

/// A straight track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcLine {
    /// The net, empty when none.
    pub net: String,
    /// The start.
    pub from_nm: (i64, i64),
    /// The end.
    pub to_nm: (i64, i64),
    /// The width.
    pub width_nm: i64,
}

/// An arc of a track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcArc {
    /// The net, empty when none.
    pub net: String,
    /// The start.
    pub from_nm: (i64, i64),
    /// The end.
    pub to_nm: (i64, i64),
    /// The centre.
    pub center_nm: (i64, i64),
    /// Whether the arc runs clockwise.
    pub clockwise: bool,
    /// The width.
    pub width_nm: i64,
}

/// A polyline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcPolyline {
    /// The net, empty when none.
    pub net: String,
    /// The width.
    pub width_nm: i64,
    /// The points; the first is the start.
    pub points: Vec<PolyPoint>,
}

/// A region of copper (or of mask, or of legend): a contour with cutouts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcRegion {
    /// The net, empty when none.
    pub net: String,
    /// The outer polygon.
    pub outer: Vec<PolyPoint>,
    /// The cutouts.
    pub cutouts: Vec<Vec<PolyPoint>>,
}

/// What one layer of a step holds. The counts are always kept; the geometry only when the reader
/// is asked to keep it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerContent {
    /// The layer it is about.
    pub layer: String,
    /// The pads (`Pad`).
    pub pads: usize,
    /// The straight lines.
    pub lines: usize,
    /// The arcs.
    pub arcs: usize,
    /// The polylines.
    pub polylines: usize,
    /// The regions (`Contour`).
    pub regions: usize,
    /// The holes.
    pub holes: usize,
    /// The slots (`SlotCavity`).
    pub slots: usize,
    /// The groups of features placed with a mirror, a scale or a turn.
    pub placed_features: usize,
    /// The geometry, when kept: lines.
    pub line_items: Vec<IpcLine>,
    /// The geometry, when kept: arcs.
    pub arc_items: Vec<IpcArc>,
    /// The geometry, when kept: polylines.
    pub polyline_items: Vec<IpcPolyline>,
    /// The geometry, when kept: regions.
    pub region_items: Vec<IpcRegion>,
}

/// A part placed on the board.
#[derive(Debug, Clone, PartialEq)]
pub struct IpcComponent {
    /// The reference designator.
    pub ref_des: String,
    /// The package.
    pub package: String,
    /// The part (the key of the bill of materials).
    pub part: String,
    /// The layer it is placed on.
    pub layer: String,
    /// The way it is mounted, as written.
    pub mount: String,
    /// The place.
    pub x_nm: i64,
    /// The place.
    pub y_nm: i64,
    /// The rotation in degrees.
    pub rotation_deg: f64,
    /// Whether the part is mirrored.
    pub mirror: bool,
}

/// A package (footprint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcPackage {
    /// The name.
    pub name: String,
    /// The kind, as written.
    pub kind: String,
    /// The height in nanometres, when given.
    pub height_nm: Option<i64>,
    /// The number of pins.
    pub pins: usize,
}

/// A padstack definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcPadstack {
    /// The name.
    pub name: String,
    /// The holes.
    pub holes: Vec<Hole>,
    /// The layers the pads are on, with their use (`REGULAR`, `ANTIPAD`, `THERMAL`).
    pub pads: Vec<(String, String)>,
}

/// A net of the step, with the pins on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcNet {
    /// The name.
    pub name: String,
    /// The class, as written (`SIGNAL`, `GROUND`, `POWER`, ...).
    pub class: String,
    /// The pins on it as (reference designator, pin).
    pub pins: Vec<(String, String)>,
}

/// The physical route of a net: how many points it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcPhyNet {
    /// The name.
    pub name: String,
    /// The points (`PhyNetPoint`).
    pub points: usize,
}

/// A step: the board, a pallet or a chip.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IpcStep {
    /// The name.
    pub name: String,
    /// The origin of the step, when given.
    pub datum_nm: Option<(i64, i64)>,
    /// The outline and its cutouts.
    pub profile: Option<IpcOutline>,
    /// The padstack definitions.
    pub padstacks: Vec<IpcPadstack>,
    /// The holes of the layers (and of the padstacks of the older form of the file).
    pub holes: Vec<Hole>,
    /// The packages.
    pub packages: Vec<IpcPackage>,
    /// The placed parts.
    pub components: Vec<IpcComponent>,
    /// The nets with their pins.
    pub nets: Vec<IpcNet>,
    /// The physical nets.
    pub phy_nets: Vec<IpcPhyNet>,
    /// What each layer holds, in the order of the file.
    pub contents: Vec<LayerContent>,
    /// The zones of the stack-up (`StackupZone`) and the bend areas (`BendArea`).
    pub zones: usize,
}

/// A line of the bill of materials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcBomItem {
    /// The part (`OEMDesignNumberRef`).
    pub part: String,
    /// The quantity, when given.
    pub quantity: Option<u64>,
    /// The number of pins, when given.
    pub pin_count: Option<u64>,
    /// The category, as written (`ELECTRICAL`, `MECHANICAL`, `MATERIAL`, ...).
    pub category: String,
    /// The reference designators.
    pub ref_des: Vec<String>,
}

/// A bill of materials.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IpcBom {
    /// The name.
    pub name: String,
    /// The lines.
    pub items: Vec<IpcBomItem>,
}

/// A dictionary of the content (shapes, colours, line descriptions, ...), counted and not kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryInfo {
    /// The element (`DictionaryStandard`, ...).
    pub name: String,
    /// The unit it is written in, when it has one.
    pub unit: Option<Unit>,
    /// The elements in it.
    pub elements: usize,
}

/// A file read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IpcDoc {
    /// The revision (`A`, `B`, `C`), empty when the file does not say.
    pub revision: String,
    /// The modes of the content (`FULL`, `DESIGN`, `STACKUP`, ...).
    pub modes: Vec<String>,
    /// The unit of the lengths of the header; millimetres when the file does not say.
    pub units: Option<Unit>,
    /// The software that wrote the file, as the history record says.
    pub software: String,
    /// The dictionaries.
    pub dictionaries: Vec<DictionaryInfo>,
    /// The layers.
    pub layers: Vec<IpcLayer>,
    /// The specifications of the header.
    pub specs: Vec<IpcSpec>,
    /// The stack-ups.
    pub stackups: Vec<IpcStackup>,
    /// The steps.
    pub steps: Vec<IpcStep>,
    /// The bills of materials.
    pub boms: Vec<IpcBom>,
    /// The elements that were not read, with how many there were of each.
    pub skipped: BTreeMap<String, usize>,
    /// What the reader says about the file, in a fixed order.
    pub findings: Vec<Finding>,
}

/// The counts of a file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Counts {
    /// Layers of the file.
    pub layers: usize,
    /// Copper layers (signal and plane).
    pub copper_layers: usize,
    /// Layers of the stack-up of the first stack-up.
    pub stackup_layers: usize,
    /// Holes.
    pub holes: usize,
    /// Holes that are plated, vias included.
    pub holes_plated: usize,
    /// Holes that are not plated.
    pub holes_non_plated: usize,
    /// Slots.
    pub slots: usize,
    /// Placed parts.
    pub components: usize,
    /// Packages.
    pub packages: usize,
    /// Padstack definitions.
    pub padstacks: usize,
    /// Nets with their pins.
    pub nets: usize,
    /// Pins on the nets.
    pub net_pins: usize,
    /// Physical nets.
    pub phy_nets: usize,
    /// Pads.
    pub pads: usize,
    /// Lines.
    pub lines: usize,
    /// Arcs.
    pub arcs: usize,
    /// Polylines.
    pub polylines: usize,
    /// Regions.
    pub regions: usize,
    /// Lines of the bill of materials.
    pub bom_items: usize,
    /// Steps.
    pub steps: usize,
}

impl IpcDoc {
    /// The counts of the whole file.
    pub fn counts(&self) -> Counts {
        let mut c = Counts {
            layers: self.layers.len(),
            copper_layers: self
                .layers
                .iter()
                .filter(|l| l.function.is_copper())
                .count(),
            stackup_layers: self.stackups.first().map_or(0, |s| s.layers.len()),
            steps: self.steps.len(),
            ..Counts::default()
        };
        for step in &self.steps {
            c.holes += step.holes.len();
            for h in &step.holes {
                match h.plating {
                    Plating::Plated | Plating::Via => c.holes_plated += 1,
                    Plating::NonPlated => c.holes_non_plated += 1,
                    Plating::Unknown => {}
                }
            }
            c.components += step.components.len();
            c.packages += step.packages.len();
            c.padstacks += step.padstacks.len();
            c.nets += step.nets.len();
            c.net_pins += step.nets.iter().map(|n| n.pins.len()).sum::<usize>();
            c.phy_nets += step.phy_nets.len();
            for content in &step.contents {
                c.pads += content.pads;
                c.lines += content.lines;
                c.arcs += content.arcs;
                c.polylines += content.polylines;
                c.regions += content.regions;
                c.slots += content.slots;
            }
        }
        c.bom_items = self.boms.iter().map(|b| b.items.len()).sum();
        c
    }

    /// The layer with a name.
    pub fn layer(&self, name: &str) -> Option<&IpcLayer> {
        self.layers.iter().find(|l| l.name == name)
    }
}
