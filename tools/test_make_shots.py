# =============================================================================
# EH3D-PHOENIX-PCB - tools/test_make_shots.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""The pure part of the pictures of the windows: the plan, the names, the command lines, the environment of a run and
the check of a PNG, with PNG files built here. The program itself is never started."""
from __future__ import annotations

import os
import struct
import tempfile
import unittest
import zlib
from pathlib import Path

import make_shots as shots


def png(width: int, height: int, pixel, color: int = 6, filter_type: int = 0) -> bytes:
    """A PNG of 8 bits built by hand; `pixel(x, y)` gives the channels of a pixel. Filter 0 writes the bytes as they
    are and filter 2 writes each row as the difference from the row above."""
    rows = [bytes(v for x in range(width) for v in pixel(x, y)) for y in range(height)]
    raw = bytearray()
    for y, row in enumerate(rows):
        if filter_type == 2:
            above = rows[y - 1] if y else bytes(len(row))
            raw += bytes([2]) + bytes((a - b) & 255 for a, b in zip(row, above))
        else:
            raw += bytes([0]) + row

    def chunk(kind: bytes, body: bytes) -> bytes:
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

    header = struct.pack(">IIBBBBB", width, height, 8, color, 0, 0, 0)
    return shots.PNG_MAGIC + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(bytes(raw))) + chunk(b"IEND", b"")


class PlanTests(unittest.TestCase):
    def test_without_a_board_only_the_windows_that_need_none_are_taken(self) -> None:
        labels = [s.label for s in shots.plan(False, False)]
        self.assertEqual(labels, ["start", "help", "shortcuts", "appearance"])

    def test_with_a_board_the_documents_the_3d_window_and_the_panel_come_after_the_start_screen(self) -> None:
        labels = [s.label for s in shots.plan(True, False)]
        self.assertEqual(
            labels,
            ["start", "document", "document-light", "3d", "panel", "layers", "search", "help", "shortcuts", "appearance"],
        )
        self.assertNotIn("gerber", labels)
        self.assertIn("gerber", [s.label for s in shots.plan(True, True)])

    def test_the_names_are_numbered_in_the_order_of_the_plan(self) -> None:
        names = [shots.file_name(n, s) for n, s in enumerate(shots.plan(True, False), 1)]
        self.assertEqual(names[0], "phoenix-pcb-01-start.png")
        self.assertEqual(names[2], "phoenix-pcb-03-document-light.png")
        self.assertEqual(names[3], "phoenix-pcb-04-3d.png")
        self.assertEqual(len(set(names)), len(names))


class ArgumentTests(unittest.TestCase):
    def test_a_size_is_two_sides_inside_the_limits_of_the_program(self) -> None:
        self.assertEqual(shots.parse_size("1600x900"), (1600, 900))
        self.assertEqual(shots.parse_size("320X8192"), (320, 8192))
        for bad in ("", "1600", "axb", "319x900", "1600x8193", "1x2x3"):
            with self.assertRaises(ValueError, msg=bad):
                shots.parse_size(bad)

    def test_the_command_line_of_a_picture_names_the_window_the_theme_and_the_file_to_open(self) -> None:
        exe, out, board = Path("gui.exe"), Path("out/p.png"), Path("b.brd")
        args = shots.program_arguments(exe, shots.Shot("document", "light"), out, "1600x900", 12, board, None)
        self.assertEqual(
            args,
            ["gui.exe", "--shot", str(out), "--replace", "--size", "1600x900", "--theme", "light", "--wait", "12",
             "--window", "document", "--open", "b.brd"],
        )
        self.assertNotIn("--open", shots.program_arguments(exe, shots.Shot("help"), out, "1600x900", 6, board, None))
        folder = Path("gerbers")
        self.assertEqual(shots.program_arguments(exe, shots.Shot("gerber"), out, "800x600", 6, board, folder)[-2:], ["--open", "gerbers"])

    def test_the_example_board_is_the_first_known_one_that_exists(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.assertIsNone(shots.find_example(root))
            (root / "examples" / "tutorials").mkdir(parents=True)
            (root / "examples" / "tutorials" / "tut.brd").write_text("x")
            self.assertEqual(shots.find_example(root), root / "examples" / "tutorials" / "tut.brd")
            (root / "examples" / "tutorials" / "led_board.brd").write_text("x")
            self.assertEqual(shots.find_example(root).name, "led_board.brd")


class EnvironmentTests(unittest.TestCase):
    def test_every_variable_that_leads_to_the_settings_points_into_the_scratch_folder(self) -> None:
        home = Path("scratch")
        env = shots.isolated_environment({"APPDATA": "real", "HOME": "real", "PATH": "keep"}, home)
        for name in ("APPDATA", "LOCALAPPDATA", "XDG_CONFIG_HOME", "USERPROFILE", "HOME", "TEMP", "TMP"):
            self.assertTrue(env[name].startswith("scratch"), name)
        self.assertEqual(env["PATH"], "keep")

    def test_the_drive_d_is_recognised_where_there_are_drive_letters(self) -> None:
        self.assertFalse(shots.on_drive_d(Path("C:/pictures")))
        if os.name == "nt":
            self.assertTrue(shots.on_drive_d(Path("D:/pictures")))


class PictureTests(unittest.TestCase):
    def test_a_picture_of_one_colour_is_empty(self) -> None:
        stats = shots.analyse_png(png(40, 30, lambda x, y: (0, 0, 0, 255)))
        self.assertEqual((stats["width"], stats["height"], stats["colours"], stats["different"]), (40, 30, 1, 0))

    def test_a_drawn_picture_has_colours_and_different_pixels_with_either_filter_and_with_rgb(self) -> None:
        def drawn(x: int, y: int) -> tuple[int, int, int, int]:
            return (200, 120, 0, 255) if (x + y) % 3 == 0 else (20, 20, 20, 255)

        for filter_type in (0, 2):
            stats = shots.analyse_png(png(30, 20, drawn, filter_type=filter_type))
            self.assertEqual(stats["colours"], 2, filter_type)
            # the pixel (0, 0) is the first one and has the first colour: the others that differ are the 400 pixels
            # of the second colour; of the 600 pixels 200 have (x + y) % 3 == 0
            self.assertEqual(stats["different"], 400, filter_type)
        rgb = shots.analyse_png(png(8, 8, lambda x, y: (x * 30, y * 30, 7), color=2))
        self.assertEqual((rgb["colours"], rgb["different"]), (64, 63))

    def test_what_is_not_a_readable_png_is_refused(self) -> None:
        for data in (b"", b"GIF89a", shots.PNG_MAGIC, shots.PNG_MAGIC + b"\x00\x00\x00\x00IEND\x00\x00\x00\x00"):
            with self.assertRaises(ValueError):
                shots.analyse_png(data)

    def test_the_check_of_a_file_names_what_is_wrong(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.assertEqual(shots.check_picture(root / "missing.png", 400), ["the picture was not written"])
            (root / "empty.png").write_bytes(b"")
            self.assertEqual(shots.check_picture(root / "empty.png", 400), ["the file is empty"])
            (root / "black.png").write_bytes(png(40, 30, lambda x, y: (0, 0, 0, 255)))
            self.assertEqual(shots.check_picture(root / "black.png", 400), ["the picture is empty or of one colour"])
            (root / "good.png").write_bytes(png(40, 30, lambda x, y: ((x * 6) % 256, (y * 8) % 256, (x * y) % 256, 255)))
            self.assertEqual(shots.check_picture(root / "good.png", 400), [])
            self.assertIn("the limit is 0 KB", shots.check_picture(root / "good.png", 0)[0])
            (root / "text.png").write_bytes(b"not a picture")
            self.assertTrue(shots.check_picture(root / "text.png", 400)[0].startswith("cannot be read as a PNG"))


if __name__ == "__main__":
    unittest.main()
