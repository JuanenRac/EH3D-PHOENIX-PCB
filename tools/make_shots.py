#!/usr/bin/env python3
# =============================================================================
# EH3D-PHOENIX-PCB - tools/make_shots.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""Takes the pictures of the windows of `phoenix` for the documentation.

The program is started once for each window with its picture mode (`--shot`, see docs/VIEWER.md), with a size, a
theme, the window to show and, when the window needs one, an example board of the repository. Each run has its own
settings folder inside a temporary folder (APPDATA, HOME and the rest of the variables point there), so the settings
and the recent files of the person are never read or written, and nothing is written on the drive D:. A program that
does not finish in time is stopped by its own process number and by no other.

    py -3.14 -I tools/make_shots.py --exe PATH/phoenix.exe [--out images/screens] [--size 1600x900]
                                    [--wait 12] [--timeout 60] [--only start,document] [--gerber FOLDER]
                                    [--max-kb 400] [--dry-run]

The pictures are named `phoenix-pcb-NN-<window>.png` (`<window>-light` for the light theme) and each one is checked:
it must be a PNG that is not empty or all one colour, and it must weigh less than `--max-kb`. Exit codes: 0 every
picture is right, 1 a picture is missing or wrong, 64 wrong use. Only the standard library is used.
"""
from __future__ import annotations

import argparse
import os
import struct
import subprocess
import sys
import tempfile
import zlib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SIZE = "1600x900"
DEFAULT_WAIT = 12
DEFAULT_TIMEOUT = 60
DEFAULT_MAX_KB = 400
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
# the windows that need no document, and the ones that need a board
NO_FILE = ("start", "help", "shortcuts", "appearance")
ON_BOARD = ("document", "3d", "panel", "layers", "search")
BOARD_EXAMPLES = ("examples/tutorials/led_board.brd", "examples/tutorials/tut.brd")


@dataclass(frozen=True)
class Shot:
    """One picture: the window and the theme."""

    window: str
    theme: str = "dark"

    @property
    def label(self) -> str:
        return self.window if self.theme == "dark" else f"{self.window}-{self.theme}"


def parse_size(text: str) -> tuple[int, int]:
    """`1600x900` as (1600, 900), each side from 320 to 8192 as the program asks."""
    try:
        width, height = (int(part) for part in text.lower().split("x"))
    except ValueError as error:
        raise ValueError(f"the size is WIDTHxHEIGHT, as 1600x900, not {text!r}") from error
    if not all(320 <= side <= 8192 for side in (width, height)):
        raise ValueError("each side of the size is from 320 to 8192 pixels")
    return width, height


def find_example(root: Path) -> Path | None:
    """The board of the repository the pictures open: the first of the known examples that exists."""
    for name in BOARD_EXAMPLES:
        if (root / name).is_file():
            return root / name
    found = sorted((root / "examples").glob("**/*.brd")) if (root / "examples").is_dir() else []
    return found[0] if found else None


def plan(has_board: bool, gerber: bool) -> list[Shot]:
    """The pictures to take, in order. Without an example board only the windows that need none are shown."""
    shots = [Shot("start")]
    if has_board:
        shots += [Shot("document"), Shot("document", "light"), Shot("3d"), Shot("panel"), Shot("layers"), Shot("search")]
    if gerber:
        shots.append(Shot("gerber"))
    shots += [Shot(window) for window in NO_FILE if window != "start"]
    return shots


def file_name(number: int, shot: Shot) -> str:
    return f"phoenix-pcb-{number:02d}-{shot.label}.png"


def program_arguments(exe: Path, shot: Shot, output: Path, size: str, wait: int, example: Path | None, gerber: Path | None) -> list[str]:
    """The command line that takes one picture. `--replace` is given because the pictures are renewed."""
    args = [str(exe), "--shot", str(output), "--replace", "--size", size, "--theme", shot.theme, "--wait", str(wait), "--window", shot.window]
    opened = gerber if shot.window == "gerber" else (example if shot.window in ON_BOARD else None)
    if opened is not None:
        args += ["--open", str(opened)]
    return args


def on_drive_d(path: Path) -> bool:
    return path.resolve().drive.upper() == "D:"


def isolated_environment(base: dict[str, str], home: Path) -> dict[str, str]:
    """The variables of one run: everything the program could use to find the settings of the person points into `home`."""
    env = dict(base)
    config = home / "config"
    for name in ("APPDATA", "LOCALAPPDATA", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME"):
        env[name] = str(config)
    for name in ("USERPROFILE", "HOME"):
        env[name] = str(home)
    for name in ("TEMP", "TMP", "TMPDIR"):
        env[name] = str(home / "tmp")
    return env


def analyse_png(data: bytes) -> dict[str, int]:
    """Reads a PNG of 8 bits per channel, RGB or RGBA, with the standard library, and counts what is drawn.

    The answer holds the width, the height, the colours found (up to 256) and the pixels that differ from the first
    one. A picture that is empty or all of one colour has one colour and no different pixel. Raises ValueError for
    anything else.
    """
    if data[:8] != PNG_MAGIC:
        raise ValueError("not a PNG file")
    position, header, idat = 8, None, bytearray()
    while position + 8 <= len(data):
        length, kind = struct.unpack(">I4s", data[position : position + 8])
        body = data[position + 8 : position + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
        position += 12 + length
    if header is None or not idat:
        raise ValueError("the PNG has no header or no image data")
    width, height, depth, color, _, _, interlace = header
    if depth != 8 or color not in (2, 6) or interlace != 0:
        raise ValueError("only PNG files of 8 bits, RGB or RGBA, not interlaced, are read")
    pixel = 4 if color == 6 else 3
    stride = width * pixel
    raw = zlib.decompress(bytes(idat))
    if len(raw) != height * (stride + 1):
        raise ValueError("the image data has the wrong length")
    colours: set[bytes] = set()
    first = None
    different = 0
    previous = bytearray(stride)
    for row in range(height):
        start = row * (stride + 1)
        kind = raw[start]
        line = bytearray(raw[start + 1 : start + 1 + stride])
        unfilter(kind, line, previous, pixel)
        for offset in range(0, stride, pixel):
            value = bytes(line[offset : offset + pixel])
            if first is None:
                first = value
            elif value != first:
                different += 1
            if len(colours) < 256:
                colours.add(value)
        previous = line
    return {"width": width, "height": height, "colours": len(colours), "different": different}


def unfilter(kind: int, line: bytearray, previous: bytearray, pixel: int) -> None:
    """Undoes the filter of one row of a PNG in place."""
    if kind == 0:
        return
    if kind not in (1, 2, 3, 4):
        raise ValueError(f"unknown PNG filter {kind}")
    for i in range(len(line)):
        left = line[i - pixel] if i >= pixel else 0
        up = previous[i]
        upper_left = previous[i - pixel] if i >= pixel else 0
        if kind == 1:
            add = left
        elif kind == 2:
            add = up
        elif kind == 3:
            add = (left + up) // 2
        else:
            estimate = left + up - upper_left
            near_left, near_up, near_upper = abs(estimate - left), abs(estimate - up), abs(estimate - upper_left)
            add = left if near_left <= near_up and near_left <= near_upper else (up if near_up <= near_upper else upper_left)
        line[i] = (line[i] + add) & 255


def check_picture(path: Path, max_kb: int) -> list[str]:
    """What is wrong with a picture, as a list of reasons; empty when it is right."""
    if not path.is_file():
        return ["the picture was not written"]
    size = path.stat().st_size
    problems = []
    if size == 0:
        return ["the file is empty"]
    if size >= max_kb * 1024:
        problems.append(f"{size // 1024} KB, the limit is {max_kb} KB")
    try:
        stats = analyse_png(path.read_bytes())
    except (ValueError, zlib.error, struct.error) as error:
        return problems + [f"cannot be read as a PNG: {error}"]
    if stats["colours"] < 4 or stats["different"] < max(1, stats["width"] * stats["height"] // 1000):
        problems.append("the picture is empty or of one colour")
    return problems


def run_shot(args: list[str], env: dict[str, str], timeout: int) -> tuple[int | None, str]:
    """Runs the program once. A program that does not finish in time is stopped by its own process number."""
    process = subprocess.Popen(args, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    try:
        _, error = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        process.kill()
        process.communicate()
        return None, f"did not finish in {timeout} s and was stopped"
    return process.returncode, (error or "").strip()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Takes the pictures of the windows of phoenix for the documentation.")
    parser.add_argument("--exe", required=True, type=Path, help="the program phoenix, built with the gui feature")
    parser.add_argument("--out", type=Path, default=ROOT / "images" / "screens", help="the folder of the pictures")
    parser.add_argument("--size", default=DEFAULT_SIZE)
    parser.add_argument("--wait", type=int, default=DEFAULT_WAIT, help="frames drawn before each picture")
    parser.add_argument("--timeout", type=int, default=DEFAULT_TIMEOUT, help="seconds each run may take")
    parser.add_argument("--max-kb", type=int, default=DEFAULT_MAX_KB, help="the most a picture may weigh")
    parser.add_argument("--only", default="", help="labels to take, separated by commas (start, document-light, ...)")
    parser.add_argument("--gerber", type=Path, help="a folder of Gerber and Excellon files for the window gerber")
    parser.add_argument("--root", type=Path, default=ROOT, help="the repository the example board is taken from")
    parser.add_argument("--dry-run", action="store_true", help="print the command lines and run nothing")
    try:
        options = parser.parse_args(argv)
    except SystemExit as stop:
        return 64 if stop.code else 0
    try:
        parse_size(options.size)
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        return 64
    if not options.dry_run and not options.exe.is_file():
        print(f"error: {options.exe} is not a file", file=sys.stderr)
        return 64
    if on_drive_d(options.out):
        print("error: the pictures are never written on the drive D:", file=sys.stderr)
        return 64
    example = find_example(options.root)
    shots = plan(example is not None, options.gerber is not None)
    wanted = {part.strip() for part in options.only.split(",") if part.strip()}
    numbered = [(number, shot) for number, shot in enumerate(shots, 1) if not wanted or shot.label in wanted]
    unknown = wanted - {shot.label for shot in shots}
    if unknown:
        print(f"error: no such picture in the plan: {', '.join(sorted(unknown))}", file=sys.stderr)
        return 64
    if not options.dry_run:
        options.out.mkdir(parents=True, exist_ok=True)
    failures = 0
    with tempfile.TemporaryDirectory(prefix="phoenix-shots-") as scratch:
        home = Path(scratch)
        if on_drive_d(home):
            print("error: the temporary folder is on the drive D:", file=sys.stderr)
            return 64
        for name in ("config", "tmp"):
            (home / name).mkdir()
        env = isolated_environment(dict(os.environ), home)
        for number, shot in numbered:
            target = options.out / file_name(number, shot)
            args = program_arguments(options.exe, shot, target, options.size, options.wait, example, options.gerber)
            if options.dry_run:
                print(" ".join(args))
                continue
            code, message = run_shot(args, env, options.timeout)
            problems = [] if code == 0 else [f"exit code {code}: {message}" if code is not None else message]
            problems += check_picture(target, options.max_kb) if code == 0 else []
            if problems:
                failures += 1
            print(f"{'FAIL' if problems else 'ok  '} {target.name}" + "".join(f"\n       {p}" for p in problems))
    if not options.dry_run:
        print(f"{len(numbered) - failures} of {len(numbered)} pictures are right")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
