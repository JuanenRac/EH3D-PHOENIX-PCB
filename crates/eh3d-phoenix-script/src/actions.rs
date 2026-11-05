// SPDX-License-Identifier: GPL-3.0-or-later
//! What a command means.
//!
//! [`interpret`] turns a command into an [`Action`] with typed arguments, or says why it cannot.
//! The same function serves the run and the compatibility report, so the report (which never runs
//! anything) and the run always agree on which forms of a command are understood. It does not look
//! at a board: a name that does not exist, a layer that is missing or a number that does not fit
//! are found when the action is carried out.

use crate::codes::{
    ARGUMENT_INVALID, A_ENGINE, A_FILES, A_FONT, A_IGNORED, A_INCLUDE, A_SELECT, A_SILK, A_VIA,
    A_VIEW, C_CURVE, C_KNOWN_PENDING, C_LAYER_CREATE, C_MIRROR, C_PICK, C_SET_OPTION,
    NUMBER_INVALID, PENDING_FORM, UNKNOWN_COMMAND,
};
use crate::parse::{Arg, ArgKind, Command};
use crate::settings::ScriptSettings;
use crate::values::{Len, Rot, Unit};

/// How well a command is covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// The same meaning, with the same effect on the design.
    Equivalent,
    /// Carried out, with a difference that is named.
    Adapted,
    /// Not carried out, with a cause that is named.
    Pending,
}

impl Status {
    /// The word used in the JSON report.
    pub fn key(self) -> &'static str {
        match self {
            Status::Equivalent => "equivalent",
            Status::Adapted => "adapted",
            Status::Pending => "pending",
        }
    }
}

/// The reason for a status that is not `Equivalent`: a code of the registry and the case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// The code of the cause or of the difference.
    pub code: &'static str,
    /// What is specific to the case, in English.
    pub detail: String,
}

/// Why a command cannot be carried out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reject {
    /// The code: an unknown command, a bad argument, or the cause of a pending form.
    pub code: &'static str,
    /// What is specific to the case, in English.
    pub detail: String,
}

impl Reject {
    fn new(code: &'static str, detail: impl Into<String>) -> Reject {
        Reject {
            code,
            detail: detail.into(),
        }
    }
}

/// A coordinate as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pt {
    /// The horizontal coordinate.
    pub x: Len,
    /// The vertical coordinate.
    pub y: Len,
}

/// A layer as written: a number of the original workflow or a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerRef {
    /// A number, resolved through the layer table of the settings.
    Number(u32),
    /// A name, matched to a layer of the board without regard to case.
    Name(String),
}

/// One item of `DISPLAY`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayOp {
    /// Show every layer.
    All,
    /// Show no layer.
    None,
    /// Go back to the previous display.
    Last,
    /// Show a layer.
    Show(String),
    /// Hide a layer.
    Hide(String),
}

/// What `GRID` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridAction {
    /// Set the unit, the size and whether it is on.
    Set {
        /// The new unit.
        unit: Option<Unit>,
        /// The new size.
        size: Option<Len>,
        /// On or off.
        on: Option<bool>,
    },
    /// Go back to the previous grid.
    Last,
    /// Go back to the grid of the settings.
    Default,
    /// Lines or dots: how the grid is drawn.
    Style,
}

/// What `CHANGE` sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeWhat {
    /// The width of wires and strokes.
    Width(Len),
    /// The drill of vias.
    Drill(Len),
    /// The height of texts.
    Size(Len),
    /// The layer.
    Layer(LayerRef),
}

/// What `GROUP` selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupAction {
    /// Every object.
    All,
    /// Nothing.
    None,
    /// The named components and nets.
    Names(Vec<String>),
}

/// A command with typed arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// `GRID`.
    Grid(GridAction),
    /// `LAYER` with one layer: the layer new objects go to.
    Layer(LayerRef),
    /// `DISPLAY`.
    Display(Vec<DisplayOp>),
    /// `CHANGE`: the default for what is drawn next and, for the named objects or the selection,
    /// the value of what exists.
    Change {
        /// What is set.
        what: ChangeWhat,
        /// The named objects, empty for the selection.
        names: Vec<String>,
    },
    /// `WIRE`.
    Wire {
        /// The signal.
        net: Option<String>,
        /// The width.
        width: Option<Len>,
        /// A line style was given.
        style: bool,
        /// The points.
        points: Vec<Pt>,
    },
    /// `VIA`.
    Via {
        /// The signal.
        net: Option<String>,
        /// The diameter.
        diameter: Option<Len>,
        /// A shape was given.
        shape: bool,
        /// Where.
        at: Pt,
    },
    /// `CIRCLE`.
    Circle {
        /// The width of the line.
        width: Option<Len>,
        /// The centre.
        center: Pt,
        /// A point of the circle.
        through: Pt,
    },
    /// `RECT`.
    Rect {
        /// One corner.
        a: Pt,
        /// The opposite corner.
        b: Pt,
    },
    /// `TEXT`.
    Text {
        /// The characters.
        text: String,
        /// The rotation.
        rot: Option<Rot>,
        /// Where.
        at: Pt,
    },
    /// `MOVE` a component to a place.
    Move {
        /// The reference designator.
        name: String,
        /// Where.
        to: Pt,
    },
    /// `ROTATE`.
    Rotate {
        /// The rotation.
        rot: Rot,
        /// The named components, empty for the selection.
        names: Vec<String>,
    },
    /// `DELETE`.
    Delete {
        /// The named components, empty for the selection.
        names: Vec<String>,
    },
    /// `NAME`.
    Name {
        /// The present name.
        old: String,
        /// The new name.
        new: String,
    },
    /// `VALUE`.
    Value {
        /// The component.
        name: String,
        /// The new value.
        value: String,
    },
    /// `SET` of an option that only changes the original interface.
    Set {
        /// The option in capitals.
        option: String,
    },
    /// `GROUP`.
    Group(GroupAction),
    /// `RATSNEST`.
    Ratsnest,
    /// `DRC`.
    Drc,
    /// `ERC`.
    Erc,
    /// `WRITE`.
    Write(String),
    /// `OPEN`.
    Open(String),
    /// `SCRIPT`.
    Script(String),
}

const WIRE_STYLES: [&str; 8] = [
    "ROUND",
    "MITER",
    "LONG",
    "FLAT",
    "CONTINUOUS",
    "LONGDASH",
    "SHORTDASH",
    "DASHDOT",
];
const VIA_SHAPES: [&str; 5] = ["ROUND", "SQUARE", "OCTAGON", "LONG", "OFFSET"];

fn is_one_of(word: &str, list: &[&str]) -> bool {
    list.iter().any(|w| w.eq_ignore_ascii_case(word))
}

fn bad(detail: &str) -> Reject {
    Reject::new(ARGUMENT_INVALID, detail)
}

fn point(arg: &Arg) -> Result<Option<Pt>, Reject> {
    let ArgKind::Point(x, y) = &arg.kind else {
        return Ok(None);
    };
    match (Len::parse(x), Len::parse(y)) {
        (Some(x), Some(y)) => Ok(Some(Pt { x, y })),
        _ => Err(Reject::new(
            NUMBER_INVALID,
            format!("the coordinate ({x} {y}) is not two lengths"),
        )),
    }
}

fn need_point(arg: &Arg) -> Result<Pt, Reject> {
    point(arg)?.ok_or_else(|| bad("a coordinate in parentheses was expected"))
}

fn layer_ref(arg: &Arg) -> Result<LayerRef, Reject> {
    match &arg.kind {
        ArgKind::Word(w) if !w.is_empty() && w.bytes().all(|b| b.is_ascii_digit()) => w
            .parse::<u32>()
            .map(LayerRef::Number)
            .map_err(|_| Reject::new(NUMBER_INVALID, format!("layer number {w}"))),
        ArgKind::Word(w) | ArgKind::Text(w) => Ok(LayerRef::Name(w.clone())),
        ArgKind::Point(_, _) => Err(bad("a layer was expected")),
    }
}

/// The names in the arguments; a coordinate means selecting by pointing.
fn names(args: &[Arg]) -> Result<Vec<String>, Reject> {
    let mut out = Vec::new();
    for arg in args {
        match &arg.kind {
            ArgKind::Word(w) | ArgKind::Text(w) => out.push(w.clone()),
            ArgKind::Point(_, _) => {
                return Err(Reject::new(
                    C_PICK,
                    "the object is chosen by a coordinate".to_owned(),
                ))
            }
        }
    }
    Ok(out)
}

fn no_args(cmd: &Command) -> Result<(), Reject> {
    if cmd.args.is_empty() {
        Ok(())
    } else {
        Err(bad("this command takes no arguments"))
    }
}

fn one_text(cmd: &Command) -> Result<String, Reject> {
    match cmd.args.as_slice() {
        [a] => a
            .text()
            .map(str::to_owned)
            .ok_or_else(|| bad("a file name was expected")),
        _ => Err(bad("exactly one file name was expected")),
    }
}

fn len_arg(arg: Option<&Arg>) -> Result<Len, Reject> {
    arg.and_then(Arg::text)
        .and_then(Len::parse)
        .ok_or_else(|| bad("a length was expected"))
}

fn grid(cmd: &Command) -> Result<Action, Reject> {
    let (mut unit, mut size, mut on) = (None, None, None);
    let (mut style, mut last, mut default) = (false, false, false);
    for arg in &cmd.args {
        let Some(word) = arg.text() else {
            return Err(bad("GRID takes words and numbers, not coordinates"));
        };
        if let Some(u) = Unit::from_word(word) {
            unit = Some(u);
            continue;
        }
        match word.to_ascii_lowercase().as_str() {
            "on" => on = Some(true),
            "off" => on = Some(false),
            "last" => last = true,
            "default" => default = true,
            "lines" | "dots" => style = true,
            "finest" | "alt" | "multiple" => {
                return Err(Reject::new(
                    PENDING_FORM,
                    format!("GRID {}", word.to_ascii_uppercase()),
                ))
            }
            _ => match Len::parse(word) {
                Some(l) if size.is_none() => size = Some(l),
                _ => return Err(bad("a unit, a size, on or off was expected")),
            },
        }
    }
    let nothing_else = unit.is_none() && size.is_none() && on.is_none();
    if last && nothing_else && !default && !style {
        return Ok(Action::Grid(GridAction::Last));
    }
    if default && nothing_else && !last && !style {
        return Ok(Action::Grid(GridAction::Default));
    }
    if style && nothing_else && !last && !default {
        return Ok(Action::Grid(GridAction::Style));
    }
    if last || default || nothing_else {
        return Err(bad("GRID needs a unit, a size or on or off"));
    }
    Ok(Action::Grid(GridAction::Set { unit, size, on }))
}

fn layer(cmd: &Command) -> Result<Action, Reject> {
    match cmd.args.as_slice() {
        [a] => Ok(Action::Layer(layer_ref(a)?)),
        [a, b] if matches!(layer_ref(a), Ok(LayerRef::Number(_))) && b.text().is_some() => {
            Err(Reject::new(C_LAYER_CREATE, "LAYER number name"))
        }
        _ => Err(bad("one layer was expected")),
    }
}

fn display(cmd: &Command) -> Result<Action, Reject> {
    if cmd.args.is_empty() {
        return Err(bad("DISPLAY needs ALL, NONE, LAST or layers"));
    }
    let mut ops = Vec::new();
    for arg in &cmd.args {
        let Some(word) = arg.text() else {
            return Err(bad("a layer was expected"));
        };
        let op = match word.to_ascii_lowercase().as_str() {
            "all" => DisplayOp::All,
            "none" => DisplayOp::None,
            "last" => DisplayOp::Last,
            _ => {
                if let Some(rest) = word.strip_prefix('-') {
                    DisplayOp::Hide(rest.to_owned())
                } else {
                    DisplayOp::Show(word.strip_prefix('+').unwrap_or(word).to_owned())
                }
            }
        };
        match &op {
            DisplayOp::Show(n) | DisplayOp::Hide(n) if n.is_empty() => {
                return Err(bad("a layer was expected after the sign"))
            }
            _ => {}
        }
        ops.push(op);
    }
    Ok(Action::Display(ops))
}

fn change(cmd: &Command) -> Result<Action, Reject> {
    let Some(first) = cmd.args.first() else {
        return Err(bad("CHANGE needs an option"));
    };
    let Some(option) = first.text() else {
        return Err(bad("CHANGE needs an option"));
    };
    let option = option.to_ascii_uppercase();
    let (what, rest) = match option.as_str() {
        "WIDTH" => (
            ChangeWhat::Width(len_arg(cmd.args.get(1))?),
            cmd.args.get(2..).unwrap_or(&[]),
        ),
        "DRILL" => (
            ChangeWhat::Drill(len_arg(cmd.args.get(1))?),
            cmd.args.get(2..).unwrap_or(&[]),
        ),
        "SIZE" => (
            ChangeWhat::Size(len_arg(cmd.args.get(1))?),
            cmd.args.get(2..).unwrap_or(&[]),
        ),
        "LAYER" => {
            let layer = cmd
                .args
                .get(1)
                .ok_or_else(|| bad("a layer was expected"))
                .and_then(layer_ref)?;
            (ChangeWhat::Layer(layer), cmd.args.get(2..).unwrap_or(&[]))
        }
        _ => return Err(Reject::new(PENDING_FORM, format!("CHANGE {option}"))),
    };
    Ok(Action::Change {
        what,
        names: names(rest)?,
    })
}

fn wire(cmd: &Command) -> Result<Action, Reject> {
    let (mut net, mut width, mut style) = (None, None, false);
    let mut points = Vec::new();
    for arg in &cmd.args {
        match &arg.kind {
            ArgKind::Point(_, _) => points.push(need_point(arg)?),
            ArgKind::Text(t) => {
                if points.is_empty() && net.is_none() {
                    net = Some(t.clone());
                } else {
                    return Err(bad("the signal name comes once, before the points"));
                }
            }
            ArgKind::Word(w) if points.is_empty() => {
                if width.is_none() && Len::is_number(w) {
                    width = Len::parse(w);
                } else if is_one_of(w, &WIRE_STYLES) {
                    style = true;
                } else if net.is_none() && !Len::is_number(w) {
                    net = Some(w.clone());
                } else {
                    return Err(bad("an argument of WIRE is not understood"));
                }
            }
            ArgKind::Word(w) => {
                if Len::is_number(w) || w.starts_with('@') {
                    return Err(Reject::new(C_CURVE, "a bend between two points"));
                }
                return Err(bad("an argument of WIRE is not understood"));
            }
        }
    }
    if points.len() < 2 {
        return Err(bad("WIRE needs at least two points"));
    }
    Ok(Action::Wire {
        net,
        width,
        style,
        points,
    })
}

fn via(cmd: &Command) -> Result<Action, Reject> {
    let (mut net, mut diameter, mut shape, mut at) = (None, None, false, None);
    for arg in &cmd.args {
        match &arg.kind {
            ArgKind::Point(_, _) => {
                if at.is_some() {
                    return Err(bad("VIA takes one coordinate"));
                }
                at = Some(need_point(arg)?);
            }
            ArgKind::Text(t) => {
                if net.is_some() {
                    return Err(bad("the signal name comes once"));
                }
                net = Some(t.clone());
            }
            ArgKind::Word(w) => {
                if diameter.is_none() && Len::is_number(w) {
                    diameter = Len::parse(w);
                } else if is_one_of(w, &VIA_SHAPES) {
                    shape = true;
                } else if net.is_none() && !Len::is_number(w) {
                    net = Some(w.clone());
                } else {
                    return Err(bad("an argument of VIA is not understood"));
                }
            }
        }
    }
    let at = at.ok_or_else(|| bad("VIA needs a coordinate"))?;
    Ok(Action::Via {
        net,
        diameter,
        shape,
        at,
    })
}

fn circle(cmd: &Command) -> Result<Action, Reject> {
    let mut width = None;
    let mut points = Vec::new();
    for arg in &cmd.args {
        match &arg.kind {
            ArgKind::Point(_, _) => points.push(need_point(arg)?),
            ArgKind::Word(w) if points.is_empty() && width.is_none() => {
                width = Some(Len::parse(w).ok_or_else(|| bad("a width was expected"))?);
            }
            _ => return Err(bad("an argument of CIRCLE is not understood")),
        }
    }
    match points.as_slice() {
        [center, through] => Ok(Action::Circle {
            width,
            center: *center,
            through: *through,
        }),
        _ => Err(bad("CIRCLE needs the centre and a point of the circle")),
    }
}

fn rect(cmd: &Command) -> Result<Action, Reject> {
    match cmd.args.as_slice() {
        [a, b] => Ok(Action::Rect {
            a: need_point(a)?,
            b: need_point(b)?,
        }),
        _ => Err(bad("RECT needs two opposite corners")),
    }
}

fn text(cmd: &Command) -> Result<Action, Reject> {
    let mut args = cmd.args.iter();
    let content = match args.next().map(|a| &a.kind) {
        Some(ArgKind::Text(t)) => t.clone(),
        _ => return Err(bad("TEXT needs a text in quotes")),
    };
    let (mut rot, mut at) = (None, None);
    for arg in args {
        match &arg.kind {
            ArgKind::Point(_, _) if at.is_none() => at = Some(need_point(arg)?),
            ArgKind::Word(w) if rot.is_none() && at.is_none() => {
                rot = Some(Rot::parse(w).ok_or_else(|| bad("a rotation like R90 was expected"))?);
            }
            _ => return Err(bad("an argument of TEXT is not understood")),
        }
    }
    let at = at.ok_or_else(|| bad("TEXT needs a coordinate"))?;
    Ok(Action::Text {
        text: content,
        rot,
        at,
    })
}

fn move_(cmd: &Command) -> Result<Action, Reject> {
    match cmd.args.as_slice() {
        [first, ..] if matches!(first.kind, ArgKind::Point(_, _)) => Err(Reject::new(
            C_PICK,
            "the object is chosen by a coordinate".to_owned(),
        )),
        [name, to] => Ok(Action::Move {
            name: name
                .text()
                .ok_or_else(|| bad("a name was expected"))?
                .to_owned(),
            to: need_point(to)?,
        }),
        _ => Err(bad("MOVE needs a name and a coordinate")),
    }
}

fn rotate(cmd: &Command) -> Result<Action, Reject> {
    let Some(first) = cmd.args.first() else {
        return Err(bad("ROTATE needs a rotation like R90"));
    };
    if matches!(first.kind, ArgKind::Point(_, _)) {
        return Err(Reject::new(C_PICK, "the object is chosen by a coordinate"));
    }
    let rot = first
        .text()
        .and_then(Rot::parse)
        .ok_or_else(|| bad("a rotation like R90 was expected"))?;
    let names = names(&cmd.args[1..])?;
    if rot.mirror {
        return Err(Reject::new(C_MIRROR, "ROTATE with M"));
    }
    Ok(Action::Rotate { rot, names })
}

fn pair(cmd: &Command, what: &str) -> Result<(String, String), Reject> {
    match cmd.args.as_slice() {
        [a, b] => match (a.text(), b.text()) {
            (Some(a), Some(b)) => Ok((a.to_owned(), b.to_owned())),
            _ => Err(bad(what)),
        },
        _ => Err(bad(what)),
    }
}

fn set(cmd: &Command, settings: &ScriptSettings) -> Result<Action, Reject> {
    let option = cmd
        .args
        .first()
        .and_then(Arg::text)
        .ok_or_else(|| bad("SET needs an option"))?
        .to_ascii_uppercase();
    if settings.is_ignored_option(&option) {
        Ok(Action::Set { option })
    } else {
        Err(Reject::new(C_SET_OPTION, format!("SET {option}")))
    }
}

fn group(cmd: &Command) -> Result<Action, Reject> {
    if cmd.args.is_empty() {
        return Err(bad("GROUP needs ALL, NONE or names"));
    }
    let list = names(&cmd.args)?;
    if let [only] = list.as_slice() {
        if only.eq_ignore_ascii_case("all") {
            return Ok(Action::Group(GroupAction::All));
        }
        if only.eq_ignore_ascii_case("none") {
            return Ok(Action::Group(GroupAction::None));
        }
    }
    Ok(Action::Group(GroupAction::Names(list)))
}

/// Understands a command, or says why it cannot be carried out.
pub fn interpret(cmd: &Command, settings: &ScriptSettings) -> Result<Action, Reject> {
    match cmd.name.as_str() {
        "GRID" => grid(cmd),
        "LAYER" => layer(cmd),
        "DISPLAY" => display(cmd),
        "CHANGE" => change(cmd),
        "WIRE" => wire(cmd),
        "VIA" => via(cmd),
        "CIRCLE" => circle(cmd),
        "RECT" => rect(cmd),
        "TEXT" => text(cmd),
        "MOVE" => move_(cmd),
        "ROTATE" => rotate(cmd),
        "DELETE" => Ok(Action::Delete {
            names: names(&cmd.args)?,
        }),
        "NAME" => {
            let (old, new) = pair(cmd, "NAME needs the present name and the new one")?;
            Ok(Action::Name { old, new })
        }
        "VALUE" => {
            let (name, value) = pair(cmd, "VALUE needs a component and a value")?;
            Ok(Action::Value { name, value })
        }
        "SET" => set(cmd, settings),
        "GROUP" => group(cmd),
        "RATSNEST" => no_args(cmd).map(|_| Action::Ratsnest),
        "DRC" => no_args(cmd).map(|_| Action::Drc),
        "ERC" => no_args(cmd).map(|_| Action::Erc),
        "WRITE" => one_text(cmd).map(Action::Write),
        "OPEN" => one_text(cmd).map(Action::Open),
        "SCRIPT" => one_text(cmd).map(Action::Script),
        other => {
            if settings.is_known_pending(other) {
                Err(Reject::new(C_KNOWN_PENDING, other))
            } else {
                Err(Reject::new(UNKNOWN_COMMAND, other))
            }
        }
    }
}

fn adapted(code: &'static str, detail: &str) -> (Status, Option<Note>) {
    (
        Status::Adapted,
        Some(Note {
            code,
            detail: detail.to_owned(),
        }),
    )
}

impl Action {
    /// How well the command is covered, and the difference when it is not the same.
    pub fn status(&self) -> (Status, Option<Note>) {
        match self {
            Action::Grid(GridAction::Style) => {
                adapted(A_VIEW, "the grid is drawn as lines or dots")
            }
            Action::Display(_) => adapted(A_VIEW, "the display of layers"),
            Action::Wire { style: true, .. } => adapted(A_IGNORED, "the line style is ignored"),
            Action::Via { shape: true, .. } => adapted(A_VIA, "the shape of the via"),
            Action::Circle { .. } => {
                adapted(A_SILK, "a circle, as two arcs, on a silkscreen layer")
            }
            Action::Rect { .. } => {
                adapted(A_SILK, "a rectangle, as its outline, on a silkscreen layer")
            }
            Action::Text { .. } => adapted(A_FONT, "the letters differ from the original font"),
            Action::Set { option } => adapted(A_IGNORED, &format!("SET {option}")),
            Action::Group(_) => adapted(A_SELECT, "GROUP by names"),
            Action::Ratsnest => adapted(A_ENGINE, "the ratsnest of this tool"),
            Action::Drc => adapted(
                A_ENGINE,
                "the rule check of this tool, with the rules given to the run",
            ),
            Action::Erc => adapted(
                A_ENGINE,
                "the electrical rule check of this tool, on a schematic",
            ),
            Action::Write(_) => adapted(A_FILES, "WRITE writes a board as a JSON file"),
            Action::Open(_) => adapted(A_FILES, "OPEN reads a board from a JSON file"),
            Action::Script(_) => adapted(A_INCLUDE, "SCRIPT"),
            _ => (Status::Equivalent, None),
        }
    }
}

/// The status of a command without carrying it out: `interpret` and then [`Action::status`]; a
/// command that cannot be understood is pending with its cause.
pub fn classify(cmd: &Command, settings: &ScriptSettings) -> (Status, Option<Note>) {
    match interpret(cmd, settings) {
        Ok(action) => action.status(),
        Err(reject) => (
            Status::Pending,
            Some(Note {
                code: reject.code,
                detail: reject.detail,
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_script;

    fn first(text: &str) -> Command {
        let s = ScriptSettings::default();
        let p = parse_script(text, &s);
        assert!(p.issues.is_empty(), "{text}: {:?}", p.issues);
        p.commands[0].clone()
    }

    fn status_of(text: &str) -> (Status, Option<&'static str>) {
        let (status, note) = classify(&first(text), &ScriptSettings::default());
        (status, note.map(|n| n.code))
    }

    #[test]
    fn grid_forms() {
        assert_eq!(
            interpret(&first("GRID mm 0.5 on"), &ScriptSettings::default()),
            Ok(Action::Grid(GridAction::Set {
                unit: Some(Unit::Mm),
                size: Len::parse("0.5"),
                on: Some(true)
            }))
        );
        assert_eq!(status_of("GRID LAST"), (Status::Equivalent, None));
        assert_eq!(status_of("GRID default"), (Status::Equivalent, None));
        assert_eq!(status_of("GRID inch 0.05"), (Status::Equivalent, None));
        assert_eq!(status_of("GRID 50mil"), (Status::Equivalent, None));
        assert_eq!(status_of("GRID lines"), (Status::Adapted, Some(A_VIEW)));
        assert_eq!(
            status_of("GRID alt mm"),
            (Status::Pending, Some(PENDING_FORM))
        );
        assert_eq!(status_of("GRID"), (Status::Pending, Some(ARGUMENT_INVALID)));
        assert_eq!(
            status_of("GRID last mm"),
            (Status::Pending, Some(ARGUMENT_INVALID))
        );
        assert_eq!(
            status_of("GRID (1 1)"),
            (Status::Pending, Some(ARGUMENT_INVALID))
        );
    }

    #[test]
    fn the_forms_of_each_command_have_their_status() {
        for (text, status, code) in [
            ("LAYER Top", Status::Equivalent, None),
            ("LAYER 21", Status::Equivalent, None),
            ("LAYER 100 Mine", Status::Pending, Some(C_LAYER_CREATE)),
            ("LAYER", Status::Pending, Some(ARGUMENT_INVALID)),
            ("DISPLAY ALL", Status::Adapted, Some(A_VIEW)),
            (
                "DISPLAY NONE Top -Bottom +tPlace",
                Status::Adapted,
                Some(A_VIEW),
            ),
            ("DISPLAY", Status::Pending, Some(ARGUMENT_INVALID)),
            ("CHANGE WIDTH 0.3", Status::Equivalent, None),
            ("CHANGE DRILL 0.4mm 'GND'", Status::Equivalent, None),
            ("CHANGE SIZE 1", Status::Equivalent, None),
            ("CHANGE LAYER Bottom", Status::Equivalent, None),
            ("CHANGE WIDTH 0.3 (1 1)", Status::Pending, Some(C_PICK)),
            ("CHANGE FONT vector", Status::Pending, Some(PENDING_FORM)),
            ("CHANGE WIDTH", Status::Pending, Some(ARGUMENT_INVALID)),
            ("WIRE 'GND' 0.3 (0 0) (1 1) (1 2)", Status::Equivalent, None),
            ("WIRE GND (0 0) (1 1)", Status::Equivalent, None),
            ("WIRE ROUND (0 0) (1 1)", Status::Adapted, Some(A_IGNORED)),
            ("WIRE (0 0) (1 1)", Status::Equivalent, None),
            ("WIRE (0 0)", Status::Pending, Some(ARGUMENT_INVALID)),
            ("WIRE (0 0) -90 (1 1)", Status::Pending, Some(C_CURVE)),
            ("WIRE (0 0) @ (1 1)", Status::Pending, Some(C_CURVE)),
            ("VIA 'GND' (5 5)", Status::Equivalent, None),
            ("VIA 'GND' 0.8 (5 5)", Status::Equivalent, None),
            ("VIA 'GND' round (5 5)", Status::Adapted, Some(A_VIA)),
            ("VIA 'GND'", Status::Pending, Some(ARGUMENT_INVALID)),
            ("CIRCLE (0 0) (1 0)", Status::Adapted, Some(A_SILK)),
            ("CIRCLE 0.2 (0 0) (1 0)", Status::Adapted, Some(A_SILK)),
            ("CIRCLE (0 0)", Status::Pending, Some(ARGUMENT_INVALID)),
            ("RECT (0 0) (4 3)", Status::Adapted, Some(A_SILK)),
            ("RECT (0 0)", Status::Pending, Some(ARGUMENT_INVALID)),
            ("TEXT 'hi' R90 (1 1)", Status::Adapted, Some(A_FONT)),
            ("TEXT 'hi' (1 1)", Status::Adapted, Some(A_FONT)),
            ("TEXT hi (1 1)", Status::Pending, Some(ARGUMENT_INVALID)),
            ("MOVE R1 (10 10)", Status::Equivalent, None),
            ("MOVE (10 10) (12 12)", Status::Pending, Some(C_PICK)),
            ("MOVE R1", Status::Pending, Some(ARGUMENT_INVALID)),
            ("ROTATE R90 R1 R2", Status::Equivalent, None),
            ("ROTATE =R45", Status::Equivalent, None),
            ("ROTATE MR90 R1", Status::Pending, Some(C_MIRROR)),
            ("ROTATE R90 (1 1)", Status::Pending, Some(C_PICK)),
            ("ROTATE 90 R1", Status::Pending, Some(ARGUMENT_INVALID)),
            ("DELETE R1", Status::Equivalent, None),
            ("DELETE", Status::Equivalent, None),
            ("DELETE (1 1)", Status::Pending, Some(C_PICK)),
            ("NAME R1 R10", Status::Equivalent, None),
            ("NAME R1", Status::Pending, Some(ARGUMENT_INVALID)),
            ("VALUE R1 '10k'", Status::Equivalent, None),
            ("SET CONFIRM YES", Status::Adapted, Some(A_IGNORED)),
            ("SET WIRE_BEND 2", Status::Adapted, Some(A_IGNORED)),
            ("SET PICK_AREA 3", Status::Pending, Some(C_SET_OPTION)),
            ("SET", Status::Pending, Some(ARGUMENT_INVALID)),
            ("GROUP R1 R2", Status::Adapted, Some(A_SELECT)),
            ("GROUP ALL", Status::Adapted, Some(A_SELECT)),
            ("GROUP (0 0) (9 9)", Status::Pending, Some(C_PICK)),
            ("RATSNEST", Status::Adapted, Some(A_ENGINE)),
            ("DRC", Status::Adapted, Some(A_ENGINE)),
            ("ERC", Status::Adapted, Some(A_ENGINE)),
            ("DRC 1", Status::Pending, Some(ARGUMENT_INVALID)),
            ("WRITE 'out/a.json'", Status::Adapted, Some(A_FILES)),
            ("OPEN 'a.json'", Status::Adapted, Some(A_FILES)),
            ("OPEN", Status::Pending, Some(ARGUMENT_INVALID)),
            ("SCRIPT 'more.scr'", Status::Adapted, Some(A_INCLUDE)),
            ("ROUTE", Status::Pending, Some(C_KNOWN_PENDING)),
            ("route GND", Status::Pending, Some(C_KNOWN_PENDING)),
            ("FROBNICATE 1", Status::Pending, Some(UNKNOWN_COMMAND)),
        ] {
            assert_eq!(status_of(text), (status, code), "{text}");
        }
    }

    #[test]
    fn a_coordinate_that_is_not_two_lengths_is_a_number_problem() {
        let r = interpret(&first("MOVE R1 (a b)"), &ScriptSettings::default()).unwrap_err();
        assert_eq!(r.code, NUMBER_INVALID);
        let r = interpret(&first("WIRE (0 0) (1 x)"), &ScriptSettings::default()).unwrap_err();
        assert_eq!(r.code, NUMBER_INVALID);
    }

    #[test]
    fn the_typed_arguments_keep_what_was_written() {
        let s = ScriptSettings::default();
        match interpret(&first("WIRE 'GND' 0.3mm (0 0) (10mil 5)"), &s).unwrap() {
            Action::Wire {
                net,
                width,
                style,
                points,
            } => {
                assert_eq!(net.as_deref(), Some("GND"));
                assert_eq!(width, Len::parse("0.3mm"));
                assert!(!style);
                assert_eq!(points.len(), 2);
                assert_eq!(points[1].x, Len::parse("10mil").unwrap());
            }
            other => panic!("{other:?}"),
        }
        match interpret(&first("TEXT 'R1' MR90 (1 2)"), &s).unwrap() {
            Action::Text { text, rot, .. } => {
                assert_eq!(text, "R1");
                assert!(rot.unwrap().mirror);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            interpret(&first("DISPLAY NONE Top -Bottom +21"), &s).unwrap(),
            Action::Display(vec![
                DisplayOp::None,
                DisplayOp::Show("Top".to_owned()),
                DisplayOp::Hide("Bottom".to_owned()),
                DisplayOp::Show("21".to_owned()),
            ])
        );
        assert_eq!(
            interpret(&first("LAYER 21"), &s).unwrap(),
            Action::Layer(LayerRef::Number(21))
        );
        assert_eq!(
            interpret(&first("LAYER tPlace"), &s).unwrap(),
            Action::Layer(LayerRef::Name("tPlace".to_owned()))
        );
    }

    #[test]
    fn the_known_commands_come_from_the_settings() {
        let mut s = ScriptSettings::default();
        s.known_pending.push("FROBNICATE".to_owned());
        let (status, note) = classify(&first("FROBNICATE"), &s);
        assert_eq!(status, Status::Pending);
        assert_eq!(note.unwrap().code, C_KNOWN_PENDING);
        s.set_ignored.push("PICK_AREA".to_owned());
        assert_eq!(classify(&first("SET PICK_AREA 3"), &s).0, Status::Adapted);
    }
}
