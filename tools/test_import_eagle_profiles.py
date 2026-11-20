# =============================================================================
# EH3D-PHOENIX-PCB - tools/test_import_eagle_profiles.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""The tool that makes the profiles of the manufacturers: lengths in exact nanometres, the roles and the names of
the layer files, and the profiles that it wrote. The inputs are synthetic text in the formats of the sources; no file of
any other program is part of the tests."""
from __future__ import annotations

import json
import random
import re
import unittest
from pathlib import Path

import import_eagle_profiles as tool

ROOT = Path(__file__).resolve().parents[1]

DRU = """description[en] = anything here is ignored\\n
layerSetup = (1+2*15+16)
mdWireWire = 6mil
mdWirePad = 0.2mm
mdCopperDimension = 0.0130inch
mdDrill = 8mil
msWidth = 0.15mm
msDrill = 0.3mm
rlMinPadTop = 10mil
rlMinViaOuter = 0.125mm
"""

INI = """[CAM Processor Job]
Description[en]="ignored"
Section=Sec_1
Section=Sec_2
Section=Sec_3
Section=Sec_4
Section=Sec_5
Section=Sec_6

[Sec_1]
Name[en]="Top"
Device="GERBER_RS274X"
Output=".TOP"
Layers=" 1 17 18"

[Sec_2]
Name[en]="Inner 1"
Device="GERBER_RS274X"
Output="%P/%N.L2"
Layers=" 2 17 18"

[Sec_3]
Name[en]="Inner 2"
Device="GERBER_RS274X"
Output="%P/%N.L3"
Layers=" 15 17 18"

[Sec_4]
Name[en]="Bottom"
Device="GERBER_RS274X"
Output="%N.BOTTOM"
Layers=" 16 17 18 20"

[Sec_5]
Name[en]="Drill"
Device="EXCELLON"
Output="%N.DRD"
Layers=" 44 45"

[Sec_6]
Name[en]="Dimension"
Device="GERBER_RS274X"
Output=".DIM"
Layers=" 20"
"""


def gerber(fmt: str, layers: list[int]) -> dict:
    return {"type": "gerber_layer", "filename_format": fmt, "layers": layers, "board_outline": False}


JSON_CAM = json.dumps({
    "units": "metric",
    "outputs": [
        {"output_type": "gerber", "format_specifier": {"integer": 3, "decimal": 4}, "outputs": [
            gerber("%N.GKO", [46, 20]), gerber("%N_%FF.GTL", [1, 17, 18]),
            gerber("%N_%FF.G1", [2, 17, 18]), gerber("%N_%FF.G2", [15, 17, 18]),
            gerber("%N_%FF.GBL", [16, 17, 18]), gerber("%N_%FF.GTS", [29]),
            gerber("%N_%FF.GBO", [22, 26]),
        ]},
        {"output_type": "drill", "format_specifier": {"integer": 3, "decimal": 3}, "outputs": [
            {"type": "autodrills", "filename_format": "%N_drill.XLN"},
        ]},
        {"output_type": "assembly", "outputs": []},
    ],
})


class LengthTests(unittest.TestCase):
    def test_every_unit_gives_exact_nanometres(self) -> None:
        for text, nm in [("8mil", 203_200), ("0.205mm", 205_000), ("0.0130inch", 330_200), ("0.1499mm", 149_900),
                         ("0mil", 0), ("2.5mil", 63_500), ("12micron", 12_000)]:
            self.assertEqual(tool.to_nm(text), nm, text)

    def test_a_length_that_is_not_a_whole_number_of_nanometres_stops_the_run(self) -> None:
        for text in ["0.0000001mm", "1.00000004mm"]:
            with self.assertRaises(tool.ImportError_):
                tool.to_nm(text)
        for text in ["", "mil", "5 parsecs", "1,5mm"]:
            with self.assertRaises(tool.ImportError_):
                tool.to_nm(text)

    def test_random_mil_values_are_exact(self) -> None:
        rng = random.Random(17)
        for _ in range(200):
            tenths = rng.randrange(0, 5000)  # a value in tenths of a mil: 2540 nm each
            text = f"{tenths // 10}.{tenths % 10}mil"
            self.assertEqual(tool.to_nm(text), tenths * 2540, text)

    def test_the_layers_of_a_stack_are_counted(self) -> None:
        for setup, count in [("(1*16)", 2), ("16", 1), ("(1*2*15*16)", 4), ("(1+2*15+16)", 4),
                             ("(1+2*3+14*15+16)", 6)]:
            self.assertEqual(tool.layer_count(setup), count, setup)
        self.assertIsNone(tool.layer_count(None))


class CapabilityProfileTests(unittest.TestCase):
    def test_the_numbers_of_the_rules_go_to_the_lines_of_the_profile(self) -> None:
        text, facts = tool.dfm_profile("x-dru", "Maker", tool.parse_dru(DRU))
        self.assertEqual(facts, {"min_track_width_nm": 150_000, "min_clearance_nm": 152_400,
                                 "min_drill_nm": 300_000, "min_annular_ring_nm": 125_000,
                                 "min_edge_clearance_nm": 330_200, "min_drill_spacing_nm": 203_200,
                                 "min_pad_ring_nm": 254_000, "max_copper_layers": 4})
        lines = {k.strip(): v.strip() for k, v in (l.split("=", 1) for l in text.splitlines()
                                                    if l and not l.startswith("#"))}
        for key, value in facts.items():
            self.assertEqual(lines[key], str(value))
        self.assertEqual(lines["verified"], "false")
        self.assertEqual(lines["source"], "design rules of the manufacturer as shipped with EAGLE")

    def test_a_line_that_the_rules_do_not_give_is_left_out(self) -> None:
        text, facts = tool.dfm_profile("x-dru", "Maker", {"msWidth": "6mil"})
        self.assertEqual(facts, {"min_track_width_nm": 152_400})
        self.assertNotIn("min_drill_nm", text)
        self.assertNotIn("max_copper_layers", text)
        for forbidden in ["min_via_diameter_nm", "min_mask_bridge_nm", "max_holes", "max_board_long_nm"]:
            self.assertNotIn(forbidden, text)

    def test_the_text_has_no_path_no_date_and_is_the_same_every_time(self) -> None:
        once = tool.dfm_profile("x-dru", "Maker", tool.parse_dru(DRU))[0]
        again = tool.dfm_profile("x-dru", "Maker", tool.parse_dru(DRU))[0]
        self.assertEqual(once, again)
        self.assertNotRegex(once, r"\b20[0-9]{2}\b|[A-Za-z]:[\\\\/]|/Users|TRABAJO")


class RoleAndNameTests(unittest.TestCase):
    def test_the_role_of_an_output_comes_from_its_layers(self) -> None:
        cases = {
            (1, 17, 18): "copper_top", (16, 17, 18, 20): "copper_bottom", (2, 17, 18): "copper_inner",
            (15, 17, 18, 20): "copper_inner", (29,): "mask_top", (20, 30): "mask_bottom",
            (31,): "paste_top", (20, 32): "paste_bottom", (21, 25): "silk_top", (22, 26): "silk_bottom",
            (46, 20): "outline", (20,): "outline", (51,): "silk_top", (52,): "silk_bottom",
        }
        for layers, role in cases.items():
            self.assertEqual(tool.role_of(list(layers)), role, layers)
        self.assertIsNone(tool.role_of([44, 45]))

    def test_a_name_is_the_board_then_what_the_job_puts_after_it(self) -> None:
        for fmt, template in [("%N.GTL", "{board}.GTL"), ("%N_%FF.GTL", "{board}.GTL"),
                              (".slk", "{board}.slk"), ("%P/%N.build/%N.t.ger", "{board}.t.ger"),
                              ("./%N.top", "{board}.top"), ("fixed.txt", "fixed.txt")]:
            self.assertEqual(tool.suffix_of(fmt), template, fmt)
        self.assertEqual(tool.suffix_of("%DRILLPREFIX/drills_%FROM_%TO.xln", (1, 16)), "drills_1_16.xln")
        with self.assertRaises(tool.ImportError_):
            tool.suffix_of("%N.%ZZ")

    def test_the_inner_layers_share_one_template(self) -> None:
        self.assertEqual(tool.inner_template("x", ["{board}.G1", "{board}.G2"]), "{board}.G{inner}")
        self.assertEqual(tool.inner_template("x", ["{board}.L2", "{board}.L3"]), "{board}.L{layer}")
        self.assertEqual(tool.inner_template("x", ["{board}.in"]), "{board}.in")
        with self.assertRaises(tool.ImportError_):
            tool.inner_template("x", ["{board}.A", "{board}.B"])


class ExportProfileTests(unittest.TestCase):
    def test_a_job_in_the_old_format_gives_names_order_and_drill(self) -> None:
        profile, facts = tool.cam_profile("t-cam", "Test", INI)
        names = profile["gerber"]["names"]
        self.assertEqual(names["copper_top"], "{board}.TOP")
        self.assertEqual(names["copper_inner"], "{board}.L{layer}")
        self.assertEqual(names["copper_bottom"], "{board}.BOTTOM")
        self.assertEqual(names["outline"], "{board}.DIM")
        self.assertEqual(names["order"], ["copper_top", "copper_inner", "copper_bottom", "outline"])
        self.assertEqual(profile["drill"], {"plated_file": "{board}.DRD", "non_plated_file": "{board}_npth.DRD"})
        self.assertNotIn("decimal_digits", profile["gerber"])
        self.assertIsNone(facts["gerber_format"])
        self.assertEqual(facts["inner_layers"], [2, 15])

    def test_a_job_in_the_new_format_gives_the_formats_too(self) -> None:
        profile, facts = tool.cam_profile("t-cam", "Test", JSON_CAM)
        names = profile["gerber"]["names"]
        self.assertEqual(names["copper_inner"], "{board}.G{inner}")
        self.assertEqual(names["silk_bottom"], "{board}.GBO")
        self.assertEqual(profile["gerber"]["decimal_digits"], 4)
        self.assertEqual(profile["drill"], {"plated_file": "{board}_drill.XLN",
                                            "non_plated_file": "{board}_drill_npth.XLN", "style": "fixed",
                                            "fixed_integer_digits": 3, "fixed_decimal_digits": 3})
        self.assertEqual(facts["gerber_format"], (3, 4))
        self.assertEqual(facts["drill_format"], (3, 3))
        self.assertIn("3.4", tool.describe_cam("Test", facts))

    def test_the_same_job_gives_the_same_profile(self) -> None:
        self.assertEqual(tool.cam_profile("a", "A", JSON_CAM), tool.cam_profile("a", "A", JSON_CAM))

    def test_two_names_for_one_role_stop_the_run(self) -> None:
        job = json.dumps({"outputs": [{"output_type": "gerber", "outputs": [
            gerber("%N.GTL", [1]), gerber("%N.TOP", [1, 17])]}]})
        with self.assertRaises(tool.ImportError_):
            tool.cam_profile("a", "A", job)


class WrittenProfilesTests(unittest.TestCase):
    """The files that the tool wrote and that are versioned."""

    def test_each_capability_profile_is_a_list_of_known_keys_with_whole_numbers(self) -> None:
        known = {"name", "source", "verified", "min_track_width_nm", "min_clearance_nm", "min_drill_nm",
                 "min_annular_ring_nm", "min_edge_clearance_nm", "min_drill_spacing_nm", "min_pad_ring_nm",
                 "max_copper_layers"}
        for name, _, _ in tool.DRU_PROFILES:
            path = ROOT / "config" / "dfm" / f"{name}.cfg"
            keys = []
            for line in path.read_text(encoding="utf-8").splitlines():
                if not line.strip() or line.startswith("#"):
                    continue
                key, value = (p.strip() for p in line.split("=", 1))
                keys.append(key)
                self.assertIn(key, known, f"{name}: {key}")
                if key.endswith("_nm") or key == "max_copper_layers":
                    self.assertRegex(value, r"^[0-9]+$", f"{name}: {key}")
            self.assertEqual(len(keys), len(set(keys)), name)
            self.assertIn("verified", keys)

    def test_the_export_profiles_are_the_ones_the_tool_lists(self) -> None:
        doc = json.loads((ROOT / "config" / "export_profiles_makers.json").read_text(encoding="utf-8"))
        self.assertEqual(doc["format"], "phoenix-export/1")
        listed = [n for n, _, _ in tool.CAM_PROFILES] + [n for n, *_ in tool.EXTRA_EXPORT]
        self.assertEqual([p["name"] for p in doc["profiles"]], listed)
        for profile in doc["profiles"]:
            self.assertTrue(profile["description"], profile["name"])
            self.assertNotRegex(profile["description"], r"\b20[0-9]{2}\b|[A-Za-z]:[\\\\/]|TRABAJO")
            names = profile["gerber"]["names"]
            for role, template in names.items():
                if role == "order":
                    self.assertTrue(set(template) <= set(tool.ROLES), profile["name"])
                    continue
                self.assertIn(role, tool.ROLES)
                self.assertEqual(sorted(set(re.findall(r"\{(\w+)\}", template)) - {"board", "inner", "layer"}), [],
                                 template)

    def test_a_listed_name_is_not_used_twice(self) -> None:
        names = [n for n, _, _ in tool.DRU_PROFILES] + [n for n, _, _ in tool.CAM_PROFILES]
        names += [n for n, *_ in tool.EXTRA_EXPORT]
        self.assertEqual(len(names), len(set(names)))


if __name__ == "__main__":
    unittest.main()
