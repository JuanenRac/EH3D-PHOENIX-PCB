# =============================================================================
# EH3D-PHOENIX-PCB - tools/coverage_inventory.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
"""Inventory of the code of production that no test names.

For every `.rs` file under `crates/*/src` it finds out whether a test exercises it: a `#[test]` in the file itself,
a `#[test]` in a test file of the crate (`tests/*.rs`, `*_tests.rs`, `tests.rs`, or any other crate) that names the
module or one of its public functions. It also lists the public functions that no test code names. This is a count
of names, not a measurement of lines run: it finds the code nobody tried to test, and says nothing about how well
the rest is tested.

    python -I tools/coverage_inventory.py            # modules without any test, then the functions, by size
    python -I tools/coverage_inventory.py --all      # every module with its numbers
"""
from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"
PUB_FN = re.compile(r"(?m)^\s*pub\s+(?:const\s+)?(?:unsafe\s+)?fn\s+([a-z_][a-z0-9_]*)")
TEST_ATTR = re.compile(r"#\[(?:tokio::)?test\b")
CFG_TEST_MOD = re.compile(r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?mod\s+(\w+)")


@dataclass
class Module:
    crate: str
    path: Path
    lines: int
    own_tests: int
    functions: list[str] = field(default_factory=list)
    unreferenced: list[str] = field(default_factory=list)
    named_by_tests: bool = False

    @property
    def name(self) -> str:
        return self.path.stem

    @property
    def where(self) -> str:
        return self.path.relative_to(ROOT).as_posix()


def is_test_file(path: Path) -> bool:
    posix = path.as_posix()
    return "/tests/" in posix or path.name.endswith("_tests.rs") or path.name == "tests.rs" or path.name.startswith("test_")


def strip_comments(text: str) -> str:
    text = re.sub(r"(?s)/\*.*?\*/", " ", text)
    return re.sub(r"(?m)//.*$", "", text)


def production_part(text: str) -> str:
    """The text of a file up to the module of tests that closes it."""
    m = CFG_TEST_MOD.search(text)
    return text[: m.start()] if m else text


def test_part(text: str) -> str:
    m = CFG_TEST_MOD.search(text)
    return text[m.start() :] if m else ""


def load() -> tuple[list[Module], str]:
    files = [p for p in sorted(CRATES.rglob("*.rs")) if "/target/" not in p.as_posix()]
    test_text: list[str] = []
    sources: list[tuple[Path, str]] = []
    for path in files:
        text = strip_comments(path.read_text(encoding="utf-8"))
        if is_test_file(path):
            test_text.append(text)
        else:
            test_text.append(test_part(text))
            sources.append((path, text))
    corpus = "\n".join(test_text)
    modules: list[Module] = []
    for path, text in sources:
        if "/src/" not in path.as_posix():
            continue
        crate = path.relative_to(CRATES).parts[0]
        production = production_part(text)
        own = len(TEST_ATTR.findall(text))
        # the tests of a module may live beside it, in `<name>_tests.rs`, `<name>_test.rs` or `tests_<name>.rs`
        for sibling in (f"{path.stem}_tests.rs", f"{path.stem}_test.rs", f"tests_{path.stem}.rs"):
            other = path.parent / sibling
            if other.is_file():
                own += len(TEST_ATTR.findall(other.read_text(encoding="utf-8")))
        lines = production.count("\n") + 1
        functions = sorted(set(PUB_FN.findall(production)))
        modules.append(Module(crate, path, lines, own, functions))
    return modules, corpus


def analyse() -> list[Module]:
    modules, corpus = load()
    words = set(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", corpus))
    for module in modules:
        module.unreferenced = [f for f in module.functions if f not in words]
        module.named_by_tests = module.own_tests > 0 or module.name in words
    return modules


def is_window(module: Module) -> bool:
    return module.name.endswith("_gui") or module.name in ("gui", "main", "gui_main")


def candidates(modules: list[Module]) -> list[tuple[Module, list[str]]]:
    """Modules of the engine (no window, no wrapper of a command) with public functions that no test code names."""
    found = []
    for module in sorted(modules, key=lambda m: -m.lines):
        names = [f for f in module.unreferenced if not f.endswith("_command")]
        if names and not is_window(module):
            found.append((module, names))
    return found


def main(argv: list[str]) -> int:
    modules = analyse()
    bare = sorted((m for m in modules if m.own_tests == 0 and m.lines > 20), key=lambda m: -m.lines)
    print(f"{len(modules)} modules, {sum(m.own_tests for m in modules)} tests in the sources")
    print("\nModules with no test of their own (production lines, public functions, functions no test names, module named by a test):")
    for m in bare:
        print(f"  {m.lines:5} {len(m.functions):3} {len(m.unreferenced):3} {'named' if m.named_by_tests else 'NOT NAMED':9} {m.where}")
    print("\nPublic functions that no test code names (engine modules, by size; the wrappers of commands are exercised through `run`):")
    for module, names in candidates(modules):
        print(f"  {module.lines:5} {module.where}: {', '.join(names)}")
    if "--windows" in argv:
        print("\nWindow modules:")
        for m in sorted(modules, key=lambda m: -m.lines):
            if is_window(m):
                print(f"  {m.lines:5} {m.where}: {', '.join(m.unreferenced)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
