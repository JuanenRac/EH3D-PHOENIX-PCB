#!/usr/bin/env python3
# =============================================================================
# EH3D-PHOENIX-PCB - tools/make_kit.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""Writes the starter kit in `kit/`: the libraries, the sample boards and schematics, the expected
output of the user programs and the catalogue (`kit/manifest.json`).

    python tools/make_kit.py            # writes the files
    python tools/make_kit.py --check    # exit 1 when a file of the kit is not what this script writes

Everything in the kit is made here or written by hand for this project. No file, drawing or table of
another program, maker or collection is read or copied. The sizes are nominal figures of public
standards (named next to each family) and the land patterns are worked out here from them: nothing
was compared with a datasheet or soldered, so every part is marked as not verified.

All lengths in this file are integer micrometres. The user programs (`kit/ulp/*.ulp`), the rule
files (`kit/rules/`) and the export profiles (`kit/cam/`) are written by hand; this script only
reads them to list them and to compute their hashes.
"""
from __future__ import annotations

import hashlib
import heapq
import json
import math
import sys
from pathlib import Path
from xml.sax.saxutils import escape, quoteattr

ROOT = Path(__file__).resolve().parents[1]
KIT = ROOT / "kit"
LICENSE_ID = "GPL-3.0-or-later"
PROVENANCE = "own"
BOARD_LIBRARY = "eh3d-kit"

LANGS = ("en", "es", "de", "fr", "it", "ja", "zh", "pt")


def mm(value: int) -> str:
    """Micrometres as the shortest exact decimal in millimetres."""
    sign = "-" if value < 0 else ""
    whole, frac = divmod(abs(value), 1000)
    if frac == 0:
        return f"{sign}{whole}"
    return f"{sign}{whole}." + f"{frac:03d}".rstrip("0")


# --------------------------------------------------------------------------------------------------
# footprints

def smd(name, x, y, dx, dy):
    return {"kind": "smd", "name": name, "x": x, "y": y, "dx": dx, "dy": dy}


def tht(name, x, y, drill, diameter, square=False):
    return {"kind": "pad", "name": name, "x": x, "y": y, "drill": drill, "diameter": diameter, "square": square}


def line(layer, x1, y1, x2, y2, width=150):
    return {"kind": "wire", "layer": layer, "x1": x1, "y1": y1, "x2": x2, "y2": y2, "width": width}


def box(layer, hx, hy, width=150):
    return [line(layer, -hx, -hy, hx, -hy, width), line(layer, hx, -hy, hx, hy, width),
            line(layer, hx, hy, -hx, hy, width), line(layer, -hx, hy, -hx, -hy, width)]


def texts(top):
    return [{"kind": "text", "layer": 25, "x": 0, "y": top, "size": 1000, "text": ">NAME"},
            {"kind": "text", "layer": 27, "x": 0, "y": -top - 1000, "size": 1000, "text": ">VALUE"}]


# name -> (description, source of the sizes, items)
PACKAGES: dict[str, tuple[str, str, list]] = {}


def package(name, description, source, items):
    PACKAGES[name] = (description, source, items)


CHIP_SOURCE = "EIA / IEC 60115-8 chip size codes (nominal body); pads worked out for this kit"
# code: (pad centre, pad width, pad height, body half length, body half width)
CHIPS = {"0402": (480, 560, 620, 500, 250), "0603": (800, 900, 950, 800, 400),
         "0805": (950, 1150, 1450, 1000, 625), "1206": (1500, 1150, 1750, 1600, 800)}

for code, (centre, dx, dy, half, wide) in CHIPS.items():
    gap = centre - dx // 2
    edge = dy // 2 + 300
    items = [smd("1", -centre, 0, dx, dy), smd("2", centre, 0, dx, dy)]
    items += [line(21, -gap, edge, gap, edge), line(21, -gap, -edge, gap, -edge)]
    items += box(51, half, wide, 100)
    package(f"CHIP-{code}", f"Chip resistor or capacitor {code}", CHIP_SOURCE, items + texts(edge + 300))

for code in ("0603", "0805"):
    centre, dx, dy, half, wide = CHIPS[code]
    gap = centre - dx // 2
    edge = dy // 2 + 300
    mark = centre + dx // 2 + 350
    items = [smd("1", -centre, 0, dx, dy), smd("2", centre, 0, dx, dy)]
    items += [line(21, -gap, edge, gap, edge), line(21, -gap, -edge, gap, -edge), line(21, -mark, -edge, -mark, edge)]
    items += box(51, half, wide, 100)
    package(f"LED-{code}", f"Chip LED {code}, pad 1 is the cathode (marked)", CHIP_SOURCE, items + texts(edge + 300))

for pitch, label in ((7620, "7.62"), (10160, "10.16")):
    half = pitch // 2
    body = half - 1300
    items = [tht("1", -half, 0, 800, 1600, True), tht("2", half, 0, 800, 1600)]
    items += box(21, body, 1200)
    package(f"R-AXIAL-{label}", f"Axial resistor, lead pitch {label} mm", "0.1 inch lead grid; pads worked out for this kit", items + texts(1500))

for pitch, label in ((2500, "2.5"), (5000, "5.0")):
    half = pitch // 2
    items = [tht("1", -half, 0, 700, 1400, True), tht("2", half, 0, 700, 1400)]
    items += [line(21, -half - 1000, 1100, half + 1000, 1100), line(21, -half - 1000, -1100, half + 1000, -1100)]
    package(f"C-DISC-{label}", f"Disc or film capacitor, lead pitch {label} mm", "IEC 60384 usual lead pitches; pads worked out for this kit", items + texts(1400))

for name, pitch, drill, diameter, radius in (("C-ELEC-D5", 2000, 700, 1300, 2750), ("C-ELEC-D8", 3500, 800, 1600, 4250)):
    half = pitch // 2
    items = [tht("1", -half, 0, drill, diameter, True), tht("2", half, 0, drill, diameter),
             {"kind": "circle", "layer": 21, "x": 0, "y": 0, "radius": radius, "width": 150}]
    package(name, f"Radial electrolytic capacitor {radius * 2 // 1000} mm class, pad 1 is positive", "IEC 60384-4 usual can sizes; pads worked out for this kit", items + texts(radius + 300))

for name, drill, diameter, radius in (("LED-THT-3MM", 800, 1600, 2000), ("LED-THT-5MM", 900, 1800, 3000)):
    items = [tht("1", -1270, 0, drill, diameter, True), tht("2", 1270, 0, drill, diameter),
             {"kind": "circle", "layer": 21, "x": 0, "y": 0, "radius": radius + 200, "width": 150}]
    package(name, f"Through hole LED {name.rsplit('-', 1)[1].lower()}, lead pitch 2.54 mm, pad 1 is the cathode", "T-1 and T-1 3/4 lamp sizes; pads worked out for this kit", items + texts(radius + 500))

package("SOT-23", "Small outline transistor, 3 pins", "JEDEC TO-236 nominal sizes; pads worked out for this kit",
        [smd("1", -950, -1000, 700, 900), smd("2", 950, -1000, 700, 900), smd("3", 0, 1000, 700, 900),
         line(21, -1600, -200, -1600, 900), line(21, 1600, -200, 1600, 900)] + box(51, 1450, 650, 100) + texts(1900))

package("SOT-223", "Small outline transistor with a tab, 4 pins", "JEDEC TO-261 nominal sizes; pads worked out for this kit",
        [smd("1", -2300, -3100, 1000, 1800), smd("2", 0, -3100, 1000, 1800), smd("3", 2300, -3100, 1000, 1800),
         smd("4", 0, 3100, 3400, 1800), line(21, -3500, -1700, -3500, 1700), line(21, 3500, -1700, 3500, 1700)]
        + box(51, 3250, 1750, 100) + texts(4300))

for name, centre, dx, dy, half in (("SOD-123", 1650, 1000, 1100, 1350), ("SOD-323", 1100, 750, 600, 850)):
    gap = centre - dx // 2
    edge = dy // 2 + 300
    mark = centre + dx // 2 + 350
    package(name, f"Small outline diode {name}, pad 1 is the cathode (marked)", f"JEDEC {name} nominal sizes; pads worked out for this kit",
            [smd("1", -centre, 0, dx, dy), smd("2", centre, 0, dx, dy), line(21, -gap, edge, gap, edge),
             line(21, -gap, -edge, gap, -edge), line(21, -mark, -edge, -mark, edge)] + box(51, half, dy // 2, 100) + texts(edge + 300))


def dual_row(name, pins, pitch, centre, dx, dy, body_half_x, description, source):
    half = pins // 2
    top = (half - 1) * pitch // 2
    items = []
    for i in range(half):
        items.append(smd(str(i + 1), -centre, top - i * pitch, dx, dy))
    for i in range(half):
        items.append(smd(str(half + i + 1), centre, -top + i * pitch, dx, dy))
    end = top + dy // 2 + 350
    items += [line(21, -body_half_x, end, body_half_x, end), line(21, -body_half_x, -end, body_half_x, -end),
              {"kind": "circle", "layer": 21, "x": -centre - dx // 2 - 450, "y": top, "radius": 150, "width": 150}]
    items += box(51, body_half_x, end - 200, 100)
    package(name, description, source, items + texts(end + 300))


for pins in (8, 14, 16):
    dual_row(f"SOIC-{pins}", pins, 1270, 2700, 1550, 600, 1950, f"Small outline package, {pins} pins, pitch 1.27 mm, body 3.9 mm",
             "JEDEC MS-012 nominal sizes; pads worked out for this kit")
for pins in (14, 16, 20):
    dual_row(f"TSSOP-{pins}", pins, 650, 2850, 1350, 400, 2200, f"Thin shrink small outline package, {pins} pins, pitch 0.65 mm, body 4.4 mm",
             "JEDEC MO-153 nominal sizes; pads worked out for this kit")

HEADER_SOURCE = "0.1 inch (2.54 mm) pin strip grid; pads worked out for this kit"
for count in (2, 3, 4, 5, 6, 8, 15, 20):
    first = -(count - 1) * 1270
    items = [tht(str(i + 1), first + i * 2540, 0, 1000, 1700, i == 0) for i in range(count)]
    items += box(21, count * 1270, 1270)
    package(f"HDR-1X{count:02d}", f"Pin header, 1 row of {count}, pitch 2.54 mm", HEADER_SOURCE, items + texts(1600))
for count in (3, 5):
    first = -(count - 1) * 1270
    items = []
    for i in range(count):
        items.append(tht(str(2 * i + 1), first + i * 2540, -1270, 1000, 1700, i == 0))
        items.append(tht(str(2 * i + 2), first + i * 2540, 1270, 1000, 1700))
    items += box(21, count * 1270, 2540)
    package(f"HDR-2X{count:02d}", f"Pin header, 2 rows of {count}, pitch 2.54 mm", HEADER_SOURCE, items + texts(2900))

for count in (2, 3):
    first = -(count - 1) * 2500
    items = [tht(str(i + 1), first + i * 5000, 0, 1300, 2400, i == 0) for i in range(count)]
    items += box(21, count * 2500, 3800)
    package(f"TERM-{count}P-5.0", f"Screw terminal, {count} poles, pitch 5.0 mm (generic)", "usual 5.0 mm terminal pitch; generic outline, pads worked out for this kit", items + texts(4100))


def module(name, per_row, row_spacing, description):
    half = row_spacing // 2
    top = (per_row - 1) * 1270
    items = []
    for i in range(per_row):
        items.append(tht(str(i + 1), -half, top - i * 2540, 1000, 1700, i == 0))
    for i in range(per_row):
        items.append(tht(str(per_row + i + 1), half, -top + i * 2540, 1000, 1700))
    items += box(21, half + 1500, top + 1500)
    package(name, description, "indicative only: a generic module on 2.54 mm pin strips, not the drawing of any product", items + texts(top + 1800))


module("MODULE-ESP32-TYPE", 15, 25400, "Module of the ESP32 type, 2 x 15 pins, rows 25.4 mm apart (indicative sizes: measure yours)")
module("MODULE-STM32-TYPE", 20, 15240, "Module of the STM32 type, 2 x 20 pins, rows 15.24 mm apart (indicative sizes: measure yours)")


# --------------------------------------------------------------------------------------------------
# symbols and device sets

def pin(name, x, y, rot=""):
    return {"kind": "pin", "name": name, "x": x, "y": y, "rot": rot}


SYMBOLS: dict[str, list] = {}


def sym_texts(top):
    return [{"kind": "text", "layer": 95, "x": -2540, "y": top, "size": 1778, "text": ">NAME"},
            {"kind": "text", "layer": 96, "x": -2540, "y": -top - 1778, "size": 1778, "text": ">VALUE"}]


def two_pin(name, drawing):
    SYMBOLS[name] = [pin("1", -5080, 0), pin("2", 5080, 0, "R180")] + drawing + sym_texts(2540)


two_pin("RESISTOR", box(94, 2540, 1016, 254))
two_pin("CAPACITOR", [line(94, -2540, 0, -508, 0, 152), line(94, -508, -1524, -508, 1524, 406),
                      line(94, 508, -1524, 508, 1524, 406), line(94, 508, 0, 2540, 0, 152)])
two_pin("CAPACITOR-POL", [line(94, -2540, 0, -508, 0, 152), line(94, -508, -1524, -508, 1524, 406),
                          line(94, 508, -1524, 508, 1524, 406), line(94, 508, 0, 2540, 0, 152),
                          line(94, -2032, 1016, -1016, 1016, 152), line(94, -1524, 508, -1524, 1524, 152)])
two_pin("DIODE", [line(94, -2540, 0, 2540, 0, 152), line(94, -1270, -1270, -1270, 1270, 254),
                  line(94, 1270, -1270, 1270, 1270, 254), line(94, 1270, 1270, -1270, 0, 254),
                  line(94, -1270, 0, 1270, -1270, 254)])
two_pin("LED", SYMBOLS["DIODE"][2:-2] + [line(94, 0, 1778, 1016, 2794, 152), line(94, 1270, 1524, 2286, 2540, 152)])

SYMBOLS["TRANSISTOR-NPN"] = [pin("1", -5080, 0), pin("2", 5080, -2540, "R180"), pin("3", 5080, 2540, "R180"),
                             line(94, -2540, 0, -1270, 0, 152), line(94, -1270, -1778, -1270, 1778, 406),
                             line(94, -1270, 762, 2540, 2540, 152), line(94, -1270, -762, 2540, -2540, 152),
                             line(94, 1524, -2540, 2540, -2540, 152), line(94, 1778, -1524, 2540, -2540, 152)] + sym_texts(3810)
SYMBOLS["TRANSISTOR-NMOS"] = [pin("1", -5080, 0), pin("2", 5080, -2540, "R180"), pin("3", 5080, 2540, "R180"),
                              line(94, -2540, 0, -1270, 0, 152), line(94, -1270, -1778, -1270, 1778, 254),
                              line(94, -508, -2286, -508, 2286, 254), line(94, -508, 2032, 2540, 2540, 152),
                              line(94, -508, -2032, 2540, -2540, 152)] + sym_texts(3810)
SYMBOLS["REGULATOR"] = [pin("3", -7620, 2540), pin("1", -7620, -2540), pin("2", 7620, 2540, "R180"),
                        pin("4", 7620, -2540, "R180")] + box(94, 5080, 5080, 254) + sym_texts(5334)


def box_symbol(name, left, right):
    rows = max(len(left), len(right))
    top = (rows - 1) * 1270
    items = [pin(str(n), -10160, top - i * 2540) for i, n in enumerate(left)]
    items += [pin(str(n), 10160, top - i * 2540, "R180") for i, n in enumerate(right)]
    SYMBOLS[name] = items + box(94, 7620, top + 2540, 254) + sym_texts(top + 2794)


for pins in (8, 14, 16, 20):
    box_symbol(f"IC-{pins}", list(range(1, pins // 2 + 1)), list(range(pins, pins // 2, -1)))
for count in (2, 3, 4, 5, 6, 8, 15, 20):
    box_symbol(f"PINS-1X{count:02d}", list(range(1, count + 1)), [])
for count in (3, 5):
    box_symbol(f"PINS-2X{count:02d}", list(range(1, 2 * count, 2)), list(range(2, 2 * count + 1, 2)))
box_symbol("TERMINAL-2", [1, 2], [])
box_symbol("TERMINAL-3", [1, 2, 3], [])
box_symbol("MODULE-30", list(range(1, 16)), list(range(30, 15, -1)))
box_symbol("MODULE-40", list(range(1, 21)), list(range(40, 20, -1)))

# device set -> (prefix, symbol, {device name: package}, description)
DEVICESETS: dict[str, tuple[str, str, dict[str, str], str]] = {
    "R": ("R", "RESISTOR", {**{c: f"CHIP-{c}" for c in CHIPS}, "AXIAL-7.62": "R-AXIAL-7.62", "AXIAL-10.16": "R-AXIAL-10.16"}, "Resistor"),
    "C": ("C", "CAPACITOR", {**{c: f"CHIP-{c}" for c in CHIPS}, "DISC-2.5": "C-DISC-2.5", "DISC-5.0": "C-DISC-5.0"}, "Capacitor"),
    "C-POL": ("C", "CAPACITOR-POL", {"D5": "C-ELEC-D5", "D8": "C-ELEC-D8"}, "Electrolytic capacitor, pin 1 positive"),
    "LED": ("D", "LED", {"0603": "LED-0603", "0805": "LED-0805", "3MM": "LED-THT-3MM", "5MM": "LED-THT-5MM"}, "LED, pin 1 cathode, pin 2 anode"),
    "DIODE": ("D", "DIODE", {"SOD-123": "SOD-123", "SOD-323": "SOD-323"}, "Diode, pin 1 cathode, pin 2 anode"),
    "NPN": ("Q", "TRANSISTOR-NPN", {"SOT-23": "SOT-23"}, "NPN transistor, 1 base, 2 emitter, 3 collector (check your part)"),
    "NMOS": ("Q", "TRANSISTOR-NMOS", {"SOT-23": "SOT-23"}, "N channel MOSFET, 1 gate, 2 source, 3 drain (check your part)"),
    "REGULATOR": ("U", "REGULATOR", {"SOT-223": "SOT-223"}, "Linear regulator, 1 ground, 2 and 4 output, 3 input (check your part)"),
    "IC8": ("U", "IC-8", {"SOIC": "SOIC-8"}, "Generic integrated circuit, 8 pins"),
    "IC14": ("U", "IC-14", {"SOIC": "SOIC-14", "TSSOP": "TSSOP-14"}, "Generic integrated circuit, 14 pins"),
    "IC16": ("U", "IC-16", {"SOIC": "SOIC-16", "TSSOP": "TSSOP-16"}, "Generic integrated circuit, 16 pins"),
    "IC20": ("U", "IC-20", {"TSSOP": "TSSOP-20"}, "Generic integrated circuit, 20 pins"),
    **{f"HDR-1X{n:02d}": ("J", f"PINS-1X{n:02d}", {"": f"HDR-1X{n:02d}"}, f"Pin header 1 x {n}") for n in (2, 3, 4, 5, 6, 8, 15, 20)},
    **{f"HDR-2X{n:02d}": ("J", f"PINS-2X{n:02d}", {"": f"HDR-2X{n:02d}"}, f"Pin header 2 x {n}") for n in (3, 5)},
    "TERMINAL-2P": ("X", "TERMINAL-2", {"": "TERM-2P-5.0"}, "Screw terminal, 2 poles"),
    "TERMINAL-3P": ("X", "TERMINAL-3", {"": "TERM-3P-5.0"}, "Screw terminal, 3 poles"),
    "MODULE-ESP32-TYPE": ("M", "MODULE-30", {"": "MODULE-ESP32-TYPE"}, "Module of the ESP32 type (indicative)"),
    "MODULE-STM32-TYPE": ("M", "MODULE-40", {"": "MODULE-STM32-TYPE"}, "Module of the STM32 type (indicative)"),
}

LIBRARIES = {
    "passives": ["R", "C", "C-POL"],
    "semiconductors": ["LED", "DIODE", "NPN", "NMOS", "REGULATOR"],
    "ics": ["IC8", "IC14", "IC16", "IC20"],
    "connectors": [n for n in DEVICESETS if n.startswith(("HDR-", "TERMINAL-"))],
    "modules": ["MODULE-ESP32-TYPE", "MODULE-STM32-TYPE"],
}


# --------------------------------------------------------------------------------------------------
# XML

def tag(name, attrs, children=None, text=None):
    head = name + "".join(f" {k}={quoteattr(str(v))}" for k, v in attrs)
    if text is not None:
        return [f"<{head}>{escape(text)}</{name}>"]
    if not children:
        return [f"<{head}/>"]
    return [f"<{head}>"] + [row for child in children for row in child] + [f"</{name}>"]


def item_xml(item):
    kind = item["kind"]
    if kind == "smd":
        return tag("smd", [("name", item["name"]), ("x", mm(item["x"])), ("y", mm(item["y"])), ("dx", mm(item["dx"])), ("dy", mm(item["dy"])), ("layer", 1)])
    if kind == "pad":
        attrs = [("name", item["name"]), ("x", mm(item["x"])), ("y", mm(item["y"])), ("drill", mm(item["drill"])), ("diameter", mm(item["diameter"]))]
        if item["square"]:
            attrs.append(("shape", "square"))
        return tag("pad", attrs)
    if kind == "wire":
        return tag("wire", [("x1", mm(item["x1"])), ("y1", mm(item["y1"])), ("x2", mm(item["x2"])), ("y2", mm(item["y2"])), ("width", mm(item["width"])), ("layer", item["layer"])])
    if kind == "circle":
        return tag("circle", [("x", mm(item["x"])), ("y", mm(item["y"])), ("radius", mm(item["radius"])), ("width", mm(item["width"])), ("layer", item["layer"])])
    if kind == "text":
        return tag("text", [("x", mm(item["x"])), ("y", mm(item["y"])), ("size", mm(item["size"])), ("layer", item["layer"])], text=item["text"])
    if kind == "pin":
        attrs = [("name", item["name"]), ("x", mm(item["x"])), ("y", mm(item["y"])), ("length", "short"), ("direction", "pas")]
        if item["rot"]:
            attrs.append(("rot", item["rot"]))
        return tag("pin", attrs)
    raise ValueError(kind)


def layers_xml(numbers):
    names = {1: "Top", 16: "Bottom", 20: "Dimension", 21: "tPlace", 22: "bPlace", 25: "tNames", 27: "tValues", 29: "tStop",
             30: "bStop", 31: "tCream", 32: "bCream", 51: "tDocu", 52: "bDocu", 91: "Nets", 94: "Symbols", 95: "Names", 96: "Values"}
    return tag("layers", [], [tag("layer", [("number", n), ("name", names[n]), ("color", 1), ("fill", 1), ("visible", "yes"), ("active", "yes")]) for n in numbers])


def library_sections(setnames, extra_packages=None):
    """The packages, symbols and device sets that the named device sets use."""
    package_names, symbol_names = [], []
    for name in setnames:
        _, symbol, devices, _ = DEVICESETS[name]
        if symbol not in symbol_names:
            symbol_names.append(symbol)
        for pkg in devices.values():
            if pkg not in package_names:
                package_names.append(pkg)
    packages = []
    for name in package_names:
        description, source, items = PACKAGES[name]
        text = f"{description}. Sizes: {source}. Not verified against a datasheet."
        packages.append(tag("package", [("name", name)], [tag("description", [], text=text)] + [item_xml(i) for i in items]))
    symbols = [tag("symbol", [("name", n)], [item_xml(i) for i in SYMBOLS[n]]) for n in symbol_names]
    sets = []
    for name in setnames:
        prefix, symbol, devices, description = DEVICESETS[name]
        pins = [i["name"] for i in SYMBOLS[symbol] if i["kind"] == "pin"]
        device_rows = []
        for device, pkg in devices.items():
            pads = {i["name"] for i in PACKAGES[pkg][2] if i["kind"] in ("pad", "smd")}
            if pads != set(pins):
                raise ValueError(f"{name} {device}: pins {sorted(pins)} and pads {sorted(pads)} differ")
            connects = tag("connects", [], [tag("connect", [("gate", "G$1"), ("pin", p), ("pad", p)]) for p in pins])
            technologies = tag("technologies", [], [tag("technology", [("name", "")])])
            device_rows.append(tag("device", [("name", device), ("package", pkg)], [connects, technologies]))
        sets.append(tag("deviceset", [("name", name), ("prefix", prefix), ("uservalue", "yes")],
                        [tag("description", [], text=description), tag("gates", [], [tag("gate", [("name", "G$1"), ("symbol", symbol), ("x", 0), ("y", 0)])]),
                         tag("devices", [], device_rows)]))
    return [tag("packages", [], packages), tag("symbols", [], symbols), tag("devicesets", [], sets)]


def document(body):
    rows = ['<?xml version="1.0" encoding="utf-8"?>', "<!-- EH3D-PHOENIX-PCB starter kit. SPDX-License-Identifier: GPL-3.0-or-later. Own work, not verified. -->"]
    rows += tag("eagle", [("version", "9.6.2")], [tag("drawing", [], body)])
    return "\n".join(rows) + "\n"


def library_file(name):
    description = tag("description", [], text=f"EH3D-PHOENIX-PCB starter kit, {name}. Own work, GPL-3.0-or-later. Not verified.")
    return document([tag("settings", []), layers_xml([1, 16, 21, 22, 25, 27, 29, 30, 31, 32, 51, 52, 94, 95, 96]),
                     tag("library", [], [description] + library_sections(LIBRARIES[name]))])


# --------------------------------------------------------------------------------------------------
# sample designs
# part: (reference, device set, device, value, x, y, rotation, sheet column, sheet row)

def P(ref, deviceset, device, value, x, y, rot=0):
    return {"ref": ref, "set": deviceset, "device": device, "value": value, "x": x, "y": y, "rot": rot}


def pins_to(ref, numbers):
    return [f"{ref}.{n}" for n in numbers]


DESIGNS: list[dict] = []


def design(name, title, size, parts, nets, width=300, widths=None):
    DESIGNS.append({"name": name, "title": title, "size": size, "parts": parts, "nets": nets, "width": width, "widths": widths or {}})


design("led_indicator", "LED with its resistor", (25000, 15000),
       [P("J1", "HDR-1X02", "", "PWR", 4000, 7500, 90), P("R1", "R", "0805", "1k", 11000, 10000), P("D1", "LED", "0805", "GREEN", 18000, 6000)],
       {"VCC": ["J1.1", "R1.1"], "LEDA": ["R1.2", "D1.2"], "GND": ["D1.1", "J1.2"]}, 400)

design("voltage_divider", "Voltage divider with a test header", (25000, 18000),
       [P("J1", "HDR-1X03", "", "SIG", 4000, 9000, 90), P("R1", "R", "0603", "10k", 13000, 12000), P("R2", "R", "0603", "4k7", 17000, 6000)],
       {"VIN": ["J1.1", "R1.1"], "TAP": ["R1.2", "R2.1", "J1.2"], "GND": ["R2.2", "J1.3"]})

design("led_bar", "Four LEDs with their resistors", (32000, 30000),
       [P("J1", "HDR-1X05", "", "LEDS", 4000, 15000, 90)]
       + [P(f"R{i + 1}", "R", "0603", "330", 14000, 24000 - i * 6000) for i in range(4)]
       + [P(f"D{i + 1}", "LED", "0603", "RED", 22000, 24000 - i * 6000) for i in range(4)],
       {"IN1": ["J1.1", "R1.1"], "IN2": ["J1.2", "R2.1"], "IN3": ["J1.3", "R3.1"], "IN4": ["J1.4", "R4.1"],
        "A1": ["R1.2", "D1.2"], "A2": ["R2.2", "D2.2"], "A3": ["R3.2", "D3.2"], "A4": ["R4.2", "D4.2"],
        "GND": ["D1.1", "D2.1", "D3.1", "D4.1", "J1.5"]})

design("transistor_switch", "Low side switch with a clamp diode", (34000, 24000),
       [P("J1", "HDR-1X03", "", "CTRL", 4000, 12000, 90), P("R1", "R", "0805", "1k", 11000, 15000), P("R2", "R", "0805", "100k", 13000, 7000),
        P("Q1", "NPN", "SOT-23", "NPN", 19000, 12000), P("D1", "DIODE", "SOD-123", "CLAMP", 23000, 18000), P("X1", "TERMINAL-2P", "", "LOAD", 28000, 10000, 90)],
       {"CTRL": ["J1.2", "R1.1"], "BASE": ["R1.2", "Q1.1", "R2.1"], "GND": ["R2.2", "Q1.2", "J1.3"],
        "COIL": ["Q1.3", "D1.2", "X1.1"], "VCC": ["J1.1", "D1.1", "X1.2"]})

design("soic8_decoupled", "Eight pin circuit with its capacitor and two headers", (32000, 22000),
       [P("J1", "HDR-1X04", "", "LEFT", 4000, 11000, 90), P("U1", "IC8", "SOIC", "GENERIC8", 16000, 11000),
        P("C1", "C", "0603", "100n", 17000, 17500), P("J2", "HDR-1X04", "", "RIGHT", 28000, 11000, 90)],
       {"S1": ["J1.1", "U1.1"], "S2": ["J1.2", "U1.2"], "S3": ["J1.3", "U1.3"], "GND": ["J1.4", "U1.4", "C1.1"],
        "S5": ["U1.5", "J2.1"], "S6": ["U1.6", "J2.2"], "S7": ["U1.7", "J2.3"], "VCC": ["U1.8", "C1.2", "J2.4"]})

design("tssop14_breakout", "Breakout of a 14 pin thin package", (36000, 30000),
       [P("J1", "HDR-1X08", "", "LEFT", 4000, 15000, 270), P("U1", "IC14", "TSSOP", "GENERIC14", 18000, 15000),
        P("J2", "HDR-1X08", "", "RIGHT", 32000, 15000, 90), P("C1", "C", "0402", "100n", 18000, 23000)],
       {**{f"P{n}": [f"J1.{n}", f"U1.{n}"] for n in range(1, 7)}, "GND": ["J1.7", "U1.7", "C1.1", "J1.8"],
        **{f"P{n}": [f"U1.{n}", f"J2.{n - 7}"] for n in range(8, 14)}, "VCC": ["U1.14", "C1.2", "J2.7", "J2.8"]}, 200)

design("soic16_breakout", "Breakout of a 16 pin small outline package", (38000, 32000),
       [P("J1", "HDR-1X08", "", "LEFT", 4000, 16000, 270), P("U1", "IC16", "SOIC", "GENERIC16", 19000, 16000),
        P("J2", "HDR-1X08", "", "RIGHT", 34000, 16000, 90), P("C1", "C", "0805", "100n", 19000, 25000)],
       {**{f"P{n}": [f"J1.{n}", f"U1.{n}"] for n in range(1, 8)}, "GND": ["J1.8", "U1.8", "C1.1"],
        **{f"P{n}": [f"U1.{n}", f"J2.{n - 8}"] for n in range(9, 16)}, "VCC": ["U1.16", "C1.2", "J2.8"]}, 250)

design("tht_led_pair", "Two LEDs with through hole parts", (36000, 28000),
       [P("J1", "HDR-1X02", "", "PWR", 4000, 14000, 90), P("C1", "C-POL", "D5", "10u", 10000, 14000, 270),
        P("R1", "R", "AXIAL-10.16", "470", 20000, 21000), P("R2", "R", "AXIAL-7.62", "470", 20000, 7000),
        P("D1", "LED", "3MM", "YELLOW", 30000, 20000), P("D2", "LED", "5MM", "BLUE", 30000, 8000)],
       {"VCC": ["J1.1", "C1.1", "R1.1", "R2.1"], "A1": ["R1.2", "D1.2"], "A2": ["R2.2", "D2.2"],
        "GND": ["J1.2", "C1.2", "D1.1", "D2.1"]}, 500)

design("power_input", "Protected power input with an indicator", (44000, 26000),
       [P("X1", "TERMINAL-2P", "", "IN", 5500, 13000, 90), P("D1", "DIODE", "SOD-123", "SERIES", 14000, 18000),
        P("C1", "C-POL", "D8", "220u", 21000, 13000, 270), P("R1", "R", "1206", "2k2", 30000, 18000),
        P("D2", "LED", "0805", "GREEN", 30000, 8000), P("J1", "HDR-1X02", "", "OUT", 39000, 13000, 90)],
       {"VIN": ["X1.1", "D1.2"], "VOUT": ["D1.1", "C1.1", "R1.1", "J1.1"], "LEDA": ["R1.2", "D2.2"],
        "GND": ["X1.2", "C1.2", "D2.1", "J1.2"]}, 800, {"LEDA": 300})

design("regulator_sot223", "Linear regulator with its capacitors", (42000, 26000),
       [P("X1", "TERMINAL-2P", "", "IN", 5500, 13000, 90), P("C1", "C", "0805", "10u", 13000, 13000, 90),
        P("U1", "REGULATOR", "SOT-223", "REG", 22000, 13000), P("C2", "C", "0805", "22u", 31000, 13000, 90),
        P("J1", "HDR-1X03", "", "OUT", 37000, 13000, 90)],
       {"VIN": ["X1.1", "C1.2", "U1.3"], "VOUT": ["U1.2", "U1.4", "C2.2", "J1.1"],
        "GND": ["X1.2", "C1.1", "U1.1", "C2.1", "J1.2", "J1.3"]}, 600)

design("esp32_carrier", "Carrier of a module of the ESP32 type", (52000, 50000),
       [P("M1", "MODULE-ESP32-TYPE", "", "ESP32-TYPE", 26000, 25000), P("J1", "HDR-1X15", "", "LEFT", 5000, 25000, 270),
        P("J2", "HDR-1X15", "", "RIGHT", 47000, 25000, 90), P("C1", "C", "0805", "10u", 24000, 45000)],
       {"V3V3": ["M1.1", "J1.1", "C1.1"], "GND": ["M1.2", "J1.2", "C1.2"],
        **{f"IO{n}": [f"M1.{n}", f"J1.{n}"] for n in range(3, 16)},
        **{f"IO{n}": [f"M1.{n}", f"J2.{n - 15}"] for n in range(16, 31)}})

design("stm32_carrier", "Carrier of a module of the STM32 type", (46000, 60000),
       [P("M1", "MODULE-STM32-TYPE", "", "STM32-TYPE", 23000, 30000), P("J1", "HDR-1X20", "", "LEFT", 5000, 30000, 270),
        P("J2", "HDR-1X20", "", "RIGHT", 41000, 30000, 90), P("C1", "C", "0603", "100n", 22000, 57000)],
       {"V3V3": ["M1.1", "J1.1", "C1.1"], "GND": ["M1.2", "J1.2", "C1.2"],
        **{f"IO{n}": [f"M1.{n}", f"J1.{n}"] for n in range(3, 21)},
        **{f"IO{n}": [f"M1.{n}", f"J2.{n - 20}"] for n in range(21, 41)}})


# --------------------------------------------------------------------------------------------------
# placement and routing

CLEARANCE = 200
EDGE_CLEARANCE = 500
VIA_DRILL = 400
VIA_DIAMETER = 800
GRID = 250
VIA_COST = 12.0


def turn(x, y, rot):
    return {0: (x, y), 90: (-y, x), 180: (-x, -y), 270: (y, -x)}[rot]


def placed_pads(d):
    """Every pad of a design in the order of the parts and of the pads of their package."""
    net_of = {pin_id: net for net, pins in d["nets"].items() for pin_id in pins}
    pads = []
    for part in d["parts"]:
        pkg = DEVICESETS[part["set"]][2][part["device"]]
        for item in PACKAGES[pkg][2]:
            if item["kind"] not in ("pad", "smd"):
                continue
            dx, dy = turn(item["x"], item["y"], part["rot"])
            if item["kind"] == "smd":
                w, h, drill = item["dx"], item["dy"], 0
            else:
                w = h = item["diameter"]
                drill = item["drill"]
            if part["rot"] in (90, 270):
                w, h = h, w
            pads.append({"ref": part["ref"], "number": item["name"], "x": part["x"] + dx, "y": part["y"] + dy, "w": w, "h": h,
                         "drill": drill, "square": item.get("square", False), "smd": item["kind"] == "smd",
                         "net": net_of.get(f"{part['ref']}.{item['name']}", "")})
    return pads


def point_segment(px, py, ax, ay, bx, by):
    abx, aby = bx - ax, by - ay
    length = abx * abx + aby * aby
    t = 0.0 if length == 0 else max(0.0, min(1.0, ((px - ax) * abx + (py - ay) * aby) / length))
    return math.hypot(px - ax - t * abx, py - ay - t * aby)


def point_rect(px, py, pad):
    dx = max(abs(px - pad["x"]) - pad["w"] / 2, 0.0)
    dy = max(abs(py - pad["y"]) - pad["h"] / 2, 0.0)
    return math.hypot(dx, dy)


class Router:
    """A plain grid search on the two copper layers: enough for the small boards of the kit."""

    def __init__(self, d, pads):
        self.size = d["size"]
        self.pads = pads
        self.wires = []  # (net, layer, x1, y1, x2, y2, width)
        self.vias = []  # (net, x, y)

    def near(self, net, layer, box_, half):
        """The obstacles of other nets that a track of half width `half` inside the box could come too close to."""
        x0, x1, y0, y1 = box_
        out = []
        for pad in self.pads:
            if pad["net"] == net or (pad["smd"] and layer != 1):
                continue
            rx, ry = pad["w"] / 2 + half + CLEARANCE, pad["h"] / 2 + half + CLEARANCE
            if x0 <= pad["x"] + rx and x1 >= pad["x"] - rx and y0 <= pad["y"] + ry and y1 >= pad["y"] - ry:
                out.append(("pad", pad))
        for wire in self.wires:
            other, at, ax, ay, bx, by, w = wire
            reach = half + w / 2 + CLEARANCE
            if other != net and at == layer and x0 <= max(ax, bx) + reach and x1 >= min(ax, bx) - reach and y0 <= max(ay, by) + reach and y1 >= min(ay, by) - reach:
                out.append(("wire", wire))
        reach = half + VIA_DIAMETER / 2 + CLEARANCE
        for other, vx, vy in self.vias:
            if other != net and x0 <= vx + reach and x1 >= vx - reach and y0 <= vy + reach and y1 >= vy - reach:
                out.append(("via", (vx, vy)))
        return out

    def free_at(self, obstacles, x, y, half):
        width, height = self.size
        if x - half < EDGE_CLEARANCE or y - half < EDGE_CLEARANCE or x + half > width - EDGE_CLEARANCE or y + half > height - EDGE_CLEARANCE:
            return False
        for kind, item in obstacles:
            if kind == "pad" and point_rect(x, y, item) < half + CLEARANCE:
                return False
            if kind == "wire" and point_segment(x, y, item[2], item[3], item[4], item[5]) < half + item[6] / 2 + CLEARANCE:
                return False
            if kind == "via" and math.hypot(x - item[0], y - item[1]) < half + VIA_DIAMETER / 2 + CLEARANCE:
                return False
        return True

    def clear(self, net, layer, x, y, half):
        return self.free_at(self.near(net, layer, (x, x, y, y), half), x, y, half)

    def blocked(self, net, layer, half):
        """The grid nodes where the centre of a track of this half width may not be: drawn once for each search."""
        out = set()

        def mark(x0, x1, y0, y1, inside):
            for gx in range(math.ceil(x0 / GRID), math.floor(x1 / GRID) + 1):
                for gy in range(math.ceil(y0 / GRID), math.floor(y1 / GRID) + 1):
                    if (gx, gy) not in out and inside(gx * GRID, gy * GRID):
                        out.add((gx, gy))

        for pad in self.pads:
            if pad["net"] == net or (pad["smd"] and layer != 1):
                continue
            reach = half + CLEARANCE
            mark(pad["x"] - pad["w"] / 2 - reach, pad["x"] + pad["w"] / 2 + reach, pad["y"] - pad["h"] / 2 - reach, pad["y"] + pad["h"] / 2 + reach,
                 lambda x, y, pad=pad, reach=reach: point_rect(x, y, pad) < reach)
        for other, at, x1, y1, x2, y2, w in self.wires:
            if other == net or at != layer:
                continue
            reach = half + w / 2 + CLEARANCE
            mark(min(x1, x2) - reach, max(x1, x2) + reach, min(y1, y2) - reach, max(y1, y2) + reach,
                 lambda x, y, a=(x1, y1, x2, y2), reach=reach: point_segment(x, y, *a) < reach)
        for other, vx, vy in self.vias:
            if other == net:
                continue
            reach = half + VIA_DIAMETER / 2 + CLEARANCE
            mark(vx - reach, vx + reach, vy - reach, vy + reach, lambda x, y, c=(vx, vy), reach=reach: math.hypot(x - c[0], y - c[1]) < reach)
        return out

    def segment_clear(self, net, layer, a, b, half):
        steps = max(1, int(math.hypot(b[0] - a[0], b[1] - a[1]) / 50) + 1)
        obstacles = self.near(net, layer, (min(a[0], b[0]), max(a[0], b[0]), min(a[1], b[1]), max(a[1], b[1])), half + 5)
        return all(self.free_at(obstacles, a[0] + (b[0] - a[0]) * i / steps, a[1] + (b[1] - a[1]) * i / steps, half + 5) for i in range(steps + 1))

    def via_blocked(self, net):
        """The grid nodes where a via of this net may not be."""
        half = VIA_DIAMETER / 2 + 10
        out = self.blocked(net, 1, half) | self.blocked(net, 16, half)
        discs = [(pad["x"], pad["y"], pad, VIA_DIAMETER / 2 + 150) for pad in self.pads]
        for x, y, pad, reach in discs:
            for gx in range(math.ceil((x - pad["w"] / 2 - reach) / GRID), math.floor((x + pad["w"] / 2 + reach) / GRID) + 1):
                for gy in range(math.ceil((y - pad["h"] / 2 - reach) / GRID), math.floor((y + pad["h"] / 2 + reach) / GRID) + 1):
                    if point_rect(gx * GRID, gy * GRID, pad) < reach:
                        out.add((gx, gy))
        reach = VIA_DIAMETER + CLEARANCE
        for _, x, y in self.vias:
            for gx in range(math.ceil((x - reach) / GRID), math.floor((x + reach) / GRID) + 1):
                for gy in range(math.ceil((y - reach) / GRID), math.floor((y + reach) / GRID) + 1):
                    if math.hypot(gx * GRID - x, gy * GRID - y) < reach:
                        out.add((gx, gy))
        return out

    def entries(self, net, pad, width):
        """The grid nodes a track can start from, next to the centre of a pad."""
        out = []
        layers = (1,) if pad["smd"] else (1, 16)
        base_x, base_y = round(pad["x"] / GRID), round(pad["y"] / GRID)
        for layer in layers:
            for gx in range(base_x - 2, base_x + 3):
                for gy in range(base_y - 2, base_y + 3):
                    node = (gx * GRID, gy * GRID)
                    if self.segment_clear(net, layer, (pad["x"], pad["y"]), node, width / 2):
                        out.append(((gx, gy, layer), math.hypot(node[0] - pad["x"], node[1] - pad["y"]) / GRID))
        return out

    def route(self, net, start, goals, width):
        targets = {}
        for pad in goals:
            for node, cost in self.entries(net, pad, width):
                targets.setdefault(node, (pad, cost))
        queue, best, back = [], {}, {}
        ends = sorted({(node[0], node[1]) for node in targets})
        boxed = (min(x for x, _ in ends), max(x for x, _ in ends), min(y for _, y in ends), max(y for _, y in ends)) if ends else (0, 0, 0, 0)

        def ahead(node):
            """Never more than what is left: the octile distance to the box of the targets."""
            dx = max(boxed[0] - node[0], node[0] - boxed[1], 0)
            dy = max(boxed[2] - node[1], node[1] - boxed[3], 0)
            return max(dx, dy) + 0.4143 * min(dx, dy)

        for node, cost in self.entries(net, start, width):
            best[node] = cost
            heapq.heappush(queue, (cost + ahead(node), cost, node))
        half = width / 2 + 40
        blocked = {layer: self.blocked(net, layer, half) for layer in (1, 16)}
        low = math.ceil((EDGE_CLEARANCE + half) / GRID)
        high_x = math.floor((self.size[0] - EDGE_CLEARANCE - half) / GRID)
        high_y = math.floor((self.size[1] - EDGE_CLEARANCE - half) / GRID)
        no_via = self.via_blocked(net)
        low_via = math.ceil((EDGE_CLEARANCE + VIA_DIAMETER / 2 + 10) / GRID)
        high_via_x = math.floor((self.size[0] - EDGE_CLEARANCE - VIA_DIAMETER / 2 - 10) / GRID)
        high_via_y = math.floor((self.size[1] - EDGE_CLEARANCE - VIA_DIAMETER / 2 - 10) / GRID)
        steps = ((1, 0, 1.0), (-1, 0, 1.0), (0, 1, 1.0), (0, -1, 1.0), (1, 1, 1.4143), (-1, 1, 1.4143), (1, -1, 1.4143), (-1, -1, 1.4143))
        while queue:
            _, cost, node = heapq.heappop(queue)
            if cost > best[node]:
                continue
            if node in targets:
                path = [node]
                while node in back:
                    node = back[node]
                    path.append(node)
                path.reverse()
                return path, targets[path[-1]][0]
            gx, gy, layer = node
            neighbours = []
            for sx, sy, step in steps:
                nxt = (gx + sx, gy + sy, layer)
                if low <= nxt[0] <= high_x and low <= nxt[1] <= high_y and (nxt[0], nxt[1]) not in blocked[layer]:
                    turned = node in back and (gx - back[node][0], gy - back[node][1]) != (sx, sy)
                    neighbours.append((nxt, step + (0.05 if turned else 0.0)))
            other = (gx, gy, 17 - layer)
            if low_via <= gx <= high_via_x and low_via <= gy <= high_via_y and (gx, gy) not in no_via:
                neighbours.append((other, VIA_COST))
            for nxt, step in neighbours:
                total = cost + step
                if total < best.get(nxt, 1e18):
                    best[nxt] = total
                    back[nxt] = node
                    heapq.heappush(queue, (total + ahead(nxt), total, nxt))
        raise ValueError(f"no route for {net} from {start['ref']}.{start['number']}")

    def add(self, net, start, end, path, width):
        """Turns a path of grid nodes into straight pieces, joining the ones that go the same way."""
        points = [((start["x"], start["y"]), path[0][2])] + [((n[0] * GRID, n[1] * GRID), n[2]) for n in path] + [((end["x"], end["y"]), path[-1][2])]
        pieces = []
        for (a, layer_a), (b, layer_b) in zip(points, points[1:]):
            if layer_a != layer_b:
                self.vias.append((net, a[0], a[1]))
                continue
            if a == b:
                continue
            if pieces:
                prev = pieces[-1]
                ux, uy = prev[1][0] - prev[0][0], prev[1][1] - prev[0][1]
                vx, vy = b[0] - a[0], b[1] - a[1]
                if prev[2] == layer_a and prev[1] == a and ux * vy == uy * vx and ux * vx + uy * vy > 0:
                    pieces[-1] = (prev[0], b, layer_a)
                    continue
            pieces.append((a, b, layer_a))
        for a, b, layer in pieces:
            self.wires.append((net, layer, a[0], a[1], b[0], b[1], width))


def route_design(d):
    pads = placed_pads(d)
    by_name = {f"{p['ref']}.{p['number']}": p for p in pads}
    order = sorted(d["nets"], key=lambda n: (max_span(d["nets"][n], by_name), n))
    last = None
    for attempt in range(len(order) + 1):
        router = Router(d, pads)
        failed = None
        for net in order:
            width = d["widths"].get(net, d["width"])
            pins = [by_name[p] for p in d["nets"][net]]
            done, waiting = [pins[0]], pins[1:]
            try:
                while waiting:
                    waiting.sort(key=lambda p: min(math.hypot(p["x"] - q["x"], p["y"] - q["y"]) for q in done))
                    pad = waiting.pop(0)
                    path, goal = router.route(net, pad, done, width)
                    router.add(net, pad, goal, path, width)
                    done.append(pad)
            except ValueError as error:
                failed, last = net, error
                break
        if failed is None:
            return pads, router
        order.remove(failed)
        order.insert(0, failed)
    raise ValueError(f"{d['name']}: {last}")


def max_span(pins, by_name):
    xs = [by_name[p]["x"] for p in pins]
    ys = [by_name[p]["y"] for p in pins]
    return max(xs) - min(xs) + max(ys) - min(ys)


ROUTED: dict[str, dict] = {}


def model(d):
    """The design as the board reader of the program sees it: what the user programs walk over."""
    if d["name"] in ROUTED:
        return ROUTED[d["name"]]
    pads, router = route_design(d)
    pad_names = {f"{p['ref']}.{p['number']}" for p in pads}
    for net, pins in d["nets"].items():
        for pin_id in pins:
            if pin_id not in pad_names:
                raise ValueError(f"{d['name']}: net {net} names the pin {pin_id} that does not exist")
    signals = []
    for net in d["nets"]:
        signals.append({"name": net,
                        "vias": [(x, y) for n, x, y in router.vias if n == net],
                        "wires": [(layer, x1, y1, x2, y2, w) for n, layer, x1, y1, x2, y2, w in router.wires if n == net]})
    parts = []
    for part in d["parts"]:
        pkg = DEVICESETS[part["set"]][2][part["device"]]
        parts.append({**part, "package": pkg, "pads": [p for p in pads if p["ref"] == part["ref"]]})
    ROUTED[d["name"]] = {"parts": parts, "signals": signals}
    return ROUTED[d["name"]]


def used_sets(d):
    names = []
    for part in d["parts"]:
        if part["set"] not in names:
            names.append(part["set"])
    return names


def board_file(d):
    m = model(d)
    width, height = d["size"]
    outline = [line(20, 0, 0, width, 0, 100), line(20, width, 0, width, height, 100), line(20, width, height, 0, height, 100), line(20, 0, height, 0, 0, 100)]
    package_names = []
    for part in m["parts"]:
        if part["package"] not in package_names:
            package_names.append(part["package"])
    packages = [tag("package", [("name", n)], [item_xml(i) for i in PACKAGES[n][2] if i["kind"] in ("pad", "smd")]) for n in package_names]
    elements = []
    for part in m["parts"]:
        attrs = [("name", part["ref"]), ("library", BOARD_LIBRARY), ("package", part["package"]), ("value", part["value"]), ("x", mm(part["x"])), ("y", mm(part["y"]))]
        if part["rot"]:
            attrs.append(("rot", f"R{part['rot']}"))
        elements.append(tag("element", attrs))
    signals = []
    for signal in m["signals"]:
        rows = [tag("contactref", [("element", p.split(".")[0]), ("pad", p.split(".")[1])]) for p in d["nets"][signal["name"]]]
        rows += [tag("via", [("x", mm(x)), ("y", mm(y)), ("extent", "1-16"), ("drill", mm(VIA_DRILL)), ("diameter", mm(VIA_DIAMETER))]) for x, y in signal["vias"]]
        rows += [tag("wire", [("x1", mm(x1)), ("y1", mm(y1)), ("x2", mm(x2)), ("y2", mm(y2)), ("width", mm(w)), ("layer", layer)])
                 for layer, x1, y1, x2, y2, w in signal["wires"]]
        signals.append(tag("signal", [("name", signal["name"])], rows))
    rules = [("mdWireWire", "0.15mm"), ("mdWirePad", "0.15mm"), ("mdWireVia", "0.15mm"), ("mdPadPad", "0.15mm"), ("mdPadVia", "0.15mm"),
             ("mdViaVia", "0.15mm"), ("mdSmdPad", "0.15mm"), ("mdSmdVia", "0.15mm"), ("mdSmdSmd", "0.15mm"), ("mdCopperDimension", "0.3mm"),
             ("mdDrill", "0.25mm"), ("msWidth", "0.15mm"), ("msDrill", "0.3mm"), ("rlMinPadTop", "0.15mm"), ("rlMinPadBottom", "0.15mm"),
             ("rlMinViaOuter", "0.15mm")]
    board = tag("board", [], [
        tag("plain", [], [item_xml(i) for i in outline]),
        tag("libraries", [], [tag("library", [("name", BOARD_LIBRARY)], [tag("packages", [], packages)])]),
        tag("designrules", [("name", "eh3d-kit")], [tag("param", [("name", n), ("value", v)]) for n, v in rules]),
        tag("elements", [], elements),
        tag("signals", [], signals),
    ])
    return document([tag("settings", []), layers_xml([1, 16, 20, 21, 22, 29, 30, 31, 32, 51, 52]), board])


def sheet_places(d):
    """Where each symbol stands on the sheet: columns of four, far enough apart not to touch."""
    places, x, column = {}, 30000, []

    def half_height(part):
        symbol = DEVICESETS[part["set"]][1]
        return max(abs(i["y"]) for i in SYMBOLS[symbol] if i["kind"] == "pin") + 7620

    y = 20000
    for part in d["parts"]:
        h = half_height(part)
        if column and y + 2 * h > 190000:
            x += 55880
            y, column = 20000, []
        places[part["ref"]] = (x, ((y + h) // 2540) * 2540)
        y += 2 * h
        column.append(part)
    return places


def schematic_file(d):
    places = sheet_places(d)
    parts = [tag("part", [("name", p["ref"]), ("library", BOARD_LIBRARY), ("deviceset", p["set"]), ("device", p["device"]), ("value", p["value"])]) for p in d["parts"]]
    instances = [tag("instance", [("part", p["ref"]), ("gate", "G$1"), ("x", mm(places[p["ref"]][0])), ("y", mm(places[p["ref"]][1]))]) for p in d["parts"]]
    by_ref = {p["ref"]: p for p in d["parts"]}
    nets = []
    for net, pins in d["nets"].items():
        segments = []
        for pin_id in pins:
            ref, number = pin_id.split(".")
            symbol = DEVICESETS[by_ref[ref]["set"]][1]
            at = next(i for i in SYMBOLS[symbol] if i["kind"] == "pin" and i["name"] == number)
            x, y = places[ref][0] + at["x"], places[ref][1] + at["y"]
            end = x + (2540 if at["rot"] == "R180" else -2540)
            segments.append(tag("segment", [], [
                tag("pinref", [("part", ref), ("gate", "G$1"), ("pin", number)]),
                tag("wire", [("x1", mm(x)), ("y1", mm(y)), ("x2", mm(end)), ("y2", mm(y)), ("width", "0.152"), ("layer", 91)]),
                tag("label", [("x", mm(end)), ("y", mm(y)), ("size", "1.778"), ("layer", 95)]),
            ]))
        nets.append(tag("net", [("name", net), ("class", 0)], segments))
    library = tag("library", [("name", BOARD_LIBRARY)], library_sections(used_sets(d)))
    schematic = tag("schematic", [], [
        tag("libraries", [], [library]),
        tag("parts", [], parts),
        tag("sheets", [], [tag("sheet", [], [tag("instances", [], instances), tag("nets", [], nets)])]),
    ])
    return document([tag("settings", []), layers_xml([91, 94, 95, 96]), schematic])


# --------------------------------------------------------------------------------------------------
# what the user programs of the kit print, worked out here from the same data

def u2mm(um):
    return um * 1000 / 1000000


def length_um_in_nm(w):
    """The length of a wire in nanometres, cut to a whole number as the evaluator does."""
    _, x1, y1, x2, y2, _ = w
    return math.isqrt(((x2 - x1) * 1000) ** 2 + ((y2 - y1) * 1000) ** 2)


LAYER_NUMBERS = {1: 1, 16: 2}  # the place of the layer among the layers of the file, from 1
BOARD_LAYERS = ["Top", "Bottom", "Dimension", "tPlace", "bPlace", "tStop", "bStop", "tCream", "bCream", "tDocu", "bDocu"]


def group(keys):
    out = {}
    for key in keys:
        out[key] = out.get(key, 0) + 1
    return out


def exp_count_parts(m, d):
    counts = group(p["ref"][0] for p in m["parts"])
    return "".join(f"{k}  {v}\n" for k, v in counts.items()) + f"total {len(m['parts'])}\n"


def bom_groups(m):
    groups = {}
    for p in m["parts"]:
        key = f"{p['value']}|{p['package']}"
        entry = groups.setdefault(key, [p["value"], p["package"], []])
        entry[2].append(p["ref"])
    return [groups[k] for k in sorted(groups)]


def exp_bom_by_value(m, d):
    rows = "".join(f"{len(refs):3d}  {v:<12} {pkg:<18} {' '.join(refs)}\n" for v, pkg, refs in bom_groups(m))
    return rows + f"{len(bom_groups(m))} lines\n"


def exp_bom_csv(m, d):
    rows = sorted((p["ref"], p["value"], p["package"]) for p in m["parts"])
    return "reference,value,package\n" + "".join(f"{r},{v},{k}\n" for r, v, k in rows)


def all_drills(m):
    drills = [p["drill"] for part in m["parts"] for p in part["pads"] if p["drill"] > 0]
    drills += [VIA_DRILL for s in m["signals"] for _ in s["vias"]]
    return drills


def exp_drill_table(m, d):
    counts = group(all_drills(m))
    return "".join(f"{u2mm(k):5.2f} mm  x{counts[k]}\n" for k in sorted(counts)) + f"{len(all_drills(m))} holes\n"


def exp_hole_summary(m, d):
    pads = [p for part in m["parts"] for p in part["pads"] if p["drill"] > 0]
    vias = sum(len(s["vias"]) for s in m["signals"])
    return f"pads with drill {len(pads)}\nvias {vias}\nunplated holes 0\ntotal {len(pads) + vias}\n"


def net_length_mm(signal):
    total = 0.0
    for w in signal["wires"]:
        total += length_um_in_nm(w) / 1000000
    return total


def exp_net_lengths(m, d):
    return "".join(f"{s['name']:<8} {net_length_mm(s):8.2f} mm  {len(s['vias'])} vias\n" for s in m["signals"])


def exp_longest_net(m, d):
    best, name = 0.0, ""
    for s in m["signals"]:
        value = net_length_mm(s)
        if value > best:
            best, name = value, s["name"]
    return f"longest net {name}: {best:.2f} mm\n"


def all_wires(m):
    return [w for s in m["signals"] for w in s["wires"]]


def exp_track_widths(m, d):
    totals = {}
    for w in all_wires(m):
        totals[w[5]] = totals.get(w[5], 0.0) + length_um_in_nm(w) / 1000000
    return "".join(f"{u2mm(k):5.2f} mm wide  {totals[k]:8.2f} mm\n" for k in sorted(totals))


def exp_via_list(m, d):
    rows, count = "", 0
    for s in m["signals"]:
        for x, y in s["vias"]:
            count += 1
            rows += f"{s['name']:<8} {u2mm(x):7.2f} {u2mm(y):7.2f} {u2mm(VIA_DRILL):.2f}/{u2mm(VIA_DIAMETER):.2f}\n"
    return rows + f"{count} vias\n"


def exp_pins_per_net(m, d):
    rows = ""
    for s in m["signals"]:
        wanted = set(d["nets"][s["name"]])
        pins = [f"{p['ref']}.{p['number']}" for part in m["parts"] for p in part["pads"] if f"{p['ref']}.{p['number']}" in wanted]
        rows += f"{s['name']:<8} {len(pins):2d}  {' '.join(pins)}\n"
    return rows


def exp_unconnected_pads(m, d):
    free = [f"{p['ref']}.{p['number']}" for part in m["parts"] for p in part["pads"] if not p["net"]]
    return "".join(f"{n}\n" for n in free) + f"{len(free)} pads without a net\n"


def exp_pad_kinds(m, d):
    pads = [p for part in m["parts"] for p in part["pads"]]
    smd_count = sum(1 for p in pads if p["drill"] == 0)
    square = sum(1 for p in pads if p["drill"] > 0 and p["square"])
    return f"surface mount {smd_count}\nthrough hole {len(pads) - smd_count}\nsquare through hole {square}\n"


def exp_pad_extent(m, d):
    pads = [p for part in m["parts"] for p in part["pads"]]
    xs, ys = [p["x"] for p in pads], [p["y"] for p in pads]
    return (f"x {u2mm(min(xs)):.2f} to {u2mm(max(xs)):.2f} mm\ny {u2mm(min(ys)):.2f} to {u2mm(max(ys)):.2f} mm\n"
            f"size {u2mm(max(xs) - min(xs)):.2f} x {u2mm(max(ys) - min(ys)):.2f} mm\n")


def exp_placement(m, d):
    rows = "ref;x_mm;y_mm;angle;side\n"
    for p in m["parts"]:
        rows += f"{p['ref']};{u2mm(p['x']):.3f};{u2mm(p['y']):.3f};{p['rot']:.0f};top\n"
    return rows


def exp_packages_used(m, d):
    counts = group(p["package"] for p in m["parts"])
    return "".join(f"{k:<18} {counts[k]}\n" for k in sorted(counts))


def exp_layer_usage(m, d):
    rows = ""
    for number, name in enumerate(BOARD_LAYERS, 1):
        count = sum(1 for w in all_wires(m) if LAYER_NUMBERS[w[0]] == number)
        rows += f"{number:2d} {name:<10} {count} wires\n"
    return rows


def sch_pins_of(d, pins):
    """The pins of a net in the order of the parts and of the pins of their symbols."""
    out = []
    for part in d["parts"]:
        for item in SYMBOLS[DEVICESETS[part["set"]][1]]:
            if item["kind"] == "pin" and f"{part['ref']}.{item['name']}" in pins:
                out.append(f"{part['ref']}.{item['name']}")
    return out


def exp_sch_netlist(m, d):
    return "".join(f"{net}: {' '.join(sch_pins_of(d, set(pins)))}\n" for net, pins in d["nets"].items())


def exp_sch_bom(m, d):
    groups = {}
    for p in m["parts"]:
        key = f"{p['value']}|{p['package']}"
        groups.setdefault(key, []).append(p["ref"])
    rows = ""
    for key in sorted(groups):
        value, pkg = key.split("|")
        rows += f"{value};{pkg};{len(groups[key])};{' '.join(groups[key])}\n"
    return "value;package;quantity;references\n" + rows


def exp_sch_net_sizes(m, d):
    rows, total, biggest, name = "", 0, 0, ""
    for net, pins in d["nets"].items():
        rows += f"{net:<8} {len(pins)}\n"
        total += len(pins)
        if len(pins) > biggest:
            biggest, name = len(pins), net
    return rows + f"{len(d['nets'])} nets, {total} connections, largest {name} with {biggest}\n"


def exp_sch_sheet_parts(m, d):
    places = sheet_places(d)
    rows = ""
    for number, p in enumerate(d["parts"], 1):
        x, y = places[p["ref"]]
        rows += f"{number:2d} {p['ref']:<4} {u2mm(x):7.2f} {u2mm(y):7.2f}\n"
    return rows


# program -> (design, kind of input, what it prints, titles in the eight languages)
PROGRAMS = {
    "count_parts": ("transistor_switch", "board", exp_count_parts),
    "bom_by_value": ("led_bar", "board", exp_bom_by_value),
    "bom_csv": ("regulator_sot223", "board", exp_bom_csv),
    "drill_table": ("tht_led_pair", "board", exp_drill_table),
    "hole_summary": ("tssop14_breakout", "board", exp_hole_summary),
    "net_lengths": ("transistor_switch", "board", exp_net_lengths),
    "longest_net": ("power_input", "board", exp_longest_net),
    "track_widths": ("power_input", "board", exp_track_widths),
    "via_list": ("soic8_decoupled", "board", exp_via_list),
    "pins_per_net": ("voltage_divider", "board", exp_pins_per_net),
    "unconnected_pads": ("soic16_breakout", "board", exp_unconnected_pads),
    "pad_kinds": ("regulator_sot223", "board", exp_pad_kinds),
    "pad_extent": ("esp32_carrier", "board", exp_pad_extent),
    "placement": ("led_indicator", "board", exp_placement),
    "packages_used": ("tht_led_pair", "board", exp_packages_used),
    "layer_usage": ("soic8_decoupled", "board", exp_layer_usage),
    "sch_netlist": ("transistor_switch", "schematic", exp_sch_netlist),
    "sch_bom": ("led_bar", "schematic", exp_sch_bom),
    "sch_net_sizes": ("stm32_carrier", "schematic", exp_sch_net_sizes),
    "sch_sheet_parts": ("power_input", "schematic", exp_sch_sheet_parts),
}


# --------------------------------------------------------------------------------------------------
# the catalogue

TITLES = json.loads((Path(__file__).resolve().parent / "kit_titles.json").read_text(encoding="utf-8"))

EXPORTS = {
    "library": ["eagle-lbr"],
    "board": ["eagle-brd", "kicad-pcb", "gerber", "excellon", "ipc-2581"],
    "schematic": ["eagle-sch", "kicad-sch"],
    "program": [],
    "rules": [],
    "dfm": [],
    "export-profile": [],
}


def catalogue_entries():
    for name in LIBRARIES:
        yield f"library-{name}", name, "library", "eagle-lbr", [f"libraries/{name}.lbr"], None
    for d in DESIGNS:
        yield f"board-{d['name']}", d["name"], "board", "eagle-brd", [f"boards/{d['name']}.brd"], None
        yield f"schematic-{d['name']}", d["name"], "schematic", "eagle-sch", [f"schematics/{d['name']}.sch"], None
    for name, (target, kind, _) in PROGRAMS.items():
        runs_on = f"boards/{target}.brd" if kind == "board" else f"schematics/{target}.sch"
        yield f"ulp-{name.replace('_', '-')}", name, "program", "ulp", [f"ulp/{name}.ulp", f"ulp/{name}.expected"], runs_on
    for path in sorted((KIT / "rules").glob("*")):
        stem = path.name.split(".")[0]
        if path.suffix == ".cfg":
            yield f"dfm-{stem.replace('_', '-')}", stem, "dfm", "phoenix-dfm", [f"rules/{path.name}"], None
        else:
            yield f"rules-{stem.replace('_', '-')}", stem, "rules", "phoenix-rules", [f"rules/{path.name}"], None
    for path in sorted((KIT / "cam").glob("*.json")):
        yield f"cam-{path.stem.replace('_', '-')}", path.stem, "export-profile", "phoenix-export", [f"cam/{path.name}"], None


def read_bytes(path: Path) -> bytes:
    """The bytes of a hand written file with line feeds, as the repository keeps them."""
    return path.read_bytes().replace(b"\r\n", b"\n")


def generated() -> dict[str, str]:
    out = {}
    for name in LIBRARIES:
        out[f"libraries/{name}.lbr"] = library_file(name)
    for d in DESIGNS:
        out[f"boards/{d['name']}.brd"] = board_file(d)
        out[f"schematics/{d['name']}.sch"] = schematic_file(d)
    by_name = {d["name"]: d for d in DESIGNS}
    for name, (target, _, expected) in PROGRAMS.items():
        d = by_name[target]
        out[f"ulp/{name}.expected"] = expected(model(d), d)
    return out


def manifest_text(files: dict[str, str]) -> str:
    items = []
    for identifier, name, kind, form, paths, runs_on in catalogue_entries():
        listed = []
        for path in paths:
            data = files[path].encode("utf-8") if path in files else read_bytes(KIT / path)
            listed.append({"path": path, "sha256": hashlib.sha256(data).hexdigest()})
        title = TITLES[identifier]
        if len(title) != len(LANGS):
            raise ValueError(f"{identifier}: a title in each of the eight languages is needed")
        item = {"id": identifier, "name": name, "type": kind, "format": form, "license": LICENSE_ID, "provenance": PROVENANCE,
                "verified": False, "exports": EXPORTS[kind], "title": dict(zip(LANGS, title)), "files": listed}
        if runs_on:
            item["runs_on"] = runs_on
        items.append(item)
    unused = set(TITLES) - {i["id"] for i in items}
    if unused:
        raise ValueError(f"titles without a piece of the kit: {sorted(unused)}")
    document_ = {"format": "phoenix-kit/1", "license": LICENSE_ID, "provenance": PROVENANCE, "items": items}
    return json.dumps(document_, ensure_ascii=False, indent=2) + "\n"


def all_files() -> dict[str, str]:
    files = generated()
    files["manifest.json"] = manifest_text(files)
    return files


def main(argv: list[str]) -> int:
    files = all_files()
    if argv == ["--check"]:
        wrong = [p for p, text in files.items() if not (KIT / p).is_file() or read_bytes(KIT / p) != text.encode("utf-8")]
        listed = set(files) | {f["path"] for item in json.loads(files["manifest.json"])["items"] for f in item["files"]}
        extra = [p.relative_to(KIT).as_posix() for p in KIT.rglob("*") if p.is_file() and p.relative_to(KIT).as_posix() not in listed]
        for path in wrong:
            print(f"KIT: kit/{path} is not what tools/make_kit.py writes")
        for path in extra:
            print(f"KIT: kit/{path} is not in the catalogue")
        print("KIT=FAIL" if wrong or extra else "KIT=PASS")
        return 1 if wrong or extra else 0
    if argv:
        print(__doc__)
        return 0 if argv == ["--help"] else 2
    for path, text in files.items():
        target = KIT / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(text.encode("utf-8"))
    print(f"{len(files)} files written under kit/")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
