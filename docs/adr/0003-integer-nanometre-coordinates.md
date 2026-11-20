# 0003: design coordinates are integer nanometres inside plus or minus 2^30

Status: accepted. The decision was applied in the code before this record was written; the record is pending a review by the project owner (to be confirmed).

## Context

Orientation tests, clearances and the comparison of two boards must give the same answer on every machine and in every run. Floating-point coordinates do not: two routes to the same point differ in the last bits, and a test of "which side of the line" can change with the order of operations. A board needs a resolution well below a micrometre and a size of a few hundred millimetres.

## Options

* **Floating point millimetres.** Simple to read and write, and what most exchange formats carry. The predicates are not exact, equality is a tolerance, and a saved file is not stable under a round trip.
* **Integer nanometres in a 64-bit integer, no limit but the type.** Exact storage, but the cross product of two differences needs 128 bits near the limit, so every predicate would need wide arithmetic.
* **Integer nanometres inside plus or minus 2^30 (about 1.07 m).** A difference of two coordinates is below 2^31, so the cross product of two differences is below 2^63 and the orientation test is exact in a plain 64-bit integer. Constructions (intersections, offsets) use 128-bit integers with a check for overflow.
* **A limit of 2^31 with predicates in 128 bits.** Allows 2.1 m, which no board needs, and makes every predicate slower.

## Decision

Design coordinates are integers in nanometres, strictly inside plus or minus 2^30; angles are integers in micro-degrees (one turn is 360 000 000); an arc is stored as its end points plus a signed sweep, and its centre is derived. A value outside the range is a typed error (`PHOENIX-UNITS-001` to `PHOENIX-UNITS-006`), never a wrap and never a clamp. Mechanical geometry has its own rule, so that the limit of a board never caps a chassis. Fabrication and machine files use millimetres with the unit in the name; the conversion is tested.

## Reasons

* The orientation predicates are exact by construction, with no tolerance and no special cases.
* A saved board reopens to the same bytes.
* The range is far beyond any board, and a panel of a metre still fits.

## Consequences

* The limits are named constants in `eh3d-phoenix-model` (`COORD_LIMIT_NM`, `ANGLE_LIMIT_UDEG`).
* Importers convert decimals exactly or refuse the value (`PHOENIX-UNITS-004`).
* Geometry that needs floating point (the checks of Gerber files, the simulation of fields) converts at its edge, and the design keeps only integers.

## How it is verified

The tests of `eh3d-phoenix-model` (`range_is_strictly_inside_two_to_the_30`, `arithmetic_is_checked_against_the_range`, the tests of the orientation predicates) and the round-trip tests of the project format. Review date: when the first real board of the owner has been saved and reopened.
