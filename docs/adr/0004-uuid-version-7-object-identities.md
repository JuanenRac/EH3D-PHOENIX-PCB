# 0004: every design object has an immutable UUID version 7

Status: accepted. The decision was applied in the code before this record was written; the record is pending a review by the project owner (to be confirmed).

## Context

A design object (a component, a net, a track) must keep its identity when it is renamed, moved between files, undone and redone, or compared between two versions of a board. The reference designator (`R5`) is an attribute that changes, so it cannot be the identity. Ids also have to be reproducible in tests and must not depend on the clock inside a writer.

## Options

* **A counter per file.** Short, but two files cannot be merged and a copied object collides.
* **A random UUID version 4.** No collisions, but no order: a list sorted by id is shuffled at every run, and a database index scatters.
* **A ULID.** Ordered by time, but a text form of its own and no standard; the suite uses UUIDs everywhere else.
* **A UUID version 7 (RFC 9562).** A 48-bit millisecond time followed by random bits: unique, ordered by creation, and the canonical lowercase text form is a standard.

## Decision

Every design object has an immutable UUID version 7, written in canonical lowercase text, with one representation only. Names are attributes. Ids are generated when an object is created, never inside a writer: the generator takes an injected clock and an injected random source, so a test with a fixed clock and a seeded source gets the same ids every time, and writers only receive ids. A ULID is not used.

## Reasons

* Renaming never changes identity, so references (nets, pads, links between domains) stay valid.
* The order of creation is visible in the id, which helps a deterministic listing.
* One standard text form for every file.

## Consequences

* The generator, the clock and the random source are the types of `eh3d-container` (EH3D-COMMON), re-exported by `eh3d-phoenix-model`; the strict parser refuses anything but a canonical lowercase version 7 text.
* A package identity that is derived from content (not the project id, which survives copies) is a version 5 UUID and is made only when needed.

## How it is verified

The tests of the generator with a stepping clock and a deterministic source, the parser tests with non-canonical forms. Identities across saving, migration and reopening are not yet tested on real designs. Review date: with the first migration of the project format.
