# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- Add the Rust workspace with a library and the `phoenix` command line.
- Add `phoenix inventory`: counts the design files of a private folder by extension, kind and declared version, with one fingerprint of the content.
- Keep the inventory in redacted mode: no name, path or value from a file reaches the report, with a leak test.
- Bound the inventory (number of files, folder depth, file size) with a settings file, and stop with an error instead of giving a partial count.
- Run format, lints and tests of the Rust code on Linux and Windows in the CI.

## [0.0.1] - The base of the repository

- **The base that every repository of the suite has:** the manifest, the README files in eight languages, this changelog, the CI workflow, the issue and pull-request templates, and the shared checks of EH3D-DOCS (the repository check and the drawers of the two README pictures).
- **A planned project, described honestly:** the scope, the requirements (all planned), the relations with other repositories, the threat model, the decisions that apply and the rules for writing code are in `docs/`; there is no code yet.
