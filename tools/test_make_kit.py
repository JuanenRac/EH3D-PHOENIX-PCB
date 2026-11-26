#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""The starter kit is what tools/make_kit.py writes, and its catalogue and geometry are sound. The
checks of the pieces themselves (rules, round trips, user programs) are in
crates/eh3d-phoenix-cli/tests/kit.rs."""
from __future__ import annotations

import hashlib
import json
import math
import sys
import unittest
from pathlib import Path
from xml.etree import ElementTree

sys.path.insert(0, str(Path(__file__).resolve().parent))
import make_kit  # noqa: E402

KIT = make_kit.KIT


class KitTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.files = make_kit.all_files()
        cls.items = json.loads((KIT / "manifest.json").read_text(encoding="utf-8"))["items"]

    def test_the_files_are_what_the_generator_writes(self) -> None:
        for path, text in self.files.items():
            self.assertEqual(make_kit.read_bytes(KIT / path), text.encode("utf-8"), f"kit/{path}: run tools/make_kit.py")

    def test_the_catalogue_lists_every_file_with_its_hash(self) -> None:
        listed = {"manifest.json"}
        for item in self.items:
            self.assertEqual((item["license"], item["provenance"], item["verified"]), ("GPL-3.0-or-later", "own", False), item["id"])
            self.assertEqual(sorted(item["title"]), sorted(make_kit.LANGS), item["id"])
            for entry in item["files"]:
                listed.add(entry["path"])
                digest = hashlib.sha256((KIT / entry["path"]).read_bytes()).hexdigest()
                self.assertEqual(digest, entry["sha256"], f"{entry['path']}: the bytes in the folder (line ends too) differ from the catalogue")
        present = {p.relative_to(KIT).as_posix() for p in KIT.rglob("*") if p.is_file()}
        self.assertEqual(present, listed)

    def test_the_design_files_are_xml_and_the_counts_are_the_ones_promised(self) -> None:
        for path in KIT.rglob("*"):
            if path.suffix in (".lbr", ".brd", ".sch"):
                self.assertEqual(ElementTree.fromstring(path.read_bytes()).tag, "eagle", path.name)
        kinds = [item["type"] for item in self.items]
        self.assertEqual((kinds.count("library"), kinds.count("board"), kinds.count("schematic"), kinds.count("program")), (5, 12, 12, 20))

    def test_every_package_is_in_a_library_and_every_title_has_its_words(self) -> None:
        used = {pkg for names in make_kit.LIBRARIES.values() for name in names for pkg in make_kit.DEVICESETS[name][2].values()}
        self.assertEqual(used, set(make_kit.PACKAGES))
        self.assertEqual(sorted(n for names in make_kit.LIBRARIES.values() for n in names), sorted(make_kit.DEVICESETS))

    def test_the_pads_of_a_package_do_not_touch(self) -> None:
        for name, (_, _, items) in make_kit.PACKAGES.items():
            pads = [i for i in items if i["kind"] in ("pad", "smd")]
            for index, a in enumerate(pads):
                for b in pads[index + 1:]:
                    wide = [(p["dx"], p["dy"]) if p["kind"] == "smd" else (p["diameter"], p["diameter"]) for p in (a, b)]
                    gap_x = abs(a["x"] - b["x"]) - (wide[0][0] + wide[1][0]) / 2
                    gap_y = abs(a["y"] - b["y"]) - (wide[0][1] + wide[1][1]) / 2
                    self.assertGreaterEqual(max(gap_x, gap_y), 200, f"{name}: pads {a['name']} and {b['name']}")

    def test_the_vias_of_the_samples_keep_apart(self) -> None:
        for design in make_kit.DESIGNS:
            vias = [v for signal in make_kit.model(design)["signals"] for v in signal["vias"]]
            for index, a in enumerate(vias):
                for b in vias[index + 1:]:
                    self.assertGreaterEqual(math.hypot(a[0] - b[0], a[1] - b[1]), make_kit.VIA_DIAMETER + make_kit.CLEARANCE, design["name"])


if __name__ == "__main__":
    unittest.main()
