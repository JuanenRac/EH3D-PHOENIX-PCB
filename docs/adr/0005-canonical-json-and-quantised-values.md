# 0005: new JSON files use the canonical form of RFC 8785 and quantised values

Status: accepted. The decision was applied in the code before this record was written. The number 0005 and the subject are to be confirmed by the project owner: the development plan leaves this number without a stated subject and cites this decision under another number, which this repository uses for the circuit simulator.

## Context

A file that is compared, hashed or signed must have one byte form for one content. JSON allows any order of keys, any spacing and several spellings of a number. Exchange files in millimetres or degrees also carry the noise of floating-point conversions (`0.30000000000000004`).

## Options

* **Pretty-printed JSON in the order of the structs.** Readable, stable only as long as nobody reorders a struct.
* **A private canonical form.** Under our control, and one more thing to document and port.
* **The JSON Canonicalization Scheme (RFC 8785).** A published standard with test vectors: keys sorted by UTF-16 code unit, no insignificant whitespace, numbers in the shortest ECMAScript form, minimal string escapes.

## Decision

Every new JSON file of the design uses the canonical form of RFC 8785, in UTF-8 without a byte order mark, followed by one line feed. Design values are integers with the unit in the name (suffixes `_nm` and `_udeg`); values in millimetres or degrees that leave the design are quantised to six decimals before writing; an integer that no double represents is refused (`PHOENIX-JSON-001`). Physical results with an uncertainty are compared numerically and not byte for byte. The existing `.eh3dproj` 1.0.0 behaviour does not change.

## Reasons

* Hashes, signatures and byte comparisons of two writes are meaningful.
* The standard has vectors, so the writer can be tested against something that is not ours.
* One writer for every format of the suite (the shared container crate), so a second implementation does not drift.

## Consequences

* The writer is `eh3d-container::canonical_string` and `canonical_file_bytes`; `eh3d-phoenix-model::canonical` keeps the names the model has always had and pins the vectors of the standard as tests.
* A new field is written only when it is set (`skip_serializing_if`), so an absent value and a default are the same file.
* Reports for people (text, CSV) are not canonical JSON and make no such promise.

## How it is verified

The vectors of the standard in `eh3d-phoenix-model` (`rfc_8785_section_3_2_3_key_order_is_by_utf16_code_unit` and the others of the module), the tests that write twice and compare the bytes, and the quantisation tests (`quantising_is_idempotent`). Review date: when a second program writes the same format.
