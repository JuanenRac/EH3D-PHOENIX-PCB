# Testing: what is tested, how, and where the example files live

This page is the policy for tests of EH3D-PHOENIX-PCB. A test here proves something about a piece of code with an answer that was **worked out by hand** or taken from a published formula; it is never a copy of what the code printed the first time it ran.

## What the tests are, and where they live

| Kind | Where | What it proves |
|---|---|---|
| Unit tests | `#[cfg(test)] mod tests` at the end of a source file, or a file beside the module named `<module>_tests.rs` | one function or one type, with its limits and its refusals |
| Tests of a crate's public interface | `crates/<crate>/tests/*.rs` | the crate as another crate sees it: only public items |
| Tests that fill a gap | `crates/<crate>/tests/cov_*.rs` | modules that the inventory below found with little or no test (readers, writers, editor operations, numeric calculations) |
| Reference cases | `tests/oracle/*.json`, `docs/ORACLE.md` | calculations against published values, with the source and the tolerance written beside the number |
| The own corpus | `tests/corpus/<format>/`, `docs/CORPUS.md` | readers and writers on small pieces written by hand, each with an expectation file |
| Tutorials and guides | `docs/tutorials/`, `docs/tutorials/guides/`, `examples/tutorials/`, `examples/guides/` | the blocks of a tutorial or a guide marked `# test` are run and the beginning of each output is compared with the text |
| Hostile input | `config/fuzz.cfg`, `docs/SECURITY_REVIEW.md` | every reader given files that are cut, repeated, changed byte by byte, nested too deep or extreme in their numbers, with a fixed seed, and every seed cut at fixed places and repeated |
| Goldens | `tests/golden`, `crates/eh3d-phoenix-cli/tests/goldens.rs` | the exact text, JSON, CSV and HTML that a command gives, worked out by hand and compared on text with the line ends of Unix, with the identity of an object masked and no path or date in the file |
| The base of the repository | `tools/test_*.py` | versions agree, the eight READMEs exist, the eight languages agree, the inventory of untested code works |
| The repository check | `tools/ci_validate.py` | manifest, documentation, parity of the languages, the pictures |

`cargo test` runs the Rust tests; `python -I -m unittest discover -s tools -p "test_*.py"` runs the tests of the base. The CI runs both, plus `cargo fmt --check`, clippy with warnings as errors, and the check of the pictures (`tools/make_architecture.py --check`, `tools/make_structure.py --check --readmes`).

## How a test is written

* The name is a sentence that says what must be true: `a_series_capacitor_and_a_shunt_inductor_have_the_opposite_reactance`.
* Normal case, limit case and damaged input, each in its own test when the three behave differently. A damaged input is checked for the **error that is returned** (its code, its line or byte, its message), never only that "something failed".
* The expected value is computed by hand and the line of the test or a comment says how (`// 5 * 0.5 ms / 1 ms`). A value copied from the output of the code under test proves nothing.
* No test depends on the order of a `HashMap`, on the clock, on the network, on a random number without a fixed seed or on a path outside a folder that the test creates (`docs/SECURITY_REVIEW.md` lists the checks that enforce this).
* A float is compared with a tolerance that is written in the test, unless the value is exact by construction.
* A test file never uses the real files of a maker, a project or a user. Small pieces are written in the test or kept in `tests/corpus`.
* A new test must fail when the code it covers is broken: change the expected number in your head, or in a copy, and see that the test would catch it.
* Texts shown to a user have eight languages (en, es, de, fr, it, ja, zh, pt, the Portuguese of Portugal); `tools/test_i18n.py` checks every table of texts and every translated page (`docs/I18N_AUDIT.md`).

## Where the example files live

| Folder | Holds |
|---|---|
| `tests/corpus/{eagle,kicad,gerber,model,route,spice}/` | pieces written by hand, with a `NAME.expect.json` beside each |
| `tests/oracle/` | the reference cases (`board`, `field`, `netlist`, `rf`, `si`, `sim`, `stackup`) and `pending.json`, the cases that are written but not yet run |
| `tests/dxf/` | the small DXF files used by the DXF tests |
| `tests/golden/` | the expected output of commands (`check` in text, JSON, CSV and HTML) and the rule file that gives it; `PHOENIX_BLESS_GOLDENS=1` writes them again when an output changes on purpose, and the difference is read before it is committed |
| `examples/tutorials/` | the small files that the tutorials use |
| `examples/scripts/` | the example scripts, each with the output it gives |
| `config/*.cfg` | the settings of each tool; a value that is tuned lives here, not in the code |

## How to add a case to the corpus

1. Write the smallest piece that shows the behaviour, by hand, in the folder of its format (`tests/corpus/kicad/` for a KiCad file). Do not copy a file from a project of someone else.
2. Work out the numbers (counts, lengths, positions) from the piece itself, on paper.
3. Write `NAME.expect.json` (or `NAME.TAG.expect.json` when one file has several expectations): `id` (`CORPUS-<AREA>-<NNN>`, unique), `title`, `kind`, `input`, `how` (one line for each number: how it was worked out, never empty), and `expect` with the outcome (`read` or `refused`), the codes that must be seen and no other, and the measures. The fields are in `docs/CORPUS.md`.
4. A file that must be refused gets `expect.outcome = "refused"` and the code of the refusal. A reader that is expected to reach a limit uses `limits` to lower the limit for this piece so that the file stays small.
5. Run `cargo test -p eh3d-phoenix-corpus`. Then update the count of expectations in `docs/CORPUS.md`.

A reference case for a calculation is added the same way in `tests/oracle/<area>.json` with its source and its tolerance; a case that is written but not run yet goes to `pending.json` with the reason.

## The inventory of code that no test names

`python -I tools/coverage_inventory.py` reads every `.rs` file under `crates/*/src` and reports

* the modules with no test of their own (no `#[test]` in the file or in `<module>_tests.rs`), by size, with whether any test names the module;
* the public functions that no test code names, for the modules of the engine (the windows and the wrappers of commands are left apart: the wrappers are exercised through `run`).

It counts **names**, not lines run. It finds code that nobody tried to test; it says nothing about how well the rest is tested. A function can be named in a test and still be tried on one easy case. The list is the way to choose where the next tests go, and `tools/test_coverage_inventory.py` keeps the tool itself honest.

The tests in `cov_*.rs` files came from this list. They are written for the public interface only, so a function that is private to its crate is tried through the public function that uses it.

## Counting tests

The counters in the README, in `docs/STATUS.md` and in the badge are the number of `#[test]` in the sources committed to the repository (`git grep -c "#\[test\]" -- "*.rs"`, added up), and the number of tests that `unittest discover` runs in `tools`. A test that runs many cases (a loop over a table) counts once. Update the counters when you add or remove tests, in the eight READMEs and in `docs/STATUS.md`.

## What is not tested yet

Listed honestly in `docs/STATUS.md` and `docs/NOT_YET.md`: nothing here has run on a real design file of a maker or a user; the windows have never been seen on a screen; the comparison of the calculations with EAGLE, KiCad, a field solver and a SPICE program is planned and not done.
