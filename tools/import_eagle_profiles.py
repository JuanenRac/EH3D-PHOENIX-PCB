#!/usr/bin/env python3
"""Read the design rules (.dru) and the CAM jobs (.cam) that ship with EAGLE and write the profiles
of the repository.

The tool only READS what it is given. It copies no text, no licence and no name of an author: it
takes the numbers (limits, layer lists, file extensions, coordinate formats) and writes profiles of
its own. Run it again to regenerate the files; what it writes is what is versioned.

    py -3 -I tools/import_eagle_profiles.py --dru <folder of .dru> --cam <folder of .cam>

It writes (relative to the repository root, or to --out):

* config/dfm/<name>.cfg                one capability profile for each maker in DRU_PROFILES;
* config/export_profiles_makers.json   one export profile for each maker in CAM_PROFILES;

It prints the table of the figures that the documents quote.

Every length is converted to integer nanometres EXACTLY (1 mil is 25400 nm, 1 mm is 1000000 nm,
1 inch is 25400000 nm); a value that would not be an integer number of nanometres stops the run.
A key that the source does not give is left out of the profile (a line that is missing is off).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from fractions import Fraction

UNIT_NM = {"mil": Fraction(25400), "mm": Fraction(1_000_000), "inch": Fraction(25_400_000),
           "micron": Fraction(1000), "um": Fraction(1000)}

# profile name -> (path of the .dru under --dru, maker, title)
DRU_PROFILES = [
    ("pcbway-dru", "PCBway.dru", "PCBWay"),
    ("oshpark-2l-dru", "OshPark/OshPark (2 Layer).dru", "OSH Park"),
    ("oshpark-4l-dru", "OshPark/OshPark (4 Layer).dru", "OSH Park"),
    ("eurocircuits-1l-dru", "Eurocircuits/EuroCircuits (1 Layer).dru", "Eurocircuits"),
    ("eurocircuits-2l-dru", "Eurocircuits/EuroCircuits (2 Layer).dru", "Eurocircuits"),
    ("eurocircuits-4l-dru", "Eurocircuits/EuroCircuits (4 Layer).dru", "Eurocircuits"),
    ("eurocircuits-6l-dru", "Eurocircuits/EuroCircuits (6 Layer).dru", "Eurocircuits"),
    ("pcbpool-dru", "PCB-POOL.dru", "PCB-POOL"),
    ("allpcb-1l-dru", "ALLPCB/ALLPCB (1 Layer).dru", "ALLPCB"),
    ("allpcb-2l-dru", "ALLPCB/ALLPCB (2 Layer).dru", "ALLPCB"),
    ("allpcb-4l-dru", "ALLPCB/ALLPCB (4 Layer).dru", "ALLPCB"),
    ("allpcb-6l-dru", "ALLPCB/ALLPCB (6 Layer).dru", "ALLPCB"),
    ("generic-8mil-dru", "8mils.dru", "Generic 8 mil"),
    ("generic-10mil-dru", "10mils.dru", "Generic 10 mil"),
]

# profile name -> (file under --cam, maker, title)
CAM_PROFILES = [
    ("jlcpcb-2l-cam", "jlcpcb_2_layer_v9.cam", "JLCPCB 2 layers"),
    ("jlcpcb-4l-cam", "jlcpcb_4_layer_v9.cam", "JLCPCB 4 layers"),
    ("jlcpcb-6l-cam", "jlcpcb_6_layer_v9.cam", "JLCPCB 6 layers"),
    ("pcbway-cam", "PCBway-Edited.cam", "PCBWay 2 layers"),
    ("oshpark-2l-cam", "oshpark_2layer.cam", "OSH Park 2 layers"),
    ("oshpark-4l-cam", "OSHPark-official-4layer-Eagle7.2-with-build-directory.cam", "OSH Park 4 layers"),
    ("sunstone-2l-cam", "Sunstone_2layer.cam", "Sunstone 2 layers"),
    ("elecrow-cam", "Elecrow_Gerber_Generater_DrillAlign.cam", "Elecrow"),
    ("apc-2l-cam", "CAM for APC, 2-layer + silk + mask.cam", "APC 2 layers"),
    ("apc-4l-cam", "CAM for APC, 4-layer + silk + mask.cam", "APC 4 layers"),
    ("seeed-2l-cam", "seeedstudio-2layers.cam", "Seeed 2 layers"),
    ("seeed-4l-cam", "seeedstudio-4layers.cam", "Seeed 4 layers"),
    ("lpkf-2l-cam", "LPKF_2-layer.cam", "LPKF 2 layers"),
    ("advancedcircuits-2l-cam", "AdvancedCircuits_2layer.cam", "Advanced Circuits 2 layers"),
    ("isocam-cam", "BUNGARD_ISOCAM.cam", "ISOCAM of the Bungard machine"),
]

# Export profiles from public naming conventions (not from a CAM job): name -> (description, names, drill)
EXTRA_EXPORT = [
    ("jlcpcb-kicad-names",
     "File names in the KiCad style that the JLCPCB page lists (-F_Cu, -B_Cu, -Edge_Cuts, -PTH, -NPTH). "
     "Public naming of the manufacturer page; not compared with the manufacturer.",
     {"copper_top": "{board}-F_Cu.gbr", "copper_bottom": "{board}-B_Cu.gbr",
      "outline": "{board}-Edge_Cuts.gbr", "mask_top": "{board}-F_Mask.gbr",
      "silk_top": "{board}-F_SilkS.gbr", "paste_top": "{board}-F_Paste.gbr"},
     {"plated_file": "{board}-PTH.drl", "non_plated_file": "{board}-NPTH.drl"}),
    ("jlcpcb-altium-names",
     "File names in the Altium style that the JLCPCB page lists (.gtl .gbl .gko .gts .gbs .gto .gbo "
     ".gtp .gbp and a .txt drill file). The file of the holes that are not plated is the same name "
     "with _npth, a choice of ours. Public naming of the manufacturer page; not compared with the "
     "manufacturer.",
     {"copper_top": "{board}.gtl", "copper_bottom": "{board}.gbl", "outline": "{board}.gko",
      "mask_top": "{board}.gts", "mask_bottom": "{board}.gbs", "silk_top": "{board}.gto",
      "silk_bottom": "{board}.gbo", "paste_top": "{board}.gtp", "paste_bottom": "{board}.gbp"},
     {"plated_file": "{board}.txt", "non_plated_file": "{board}_npth.txt"}),
    ("jlcpcb-eagle-names",
     "File names in the EAGLE style that the JLCPCB page lists (Copper_top, Copper_bottom, Profile, "
     "Soldermask_top, Silkscreen_top, Solderpaste_top, Drill_1_16.xln). The file of the holes that "
     "are not plated is the same name with _npth, a choice of ours. Public naming of the "
     "manufacturer page; not compared with the manufacturer.",
     {"copper_top": "Copper_top.gbr", "copper_bottom": "Copper_bottom.gbr", "outline": "Profile.gbr",
      "mask_top": "Soldermask_top.gbr", "silk_top": "Silkscreen_top.gbr",
      "paste_top": "Solderpaste_top.gbr"},
     {"plated_file": "Drill_1_16.xln", "non_plated_file": "Drill_1_16_npth.xln"}),
]

SOURCE_DRU = "design rules of the manufacturer as shipped with EAGLE"
SOURCE_CAM = "CAM job of the manufacturer as shipped with EAGLE"

ROLES = ["copper_top", "copper_inner", "copper_bottom", "mask_top", "mask_bottom", "silk_top",
         "silk_bottom", "paste_top", "paste_bottom", "outline"]


class ImportError_(Exception):
    pass


# ---------------------------------------------------------------------------------- design rules

def to_nm(text: str) -> int:
    m = re.fullmatch(r"\s*(-?[0-9]+(?:\.[0-9]+)?)\s*(mil|mm|inch|micron|um)?\s*", text)
    if not m:
        raise ImportError_(f"not a length: {text!r}")
    unit = m.group(2) or "mm"
    value = Fraction(m.group(1)) * UNIT_NM[unit]
    if value.denominator != 1:
        raise ImportError_(f"{text!r} is not a whole number of nanometres")
    return int(value)


def parse_dru(text: str) -> dict:
    out = {}
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("description") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        out[key.strip()] = value.strip()
    return out


def layer_count(setup: str | None) -> int | None:
    if setup is None:
        return None
    numbers = {int(n) for n in re.findall(r"[0-9]+", setup)}
    return len(numbers) if numbers else None


CLEARANCE_KEYS = ["mdWireWire", "mdWirePad", "mdWireVia", "mdPadPad", "mdPadVia", "mdViaVia",
                  "mdSmdPad", "mdSmdVia", "mdSmdSmd", "mdViaViaSameLayer"]


def dfm_profile(name: str, title: str, dru: dict) -> tuple[str, dict]:
    """The text of a capability profile and the numbers that went into it."""
    facts: dict[str, int] = {}

    def length(key: str) -> int | None:
        return to_nm(dru[key]) if key in dru else None

    mapping = [
        ("min_track_width_nm", "msWidth", "the minimum width of a wire"),
        ("min_clearance_nm", "mdWireWire", "the distance between two wires"),
        ("min_drill_nm", "msDrill", "the minimum drill"),
        ("min_annular_ring_nm", "rlMinViaOuter", "the least ring of a via on an outer layer"),
        ("min_edge_clearance_nm", "mdCopperDimension", "the distance from copper to the board outline"),
        ("min_drill_spacing_nm", "mdDrill", "the distance between two drill holes"),
        ("min_pad_ring_nm", "rlMinPadTop", "the least ring of a pad on the top layer"),
    ]
    layers = layer_count(dru.get("layerSetup"))
    lines = [
        f"# Capability profile from the {SOURCE_DRU}: {title}"
        + (f", {layers} copper layer{'s' if layers != 1 else ''}." if layers else "."),
        "#",
        "# Made by tools/import_eagle_profiles.py from the numbers of the design rules (lengths are",
        "# converted to nanometres exactly; 1 mil is 25400, 1 mm is 1000000). Use it with",
        f"# `phoenix check --dfm-profile {name} <board>` or in `--dfm-compare`.",
        "#",
        "# A design rule file gives limits of the rule check, not what the maker can build at all: the",
        "# size of the board, the most holes, the thickness and the rest of the lines are left out, so",
        "# those lines are off. Nothing compares the figures with the maker, which is why verified",
        "# stays false (PHOENIX-DFM-006 says so in the report). Edit a figure in a copy of the file,",
        "# never in the program.",
        "#",
        "# Where a design rule has no line of the profile (the mask bridge, the diameter of a via, the",
        "# connection tolerance) the line is missing.",
    ]
    others = [to_nm(dru[k]) for k in CLEARANCE_KEYS if k in dru]
    if others and max(others) != min(others):
        lines += [
            "#",
            "# The source asks other distances too (wire to pad, pad to pad, smd to smd, via to via):",
            f"# from {min(others)} to {max(others)} nm.",
            "# The line min_clearance_nm is the distance between two wires, the one the rule check uses.",
        ]
    label = f"{title} (design rules, {layers} layer{'s' if layers != 1 else ''})" if layers else f"{title} (design rules)"
    lines += ["", f"name = {label}",
              f"source = {SOURCE_DRU}", "verified = false", ""]
    lines.append("# The limits of the usual rule check (they replace the rules of the board for this run).")
    for key, src, what in mapping:
        v = length(src)
        if v is None:
            continue
        facts[key] = v
        lines.append(f"{key} = {v}")
    if layers:
        facts["max_copper_layers"] = layers
        lines += ["", "# What the source says about the stack: the number of copper layers it sets up.",
                  f"max_copper_layers = {layers}"]
    return "\n".join(lines) + "\n", facts


# ------------------------------------------------------------------------------------ CAM jobs

LAYER_ROLE = {1: "copper_top", 16: "copper_bottom", 29: "mask_top", 30: "mask_bottom",
              31: "paste_top", 32: "paste_bottom", 21: "silk_top", 22: "silk_bottom"}


def role_of(layers: list[int], board_outline: bool = False) -> str | None:
    """The role of an output that draws these EAGLE layers, by its first telling layer."""
    s = set(layers)
    if (s & {20, 46} and not any(1 <= x <= 16 or x in (21, 22, 29, 30, 31, 32) for x in s)):
        return "outline"
    for layer in layers:
        if layer in LAYER_ROLE:
            return LAYER_ROLE[layer]
    inner = [x for x in layers if 2 <= x <= 15]
    if inner:
        return "copper_inner"
    if s & {25, 51}:
        return "silk_top"
    if s & {26, 52}:
        return "silk_bottom"
    return None


def suffix_of(fmt: str, span: tuple[int, int] | None = None) -> str:
    """The file name template of an output: the board name, then what the job puts after it."""
    name = fmt.replace("\\", "/").rsplit("/", 1)[-1]
    name = re.sub(r"_?%FF", "", name)
    if span is not None:
        name = name.replace("%FROM", str(span[0])).replace("%TO", str(span[1]))
    if "%N" in name:
        name = name.replace("%N", "{board}")
    elif name.startswith("."):
        name = "{board}" + name
    elif not name:
        name = "{board}"
    if re.search(r"%[A-Za-z]", name):
        raise ImportError_(f"a name token that the tool does not know: {fmt!r}")
    return name


def parse_ini_cam(text: str) -> dict:
    sections: dict[str, dict] = {}
    order: list[str] = []
    current = None
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.startswith("[") and line.endswith("]"):
            current = line[1:-1]
            sections.setdefault(current, {})
            continue
        if "=" in line and current is not None:
            key, value = line.split("=", 1)
            value = value.strip()
            if value.startswith('"') and value.endswith('"'):
                value = value[1:-1]
            if current == "CAM Processor Job" and key.strip() == "Section":
                order.append(value)
            elif not key.strip().startswith(("Name[", "Description[")):
                sections[current][key.strip()] = value
            else:
                sections[current].setdefault("_names", []).append(value)
    outs = []
    for sec in order:
        body = sections.get(sec, {})
        layers = [int(x) for x in re.findall(r"[0-9]+", body.get("Layers", ""))]
        outs.append({"device": body.get("Device", ""), "output": body.get("Output", ""),
                     "layers": layers})
    return {"kind": "ini", "outputs": outs}


def parse_json_cam(text: str) -> dict:
    data = json.loads(text)
    outs = []
    fmts: dict[str, tuple[int, int]] = {}

    def walk(group: dict):
        for item in group.get("outputs", []):
            kind = item.get("type")
            if kind == "gerber_layer":
                outs.append({"device": "GERBER", "output": item.get("filename_format", ""),
                             "layers": list(item.get("layers", [])),
                             "outline": bool(item.get("board_outline"))})
            elif kind in ("excellon", "autodrills"):
                outs.append({"device": "EXCELLON", "output": item.get("filename_format", ""),
                             "layers": [], "drills": item.get("drills", {}),
                             "span": (int((item.get("layers") or {}).get("from", 1)),
                                      int((item.get("layers") or {}).get("to", 16)))})
            if "outputs" in item:
                walk(item)

    for group in data.get("outputs", []):
        spec = group.get("format_specifier")
        if spec and group.get("output_type") in ("gerber", "drill"):
            fmts[group["output_type"]] = (int(spec["integer"]), int(spec["decimal"]))
        walk(group)
    return {"kind": "json", "outputs": outs, "formats": fmts, "units": data.get("units", "")}


def cam_profile(name: str, title: str, text: str) -> tuple[dict, dict]:
    job = parse_json_cam(text) if text.lstrip().startswith("{") else parse_ini_cam(text)
    names = {r: "" for r in ROLES}
    order: list[str] = []
    inner = sorted({n for o in job["outputs"] if o["device"].startswith("GERBER")
                    and role_of(o["layers"], o.get("outline", False)) == "copper_inner"
                    for n in [x for x in o["layers"] if 2 <= x <= 15][:1]})
    drill_name = ""
    drill_device = ""
    inner_list: list[str] = []
    for o in job["outputs"]:
        device = o["device"]
        if device.startswith("EXCELLON"):
            drill_name = suffix_of(o["output"], o.get("span"))
            drill_device = device
            continue
        if not device.startswith("GERBER"):
            continue
        role = role_of(o["layers"], o.get("outline", False))
        if role is None:
            raise ImportError_(f"{name}: no role for the layers {o['layers']}")
        template = suffix_of(o["output"])
        if role == "copper_inner":
            inner_list.append(template)
            if role not in order:
                order.append(role)
            continue
        if names[role] and names[role] != template:
            raise ImportError_(f"{name}: the role {role} has two names")
        names[role] = template
        if role not in order:
            order.append(role)
    if inner_list:
        names["copper_inner"] = inner_template(name, inner_list)
    profile = {"name": name, "description": "", "gerber": {"names": {}}}
    gerber = profile["gerber"]
    fmt = job.get("formats", {})
    if "gerber" in fmt:
        gerber["decimal_digits"] = fmt["gerber"][1]
    gerber["names"] = {r: names[r] for r in ROLES if names[r]}
    gerber["names"]["order"] = order
    if drill_name:
        # every job writes the plated and the not plated holes to ONE file; the export keeps two,
        # so the file of the not plated holes gets "_npth" before the extension (a choice of ours)
        stem, dot, ext = drill_name.rpartition(".")
        npth = f"{stem}_npth.{ext}" if dot else f"{drill_name}_npth"
        profile["drill"] = {"plated_file": drill_name, "non_plated_file": npth}
        if "drill" in fmt:
            profile["drill"].update({"style": "fixed", "fixed_integer_digits": fmt["drill"][0],
                                     "fixed_decimal_digits": fmt["drill"][1]})
    facts = {"gerber_format": fmt.get("gerber"), "drill_format": fmt.get("drill"),
             "drill_device": drill_device, "units": job.get("units", ""),
             "inner_layers": inner, "kind": job["kind"]}
    return profile, facts


def inner_template(name: str, templates: list[str]) -> str:
    """One template for all the inner layers: {inner} is the number of the inner layer from 1 and
    {layer} is the place in the copper stack counted from 1 at the top (the first inner layer is 2)."""
    if len(templates) == 1:
        return templates[0]
    m = re.search(r"([0-9]+)(?=[^0-9]*$)", templates[0])
    if m:
        generic = templates[0][:m.start()] + "{n}" + templates[0][m.end():]
        for token, start in (("{inner}", 1), ("{layer}", 2)):
            if all(t == generic.replace("{n}", str(i)) for i, t in enumerate(templates, start=start)):
                return generic.replace("{n}", token)
    raise ImportError_(f"{name}: the inner layers do not have one numbered name: {templates}")


def describe_cam(title: str, facts: dict) -> str:
    parts = [f"Export profile from the {SOURCE_CAM}: {title}."]
    if facts["gerber_format"]:
        i, d = facts["gerber_format"]
        parts.append(f"Gerber coordinates {i}.{d} in millimetres.")
    else:
        parts.append("The job gives no Gerber coordinate format, so the profile keeps the default.")
    if facts["drill_format"]:
        i, d = facts["drill_format"]
        parts.append(f"Excellon coordinates {i}.{d} in millimetres.")
    parts.append("Only the file names, the formats and the order of the files are taken. The job writes "
                 "the plated and the not plated holes to one drill file; the export keeps two, and the "
                 "not plated one is the same name with _npth before the extension. Not compared with "
                 "the manufacturer.")
    return " ".join(parts)


# ------------------------------------------------------------------------------------- driver

def read(path: str) -> str:
    with open(path, "rb") as f:
        return f.read().decode("utf-8", errors="replace").replace("\r\n", "\n")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dru", required=True)
    ap.add_argument("--cam", required=True)
    ap.add_argument("--out", default=os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    args = ap.parse_args(argv)

    cfg_dir = os.path.join(args.out, "config", "dfm")
    os.makedirs(cfg_dir, exist_ok=True)
    table = ["| Profile | Layers | Track nm | Clearance nm | Drill nm | Ring nm | Edge nm | Hole gap nm | Pad ring nm |",
             "|---|---|---|---|---|---|---|---|---|"]
    for name, rel, title in DRU_PROFILES:
        text, facts = dfm_profile(name, title, parse_dru(read(os.path.join(args.dru, *rel.split("/")))))
        with open(os.path.join(cfg_dir, name + ".cfg"), "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        cells = [str(facts.get(k, "-")) for k in (
            "max_copper_layers", "min_track_width_nm", "min_clearance_nm", "min_drill_nm",
            "min_annular_ring_nm", "min_edge_clearance_nm", "min_drill_spacing_nm", "min_pad_ring_nm")]
        table.append(f"| `{name}` | " + " | ".join(cells) + " |")
    profiles = []
    cam_rows = ["| Profile | Gerber format | Excellon format | Top copper | Bottom copper | Drill file | Order |",
                "|---|---|---|---|---|---|---|"]
    for name, rel, title in CAM_PROFILES:
        profile, facts = cam_profile(name, title, read(os.path.join(args.cam, rel)))
        profile["description"] = describe_cam(title, facts)
        profiles.append(profile)
        n = profile["gerber"]["names"]
        gf = "x".join(map(str, facts["gerber_format"])) if facts["gerber_format"] else "-"
        df = "x".join(map(str, facts["drill_format"])) if facts["drill_format"] else "-"
        cam_rows.append(f"| `{name}` | {gf} | {df} | `{n.get('copper_top', '-')}` | "
                        f"`{n.get('copper_bottom', '-')}` | `{profile.get('drill', {}).get('plated_file', '-')}` | "
                        f"{', '.join(n['order'])} |")
    for name, description, names, drill in EXTRA_EXPORT:
        profiles.append({"name": name, "description": description,
                         "gerber": {"names": dict(names)}, "drill": dict(drill)})
    doc = {"format": "phoenix-export/1", "profiles": profiles}
    with open(os.path.join(args.out, "config", "export_profiles_makers.json"), "w", encoding="utf-8",
              newline="\n") as f:
        json.dump(doc, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print("\n".join(table) + "\n\n" + "\n".join(cam_rows))
    print(f"{len(DRU_PROFILES)} capability profiles, {len(profiles)} export profiles")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except ImportError_ as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(1)
