// SPDX-License-Identifier: GPL-3.0-or-later
//! The command line of EH3D-PHOENIX-PCB. It has twenty commands, `inventory`, `eagle-rate`,
//! `stackup`, `materials`, `check`, `connectivity`, `erc`, `sync`, `export`, `rf`, `rfgen`, `kicad-read`, `sanitize`,
//! `sim`, `route`, `ratsnest`, `script`, `cam-import`, `schedit` and `view`.
//!
//! The editor of boards adds the command `boardedit`. The editor of libraries adds the commands `libedit` and `footprint`, and the routing through an
//! external autorouter adds `autoroute`, the filling of copper zones adds `zones` and the lengths
//! of nets and pairs add `lengths`; the import of Gerber and Excellon files adds `gerber-import`
//! and the check of their structure adds `gerber-check`; the layout of a panel of boards adds `panel`; the bill of materials with costs adds `bom`
//! and the 3D assembly of the board with the models of its components adds `assembly3d`; the run of
//! the reference cases of the calculations adds `oracle`, and the run of the own corpus of small
//! hand-written pieces adds `corpus`.
//! The writing of boards, schematics and libraries as KiCad 8 files adds `kicad-export`.
//! The simulation netlist of a schematic, made from a table of models, adds `netlist`.
//! The annotation back into the schematic, as a plan that is read before it is applied, adds
//! `annotate`.
//! The own library of footprints by the method of IPC-7351B adds `ipc`, and the bulk import of
//! EAGLE and KiCad libraries into an index adds `libindex`.
//! The pick and place files, the solder paste stencil and the fabrication package add `pnp`,
//! `stencil` and `fabpack`.
//! The local parts base adds `parts`, and the bill of materials takes its data from it.
//! The hierarchy of sheets and the buses of a schematic add `hier`.
//! The printing of schematics, boards and panels as SVG and PDF adds `print`.
//! DXF drawings add `dxf-import` (the outline of a board from the contours of a drawing) and
//! `dxf-export` (a board or a panel as a drawing).
//!
//! Exit codes: 0 done, 1 `stackup` could not compute one or more of the lines it was asked for,
//! `check`, `connectivity` and `erc` proved an error, or `sync` or `annotate` found changes to
//! make, 2 the arguments or the input are unusable.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod annotate;
mod assembly3d;
mod autoroute;
mod boardedit;
mod bom;
mod check;
mod corpus;
mod design;
mod dxf_export;
mod dxf_import;
mod export;
mod fabpack;
mod gerber_check;
mod gerber_import;
mod hier;
mod ipclib;
mod kicad;
mod kicad_export;
mod kicad_import;
mod lengths;
mod libedit;
mod materials;
mod netlist;
mod oracle;
mod pairs;
mod panel;
mod parts;
mod pnp;
mod print;
mod rf;
mod rfgen;
mod route;
mod sanitize;
mod schedit;
mod script;
mod sim;
mod stencil;
mod view;
pub mod window_args;
mod zones;

use eh3d_phoenix_corpus::{inventory, InventoryConfig};
use eh3d_phoenix_stackup::{report_json, report_text, Language, StackupJob, StackupSettings};
use std::io::Write;
use std::path::Path;

const USAGE: &str = "\
phoenix [--config FILE] inventory [--json] <folder>

  inventory   count the design files of a private corpus without showing any name or value:
              files by extension, kind and declared version, and one fingerprint of the content

phoenix eagle-rate <folder of originals> <new folder for the copies>

  eagle-rate  copy the .lbr, .brd and .sch files of a folder (the originals are only read, each copy
              is checked by SHA-256) and measure how many the EAGLE reader reads: counts, versions
              and diagnostic codes only, never a name or a value

phoenix stackup <job.json> --freq-ghz <GHz> [--material ID ...] [--settings FILE] [--lang xx] [--json]

  stackup     compute the transmission lines of a stack-up (microstrip, stripline, coplanar
              waveguide, coupled pairs): impedance, effective permittivity, losses, delay and
              guided wavelength at a frequency, or the track width for a target impedance; the
              job file holds the layers, the materials and the lines; --material adds a material
              of the database (for example isola/370hr or rogers/ro4350b) that the layers name
              by its identifier, its Dk and Df taken from the published points; a prototype
              estimate, not a simulation; --lang is one of en, es, de, fr, it, ja, zh, pt

phoenix materials list [--manufacturer NAME] [--kind KIND] [--min-tg C] [--min-dk X] [--max-dk X]
                       [--min-df X] [--max-df X] [--at-ghz F] [--phoenix] [--json]
phoenix materials show <id> [--at-ghz F] [--json]

  materials   the board materials database of the suite, every value as its datasheet publishes
              it: list filters by manufacturer, kind, glass transition, Dk and Df (at a
              frequency) and, with --phoenix, keeps the materials that serve the design tool;
              show prints one record (run `phoenix materials` alone for the kinds)

phoenix check [--rules FILE] [--json] [--lang xx] <board.brd>
phoenix connectivity [--rules FILE] [--json] [--lang xx] <schematic.sch> <board.brd>

  check         run the design rules of FILE (JSON, phoenix-rules/1) or of the board on an EAGLE board
  connectivity  shorts and unrouted nets of the board, and the board against the schematic
                (exit 1 when an error is proven; the report keeps confirmed and inconclusive apart)

phoenix erc [--config FILE] [--json] [--lang xx] [--advanced] [--severity RULE=LEVEL] [--matrix KIND.KIND=LEVEL] [--power-flag NET] <schematic> [<board>]
phoenix sync [--config FILE] [--json | --package] [--lang xx] [--backward] [--apply --out FILE] <schematic> <board>
phoenix sync --diff [--json] [--lang xx] <schematic> <board>
phoenix annotate [--config FILE] [--json] [--lang xx] [--renumber] [--apply --confirm CODE --out FILE [--board-out FILE]] <schematic> <board>

  erc           the electrical rule check of a schematic (pins on no net, pin types by a matrix in a
                settings file, inputs without a driver, supplies without a source, net names that
                look alike, references used twice, empty values, parts without a footprint, loose
                wires, nets that nothing drives, decoupling of chip power pins, typing mistakes in
                net names, labels that touch nothing); the report keeps confirmed and inconclusive
                findings apart
  sync          what the transfer of the schematic to the board (or, with --backward, the annotation
                back into the schematic) would change, by the identity of the parts; a dry run
                unless --apply writes the result to --out, and an input file is never overwritten;
                --diff lists the differences and conflicts of the two documents in both directions
  annotate      the annotation back into the schematic as a plan read before it is applied
                (references, renumbering by position, values, footprints, swaps of equivalent
                pins and gates); each change has a state and the plan a confirmation code; --apply
                needs the code and writes a NEW file, never an input or a file that exists

phoenix export [--profile NAME] [--profiles FILE] [--lang xx] [--eh3dproj FILE] --out <folder> <board>

  export        the manufacturing files of a board (Gerber X2, plated and not plated drills,
                placement, bill of materials) and manifest.json with their SHA-256, made with a
                profile of a JSON file and checked against the board before they are written
                (exit 1 when they differ)

phoenix rf touchstone|match|filter ...

  rf            everyday RF tools: Touchstone files, impedance matching and ladder filters
                (run `phoenix rf` alone for its usage)

phoenix rfgen --list [--json]
phoenix rfgen <type> --stackup FILE --freq-ghz GHZ [--board FILE] [--out FILE] [--<parameter> VALUE ...]

  rfgen         draw a microstrip RF structure (line, bend, curve, taper, stubs, matching networks,
                low-pass and band-pass filters, Wilkinson divider, branch-line coupler, patch and
                dipole antennas, SMA launch with a via fence) from the stack-up and the frequency
                and put it on a board as reversible operations, with a report of the dimensions,
                the formulas, the warnings and the rule check; a prototype never compared with an
                electromagnetic simulator (run `phoenix rfgen` alone for its options)

phoenix kicad-read [--settings FILE] [--lang xx] [--redacted] <file>

  kicad-read    read a KiCad 6 to 9 file (schematic with its sheets, board, symbol library,
                footprint or project) into the design model and summarise it: what was kept,
                changed and left out, the findings with their codes and the fidelity level

phoenix kicad-import [--settings FILE] [--lang xx] [--json] [--out FILE.json] <project>

  kicad-import  read a whole KiCad 6 to 9 project (the .kicad_pro with the .kicad_sch and the
                .kicad_pcb of the same name, a folder with one project, or one schematic or
                board) into the design model with one report; a dry run unless --out names a new
                .json file, never over an input; a prototype. The commands that take a board or
                a schematic also take a .kicad_pcb or a .kicad_sch file

phoenix kicad-export [--settings FILE] [--lang xx] [--json] [--board FILE] [--out PATH]
                     <board|schematic|library> <input>

  kicad-export  write a board, a schematic (one file for each sheet) or the footprints or symbols
                of a design as KiCad 8 files, with a report of what was written, changed and left
                out; a dry run unless --out names a folder (or the one file to write), never over
                an input or over a file that exists; a prototype never opened in KiCad

phoenix sanitize [--out BASE] [--threshold N] [--config FILE] [--lang xx] [--json] <folder>

  sanitize      find the footprints, symbols and device sets that are duplicated or nearly
                duplicated in a folder of EAGLE libraries: an inventory, a canonical geometric
                hash, a score against a threshold, groups to confirm by hand, an HTML and a JSON
                report and a plan of the master library that is never applied; the libraries
                are only read

phoenix sim <netlist or schematic.json> --op | --dc SRC START STOP STEP | --ac KIND N F1 F2 | --tran STEP STOP
              [--model FILE] [options]
phoenix sim --schematic <schematic> --op | --dc SRC START STOP STEP | --ac KIND N F1 F2 | --tran STEP STOP
              [--models FILE] [--no-erc] [options]
phoenix sim --check-filter FAMILY ORDER BAND F1 [F2] [options]

  sim           simulate a circuit with the solver of this program (modified nodal analysis, a
                prototype never compared with SPICE): the DC operating point, a DC sweep, an AC
                sweep with group delay or a transient analysis of a SPICE-style netlist (diodes,
                LEDs, Zener diodes, bipolar and MOS transistors, switches, op-amps; a file of
                models with --model; the .op, .dc, .ac or .tran line of the netlist when no
                analysis is named) or of a schematic, as CSV or JSON; run `phoenix sim` alone
                for its options. With --schematic the netlist is made from the schematic
                and a table of models and chained to the solver with no file in between

phoenix netlist [--models FILE] [--out NEW.cir] [--json] [--lang xx] [--no-erc] <schematic>

  netlist       make the simulation netlist of a schematic (EAGLE, KiCad or the design model)
                with no netlist written by hand: the model of each part from a table (by value,
                symbol name or designator prefix) or from its attributes, values such as 4k7 or
                1u5, stable names for the nets and a report of what cannot be simulated with
                codes PHOENIX-NETLIST-<NNN>, after the electrical rule check; a dry run that
                writes only to a new file named with --out, a prototype, run `phoenix netlist`
                alone for its options

phoenix hier [--config FILE] [--json] [--lang xx] <schematic>

  hier          the hierarchy of sheets of a schematic: the sheets and the sheet symbols that show
                them, the instances (a sheet shown N times is N instances, with references such
                as U1/R1), the nets by hierarchical name with the instances where they have a
                pin, and the problems of the sheet symbols, pins, ports and buses with codes
                PHOENIX-HIER-<NNN> and PHOENIX-BUS-<NNN>; it only reads, a prototype, run
                `phoenix hier` alone for its options

phoenix ratsnest [--settings FILE] [--json] [--lang xx] <board>
phoenix route [--settings FILE] [--rules FILE] [--layer NAME] [--mode around|stop] [--json]
              [--lang xx] --from ELEMENT --to ELEMENT [--out FILE] <board>

  ratsnest      the connections still to be routed on a board, net by net
  route         one track (and the vias it needs) between two elements of a net, at 0, 45 and 90
                degrees, keeping the clearances of the rules; writes the board with the track to
                a new file given with --out (an existing file is never replaced); a prototype,
                run `phoenix route` alone for its options

phoenix autoroute export-dsn|import-ses|run [--rules FILE] [--out FILE] [--lang xx] <board> [<session>]

  autoroute     routing by an external autorouter through files: write the board as a Specctra
                DSN file, read a Specctra session as a plan checked against the rules (all or
                nothing) and, only when config/autorouter.cfg switches it on and --execute is
                given, start the program; a dry run by default, a prototype, run
                `phoenix autoroute` alone for its options

phoenix gerber-import [--config FILE] [--json] [--lang xx] [--out FILE] <file or folder> ...
phoenix gerber-import [--config FILE] [--json] [--lang xx] [--profile NAME] [--profiles FILE]
                      --roundtrip <board>
phoenix gerber-check [--config FILE] [--json] [--lang xx] <file or folder> ...
phoenix panel [--config FILE] [--columns N] [--rows N] [--boards A,B,...] [--profile NAME]
              [--json] [--lang xx] [--out FOLDER] [<board>]

  gerber-import read Gerber RS-274X and Excellon files as read-only geometry for review (the layer
                of each file from its attributes or its name, apertures and macros, polarity,
                regions, arcs, drill tools, holes and slots), with a code for what cannot be read;
                the files are only read, and --roundtrip exports a board and compares the files
                read back with it; a prototype, run `phoenix gerber-import` alone for its options
  gerber-check  check the structure of Gerber X2 and Excellon files (end command, definitions
                before use, regions, attributes) with the codes PHOENIX-GX2-nnn; the files are
                only read, a prototype, run `phoenix gerber-check` alone for its options
  panel         lay a board out in a matrix, or a list of different boards, with a gap, a frame,
                tabs with mouse bite holes or V-score lines, fiducials and tooling holes from
                config/panel.cfg, and write the copper, solder mask, paste, legend and drills of
                every copy and the outline of the panel; a dry run unless --out names a new
                folder, a prototype, run `phoenix panel` alone for its options

phoenix script run [--config FILE] [--rules FILE] [--dry-run] [--out FILE] [--json] [--lang xx]
                   <script> <board or schematic>
phoenix script report [--config FILE] [--json] [--html FILE] [--lang xx] <folder or script>
phoenix cam-import [--config FILE] [--json] [--out FILE] [--lang xx] <job.cam>

  script        run a script of the old command language on a board (a dry run unless --out is
                given; all or nothing, never replaces a file), or report for each command of
                each script whether it is equivalent, adapted or pending, without running it;
                a prototype, run `phoenix script` alone for its options
  cam-import    read an output job into an export profile and report what could be translated

phoenix schedit [--erc-config FILE] [--json] [--lang xx] --apply OPERATIONS.json [--out FILE]
                <schematic>

  schedit       apply a file of editing operations (place, move, turn, mirror and delete parts,
                copy and paste, values and references, wires, net labels, power ports, bus labels)
                to a schematic, all or nothing, make the nets again from the drawing and run the
                electrical rule check on the result; a dry run unless --out names a new file
                (.json or .sch), and an input file is never written over (run `phoenix schedit`
                alone for its options)

phoenix boardedit [--rules FILE] [--settings FILE] [--json] [--lang xx] --apply OPERATIONS.json
                  [--out FILE] <board>

  boardedit     apply a file of editing operations to a board (place, move, turn, mirror, lock and
                remove components, orthogonal and 45-degree tracks, vias, change of layer, delete a
                stretch or a net, paste a net with the shortest route between two pads, tune a net
                to a length with meanders), all or nothing, with the design rules checked after
                each one: an operation that adds a rule error is refused unless it is forced; a
                dry run unless --out names a new file (.json or .brd), and an input file is never
                written over (run `phoenix boardedit` alone for its options); a prototype
                that also draws, tunes and corrects differential pairs (route_pair, tune_pair and
                fix_pair_gap)

phoenix pairs [--config FILE] [--pairs FILE] [--stackup FILE] [--json] [--lang xx] <board>

  pairs         check the differential pairs of a board: the gap, the symmetry, the uncoupled
                stretch, the lengths, the vias, the width and the impedance, with codes
                PHOENIX-PAIR-<NNN>; a dry run that writes nothing, a prototype, run `phoenix pairs`
                alone for its options

phoenix libedit [--apply OPERATIONS.json] [--plan SANITIZER.plan.json] [--out NEW.lbr]
                [--config FILE] [--json] [--lang xx] <library.lbr>
phoenix footprint <family> [--param NAME=VALUE ...] [--config FILE] [--into LIBRARY.lbr]
                  [--out NEW.lbr] [--json] [--lang xx]

  libedit       edit an EAGLE library with a file of operations (symbols, footprints, devices,
                pads, pins, openings and connections), all or nothing, check it and, with --out,
                save it as a new library that is read back and compared, with a copy of the
                original; a dry run otherwise, and a file that exists is never written over
  footprint     make a footprint (chip, SOIC, TSSOP, QFN, QFP, DIP, header) from parameters by the
                method of IPC-7351B with the values of config/footprints.cfg, a prototype that is
                not checked against a datasheet (run either command alone for its options)

phoenix ipc list|show|generate|check|compare ...
phoenix libindex <folder> [--json] [--out INDEX.json] [--config FILE] [--lang xx]

  ipc           the own library of footprints by the method of IPC-7351B: families (chip, MELF,
                SOD, SOT, SOIC, SSOP, TSSOP, QFN, DFN, QFP, BGA, DIP, headers and connectors)
                with their dimensions in data files, three density levels and the canonical
                names (RESC1608X55N, SOIC127P600X175-8N); list, show, generate (a dry run unless
                --out names a new .lbr or .kicad_mod file), check the library and compare a
                footprint of yours with the generated one as a report; the dimensions are a
                prototype, not checked against a datasheet (run `phoenix ipc` alone for its
                options)
  libindex      walk a folder of EAGLE (.lbr) and KiCad (.kicad_mod, .kicad_sym) libraries, judge
                every footprint and symbol and make a deterministic index, phoenix-libindex/1,
                with the hash of each item, without copying the files; a dry run unless --out
                names a new file, a prototype

phoenix zones fill [--config FILE] [--rules FILE] [--zones FILE] [--out FILE] [--json]
                   [--lang xx] <board>
phoenix zones check [--config FILE] [--rules FILE] [--zones FILE] [--fill FILE] [--json]
                    [--lang xx] <board>
phoenix lengths [--config FILE] [--stackup FILE] [--groups FILE] [--out FILE] [--json]
                [--lang xx] <board>

  zones         fill the copper zones of a board (clearance from the rules, thermal reliefs on
                the pads of the net, islands judged by area, width and connection) or measure a
                fill against the board; a dry run unless --out names a new file, a prototype,
                run `phoenix zones` alone for its options
  lengths       the copper length and the delay of the nets of a board, groups of nets that
                must match in length and differential pairs; a prototype, run `phoenix lengths`
                alone for its options

phoenix bom [--config FILE] [--prices FILE] [--parts FILE] [--schematic FILE] [--stackup FILE]
            [--boards N] [--csv FILE] [--json-file FILE] [--html FILE] [--json] [--lang xx]
            <board>
phoenix assembly3d [--config FILE] --models FILE [--stackup FILE] [--obj FILE] [--step FILE]
                   [--json-file FILE] [--json] [--lang xx] <board>

  bom           the bill of materials of a board grouped by value, footprint and maker, with the
                cheapest supplier from a local price file, the cost per board and per batch with
                the waste, and an estimate of the cost of the boards from example tariffs; CSV,
                JSON and HTML are written only to new files, a prototype, run `phoenix bom`
                alone for its options
  assembly3d    the board and its components as a 3D scene from the models of a table (STEP files
                or boxes), with collisions and a height limit checked on the boxes and a
                simplified OBJ and STEP export written only to new files, a prototype, run
                `phoenix assembly3d` alone for its options

phoenix parts search|show|check|import|export [--base PATH] [options] (run `phoenix parts` alone)

  parts         the local parts base: components kept in JSON and CSV files of your own (value,
                tolerance, power, voltage, footprint, symbol, maker, part number, parameters,
                simulation model, suppliers with prices by quantity break, state and
                alternatives), searched by text, value, footprint and part number, checked
                (footprints that do not exist, part numbers with other data, prices without a
                currency, price breaks out of order), imported from CSV into a new base and
                exported to a new file; nothing is fetched from a network, a prototype

phoenix pnp [--config FILE] [--maker NAME] [--parts FILE] [--dnp R1,R2,...] [--origin WORD]
            [--units mm|mil] [--lang xx] [--out FOLDER] <board>
phoenix stencil [--config FILE] [--profile NAME] [--profiles FILE] [--lang xx] [--out FOLDER]
                <board>
phoenix fabpack [--profile NAME] [--profiles FILE] [--pnp-config FILE] [--stencil-config FILE]
                [--costs FILE] [--parts FILE] [--prices FILE] [--maker NAME] [--panel-boards N]
                [--dnp R1,R2,...] [--lang xx] [--out FOLDER|FILE.zip] <board>

  pnp           the pick and place files of a board, one for each face, as CSV and JSON, with the
                centre of each part from its pads, extra rotations by footprint and maker from
                config/pnp_rotations.cfg and the origin and unit of your choice; a dry run unless
                --out names a new folder, a prototype, run `phoenix pnp` alone for its options
  stencil       the solder paste stencil of a board as Gerber files with reduced openings, large
                pads split into windows and the IPC-7525 ratios checked, from config/stencil.cfg;
                a dry run unless --out names a new folder, a prototype, run `phoenix stencil`
                alone for its options
  fabpack       the fabrication package of a board: Gerber, Excellon, pick and place, bills of
                materials of the assembly and stencil in one deterministic zip file or new folder
                with a manifest of SHA-256 sums; a dry run unless --out names a new destination,
                a prototype, run `phoenix fabpack` alone for its options

phoenix oracle run  [--dir FOLDER] [--suite NAME] [--out FILE] [--json] [--lang xx]
phoenix oracle list [--dir FOLDER] [--suite NAME]

  oracle        run the reference cases (published values with their source and tolerance)
                against the real calculations of the stack-up, RF, simulator, rules, lengths
                and zones, or list them; nothing is written unless --out names a new file, a
                prototype that does not compare anything with EAGLE, KiCad or a measurement,
                run `phoenix oracle` alone for its options

phoenix corpus run   [--dir FOLDER] [--filter TEXT] [--properties] [--settings FILE] [--out FILE] [--json] [--lang xx]
phoenix corpus list  [--dir FOLDER] [--filter TEXT] [--settings FILE]
phoenix corpus check [--dir FOLDER] [--settings FILE] [--lang xx]

  corpus        run the own corpus (small hand-written boards, schematics, libraries and
                manufacturing files in the formats the suite reads or writes, with expectations
                worked out by hand, adversarial files, round trips between formats and, with
                --properties, properties on boards made from seeds), list it, or check that it
                is consistent; nothing is written unless --out names a new file, a prototype
                that holds no file of a maker, run `phoenix corpus` alone for its options

phoenix print <schematic|board|panel> [--kind svg|pdf] [--out FOLDER|FILE] [--paper A4]
              [--scale N] [--layers LIST] [--side top|bottom] [--lang xx] [--config FILE] <file>

  print         print a schematic (a page for each sheet), a board (the assembly plan of each side
                and its layers) or a panel of boards as SVG or as a PDF made by hand, with a
                border, a title block in the language chosen and a scale bar; a dry run unless
                --out names a file or a folder that does not exist yet, a prototype whose PDF has
                not been opened in real readers, run `phoenix print` alone for its options

phoenix dxf-import <drawing.dxf> --board BOARD [--layer LAYER] [--join-tolerance-mm N]
                   [--config FILE] [--rules FILE] [--force] [--out FILE.json] [--json] [--lang xx]
phoenix dxf-export <board|panel> [--out FILE.dxf] [--config FILE] [--silk] [--json] [--lang xx] <file>

  dxf-import    the outline of a board from the contours of a DXF drawing: the pieces are joined
                into closed contours, the outer one becomes the outline and its cutouts are listed
                and not kept; what is not understood is listed; all or nothing, checked by the
                design rules before and after; a dry run unless --out names a new .json file, a
                prototype, run `phoenix dxf-import` alone for its options
  dxf-export    a board (outline, holes, outlines of the pads, silkscreen) or a panel (frame,
                scores, tabs, holes, fiducials) as a DXF drawing on layers with the names of
                config/dxf.cfg; written only to a new file, a prototype that no CAD program has
                opened, run `phoenix dxf-export` alone for its options

phoenix view [--settings FILE] [--lang xx] [<board or schematic>]
phoenix view --summary <file>  |  phoenix view --bench

  view          a viewer and editor window of a board or a schematic (a build with the gui
                feature; a prototype), a text summary of a file, or a timing of the preparation
                and the selection on three synthetic boards
";

/// Run the command line with the arguments that follow the program name. Returns the exit code.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut args = args.to_vec();
    let mut config = InventoryConfig::default();
    if args.first().map(String::as_str) == Some("--config") {
        if args.len() < 3 {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
        let parsed = std::fs::read_to_string(&args[1])
            .map_err(|e| e.to_string())
            .and_then(|text| InventoryConfig::parse(&text).map_err(|e| e.to_string()));
        match parsed {
            Ok(parsed) => config = parsed,
            Err(message) => {
                let _ = writeln!(err, "error: cannot use the configuration: {message}");
                return 2;
            }
        }
        args.drain(..2);
    }
    let rest: Vec<&str> = args.iter().map(String::as_str).collect();
    if let Some((&"stackup", tail)) = rest.split_first() {
        return stackup_command(tail, out, err);
    }
    if let Some((&"materials", tail)) = rest.split_first() {
        return materials::materials_command(tail, out, err);
    }
    if let Some((&"check", tail)) = rest.split_first() {
        return check::check(tail, out, err);
    }
    if let Some((&"connectivity", tail)) = rest.split_first() {
        return check::connectivity(tail, out, err);
    }
    if let Some((&"erc", tail)) = rest.split_first() {
        return design::erc(tail, out, err);
    }
    if let Some((&"sync", tail)) = rest.split_first() {
        return design::sync(tail, out, err);
    }
    if let Some((&"annotate", tail)) = rest.split_first() {
        return annotate::annotate(tail, out, err);
    }
    if let Some((&"export", tail)) = rest.split_first() {
        return export::export(tail, out, err);
    }
    if let Some((&"rf", tail)) = rest.split_first() {
        return rf::rf(tail, out, err);
    }
    if let Some((&"rfgen", tail)) = rest.split_first() {
        return rfgen::rfgen_command(tail, out, err);
    }
    if let Some((&"kicad-read", tail)) = rest.split_first() {
        return kicad::kicad_read(tail, out, err);
    }
    if let Some((&"kicad-import", tail)) = rest.split_first() {
        return kicad_import::kicad_import(tail, out, err);
    }
    if let Some((&"kicad-export", tail)) = rest.split_first() {
        return kicad_export::kicad_export(tail, out, err);
    }
    if let Some((&"sanitize", tail)) = rest.split_first() {
        return sanitize::sanitize_command(tail, out, err);
    }
    if let Some((&"script", tail)) = rest.split_first() {
        return script::script_command(tail, out, err);
    }
    if let Some((&"cam-import", tail)) = rest.split_first() {
        return script::cam_import_command(tail, out, err);
    }
    if let Some((&"route", tail)) = rest.split_first() {
        return route::route_command(tail, out, err);
    }
    if let Some((&"ratsnest", tail)) = rest.split_first() {
        return route::ratsnest_command(tail, out, err);
    }
    if let Some((&"autoroute", tail)) = rest.split_first() {
        return autoroute::autoroute_command(tail, out, err);
    }
    if let Some((&"gerber-import", tail)) = rest.split_first() {
        return gerber_import::gerber_import_command(tail, out, err);
    }
    if let Some((&"gerber-check", tail)) = rest.split_first() {
        return gerber_check::gerber_check_command(tail, out, err);
    }
    if let Some((&"panel", tail)) = rest.split_first() {
        return panel::panel_command(tail, out, err);
    }
    if let Some((&"print", tail)) = rest.split_first() {
        return print::print_command(tail, out, err);
    }
    if let Some((&"dxf-import", tail)) = rest.split_first() {
        return dxf_import::dxf_import_command(tail, out, err);
    }
    if let Some((&"dxf-export", tail)) = rest.split_first() {
        return dxf_export::dxf_export_command(tail, out, err);
    }
    if let Some((&"sim", tail)) = rest.split_first() {
        return sim::sim_command(tail, out, err);
    }
    if let Some((&"netlist", tail)) = rest.split_first() {
        return netlist::netlist_command(tail, out, err);
    }
    if let Some((&"hier", tail)) = rest.split_first() {
        return hier::hier_command(tail, out, err);
    }
    if let Some((&"schedit", tail)) = rest.split_first() {
        return schedit::schedit_command(tail, out, err);
    }
    if let Some((&"boardedit", tail)) = rest.split_first() {
        return boardedit::boardedit_command(tail, out, err);
    }
    if let Some((&"pairs", tail)) = rest.split_first() {
        return pairs::pairs_command(tail, out, err);
    }
    if let Some((&"libedit", tail)) = rest.split_first() {
        return libedit::libedit_command(tail, out, err);
    }
    if let Some((&"ipc", tail)) = rest.split_first() {
        return ipclib::ipc_command(tail, out, err);
    }
    if let Some((&"libindex", tail)) = rest.split_first() {
        return ipclib::libindex_command(tail, out, err);
    }
    if let Some((&"footprint", tail)) = rest.split_first() {
        return libedit::footprint_command(tail, out, err);
    }
    if let Some((&"zones", tail)) = rest.split_first() {
        return zones::zones_command(tail, out, err);
    }
    if let Some((&"lengths", tail)) = rest.split_first() {
        return lengths::lengths_command(tail, out, err);
    }
    if let Some((&"bom", tail)) = rest.split_first() {
        return bom::bom_command(tail, out, err);
    }
    if let Some((&"parts", tail)) = rest.split_first() {
        return parts::parts_command(tail, out, err);
    }
    if let Some((&"pnp", tail)) = rest.split_first() {
        return pnp::pnp_command(tail, out, err);
    }
    if let Some((&"stencil", tail)) = rest.split_first() {
        return stencil::stencil_command(tail, out, err);
    }
    if let Some((&"fabpack", tail)) = rest.split_first() {
        return fabpack::fabpack_command(tail, out, err);
    }
    if let Some((&"assembly3d", tail)) = rest.split_first() {
        return assembly3d::assembly3d_command(tail, out, err);
    }
    if let Some((&"oracle", tail)) = rest.split_first() {
        return oracle::oracle_command(tail, out, err);
    }
    if let Some((&"corpus", tail)) = rest.split_first() {
        return corpus::corpus_command(tail, out, err);
    }
    if let Some((&"view", tail)) = rest.split_first() {
        return view::view(tail, out, err);
    }
    if let ["eagle-rate", source, copies] = rest.as_slice() {
        return eagle_rate(Path::new(source), Path::new(copies), out, err);
    }
    let (json, folder) = match rest.as_slice() {
        ["inventory", "--json", folder] => (true, *folder),
        ["inventory", folder] if *folder != "--json" => (false, *folder),
        _ => {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    match inventory(Path::new(folder), &config) {
        Ok(report) => {
            let text = if json {
                report.to_json() + "\n"
            } else {
                report.to_text()
            };
            let _ = out.write_all(text.as_bytes());
            0
        }
        Err(error) => {
            // the message names a path the caller gave or one below it: the caller already knows it
            let _ = writeln!(err, "error: {error}");
            2
        }
    }
}

/// The largest job or settings file that `stackup` reads, in bytes.
const MAX_STACKUP_FILE_BYTES: u64 = 4 * 1024 * 1024;

fn read_bounded(path: &str) -> Result<String, String> {
    let length = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if length > MAX_STACKUP_FILE_BYTES {
        return Err(format!(
            "the file is larger than {MAX_STACKUP_FILE_BYTES} bytes"
        ));
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

fn stackup_command(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut file: Option<&str> = None;
    let mut freq: Option<f64> = None;
    let mut settings_path: Option<&str> = None;
    let mut language = Language::En;
    let mut json = false;
    let mut material_ids: Vec<&str> = Vec::new();
    let mut iter = args.iter().copied();
    while let Some(arg) = iter.next() {
        match arg {
            "--json" => json = true,
            "--material" => match iter.next() {
                Some(id) => material_ids.push(id),
                None => {
                    let _ = err.write_all(USAGE.as_bytes());
                    return 2;
                }
            },
            "--freq-ghz" => freq = iter.next().and_then(|v| v.parse::<f64>().ok()),
            "--settings" => settings_path = iter.next(),
            "--lang" => match iter.next().and_then(Language::from_tag) {
                Some(found) => language = found,
                None => {
                    let _ = err.write_all(USAGE.as_bytes());
                    return 2;
                }
            },
            other if !other.starts_with("--") && file.is_none() => file = Some(other),
            _ => {
                let _ = err.write_all(USAGE.as_bytes());
                return 2;
            }
        }
    }
    let (Some(file), Some(freq)) = (file, freq) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let settings = match settings_path {
        None => StackupSettings::default(),
        Some(path) => match read_bounded(path)
            .and_then(|text| StackupSettings::parse(&text).map_err(|e| e.to_string()))
        {
            Ok(settings) => settings,
            Err(message) => {
                let _ = writeln!(err, "error: cannot use the settings: {message}");
                return 2;
            }
        },
    };
    let extra = match materials::samples(&material_ids) {
        Ok(extra) => extra,
        Err(error) => {
            let _ = writeln!(err, "error {}: {}", error.code(), error.message(language));
            return 2;
        }
    };
    let job = match read_bounded(file) {
        Ok(text) => match StackupJob::parse_with(&text, extra) {
            Ok(job) => job,
            Err(error) => {
                let _ = writeln!(err, "error {}: {}", error.code(), error.message(language));
                return 2;
            }
        },
        Err(message) => {
            let _ = writeln!(err, "error: cannot read the job file: {message}");
            return 2;
        }
    };
    let outcomes = job.run(freq, &settings);
    let text = if json {
        let mut json_text = report_json(&job.stackup.name, freq, &outcomes, language);
        json_text.push('\n');
        json_text
    } else {
        report_text(&job.stackup.name, freq, &outcomes, language)
    };
    let _ = out.write_all(text.as_bytes());
    if outcomes.iter().any(|o| o.result.is_err()) {
        1
    } else {
        0
    }
}

fn eagle_rate(source: &Path, copies: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    use eh3d_phoenix_eagle::{corpus, Limits};
    match corpus::snapshot(source, copies, &["lbr", "brd", "sch"]) {
        Ok((records, problems)) => {
            let rate = corpus::measure(&records, &Limits::default());
            let _ = out.write_all(rate.to_text().as_bytes());
            let _ = writeln!(out, "copy problems: {}", problems.len());
            0
        }
        Err(error) => {
            let _ = writeln!(err, "error: {error}");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(list: &[&str]) -> (i32, String, String) {
        let args: Vec<String> = list.iter().map(|s| s.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn the_view_command_is_reached_and_its_usage_errors_exit_2() {
        let (code, out, err) = call(&["view", "--summary"]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert!(err.contains("--summary needs a file"), "{err}");
        let (code, out, _) = call(&["view", "--bench", "--settings", "/nonexistent/viewer.cfg"]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
    }

    #[test]
    fn usage_errors_exit_2() {
        for list in [
            &[][..],
            &["inventory"],
            &["inventory", "--json"],
            &["nonsense", "x"],
            &["--config", "x"],
        ] {
            let (code, out, err) = call(list);
            assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
            assert!(err.contains("inventory"));
        }
    }

    #[test]
    fn inventory_prints_text_and_json_without_names() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("TopSecret.brd"),
            "<eagle version=\"9.6.2\"></eagle>",
        )
        .unwrap();
        let folder = dir.path().to_str().unwrap();
        let (code, text, _) = call(&["inventory", folder]);
        assert_eq!(code, 0);
        assert!(text.contains("files: 1") && text.contains("version 9.6.2: 1"));
        let (code, json, _) = call(&["inventory", "--json", folder]);
        assert_eq!(code, 0);
        assert!(json.contains("\"eagle-xml\""));
        assert!(!text.contains("TopSecret") && !json.contains("TopSecret"));
    }

    #[test]
    fn eagle_rate_copies_measures_and_shows_no_names() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("originals");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("TopSecret.brd"),
            "<eagle version=\"9.6.2\"><drawing><board><elements/><signals/></board></drawing></eagle>",
        )
        .unwrap();
        std::fs::write(src.join("Broken.sch"), "not xml").unwrap();
        let copies = dir.path().join("copies");
        let (code, text, _) = call(&[
            "eagle-rate",
            src.to_str().unwrap(),
            copies.to_str().unwrap(),
        ]);
        assert_eq!(code, 0);
        assert!(text.contains("files: 2") && text.contains("version 9.6.2: 1"));
        assert!(text.contains("code PHOENIX-EAGLE-001: 1"));
        assert!(text.contains("copy problems: 0"));
        assert!(!text.contains("TopSecret") && !text.contains("Broken"));
        // a folder for the copies inside the originals is refused
        let (code, _, err) = call(&[
            "eagle-rate",
            src.to_str().unwrap(),
            src.join("in").to_str().unwrap(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("outside"));
    }

    #[test]
    fn a_missing_folder_and_a_bad_config_exit_2() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("absent");
        assert_eq!(call(&["inventory", missing.to_str().unwrap()]).0, 2);
        let cfg = dir.path().join("p.cfg");
        std::fs::write(&cfg, "max_files = lots\n").unwrap();
        let (code, _, err) = call(&[
            "--config",
            cfg.to_str().unwrap(),
            "inventory",
            dir.path().to_str().unwrap(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("configuration"));
    }

    #[test]
    fn the_config_file_changes_the_bounds() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..3 {
            std::fs::write(dir.path().join(format!("f{n}.txt")), "x").unwrap();
        }
        let cfg = dir.path().join("p.cfg");
        std::fs::write(&cfg, "max_files = 2\n").unwrap();
        let (code, _, err) = call(&[
            "--config",
            cfg.to_str().unwrap(),
            "inventory",
            dir.path().to_str().unwrap(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("number of files"));
    }

    /// A job with invented material values: one line with a width and one with a target.
    const STACKUP_JOB: &str = r#"{
        "stackup": {"name": "two-layer", "layers": [
            {"kind": "copper", "name": "top", "thickness": {"um": 35.0}},
            {"kind": "core", "name": "core", "thickness_nm": 1600000, "material": "fr4-test"},
            {"kind": "copper", "name": "bottom", "thickness": {"um": 35.0}}]},
        "materials": [{"name": "fr4-test", "dk_points": [[1.0, 4.5], [10.0, 4.3]],
                       "df_points": [[1.0, 0.02], [10.0, 0.022]]}],
        "lines": [
            {"name": "fixed", "kind": "microstrip", "layer": "top", "width_nm": 3000000},
            {"name": "fifty", "kind": "microstrip", "layer": "top", "target_ohm": 50.0}]
    }"#;

    #[test]
    fn stackup_prints_the_lines_as_text_and_as_json() {
        let dir = tempfile::tempdir().unwrap();
        let job = dir.path().join("job.json");
        std::fs::write(&job, STACKUP_JOB).unwrap();
        let path = job.to_str().unwrap();
        let (code, text, err) = call(&["stackup", path, "--freq-ghz", "2"]);
        assert_eq!((code, err.is_empty()), (0, true), "{err}");
        assert!(text.contains("line fixed (microstrip)") && text.contains("impedance:"));
        assert!(text.contains("(found from the target impedance)"));
        let (code, json, _) = call(&["stackup", "--json", "--freq-ghz", "2", path]);
        assert_eq!(code, 0);
        assert!(json.starts_with('{') && json.ends_with("}\n"));
        assert!(json.contains("\"impedance_ohm\""));
        let (code, german, _) = call(&["stackup", path, "--freq-ghz", "20", "--lang", "de"]);
        assert_eq!(code, 1);
        assert!(german.contains("Frequenz außerhalb der veröffentlichten Punkte"));
    }

    #[test]
    fn stackup_reads_the_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        let job = dir.path().join("job.json");
        std::fs::write(&job, STACKUP_JOB).unwrap();
        let path = job.to_str().unwrap();
        let settings = dir.path().join("s.cfg");
        std::fs::write(&settings, "synth_max_iterations = 1\n").unwrap();
        let (code, text, _) = call(&[
            "stackup",
            path,
            "--freq-ghz",
            "2",
            "--settings",
            settings.to_str().unwrap(),
        ]);
        // one step is not enough to find the width of the second line
        assert_eq!(code, 1);
        assert!(text.contains("PHOENIX-STACK-008"));
        std::fs::write(&settings, "nonsense\n").unwrap();
        let (code, _, err) = call(&[
            "stackup",
            path,
            "--freq-ghz",
            "2",
            "--settings",
            settings.to_str().unwrap(),
        ]);
        assert_eq!(code, 2);
        assert!(err.contains("settings"));
    }

    #[test]
    fn stackup_arguments_and_files_that_cannot_be_used_exit_2() {
        let dir = tempfile::tempdir().unwrap();
        let job = dir.path().join("job.json");
        std::fs::write(&job, STACKUP_JOB).unwrap();
        let path = job.to_str().unwrap();
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, "not json").unwrap();
        let missing = dir.path().join("absent.json");
        let (bad_path, missing_path) = (bad.to_str().unwrap(), missing.to_str().unwrap());
        for list in [
            &["stackup"][..],
            &["stackup", path],
            &["stackup", path, "--freq-ghz"],
            &["stackup", path, "--freq-ghz", "fast"],
            &["stackup", path, "--freq-ghz", "2", "--lang", "xx"],
            &["stackup", path, "--freq-ghz", "2", "--nonsense"],
            &["stackup", "--freq-ghz", "2", bad_path],
            &["stackup", "--freq-ghz", "2", missing_path],
        ] {
            let (code, out, err) = call(list);
            assert_eq!((code, out.is_empty()), (2, true), "{list:?}");
            assert!(!err.is_empty(), "{list:?}");
        }
    }
}
