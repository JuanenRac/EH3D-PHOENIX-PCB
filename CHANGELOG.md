# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- Add `AGENTS.md` with the rules for anyone who changes the repository, `docs/adr/` with the four accepted decision records written as ordinary documents, and `deny.toml` with the licence and advisory policy of the dependencies (the CI step that runs it comes once it has been tried locally).
- Add the Rust workspace with a library and the `phoenix` command line.
- Add `phoenix inventory`: counts the design files of a private folder by extension, kind and declared version, with one fingerprint of the content.
- Keep the inventory in redacted mode: no name, path or value from a file reaches the report, with a leak test.
- Bound the inventory (number of files, folder depth, file size) with a settings file, and stop with an error instead of giving a partial count.
- Run format, lints and tests of the Rust code on Linux and Windows in the CI.
- Add the crate `eh3d-phoenix-model` with lengths in integer nanometres inside a checked range of plus or minus 2^30 nm and angles in integer micro-degrees, with typed errors when a value leaves the range.
- Add orientation and segment-intersection predicates that use only 64-bit integers, tested at the corners of the range, and arcs stored as end points plus a signed sweep.
- Add exact decimal reading of millimetres and degrees, with half-to-even rounding that tells the caller when digits were dropped.
- Add UUID v7 object ids in canonical lowercase text with a generator whose clock and random source are injected, so tests are reproducible and ids grow even when the clock stands still or goes back.
- Add canonical JSON (RFC 8785) with the vectors of the standard, a final line feed, and quantisation of millimetre and degree values to six decimals before writing.

## [0.0.1] - The base of the repository

- **The base that every repository of the suite has:** the manifest, the README files in eight languages, this changelog, the CI workflow, the issue and pull-request templates, and the shared checks of EH3D-DOCS (the repository check and the drawers of the two README pictures).
- **A planned project, described honestly:** the scope, the requirements (all planned), the relations with other repositories, the threat model, the decisions that apply and the rules for writing code are in `docs/`; there is no code yet.
