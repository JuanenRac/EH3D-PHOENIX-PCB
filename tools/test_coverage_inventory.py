# =============================================================================
# EH3D-PHOENIX-PCB - tools/test_coverage_inventory.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""The inventory of the code that no test names: it reads the sources, counts the tests that sit beside a module
and does not mistake a module of a window for an engine module."""
from __future__ import annotations

import unittest

import coverage_inventory as inventory


class InventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.modules = inventory.analyse()

    def test_every_source_module_of_every_crate_is_listed(self) -> None:
        self.assertGreater(len(self.modules), 400)
        crates = {m.crate for m in self.modules}
        self.assertGreater(len(crates), 30)
        self.assertTrue(all(m.where.startswith("crates/") and "/src/" in m.where for m in self.modules))

    def test_the_tests_in_a_file_beside_the_module_count_for_the_module(self) -> None:
        arc = next(m for m in self.modules if m.where == "crates/eh3d-phoenix-boardedit/src/arc.rs")
        self.assertGreater(arc.own_tests, 0)

    def test_the_tests_in_the_module_itself_count(self) -> None:
        xml = next(m for m in self.modules if m.where == "crates/eh3d-phoenix-eagle/src/xml.rs")
        self.assertGreater(xml.own_tests, 0)

    def test_a_function_that_a_test_names_is_not_listed_as_unnamed(self) -> None:
        touchstone = next(m for m in self.modules if m.where == "crates/eh3d-phoenix-rf/src/touchstone.rs")
        self.assertIn("parse", touchstone.functions)
        self.assertNotIn("parse", touchstone.unreferenced)

    def test_the_windows_are_kept_apart_from_the_engine(self) -> None:
        engine = {m.where for m, _ in inventory.candidates(self.modules)}
        self.assertFalse([w for w in engine if w.endswith("_gui.rs") or w.endswith("/gui.rs")])

    def test_the_text_without_comments_does_not_count_a_name_in_a_comment(self) -> None:
        self.assertEqual(inventory.strip_comments("a // fn secret()\nb /* fn other() */ c").split(), ["a", "b", "c"])

    def test_the_production_part_stops_at_the_module_of_tests(self) -> None:
        text = "pub fn a() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n"
        self.assertEqual(inventory.production_part(text).strip(), "pub fn a() {}")
        self.assertIn("fn t()", inventory.test_part(text))


if __name__ == "__main__":
    unittest.main()
