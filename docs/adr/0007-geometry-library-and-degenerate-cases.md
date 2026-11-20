# 0007: polygon offsets and boolean operations come from the general library, with a benchmark of degenerate cases still to do

Status: accepted for the choice of the implementation; the benchmark that this record asks for has not been run. The record is pending a review by the project owner (to be confirmed).

## Context

Checks of Gerber files, copper zones, offsets for clearance and the comparison of shapes need boolean operations and offsets of polygons and polylines with arcs. The development plan names the candidates `geo`, `lyon`, `cavalier_contours`, `i_overlay` and `Clipper2`, and asks for the choice to be made on robustness with degenerate input (touching edges, zero-area slivers, collinear points, coincident vertices), performance and licence compatibility with GPL-3.0-or-later.

## Options

* **A published crate** for each operation (several candidates). Less code to own; each has its own handling of degenerate input, its own number type (floating point) and its own release rhythm, and a young crate is a risk that needs its own record.
* **An implementation of our own in the general library of the suite** (`eh3d-geometry` in EH3D-CORE-LIBS: polygon, offset, boolean, arc fitting), with a named tolerance and grid. More code to own; the behaviour on degenerate input is ours to test, and it is shared by every program of the suite.

## Decision

The project uses the implementation of the general library (`eh3d-geometry`), not a published crate. The tolerance and the grid of the geometry are settings (`config/gerber_dfm.cfg` for the checks of Gerber files), never constants buried in the code. A published crate may be adopted later for one operation if the benchmark below shows a real gain, with its own record.

## Consequences

* The design domain stays in integer nanometres (`docs/adr/0003-integer-nanometre-coordinates.md`); the geometry library works in floating point inside a check and returns findings with positions rounded to nanometres.
* The measured width of a sliver is an estimate and is reported as such.
* Until the benchmark exists, the checks say that their figures are not compared with another tool.

## What has to be done before this is closed

* A set of degenerate fixtures (touching and overlapping edges, slivers, collinear and repeated points, tiny arcs, shapes at the limit of the range) run through the operations, with the result checked against a property (the area of the union is not less than that of either part; an offset by zero gives the same region).
* The same fixtures through one or two of the published candidates, with the differences written down.
* Timing on small, medium and large boards.
* The comparison of clearances with the result of another CAM tool on files that the owner provides.

Review date: when the benchmark is done.
